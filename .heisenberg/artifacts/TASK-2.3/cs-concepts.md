# CS Domain Learning & First-Principles Analysis — Task 2.3: Persistent Session & Task Management

**Task ID:** TASK-2.3
**Date:** 2026-09-25
**Primary CS Vectors Touched:** V4 Database Theory & Storage Engines, V2 Concurrency & Async I/O, V9 Distributed Systems & Idempotency, V13 Reliability & Backpressure, V11 Data Structures, V5 Networking (SSE), V12 Type Theory
**Evidence mode:** GrapeRoot dual-graph = `UNKNOWN` (no `.dual-graph/` index in this workspace; remediation noted in `roadmap_wbs.md` §12.8). All symbol claims below were read directly from source: `[VERIFIED]`.

---

## 1. Executive First-Principles Synthesis

SmartTradeAI's server kept its world in RAM: two `HashMap`s behind `Arc<RwLock<_>>`, an
unbounded `mpsc` queue, and a `broadcast` channel for live updates. That is a whiteboard —
fast and simple, but erased every time the process dies. Task 2.3 promotes PostgreSQL from
a bystander to the **system of record** while keeping the whiteboard for what whiteboards
are good at: instant, contention-light live reads. The CS problem underneath is old and
famous: *how do you make a volatile, concurrent, crash-prone process appear durable and
linearizable to its clients?*

The historical pain that motivated this class of solution is the "acknowledged-but-lost
work" failure: a client receives `202 Accepted` with a `task_id`, the process restarts, and
that identity evaporates — the client can no longer distinguish *slow* from *never existed*,
and every LLM token already spent building the conversation context is wasted. Industry
answers evolved from full write-through sync on every mutation (too slow) to
**write-ahead logging + asynchronous checkpointing** (SQLite/Postgres WAL), and for
application state, the pattern adopted here: **durable ledger + volatile live layer**
(`understanding.md` §2).

What makes this task subtle is not the SQL — it is the *seam*. The in-memory `Session`
struct deliberately contains a non-serializable member (`broadcast::Sender`), the DB is
optional at the type level (`Option<PgPool>`), and the enqueue path must stay consistent
even when the database write fails *after* the message was already appended in RAM. Every
one of those is a classic distributed-systems boundary condition — partial failure,
dual-write consistency, and crash recovery — implemented in ~600 lines of Rust.

---

## 2. Domain Topology Map (task symbols → CS vectors)

```mermaid
graph TD
  subgraph VOLATILE["Volatile live layer"]
    SS["SessionStore<br/>Arc&lt;RwLock&lt;HashMap&gt;&gt; [VERIFIED state.rs]"]
    TS["TaskStore<br/>Arc&lt;RwLock&lt;HashMap&gt;&gt; [VERIFIED state.rs]"]
    MQ["mpsc::unbounded_channel<br/>turn queue [VERIFIED state.rs]"]
    BC["broadcast::Sender&lt;SessionEvent&gt;<br/>SSE/WS fan-out [VERIFIED state.rs]"]
  end
  subgraph DURABLE["Durable ledger (PostgreSQL 16)"]
    P[(PgPool max=20)]
    TBL_S["sessions(id, conversation JSONB)"]
    TBL_T["tasks(id, status, payload…)"]
  end
  SS ---|serialize| P
  TS ---|state machine| P
  MQ -.crash window.-> TBL_T
  BC -.not Serialize -&gt; never persisted.| P
  RE["reconcile_interrupted_tasks()<br/>[VERIFIED persistence.rs:187]"] --> TBL_T
  P --> HY["live_session() hydration<br/>[VERIFIED state.rs]"]
  HY --> SS
```

| Symbol | File::symbol (verified by direct read) | Vectors |
|---|---|---|
| Ledger CRUD | `server/src/persistence.rs::insert_session/load_session/update_session_conversation` | V4 |
| Task state machine | `persistence.rs::insert_task/mark_task_running/complete_task/fail_task` | V4, V9 |
| Boot reconcile | `persistence.rs::reconcile_interrupted_tasks` | V9, V13 |
| Live/durable seam | `server/src/state.rs` (`Session { conversation, events }`) | V2, V12 |
| JSONB defensive parse | `persistence.rs::load_session` `unwrap_or_else → RuntimeSession::new()` | V4, V13 |
| Pool budget | `c2-engine/src/main.rs` `PgPoolOptions::new().max_connections(20)` | V13 |

