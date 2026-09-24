# Task 2.2 — Secure Entity Identifiers: Codebase Design

> Stage: 2 — Codebase Design  
> Design-only artifact: no production code changes are included here.

## 1. Current State Snapshot

The current service has three identifier paths:

1. AppState keeps in-memory sessions and tasks in HashMap<String, ...> stores. Two process-local AtomicU64 counters create session-1 and task-1.
2. PostgreSQL sessions.id and tasks.id are VARCHAR(255), which matches the string API but does not enforce a secure format.
3. PostgreSQL strategies.id is SERIAL; the runtime returns the integer as text, and strategy route handlers parse path IDs as i64.

~~~mermaid
graph TD
    M[main.rs] --> AS[AppState::new]
    AS --> C1[AtomicU64 session counter]
    AS --> C2[AtomicU64 task counter]
    C1 --> SR[sessions HashMap]
    C2 --> TR[tasks HashMap]
    SR --> R1[sessions.rs]
    TR --> R2[turns.rs]
    R2 --> W[turn worker]
    W --> RT[runtime smarttrade_tools.rs]
    RT --> S1[strategies.id SERIAL]
    S1 --> DB[(PostgreSQL)]
    R3[strategies.rs parses i64] --> S1
~~~

Evidence: [state.rs lines 147–195](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:147), [sessions.rs lines 19–30](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/sessions.rs:19), [turns.rs lines 65–95](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:65), [migration lines 14–50](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/plugins/smarttrade-mql5/db/migrations/0001_smarttrade_core.sql:14).

## 2. Proposed State

The first implementation should keep wire fields as strings for compatibility, but every newly created entity receives a canonical UUID v4 string. A new transactional migration backfills existing session, task, strategy, and dependent audit identifiers through temporary old-to-new mapping tables, then adds UUID-format checks. The strategy SQL path stops parsing i64 and binds the opaque identifier directly...............

~~~mermaid
graph TD
    classDef added fill:#a6e3a1,stroke:#40a02b,color:#1e1e2e;
    classDef modified fill:#f9e2af,stroke:#c29d0b,color:#1e1e2e;

    M[main.rs] --> AS[AppState::new]:::modified
    AS --> G[Uuid::new_v4 allocator]:::added
    G --> SR[sessions HashMap UUID strings]:::modified
    G --> TR[tasks HashMap UUID strings]:::modified
    SR --> R1[sessions.rs]:::modified
    TR --> R2[turns.rs]:::modified
    R2 --> W[turn worker]:::modified
    W --> RT[runtime smarttrade_tools.rs]:::modified
    RT --> S1[strategies.id UUID text]:::modified
    R3[strategies.rs direct opaque ID binding]:::modified --> S1
    MIG[0002 secure entity IDs]:::added --> DB[(PostgreSQL)]
    S1 --> DB
~~~

No production changes are needed in main.rs; it already supplies the shared pool to AppState and runs migrations at boot.

## 3. File-Level Impact Analysis

### NEW: 0002_secure_entity_ids.sql

- Purpose: transactionally rotate persisted entity IDs to UUID v4 strings and update all in-database references.
- Exports: no Rust symbols; SQL migration only.
- Design: use temporary mapping tables for sessions, tasks, and strategies; update dependent columns before swapping primary-key values; recreate foreign keys and add UUID-format checks.
- Risk: highest, because it touches primary keys and foreign keys.

### MODIFY: init.sql

- What changes: align first-run Docker bootstrap schema with the secure post-migration schema; remove SERIAL/BIGSERIAL entity keys and use UUID-compatible columns.
- Why: a fresh database must not regress to numeric strategy IDs while an upgraded database uses UUIDs.
- Approximate lines: 4–87.
- Dependents: PostgreSQL bootstrap only; main.rs runs versioned migrations after connecting.

### MODIFY: state.rs

