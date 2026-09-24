# Task 2.3 — Persistent Session & Task Management: Concept-to-Code Bridge

> Stage: 1 — Conceptual Understanding
> Scope: sessions, turn tasks, conversation history, boot-time recovery
> Artifact status: complete with Mermaid fallback visuals
> Image generation status: raster generation is unavailable in this environment (same HTTP 403 limitation recorded in `task_2_3`'s predecessor, [task_2_2_understanding.md](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/designdocs/task_2_2_understanding.md:6)). No generated image is claimed below; the Mermaid diagrams in Sections 1, 5, and 6 are the verified visual fallback.

## 1. Visual Architecture

~~~mermaid
graph TD
    subgraph TODAY["Today — volatile whiteboard"]
        C1[Client] --> A1[Axum API]
        A1 --> S1[AppState]
        S1 --> M1[(sessions: HashMap in RAM)]
        S1 --> M2[(tasks: HashMap in RAM)]
        S1 --> Q1[[mpsc channel in RAM]]
        Q1 --> W1[Turn Worker]
        W1 --> M1
        W1 --> M2
        R1[Process restart / container redeploy] -. erases everything .-> M1
        R1 -. erases everything .-> M2
        R1 -. drops queued turns .-> Q1
    end

    subgraph GOAL["Goal — durable ledger with a live desk"]
        C2[Client] --> A2[Axum API]
        A2 --> S2[AppState]
        S2 --> CACHE[(in-memory live layer: broadcast, locks)]
        S2 --> POOL[PgPool]
        POOL --> DB[(PostgreSQL: sessions, tasks)]
        W2[Turn Worker] --> POOL
        BOOT[Boot sequence] -->|rehydrate + reconcile| DB
        R2[Process restart] -. state survives .-> DB
    end
~~~

The database schema for this already exists — Tasks 2.1 and 2.2 built `sessions` (with a `conversation JSONB` column) and `tasks` (with `status`, `result_type`, `message_type`, `context`, `payload`, `error` columns) in [init.sql](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/plugins/smarttrade-mql5/db/init.sql:13). Task 2.3 is the wiring: making the Rust code actually read and write those tables instead of only the in-memory maps.

## 2. The Physical Analogy

Today the server runs like a hotel whose front desk keeps every guest record on a whiteboard in the lobby. Check-ins, room changes, and pending requests are all written in marker. It is fast to read and fast to update — but when the shift changes and the building is cleaned (a process restart, a container redeploy, a crash), the whiteboard is wiped. Every guest must queue up and re-register, and any "please fix my sink" requests shouted to the maintenance runner who happened to be mid-hallway are simply gone. Persistent session and task management is installing a bound ledger behind the desk: the whiteboard is still used for what the on-duty clerk needs at arm's reach (live event broadcasts, turn locks), but every check-in and every work order is also written into the ledger the moment it happens. When a new shift starts, the clerk opens the ledger, sees which guests are checked in and which work orders were never completed, marks the interrupted ones honestly, and continues. PostgreSQL is the ledger; the in-memory `HashMap`s and the `broadcast::Sender` remain the whiteboard — useful for speed, no longer the only copy.

## 3. Why & What

### Why this task exists

Phase 1 gave the server a shared `PgPool` ([state.rs:152](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:152)), and Phase 2.1/2.2 gave PostgreSQL durable, UUID-keyed `sessions` and `tasks` tables. But every route and the turn worker still operate exclusively on process-local state:

- `SessionStore = Arc<RwLock<HashMap<SessionId, Session>>>` — [state.rs:17](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:17)
- `TaskStore = Arc<RwLock<HashMap<TaskId, TurnTask>>>` — [state.rs:19](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:19)
- Turn queue = `mpsc::unbounded_channel()` created per-process — [state.rs:158](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:158)

The only exception is strategies, which already persist through the pool in [smarttrade_tools.rs:982](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs:982). Sessions, tasks, and conversation history vanish on every restart.

### What the concept is

**Durable state with a volatile live layer.** Every mutation that a client could observe later — session creation, a message appended to a conversation, a task changing from `queued` to `running` to `completed`/`failed` — is written to PostgreSQL at the moment it happens. On boot, the server rehydrates: it can answer "list sessions", "get session history", and "get task status" from the ledger even for work that happened before the restart, and it reconciles orphaned work (tasks left in `queued`/`running` when the process died) into an honest terminal state. The in-memory structures are not deleted; they are demoted to what they are genuinely good at: the `broadcast::Sender` for live SSE/WebSocket fan-out ([state.rs:249](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:249)) and per-session turn locks.

### What breaks if we skip it

1. **Restart breaks the task-status contract.** A client submits a turn, receives `202 Accepted` with a `task_id`, and polls `GET /v1/tasks/{task_id}` ([turns.rs:133](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:133)). If the container redeploys mid-turn, the task vanishes and every poll returns 404 forever — the client cannot distinguish "still running" from "never existed".
2. **Conversation context is destroyed.** The LLM turn snapshots the conversation from RAM ([turns.rs:262](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:262)). After a restart the session is empty, so the model "forgets" the strategy being designed, and paid LLM tokens spent building that context are wasted.
3. **Queued turns are silently dropped.** Requests sitting in the unbounded channel when the process exits are lost with no record — the user got a `202 Accepted`, and nothing will ever happen.
4. **Phase 3 clustering is impossible.** Two app instances behind a load balancer cannot share `HashMap`s; without a shared durable store, sticky-session-free horizontal scaling (Tasks 3.1, 5.1, 6.3) has no foundation.

## 4. Abstraction Level Map

| Level | What lives here | SmartTradeAI example | Task 2.3 impact |
|---|---|---|---|
| Application | Route handlers, domain state, worker | `create_session`, `enqueue_turn`, `process_turn`, `AppState` | **Primary:** DB writes on every mutation; rehydration at boot |
| Framework | HTTP extraction, SSE/WebSocket | Axum `Router`, `Sse`, `WebSocketUpgrade` | None — route shapes and event payloads preserved |
| Library | SQL client, serialization | `sqlx::PgPool`, `serde_json` (JSONB round-trip) | **Primary:** query construction and conversation serialization |
| Runtime | Async scheduling, channels | Tokio `mpsc`, `broadcast`, `spawn` | Live layer stays in-memory; boot adds a reconcile pass |
| OS | Process memory lifecycle | Container restart semantics | The reason the task exists — RAM is not durable |
| Hardware | Disk, network storage | PostgreSQL data volume (Docker volume) | Indirect — where durability actually comes from |

Task 2.3 operates at the **Application** and **Library** levels. It deliberately does **not** touch the Framework level: the HTTP API contract (routes, request/response JSON) must remain byte-compatible so no client changes are needed.

## 5. Mermaid Diagrams

### 5a. Sequence — one turn, today vs. after Task 2.3

~~~mermaid
sequenceDiagram
    participant PS as Client (PowerShell)
    participant AX as Axum routes (sessions.rs / turns.rs)
    participant ST as AppState (RAM)
    participant CH as mpsc channel
    participant WK as turn worker (process_turn)
    participant DB as PostgreSQL

    PS->>AX: POST /v1/sessions
    AX->>ST: insert Session into HashMap
    Note over ST,DB: TODAY: nothing written to DB
    AX-->>PS: 201 { session_id }

    PS->>AX: POST /v1/sessions/{id}/turn
    AX->>ST: push message, insert TurnTask(queued)
    AX->>CH: send TurnRequest
    AX-->>PS: 202 { task_id, queued }

    CH->>WK: recv TurnRequest
    WK->>ST: mark_task_running, snapshot conversation
    WK->>WK: LLM turn + compile loop
    WK->>ST: write conversation back, complete_task
    Note over ST,DB: TODAY: restart anywhere above = total loss

    rect rgb(40, 60, 40)
    Note over AX,DB: AFTER TASK 2.3 — ledger writes at each mutation
    AX->>DB: INSERT sessions row (on create)
    AX->>DB: UPDATE sessions.conversation + INSERT tasks row (on enqueue)
    WK->>DB: UPDATE tasks SET status=running (on pickup)
    WK->>DB: UPDATE sessions.conversation + UPDATE tasks SET status=completed/failed (on finish)
    end
~~~

### 5b. Component decision graph — what persists and what stays volatile

~~~mermaid
graph TD
    M{State mutation in server} --> T{Type of state?}
    T -->|Session identity + conversation| D1[(sessions table)]
    T -->|Task lifecycle: queued/running/completed/failed + payload| D2[(tasks table)]
    T -->|Live SSE/WS fan-out: broadcast::Sender| V1[stays in RAM — not serializable, rebuilt on demand]
    T -->|Turn serialization lock per session| V2[stays in RAM — Task 3.1 replaces with pg advisory locks]
    T -->|Clarification round counters| V3[stays in RAM for now — candidate for tasks.context]
    D1 --> B[Boot: rehydrate readable history]
    D2 --> B2[Boot: reconcile orphaned queued/running tasks]
    B2 -->|mark interrupted + error message| D2
~~~

## 6. Data Flow Trace-Through

One complete `POST /v1/sessions/{id}/turn` request, hop by hop, with the persistence points Task 2.3 inserts marked **[NEW]**:

1. PowerShell sends `POST /v1/sessions/{id}/turn` with `{"text": "Explain RSI"}`.
2. Axum extracts `State<AppState>`, `Path<SessionId>`, and `Json<SubmitTurnRequest>` into `send_turn` — [turns.rs:44](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:44).
3. `enqueue_turn` allocates a UUID v4 `task_id` — [turns.rs:70](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:70).
4. The user message is pushed onto `session.conversation.messages` in the in-memory `Session` — [turns.rs:79](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:79). **[NEW: `UPDATE sessions SET conversation = $jsonb, updated_at = NOW() WHERE id = $1`]**
5. A `TurnTask::queued(...)` is inserted into the tasks `HashMap` — [turns.rs:83](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:83). **[NEW: `INSERT INTO tasks (id, session_id, user_id, status, message_type, context, payload) VALUES (...)`]**
6. A `Message` + `Status` event is broadcast to live SSE/WS listeners (stays volatile — the `broadcast::Sender` cannot be serialized).
7. The `TurnRequest` is sent into the mpsc channel; `202 Accepted` returns to the client with the `task_id`.
8. `run_turn_worker` receives the request and spawns `process_turn` — [turns.rs:149](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:149).
9. `process_turn` takes the per-session turn lock, then calls `mark_task_running` — [turns.rs:251](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:251). **[NEW: `UPDATE tasks SET status='running', updated_at=NOW() WHERE id=$1`]**
10. The conversation snapshot drives the LLM turn and the compile/fix loop (hops inside `ConversationRuntime` — unchanged by this task).
11. The updated conversation is written back into the in-memory session — [turns.rs:553](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:553). **[NEW: `UPDATE sessions SET conversation = $jsonb WHERE id = $1`]**
12. `complete_task` (or `fail_task`) flips status in the map — [state.rs:209](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:209). **[NEW: `UPDATE tasks SET status='completed', result_type=..., payload=..., updated_at=NOW() WHERE id=$1`]**
13. **[NEW boot path]** In `main.rs` after migrations run ([main.rs:44](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs:44)) and before serving traffic: reconcile `UPDATE tasks SET status='failed', error='interrupted by server restart' WHERE status IN ('queued','running')`, so no client ever polls a zombie task.

## 7. Cognitive Model → Code Variable Mapping

| Cognitive stage | Mental model | Rust variable / construct | Compiler & runtime enforcement |
|---|---|---|---|
| 1. Analogy | "The whiteboard the clerk reads at arm's reach" | `sessions: SessionStore`, `tasks: TaskStore` ([state.rs:17-19](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:17)) | `Arc<RwLock<HashMap>>` — shared, mutable, fast, volatile |
| 1. Analogy | "The bound ledger that survives the shift change" | `pool: Option<sqlx::PgPool>` ([state.rs:152](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:152)) | `Option` forces an explicit decision when DB is absent (in-memory/test mode, gated by Task 1.3's production enforcement) |
| 2. Constraints | "Every ledger entry is one atomic page write" | `sqlx::query(...).execute(&pool)` per mutation | Postgres row-level atomicity; multi-row writes deferred to Task 3.3 transactions |
| 2. Constraints | "A work order can only be in one honest state" | `TaskStatus` enum ([state.rs:64](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:64)) mirrored by `tasks.status VARCHAR(50)` | Rust enum exhaustiveness in code; `CHECK`-style discipline in schema conventions |
| 3. Lifetimes | "The live PA system belongs to the building, not the ledger" | `events: broadcast::Sender<SessionEvent>` ([state.rs:249](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:249)) | Not `Serialize` — the compiler itself prevents persisting it, enforcing the volatile/durable split |
| 3. Lifetimes | "When the new shift starts, open the ledger first" | Boot rehydrate + reconcile before `axum::serve` | Ordering enforced by `tokio::spawn(run_turn_worker(...))` placement after pool setup ([main.rs:54-60](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/c2-engine/src/main.rs:54)) |

## 8. Language/Stack Context (Rust)

How the pieces map onto our stack:

- **Serialization boundary.** The runtime `Session` ([runtime/session.rs:44](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/session.rs:44)) is `{ version: u32, messages: Vec<ConversationMessage> }` and already round-trips through `serde_json` (proven by `save_to_path`/`load_from_path` and its test). The `sessions.conversation JSONB` column was designed for exactly this payload. `sqlx` can bind `serde_json::Value` directly to `JSONB`.
- **The non-serializable seam.** The server's `Session` struct ([state.rs:245](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:245)) deliberately mixes the durable part (`conversation: RuntimeSession`, `created_at`) with the live part (`events: broadcast::Sender`). The clean seam is: persist `RuntimeSession`; construct a fresh `broadcast::channel(BROADCAST_CAPACITY)` whenever a live handle is needed. The compiler enforces this split because `broadcast::Sender` implements neither `Serialize` nor `Deserialize`.
- **Async discipline.** Route handlers and the worker are `async` on Tokio; `sqlx` queries are `.await`ed. A failed DB write must not panic the request path — it returns `ApiError` (500) or is logged with `tracing::error!` while the in-memory state remains consistent, so a DB blip degrades durability, not availability semantics already encoded in `ApiResult`.
- **Pattern precedent.** The strategies path already implements "pool if present, fallback otherwise" in [smarttrade_tools.rs:982-1028](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs:982) — Task 2.3 generalizes that exact pattern to sessions and tasks inside the server crate.
- **`Option<PgPool>` typing.** `AppState::new(pool: Option<sqlx::PgPool>)` ([state.rs:157](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:157)) means every persistence call site must handle `None` (unit tests construct `AppState::default()` with no pool). The type system makes the fallback explicit rather than a hidden runtime branch.

## 9. Five Alternative Approaches

| # | Alternative | Pros | Cons |
|---|---|---|---|
| 1 | **Database as the only source of truth** — delete the HashMaps; every read/write hits Postgres | Simplest mental model; zero split-brain; clustering-ready | Every SSE snapshot read and message-count list hits the DB; latency on hot paths; broadcast channels still need RAM, so a live layer exists anyway |
| 2 | **Write-through cache** (in-memory maps stay primary for live traffic; every mutation synchronously also written to Postgres; boot rehydrates reads from Postgres) | Keeps today's fast paths and event fan-out; durability at each mutation; minimal API churn; matches the strategies precedent | Two write targets must be kept consistent; failure-handling policy (fail request vs. log-and-continue) must be designed carefully |
| 3 | **Event sourcing** — append every mutation to an `events` table; rebuild state by replay | Perfect audit trail; time-travel debugging; natural fit for `audit_logs` | Large complexity jump: replay logic, snapshots, schema versioning of events; massive overkill before Phase 3 even exists |
| 4 | **File snapshots** — reuse `Session::save_to_path` to write JSON files per session | Almost no new code; human-readable | Not transactional, not queryable, not shareable across cluster nodes; Docker volume coupling; contradicts the Postgres direction of Phase 1-2 |
| 5 | **Redis as primary session store** (AOF/RDB persistence), Postgres only for strategies | Very fast; TTLs built in; good for ephemeral session data | Conversations and tasks are business records, not cache; Redis persistence is weaker than Postgres; splits durable state across two systems; Phase 5 already assigns Redis to pub/sub and rate limiting |

Stage 1 records the trade-space only; the recommended approach and its rationale belong to Stage 2 (design) after the blast-radius analysis.

## 10. Production Rationale & Consequences

### Why this is standard

- **Twelve-Factor App, factor VI (processes are stateless) and factor IV (backing services as attached resources):** any state a client can observe after a restart must live in an attached service — here, PostgreSQL. Disposability ("fast startup, graceful shutdown, robust against sudden death") is impossible while work lives in RAM.
- **Cloud-native redeploy reality:** the project's own Phase 6.3 targets AWS ECS Fargate, where every deployment replaces containers. A task queue that dies with the container is not a queue.
- **Industry precedent:** every production chat/agent platform (ChatGPT/Claude conversation history, Slack, GitHub Actions job records) persists conversation threads and job/task state in a relational or document store and treats app instances as interchangeable. Job systems from Sidekiq/Resque (Redis) to Celery (broker + result backend) to AWS Step Functions all share the invariant: after accepting work, the record of that work must outlive any single worker.

### What happens if we skip this (disaster scenarios)

1. **The redeploy that eats a paid turn.** A user submits "build me an RSI mean-reversion EA." The server returns `202 Accepted`, the LLM turn burns ~15k input tokens, MetaEditor compiles, and at second 480 of the 900-second budget, an ECS rolling deployment kills the container. The channel entry, the task record, and the conversation are gone. The client's poll loop gets 404; the strategy that compiled successfully was saved (strategies persist) but is orphaned from any session or task record the user can see. Money spent, work delivered into a void, user trust destroyed.
2. **The zombie task fleet.** The server crashes with 8 tasks in `running`. Nothing records that they died. On restart, users re-submit identical turns — doubling LLM spend — because the API gives them no way to learn their previous turn failed. Without a boot-time reconcile pass, the system cannot even *honestly say* "that work was interrupted."

---

## Workflow Checklist

- [x] Visual architecture at top of document (Mermaid fallback — raster generation unavailable, see header note)
- [x] Physical analogy included (hotel whiteboard vs. bound ledger)
- [x] Abstraction level table filled in with impact marking
- [x] At least 2 Mermaid diagrams (5a sequence, 5b decision graph, plus Section 1)
- [x] Data flow trace-through with 13 numbered hops and [NEW] persistence points
- [x] Cognitive model → code variable mapping table
- [x] 5 alternatives compared with pros/cons
- [x] 2 disaster scenarios described
- [x] All code references use clickable file links into the workspace
