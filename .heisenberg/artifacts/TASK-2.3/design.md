# Task 2.3 — Persistent Session & Task Management: Codebase Design

> Stage: 2 — Codebase Design
> Input: [understanding.md](understanding.md)
> Artifact status: design only — no code written
> Scope guard: turn-lock distribution (Task 3.1), user-id query scoping (Task 3.2), multi-table transactions (Task 3.3) are explicitly **out of scope**.

## 1. Current State Snapshot

Sessions and tasks live only in process RAM. The PostgreSQL `sessions` and `tasks` tables exist ([init.sql:13-35](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/plugins/smarttrade-mql5/db/init.sql:13)) and migrations run at boot ([main.rs:44](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs:44)), but no Rust code path reads or writes them. Strategies are the only persisted entity ([smarttrade_tools.rs:982](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs:982)).

~~~mermaid
graph TD
    subgraph "Current Architecture"
        main["main.rs<br/>(pool, migrate, spawn worker)"] --> state["state.rs<br/>AppState"]
        state --> SM[("sessions: HashMap (RAM)")]
        state --> TM[("tasks: HashMap (RAM)")]
        state --> POOL["pool: Option&lt;PgPool&gt;<br/>UNUSED for sessions/tasks"]
        sessions_rs["routes/sessions.rs<br/>create/list/get/SSE/WS"] --> SM
        turns_rs["routes/turns.rs<br/>enqueue/get_task/worker"] --> SM
        turns_rs --> TM
        turns_rs --> CH[["mpsc channel (RAM)"]]
        CH --> WORKER["run_turn_worker /<br/>process_turn"]
        WORKER --> SM
        WORKER --> TM
        POOL -.->|only used by| STRAT["smarttrade_tools.rs<br/>strategy persistence"]
    end
~~~

Key structural facts established by search (Section 4):

- Session-state call sites: `sessions.rs` (create 19, list 38, get 55, SSE 71, WS 110) and `turns.rs` (enqueue 65, snapshot 262, write-back 551, broadcast 689, dead `append_assistant_reply` 648).
- Task-state call sites: `state.rs` (`mark_task_running` 203, `complete_task` 209, `fail_task` 220) and `turns.rs` (insert 83, remove-on-channel-failure 115, `get_task` 133, fail paths 183/221, complete 597).
- All unit/integration tests construct `AppState::default()` or `AppState::new(None)` ([lib.rs:105,124](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/lib.rs:105)) — the `pool = None` in-memory path **must remain byte-identical in behavior**.
- Workspace `sqlx` features are `["runtime-tokio", "postgres", "json"]` — **no `chrono`/`time` codec** ([rust/Cargo.toml:21](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/Cargo.toml:21)). TIMESTAMPTZ columns cannot be decoded into date types without a feature change.

## 2. Proposed State

**Chosen approach: write-through persistence with lazy rehydration** (Alternative 2 from Stage 1, refined). PostgreSQL becomes the durable source of truth for session identity, conversation history, and task lifecycle. The in-memory maps remain as the *live layer* — they hold the non-serializable `broadcast::Sender`, turn locks, and the hot conversation copy for running turns. On any access to a session that is in the DB but not in memory (post-restart), the server *lazily rehydrates* a live in-memory handle from the DB row and continues.

**Failure policy (the core design decision):**
- **Accept-path writes** (`create_session`, `enqueue_turn`): persist to Postgres **first**; if the write fails, return 500 and do **not** mutate memory. The system never accepts work it failed to record.
- **Worker-path writes** (`mark running`, conversation write-back, `complete`/`fail`): update memory **and** attempt the DB write; a DB error is logged with `tracing::error!` but does not kill the turn. Paid LLM work must not be sacrificed to a storage blip; the in-memory copy remains correct for this process's lifetime.
- **`pool = None`**: every persistence call is skipped; behavior is exactly today's.

~~~mermaid
graph TD
    classDef added fill:#a6e3a1,stroke:#a6e3a1,color:#1e1e2e;
    classDef modified fill:#f9e2af,stroke:#f9e2af,color:#1e1e2e;

    subgraph "Proposed Architecture"
        main["main.rs"]:::modified --> RECON["reconcile_interrupted_tasks()<br/>queued/running → failed"]:::added
        main --> state["state.rs<br/>AppState"]
        state --> PERSIST["persistence.rs (NEW)<br/>all session/task SQL + row mapping"]:::added
        PERSIST --> DB[("PostgreSQL<br/>sessions / tasks")]
        state --> SM[("sessions: HashMap<br/>live layer only")]:::modified
        state --> TM[("tasks: HashMap<br/>live layer only")]:::modified
        sessions_rs["routes/sessions.rs"]:::modified --> PERSIST
        sessions_rs -->|lazy rehydrate on miss| SM
        turns_rs["routes/turns.rs"]:::modified --> PERSIST
        WORKER["process_turn"]:::modified --> PERSIST
    end