- What changes: remove AtomicU64 imports and fields; allocate canonical UUID v4 strings; add unit coverage for uniqueness and non-sequential shape.
- Why: process-local counters are the direct enumeration and collision source.
- Approximate lines: 1–4, 143–195.
- Dependents: sessions.rs, turns.rs, worker code, response/event types, and server integration tests.

### MODIFY: sessions.rs

- What changes: consume the UUID-backed allocator with no route-shape change; extend tests to assert UUID format and uniqueness.
- Why: session creation is the public source of session identifiers.
- Approximate lines: 19–30 and test helpers.
- Dependents: lib.rs route composition and session API tests.

### MODIFY: turns.rs

- What changes: use UUID task allocation and preserve the value across TurnContext, TurnTask, worker events, and task lookup.
- Why: task IDs are externally observable and currently restart from one.
- Approximate lines: 65–115 plus task-related assertions.
- Dependents: worker code and task status endpoint tests.

### MODIFY: routes/strategies.rs

- What changes: remove parse::<i64>() guards and bind the route ID as the chosen UUID/text representation; preserve the existing user predicate and soft-delete behavior.
- Why: UUID strategy paths currently return a false not-found before SQL is executed.
- Approximate lines: 175–287.
- Dependents: strategy API integration tests and SQL schema.

### MODIFY: runtime smarttrade_tools.rs

- What changes: generate/bind a strategy UUID for PostgreSQL insertion and keep local fallback IDs UUID-based; add focused persistence-ID tests where the database boundary can be exercised.
- Why: runtime persistence is the remaining strategy creation path.
- Approximate lines: 978–1031 and related tests.
- Dependents: server worker and runtime tool tests.

### MODIFY: server/src/lib.rs

- What changes: update hard-coded session/strategy fixtures that assert numeric or session-1 values; add an API regression test that round-trips a UUID path.
- Why: existing integration tests document the old ID shape and must lock the new contract.
- Approximate lines: 155–700, targeted only.
- Dependents: server test suite.

### MODIFY: SmartTradeAI_SRS.md

- What changes: replace stale examples (session-1) and the serial data-model statement with canonical UUID examples.
- Why: operational documentation must not teach clients to rely on enumerable IDs.
- Approximate lines: 82–95, 138–152.
- Dependents: human operators and API consumers.

## 4. Dependency Graph / Blast Radius

Repository search covered AtomicU64, allocate_session_id, allocate_task_id, parse::<i64>(), SERIAL, BIGSERIAL, strategy_id, and all strategies SQL call sites.

~~~mermaid
graph LR
    ID[ID policy] --> ST[state.rs]
    ST --> SS[sessions.rs]
    ST --> TT[turns.rs]
    TT --> WK[worker in lib.rs]
    WK --> RT[smarttrade_tools.rs]
    RT --> MIG[0002 migration]
    MIG --> INIT[init.sql]
    MIG --> SR[strategies.rs SQL assumptions]
    SR --> API[GET/PATCH/DELETE strategy routes]
    ST --> TEST[server integration tests]
    SR --> TEST
    RT --> TEST2[runtime tests]
~~~

### Search evidence

- The only counter fields and allocation functions are in [state.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:147).
- Session allocation is called by [sessions.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/sessions.rs:22).
- Task allocation is called by [turns.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:70).
- Numeric strategy parsing occurs in [strategies.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/strategies.rs:180), :216, and :265.
- Numeric schema assumptions occur in [0001_smarttrade_core.sql](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/plugins/smarttrade-mql5/db/migrations/0001_smarttrade_core.sql:38) and init.sql.

## 5. Regression Risk Matrix