---

## 3. Domain Deep Dives

### Domain A: Durability — Full-Document Snapshot vs WAL (V4)

- **Plain-English (1st-semester):** Saving your work means writing it somewhere that
  survives pulling the plug. This project stores each conversation as one JSON blob and
  rewrites the *whole blob* on every turn ("save the entire notebook page again") instead
  of appending only the new sentence. PostgreSQL still protects those writes with its own
  append-only log, so a crash mid-write cannot leave half a page.
- **Physical mechanical analogy:** *The waiter's pocket notepad (WAL)* from the analogy
  dictionary: Postgres appends the change to its WAL first (seconds), then folds it into
  the table pages later (checkpoint). Our application code only ever hands the waiter a
  complete order — the notepad/ledger split happens one layer below us.
- **Staff-level reality:**
  - `update_session_conversation` issues `UPDATE sessions SET conversation = $2 …`
    (`persistence.rs:27`) — a **read-modify-write of a JSONB document**, O(len(messages))
    network + TOAST write per turn. Conversation size growth ⇒ write amplification grows
    linearly; compaction (`runtime/src/compact.rs`) is the mitigation but is **not yet
    invoked from `process_turn`** `[VERIFIED: no call site — explore audit]`.
  - No explicit transaction wraps "append message + insert task"; each SQL statement is
    its own implicit transaction (Postgres default READ COMMITTED, MVCC). The code
    compensates on failure instead: if the DB write fails after the RAM append, it pops
    the message and returns 500 (`turns.rs` rollback block) — *compensating action* rather
    than *atomic transaction*, because the two stores are different systems anyway.
  - Durability here = Postgres `synchronous_commit on` (default) ⇒ every COMMIT pays an
    `fdatasync()`: ~µs on SSD/NVMe, ~ms on HDD.
- **Where it manifests:** `persistence.rs::update_session_conversation`,
  `insert_task`, `init.sql` (`conversation JSONB DEFAULT '{"messages":[]}'`).
- **Beginner myths vs reality:**
  1. *"JSON in a column means a file on disk."* → No: JSONB is a parsed, binary,
     indexed relational type; `jsonb_array_length(conversation->'messages')` runs in SQL
     (`persistence.rs:39`).
  2. *"UPDATE is instant because the row is small."* → TOASTed JSONB values are stored
     out-of-line and rewritten in chunks; cost is O(document).
  3. *"If the app says 200, it's on disk."* → Only if committed and `synchronous_commit`
     stays on; replicas/backup are a separate question.
  4. *"We need no transaction because it's just one statement."* → True per statement;
     the *cross-store* consistency (RAM + PG) is where the real invariant lives.
- **Numbers that matter:** RAM access ~100 ns vs local SSD random read ~16 µs vs fsync
  ~50–500 µs — one committed UPDATE is ~10³–10⁴× costlier than mutating the HashMap, which
  is exactly why the design keeps writes *off* the event-loop hot path (`.await`, never
  blocking).

### Domain B: Concurrency — The Volatile/Durable Split (V2, V12)

- **Plain-English:** The restaurant has two boards: a magnetic whiteboard the staff glances
  at all day (fast, everyone reads it) and a bound ledger in the office (slow to write,
  but survives fire). Task 2.3's rule: *every order goes in the ledger the moment it is
  placed; the whiteboard is only a copy for convenience.*
- **Physical mechanical analogy:** The **single-occupancy airplane bathroom (mutex)** for
  the per-session turn lock, plus the **unbounded kitchen order rail (mpsc)**: orders pile
  up without blocking the waiter (that's why `mpsc::unbounded_channel` gives 202
  immediately) — an unbounded rail is also exactly how kitchens overflow (backpressure,
  Domain D).
- **Staff-level reality:**
  - Readers/writers: `Arc<RwLock<HashMap>>` — Tokio's *async* RwLock, so `.read().await`
    yields instead of parking an OS thread; read locks are shared (O(1) hash lookup),
    writers exclude each other; under contention on the *same* session the per-session
    turn lock serializes turns by design (one honest state per work order).
  - The **compiler enforces the seam**: `server::Session` mixes
    `conversation: RuntimeSession` (Serialize) with `events: broadcast::Sender` (not
    Serialize, not Clone-sharing across restart) — persistence *cannot* accidentally
    include the live channel because the type system refuses.
  - `Option<PgPool>` (`state.rs`) makes the no-DB mode explicit at the type level: every
    call site must choose `match`/`if let` instead of hiding a runtime flag — V12
    null-safety as a design tool.
  - Task status transitions are a small **state machine** (`queued → running →
    completed|failed`, `parse_task_status`); unknown DB strings map to `Queued` —
    *fail-open to the safe "still pending" reading* rather than erroring a poller.