~~~

## 3. File-Level Impact Analysis

#### [NEW] [persistence.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/persistence.rs)
- **Purpose:** Single module owning every SQL statement for sessions/tasks and every row↔struct conversion. Keeps `state.rs` and route files free of query text.
- **Exports:**
  - `insert_session(pool, id) -> Result<(), sqlx::Error>`
  - `update_session_conversation(pool, id, &RuntimeSession) -> Result<(), sqlx::Error>` (serializes with `serde_json::to_value`, binds to `conversation JSONB`, touches `updated_at`)
  - `list_sessions(pool) -> Result<Vec<SessionSummaryRow>, sqlx::Error>` (id, created_at as epoch millis, message count via `jsonb_array_length`)
  - `load_session(pool, id) -> Result<Option<SessionRow>, sqlx::Error>` (created_at millis + `RuntimeSession` deserialized from JSONB)
  - `insert_task(pool, &TurnTask) -> Result<(), sqlx::Error>`
  - `update_task_running / update_task_completed / update_task_failed(pool, ...)` (status, result_type, payload, error, `updated_at = NOW()`)
  - `load_task(pool, id) -> Result<Option<TaskStatusResponse-shaped row>, sqlx::Error>`
  - `reconcile_interrupted_tasks(pool) -> Result<u64, sqlx::Error>` — one `UPDATE tasks SET status='failed', result_type='error', error='interrupted by server restart', updated_at=NOW() WHERE status IN ('queued','running')`, returning rows affected for the boot log line.