| Risk ID | Risk description | Severity | Affected feature | Mitigation |
|---|---|---|---|---|
| R-01 | Primary-key rotation leaves an orphaned task, audit row, or strategy reference | 🔴 High | Persistence and recovery | Use mapping tables in one transaction; verify row counts and FK checks before commit |
| R-02 | Fresh Docker bootstrap and SQLx migrations create different ID types | 🔴 High | First boot and redeploy | Keep init.sql and final migration aligned; run fresh and upgrade fixtures |
| R-03 | Strategy route still parses UUID as i64 | 🔴 High | Get/Patch/Delete strategy APIs | Remove numeric parsing and add UUID round-trip tests |
| R-04 | A restart or second replica reuses an in-memory ID | 🔴 High | Sessions/tasks | Remove counters entirely; generate UUID v4 per allocation |
| R-05 | API clients depend on session-1 in documentation or tests | 🟡 Medium | Client compatibility | Preserve field names/string serialization; update fixtures/docs |
| R-06 | Local fallback IDs use local-<uuid> while DB IDs are bare UUIDs | 🟡 Medium | Deployment-mode parity | Normalize local and DB strategy ID format, or document and test a deliberate prefix policy |
| R-07 | UUID-to-string SQLx binding mismatch | 🟡 Medium | Runtime persistence | Standardize one column/bind representation and compile/test against PostgreSQL 16 |
| R-08 | Unused atomic imports or stale helpers remain | 🟢 Low | Compiler/lint | cargo fmt, cargo clippy --workspace --all-targets -- -D warnings |

## 6. API Contract Stability Check

| Endpoint | Method | Request change | Response shape | Changed? |
|---|---|---|---|---|
| /v1/sessions | POST | None | { session_id: uuid } | No field change; value becomes opaque |
| /v1/sessions | GET | None | sessions with string id | No |
| /v1/sessions/{id} | GET | Path remains string-compatible | Existing details | No |
| /v1/sessions/{id}/turn | POST | None | task_id plus status | No |
| /v1/tasks/{task_id} | GET | Path remains string-compatible | Existing task status | No |
| /v1/strategies | GET | None | Strategy summaries with string id | No |
| /v1/strategies/{id} | GET/PATCH/DELETE | Path remains string-compatible | Existing strategy response/status | No; numeric-only paths are retired |

This is not an authorization change. The existing user predicate in strategy queries must remain; session/task ownership is a later Task 3.2 requirement.

## 7. Performance Impact Assessment

| Metric | Before | After | Impact |
|---|---|---|---|
| ID allocation | Atomic increment plus formatting | CSPRNG-backed UUID v4 | Small CPU/entropy cost; removes coordination and collision risk |
| In-memory lookup | HashMap<String, ...> | Same key shape, UUID content | Equivalent asymptotic behavior; slightly longer values |
| Strategy insert | DB sequence allocation | App-generated ID plus insert | Removes sequence allocation semantics; same insert count |
| Index size | Numeric strategy PK | 36-character UUID/text PK | Larger index; acceptable for this scope, measure before high-volume workloads |
| Startup migration | No ID rotation | One-time transactional rewrite | Potential lock/time cost proportional to existing rows |

## 8. Rust Quality and Pattern Assessment

- Ownership: UUID values are copied or converted once at the boundary; no shared counter or additional Arc is needed.
- Lifetimes: no new lifetime annotations are expected.
- Error handling: invalid strategy path IDs should return the existing not-found behavior or a clear bad-request error; never unwrap user-controlled paths.
- Coupling: the change decreases coupling to process lifetime but touches SQL schema, runtime persistence, and route assumptions.
- Dead code: AtomicU64 and Ordering should disappear from state.rs; numeric parsing branches in strategies.rs should disappear.
- Security boundary: UUID unpredictability reduces enumeration but does not replace AuthClaims, user filters, or future tenant scoping.

## 9. Rollback Plan

1. Stop the application before rolling back the schema if the migration has already committed.
2. Restore the database from the pre-migration backup, because rotating primary keys is a data migration and git revert alone cannot restore old values.
3. Revert the application commit with git revert <commit-hash> and rebuild the service.
4. If only application code is staged and the migration has not run, git revert is sufficient; do not use destructive worktree commands.

Estimated rollback: 5–15 minutes with a verified PostgreSQL snapshot; longer if the only recovery source is a logical export.