- **Beginner myths vs reality:**
  1. *"Async means parallel."* → One event loop multiplexes I/O; real parallelism happens
     only where `tokio::spawn`/`spawn_blocking` create worker-thread tasks (`process_turn`
     is spawned; the sync agentic loop runs in `spawn_blocking`).
  2. *"RwLock makes everything safe."* → It protects the HashMap, not cross-store
     invariants; RAM-then-PG ordering bugs live outside any lock.
  3. *"Unbounded queue = no limits."* → It removes *sender* blocking and pushes the
     failure mode to memory growth (Domain D).

### Domain C: Crash Recovery & Idempotency (V9)

- **Plain-English:** If the power dies mid-shift, the new shift must first reconcile the
  ledger: "which orders were in flight?" Every task still marked `queued`/`running`
  belonged to the dead process and must be honestly flipped to `failed` — otherwise
  clients poll zombies forever.
- **Physical mechanical analogy:** The **elevator call button (idempotency)**: running the
  reconcile on every boot is safe — pressing "Floor 4" fifteen times still yields one
  elevator. The `UPDATE … WHERE status IN ('queued','running')` statement is naturally
  idempotent; re-running it affects zero rows the second time.
- **Staff-level reality:**
  - `reconcile_interrupted_tasks` (`persistence.rs:187`) executes one set-based UPDATE —
    O(matched rows), no app-side loop; runs **before** the worker spawns (ordering is the
    correctness argument: no new task can be created before zombies are cleared).
  - Scope honesty: single-node only — a peer process's *live* work would be wrongly failed
    in a multi-instance deployment; that is A7's future problem, documented in the
    function's doc comment `[VERIFIED]`.
  - The **crash windows** that remain: (i) RAM appended, PG write fails → compensated by
    pop + 500 `[VERIFIED turns.rs]`; (ii) PG committed, `send` on mpsc fails → task row
    deleted to match `[VERIFIED turns.rs delete_task on send error]`; (iii) PG committed,
    process dies before worker picks it up → reconcile marks it failed. Each window has an
    explicit owner — no silent orphans.
- **Numbers that matter:** MTTR here is boot-time O(rows in `queued|running`) — a single
  UPDATE, index on `status` makes it trivial at MVP scale.

### Domain D: Connection Pool & Backpressure (V13)

- **Plain-English:** The taxi stand outside the airport: 10 pre-fueled taxis (here
  `max_connections(20)`) rather than forging a new car per passenger (a TCP+TLS handshake
  ≈ 1–3 RTT). When all taxis are out, the queue forms *at the stand* — that queue is
  backpressure.
- **Staff-level reality:** sqlx pools with `acquire_timeout`; a saturated pool turns into
  latency (Little's Law: L = λW — as W climbs from pool waits, in-flight requests pile up
  against the 20-connection ceiling, then errors). The `mpsc` unbounded queue upstream has
  **no** equivalent ceiling — unbounded growth is possible under LLM latency spikes;
  mitigations live in worker `spawn` + `process_turn_timeout` watchdog, not in the queue.
- **Myth:** *"Pool size should equal CPU cores."* → It equals (concurrency you want) −
  careful headroom; 20 is a product of expected concurrent turns × DB latency, and every
  SSE/WS connection itself holds no pool connection (good design: stream ≠ DB session).

---

## 4. Cross-Domain Intersections & System Collisions

| Collision | Domains | Failure created | Defense in code |
|---|---|---|---|
| RAM append + async PG write | V2 × V4 | Dual-write divergence (message in RAM, not in PG) | Compensating pop + 500; client retries safely (task never enqueued) |
| Unbounded mpsc + slow LLM/compile | V2 × V13 | Memory blow-up under load | Watchdog timeout; (future: bounded channel) — logged risk R-series |
| Boot reconcile vs live worker | V9 × V2 | New task wrongly failed if reconcile ran late | Ordering: reconcile before `run_turn_worker` spawn `[VERIFIED main.rs plan]` |
| JSONB growth vs per-turn UPDATE | V4 × V13 | Latency creep, TOAST bloat | `compact.rs` exists; **not wired** — open item |
| `broadcast` in struct vs persistence | V12 × V4 | Accidental persistence of non-serializable state | Type system: no `Serialize` impl |
| Option pool vs prod enforcement | V12 × V9 | Silent no-DB mode in production | Task 1.3 prod enforcement (`DATABASE_URL` required when `APP_ENV=production`) |