- **Timestamp strategy:** `SELECT (EXTRACT(EPOCH FROM created_at) * 1000)::BIGINT` so `TIMESTAMPTZ → i64` millis decode with the existing `postgres` feature set — **zero Cargo.toml changes** (adding sqlx's `chrono` feature was the rejected alternative; it widens the dependency tree for one conversion SQL can do).
- **Conversation JSONB guard:** `init.sql`'s column default `'{"messages":[]}'` lacks the `version` field that `RuntimeSession` ([runtime/session.rs:44](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/session.rs:44)) requires. Application inserts always write the full struct, but `load_session` must treat deserialize failure as "empty session + `tracing::warn!`" rather than propagating a 500.

#### [MODIFY] [state.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs)
- **What changes:**
  1. `Session` gains a rehydration constructor: `Session::rehydrated(id, created_at_millis, conversation)` — identical to `Session::new` but restores persisted fields and opens a fresh `broadcast::channel(BROADCAST_CAPACITY)`.
  2. New `AppState::live_session(&self, session_id) -> Option<Session>`-style helper: returns the in-memory session, or — when `pool` is `Some` and the DB has the row — loads it, inserts the rehydrated live handle into the map, and returns it. This is the single funnel for lazy rehydration used by `enqueue_turn`, SSE, WS, and the worker snapshot.
  3. `mark_task_running` / `complete_task` / `fail_task` keep their in-memory behavior and additionally fire the corresponding `persistence::update_task_*` call when `pool` is `Some`, logging (not propagating) DB errors per the worker-path failure policy.
- **Why:** Centralizes the volatile/durable seam in the type that already owns both stores.
- **Lines affected:** ~203-236 (task mutators), ~252-261 (Session constructors), plus ~40 new lines.
- **Downstream dependents:** `turns.rs`, `sessions.rs`, `lib.rs` (re-exports unchanged — no public API change).

#### [MODIFY] [routes/sessions.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/sessions.rs)
- **What changes:**
  1. `create_session`: when `pool` is `Some`, `persistence::insert_session` **first**; on DB error return 500 and do not insert into memory (accept-path policy). Then existing in-memory insert unchanged.
  2. `list_sessions`: when `pool` is `Some`, read from DB (source of truth — includes pre-restart sessions); else read the map as today. Response struct unchanged.
  3. `get_session`, `stream_session_events`, `stream_session_websocket`: replace direct map lookup with `state.live_session(id)` so post-restart sessions rehydrate and 404 only when neither store has the ID.
- **Lines affected:** ~19-130 (all five handlers).
- **Downstream dependents:** `lib.rs` route table (unchanged signatures).

#### [MODIFY] [routes/turns.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs)
- **What changes:**
  1. `enqueue_turn`: use `state.live_session(session_id)`; after pushing the message and inserting the in-memory `TurnTask`, when `pool` is `Some` perform `update_session_conversation` + `insert_task`; on DB error **roll back the in-memory mutations** (remove task, pop message) and return 500 (accept-path policy — never accept unrecorded work). Channel-send failure path (turns.rs:113-122) additionally deletes the just-inserted task row so DB and memory stay consistent.
  2. `get_task`: in-memory hit returns as today (freshest during a live turn); on miss with `pool = Some`, fall back to `persistence::load_task` so task status survives restart. 404 only when both miss.
  3. `process_turn`: session snapshot via `state.live_session`; the conversation write-back at turns.rs:551-557 additionally calls `update_session_conversation` (worker-path policy: log on error). Status transitions flow through the modified `mark_task_running`/`complete_task`/`fail_task`, so no new SQL here.
- **Lines affected:** ~65-147 (enqueue + get_task), ~247-268 (snapshot), ~550-557 (write-back), ~104-122 (channel-failure cleanup).
- **Downstream dependents:** `lib.rs` (`run_turn_worker` re-export unchanged).

#### [MODIFY] [main.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs)
- **What changes:** After migrations succeed and **before** `AppState::new` / worker spawn ([main.rs:44-60](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs:44)): call `server::reconcile_interrupted_tasks(&pool)` (re-exported from `persistence`) and log the affected row count. Ordering matters — reconcile must complete before the worker can pick up new turns, and it is idempotent across every boot.
- **Lines affected:** ~48-54 (insert ~5 lines).
- **Downstream dependents:** none (binary entry point).

#### [MODIFY] [lib.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/lib.rs)
- **What changes:** `mod persistence;` declaration + `pub use persistence::reconcile_interrupted_tasks;`. No test changes; no route changes.
- **Lines affected:** ~1-18.

#### Unchanged files (verified non-impacted)
- `runtime/*` — `RuntimeSession` serde shape untouched; `smarttrade_tools.rs` strategy persistence untouched.
- `routes/strategies.rs`, `routes/health.rs`, `middleware/auth.rs`, `llm_bridge.rs`, `mql5_extractor.rs`.
- All `Cargo.toml` files (timestamp-via-SQL decision).
- `init.sql` and migrations — **schema already sufficient; no new migration in this task.**

## 4. Dependency Graph (Blast Radius)

Search commands run (workspace root, `services/c2-engine/rust`): `state.sessions|state.tasks|.sessions.(read|write)|.tasks.(read|write)` and `mark_task_running|complete_task|fail_task|Session::new|AppState::new|AppState::default|next_clarification_round|clear_clarification_rounds`.

~~~mermaid
graph TD
    classDef hot fill:#f38ba8,stroke:#f38ba8,color:#1e1e2e;
    classDef warm fill:#f9e2af,stroke:#f9e2af,color:#1e1e2e;
    classDef cold fill:#a6e3a1,stroke:#a6e3a1,color:#1e1e2e;

    MAIN["c2-engine/main.rs"]:::warm --> LIB["server/lib.rs<br/>app(), re-exports"]:::warm
    LIB --> STATE["state.rs<br/>SessionStore / TaskStore / mutators"]:::hot
    LIB --> SESS["routes/sessions.rs"]:::hot
    LIB --> TURNS["routes/turns.rs"]:::hot
    SESS --> STATE
    TURNS --> STATE
    TURNS --> RT["runtime::ConversationRuntime /<br/>RuntimeSession"]:::cold
    STATE --> RT
    TURNS --> TOOLS["runtime::smarttrade_tools<br/>(strategy pool usage — untouched)"]:::cold
    LIBTESTS["lib.rs test module<br/>8 tests, AppState::new(None)"]:::warm --> LIB
    CONV["runtime/conversation.rs<br/>Session::new ×7 (tests)"]:::cold --> RT
~~~

Blast-radius conclusion: **exactly 2 hot files** (`state.rs`, and the two route files counted as `sessions.rs` + `turns.rs`), 3 warm files (`lib.rs`, `main.rs`, `persistence.rs` as new), and the entire `runtime` crate is cold. The runtime `Session` serde contract is the only cross-crate coupling, and it is read-only for this task.

## 5. Regression Risk Matrix

| Risk ID | Risk description | Severity | Affected feature | Mitigation strategy |
|---|---|---|---|---|
| R-01 | DB write fails on accept path → memory/DB divergence (work accepted but unrecorded) | 🔴 High | Session create, turn enqueue | Persist-first ordering; on DB error roll back in-memory insert and return 500. Client sees honest failure, no zombie work |
| R-02 | `created_at` type mismatch: DB `TIMESTAMPTZ` vs API `u64` millis breaks JSON contract | 🔴 High | All session/task response bodies | `EXTRACT(EPOCH …)*1000)::BIGINT` at SQL boundary; response structs unchanged; existing tests assert shape |
| R-03 | Conversation JSONB missing `version` (column default `{"messages":[]}`) fails `RuntimeSession` deserialize → 500 on rehydrate | 🟡 Medium | Post-restart session reads | `load_session` falls back to `RuntimeSession::new()` + `tracing::warn!`; app inserts always write full struct |
| R-04 | Tests run with `pool = None`; any unconditional DB call panics/fails 8 existing tests | 🔴 High | Entire test suite | Every persistence call gated on `if let Some(pool)`; zero test modifications planned; `cargo test -p server` must pass unchanged |
| R-05 | `get_task` reads DB row that a live in-memory turn has already advanced → stale status flicker | 🟡 Medium | Task polling | Memory-first read order (memory is authoritative while the process lives); DB is fallback only |
| R-06 | Lazy rehydrate races: two concurrent requests for the same missing session both insert into map | 🟡 Medium | SSE/WS/turn after restart | Re-check inside the write lock (double-checked insertion, same pattern as `turn_lock_for` at state.rs:191); last insert wins, both handles share nothing harmful — events route through the map-stored handle |
| R-07 | Channel-send failure cleanup (turns.rs:115) removes memory task but leaves DB row → phantom `queued` task | 🟡 Medium | Enqueue error path | Cleanup path also deletes the DB row when pool present; boot reconcile is the backstop |
| R-08 | Boot reconcile runs while another cluster node is mid-turn → marks live work failed | 🟡 Medium | Future clustering | Out of scope for single-node Task 2.3; documented as the exact problem Task 3.1's advisory locks solve; noted in code comment |
| R-09 | Worker-path DB outage spams `tracing::error!` per transition | 🟢 Low | Log volume | Log once per call site with task/session context; no retry loop |
| R-10 | Dead `append_assistant_reply` (turns.rs:648) diverges further from persisted truth | 🟢 Low | Code hygiene | Leave as-is (already `#[allow(dead_code)]`); removal is optional cleanup, not this task |
| R-11 | `EXTRACT(EPOCH)` rounding: float→BIGINT truncation loses sub-millisecond precision | 🟢 Low | created_at fidelity | API consumers only compare/order; truncation is monotonic per row and contract-compatible |
| R-12 | New module increases compile surface / unused-import warnings | 🟢 Low | Build output | `cargo clippy --workspace --all-targets -- -D warnings` gate in Stage 4 |

## 6. API Contract Stability Check

| Endpoint | Method | Request body | Response shape | Changed? |
|---|---|---|---|---|
| `/v1/sessions` | POST | none | `{ session_id }` | **No** |
| `/v1/sessions` | GET | none | `{ sessions: [{id, created_at(u64 ms), message_count}] }` | **No** (source changes, shape doesn't) |
| `/v1/sessions/{id}` | GET | none | `{ id, created_at, session }` | **No** |
| `/v1/sessions/{id}/turn` | POST | `{ message_type, text, context }` | `{ task_id, status }` | **No** |
| `/v1/sessions/{id}/message` (legacy) | POST | `{ message }` | 204 | **No** |
| `/v1/sessions/{id}/events` | GET (SSE) | none | event stream, snapshot first | **No** |
| `/v1/ws/{id}` | GET (WS) | none | event stream | **No** |
| `/v1/tasks/{task_id}` | GET | none | `{ task_id, status, result_type?, payload }` | **No** |
| `/v1/strategies*` | all | — | — | **No** (untouched) |
| `/health`, `/healthz`, `/readyz` | GET | none | unchanged | **No** |

No breaking changes. One **behavioral improvement** to flag: after a restart, previously-404 responses for pre-restart IDs now return data (sessions/tasks) — additive, not breaking.

## 7. Performance Impact Assessment

| Metric | Before | After | Impact |
|---|---|---|---|
| `POST /v1/sessions` latency | ~0.05 ms (map insert) | +1 INSERT (~1-3 ms local Postgres) | 🟡 Slightly slower, honest durability |
| Turn enqueue latency | map ops only | +1 UPDATE +1 INSERT (~2-5 ms) | 🟡 Acceptable; LLM turn dominates at seconds-scale |
| `GET /v1/sessions` | O(n) map scan | Indexed SELECT (`idx_sessions_updated_at`) | ✅ Scales past process lifetime |
| `GET /v1/tasks/{id}` (live) | map read | map read (memory-first) | ✅ Unchanged hot path |
| Worker per-turn overhead | 0 queries | 3-4 lightweight UPDATEs | 🟡 Negligible vs 8-iteration LLM loop |
| Memory per request | unchanged | unchanged (no new clones beyond existing `conversation.clone()`) | ✅ Neutral |
| Boot time | migrate only | +1 conditional UPDATE (reconcile) | ✅ <10 ms typical |
| DB connection pressure | strategies only | +~6 queries per turn lifecycle | ✅ Well under pool cap of 20 |

## 8. Quality Metrics & Rust Patterns

- **Ownership:** One new clone site — `RuntimeSession` serialized by reference (`serde_json::to_value(&session)` borrows); rehydration moves the deserialized value into the map. No gratuitous clones. The existing `conversation.clone()` at the worker snapshot stays (required by the turn-lock concurrency model).
- **Lifetimes:** No new lifetime annotations; all persistence functions take `&sqlx::PgPool` and owned/borrowed payloads with `'static` futures — compatible with `tokio::spawn` usage in the worker.
- **Error handling:** Accept-path functions return `Result<_, sqlx::Error>` mapped to `ApiError` via the existing `internal_error` helper ([state.rs:501](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:501)); worker-path call sites `match` and `tracing::error!`. **Zero new `unwrap()`/`expect()`** outside tests.
- **Coupling:** Route files depend on `state` + `persistence` only; SQL text is quarantined in one module (single-responsibility). `runtime` crate coupling unchanged. The design *decreases* coupling between HTTP handlers and storage details.
- **Type-driven seam:** `broadcast::Sender` is not `Serialize` — the compiler enforces the volatile/durable split; `Session::rehydrated` constructs a fresh channel explicitly.
- **Dead code:** `append_assistant_reply` remains `#[allow(dead_code)]` (unchanged). `persistence` exports are all consumed by `main.rs` + routes + state — no new dead code expected.
- **Idempotency:** `insert_session`/`insert_task` use plain `INSERT` (UUID v4 PKs from Task 2.2 make collisions a bug signal, not silent overwrite); reconcile `UPDATE` is naturally idempotent.

## 9. Rollback Plan

1. `git revert <task-2.3-commit>` (single commit scope: 5 modified files + 1 new file).
2. Rebuild and restart: `docker compose up c2-engine --build -d`.
3. Verify boot: `Invoke-RestMethod http://localhost:3000/health` and create/poll a session through the legacy in-memory path.
4. **Database rollback: not required.** This task adds no migration and no schema change; orphaned `sessions`/`tasks` rows written by the reverted code are inert data (old code never reads those tables). Optional cleanup: `TRUNCATE tasks; TRUNCATE sessions CASCADE;` in a dev environment only.
5. Estimated rollback time: **~5 minutes** (revert + container rebuild).

---

## Workflow Checklist

- [x] "Before" architecture diagram rendered (Section 1)
- [x] "After" architecture diagram rendered with color-coded changes (Section 2)
- [x] Every affected file listed with clickable links (Section 3)
- [x] Search used to discover all import sites and call sites (Section 4 — two query patterns, results enumerated)
- [x] Regression risks scored 🔴 / 🟡 / 🟢 (12 risks, Section 5)
- [x] API contract stability verified — zero breaking changes (Section 6)
- [x] Performance impact predicted (Section 7)
- [x] Rust ownership/lifetime/error-handling considerations documented (Section 8)
- [x] Rollback plan with git/docker commands and time estimate (Section 9)
- [x] No code written — design only