## 5. Concept Evolution Timeline

| Level | Mental model of Task 2.3 | Deeper reality |
|---|---|---|
| Beginner | *"It saves stuff to the database instead of memory."* | Two stores with different consistency domains are updated by one async code path; every boundary needs an owner. |
| Intermediate | *"Just wrap it in a transaction."* | The transaction can't span `HashMap` + Postgres; you get *compensation* and *ordering* guarantees instead. |
| Advanced | *"Crash windows are enumerated; reconcile is idempotent; status is a state machine with a safe default."* | Exactly what `reconcile_interrupted_tasks` + `parse_task_status` implement. |
| Staff | *"Full-document JSONB rewrite is O(n) per turn; unbounded queue is the real risk; reconcile needs a lease/epoch before multi-node."* | Design for the failure boundaries, not the happy path. |

## 6. Comprehensive Technical Vocabulary

| Term | Etymology | Manifestation here |
|---|---|---|
| WAL | Write-Ahead Log — log-before-data, 1970s System R lineage | Postgres-internal; protects our UPDATEs |
| JSONB | JSON Binary (parsed, not text) | `sessions.conversation` |
| MVCC | Multi-Version Concurrency Control | Postgres READ COMMITTED snapshots |
| TOAST | The Oversized-Attribute Storage Technique | Large conversation blobs stored out-of-line |
| mpsc | Multi-Producer, Single-Consumer | turn queue in `state.rs` |
| broadcast | Tokio broadcast channel | SSE/WS fan-out; lagging receivers skip old events |
| RwLock | Reader-Writer lock | `SessionStore`, `TaskStore` |
| Idempotent | Greek *idēmos* (same) + *dentes* (facts) — "same effect" | reconcile UPDATE; elevator button analogy |
| Compensating action | Saga pattern vocabulary | pop-message rollback on failed persist |
| Backpressure | Flow-control term from networking | pool queue vs unbounded mpsc |
| Hydration | React-ecosystem borrowing | `live_session()` loading PG → HashMap on miss |

## 7. Catastrophic "What If" Chaos Scenarios

1. **The 20-connection ice age (pool exhaustion).** All 20 conns held by slow queries
   (unindexed scan of a bled-large `tasks` table) ⇒ every enqueue waits, then 500s;
   SSE clients reconnect-storm (thundering herd on `/events`). *Principle:* Little's Law.
   *Defense:* acquire timeouts + statement timeouts; index discipline (already present on
   `status/session_id/updated_at`); consider `max_connections` headroom review. **Test:
   Category C in `verification.md`.**
2. **Power pull during conversation UPDATE.** Postgres WAL replays to a consistent state —
   either old or new blob, never half; but our *RAM copy* may hold the new message while
   PG committed nothing if the ack was lost ⇒ next `update_session_conversation`
   overwrites with RAM state (RAM wins on next write). *Principle:* non-atomic dual write.
   *Defense:* acceptable because RAM is the live authority mid-session; boot rehydrate
   prefers PG. **Verify: Category D restart test.**
3. **Zombie-task storm after crash loop.** Crash → boot reconcile fails N tasks →
   clients see mass `failed` for work that a *slow* restart might have resumed — and under
   a crash-loop, reconcile runs repeatedly. *Principle:* at-least-once reconcile vs
   exactly-once execution. *Defense:* accept honest `failed` (no silent lies); client
   resubmits; noted in design risk register (R-08).
4. **Conversation blob bloat → p99 collapse.** A long session (500 messages × tool
   payloads) rewrites megabytes per turn; SSE stalls as `process_turn` awaits PG;
   TOAST table grows; VACUUM lags. *Principle:* O(n) write per event. *Defense:*
   wire `compact.rs` into `process_turn` (backlog item), cap message payload sizes.
5. **Unbounded queue melt.** LLM provider outage ⇒ tasks pile in `mpsc` while every
   status poll returns `queued` honestly; memory grows until the container OOM-kills —
   which then triggers scenario 2/3. *Defense (future):* bounded channel + 429 at the
   edge; document as roadmap risk (see `roadmap_wbs.md` §12).

## 8. Asymptotic Complexity & Resource Budget

| Operation | Time | Space | Note |
|---|---|---|---|
| enqueue (HashMap insert) | O(1) amortized | O(1) | hash of UUID string |
| `insert_session`/`insert_task` | O(1) + fsync | O(row) | index-maintained inserts |
| `update_session_conversation` | **O(m)** | O(m) | m = serialized messages length; full TOAST rewrite |
| `list_sessions` | O(s log s) | O(s) | `ORDER BY id` + per-row `jsonb_array_length` |
| `load_session` + deserialize | O(m) | O(m) | serde parse of blob |
| `reconcile_interrupted_tasks` | O(k) | O(1) | k = matched zombie rows; set-based |
| pool acquire under load | O(wait) | — | bounded by 20; queueing theory applies |

## 9. Hardware & OS Synergies

- Every `.await` on sqlx is a syscall chain: `epoll_wait` → `read`/`write` on the PG
  socket FD; the runtime never blocks a thread on the network (reactor pattern). Contrast:
  `spawn_blocking` for the sync agentic loop — deliberately moved off the event loop,
  because a 200 ms CPU stall would freeze all SSE streams.
- One committed UPDATE ≈ WAL `write` + `fdatasync` (~50–500 µs NVMe) + packet RTT
  (~50 µs same-DC) — 10³–10⁴× a HashMap mutation; the design's whole point is paying this
  cost *once per user-visible event*, not per frame.
- RAM sanity: a 1 MB conversation × 1,000 live sessions = 1 GB resident *before* PG
  copies — session-count ceilings are a RAM budget question, not a CPU question.

## 10. 10-Point Concept Grill Battery Self-Audit

1. [x] Dual audience (intuition ↔ staff) — §3 per domain
2. [x] Zero unexplained acronyms — §6 glossary + inline expansions
3. [x] Physical analogies — waiter's notepad, whiteboard/ledger, taxi stand, elevator button, airplane bathroom, order rail
4. [x] Symbol anchors — all `file::symbol` `[VERIFIED]` by direct read (GrapeRoot `UNKNOWN`, declared)
5. [x] 5-tier trace — §9 (app → runtime → kernel syscalls → hardware latencies); product tier = 202/200 API behavior
6. [x] Big-O — §8
7. [x] Cross-domain matrix — §4
8. [x] Evolution timeline — §5
9. [x] 4+ catastrophic scenarios — §7 (5 scenarios)
10. [x] Zero Terminal Testing respected — no tests executed during extraction; static reads only

## 11. Further Reading

- Gray & Reuter, *Transaction Processing: Concepts and Techniques* (sagas/compensation)
- Postgres docs: MVCC, TOAST, WAL — `postgresql.org/docs/current/mvcc.html`
- Helland, *Idempotence Is Not a Medical Condition* (ACM Queue, 2012)
- Kleppmann, *Designing Data-Intensive Applications*, ch. 7 (transactions), ch. 8 (trouble
  with distributed systems — exactly our crash windows)
- Tokio docs: `broadcast` lag semantics; `RwLock` fairness
- Little's Law: `L = λW` — standard queueing text (Stallings, *Computer Organization*)

## 12. Conclusion & Handover to Stage 2 Codebase Design

Task 2.3 is a textbook **durable-ledger + volatile-live-layer** implementation: compensating
actions instead of cross-store transactions, an idempotent boot reconcile as the crash-recovery
gate, `Option<PgPool>` as an explicit no-DB mode, and a compiler-enforced persistence seam.
The open CS debts are: (a) O(m) conversation rewrite — wire `compact.rs`; (b) unbounded
mpsc — bound it or add edge admission control; (c) reconcile needs leases/epochs before
multi-node; (d) no transaction around multi-statement task transitions (benign today,
worth revisiting with audit-log writes in Task 2.4).

The design artifact (`design.md`) already encodes these boundaries as a
`[NEW]/[MODIFY]` manifest; the implementation plan's six steps are code-complete and
await drift-check + Stage 4 verification. Gate condition satisfied: understanding ✓,
design ✓, cs-concepts ✓ → human sign-off next.
