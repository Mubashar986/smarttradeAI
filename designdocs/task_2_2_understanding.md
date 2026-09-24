# Task 2.2 — Secure Entity Identifiers: Concept-to-Code Bridge

> Stage: 1 — Conceptual Understanding  
> Scope: sessions, turn tasks, and persisted strategies  
> Artifact status: complete with Mermaid fallback visuals  
> Image generation status: the required built-in raster generation was attempted but returned HTTP 403 in this environment. No generated image is claimed below; the two Mermaid diagrams are the verified visual fallback.

## 1. Visual Architecture

~~~mermaid
graph LR
    C[Client] --> A[Axum API]
    A --> S[AppState]
    S --> G[UUID v4 generator]
    G --> SS[Session ID]
    G --> TT[Task ID]
    G --> ST[Strategy ID]
    SS --> P[(PostgreSQL)]
    TT --> P
    ST --> P
    P --> R[Opaque IDs in JSON responses]
    X[Old: session-1 / task-1 / SERIAL 1] -. enumeration risk .-> W[Attacker can guess neighbors]
    G -->|unpredictable, collision-resistant| R
~~~

The design keeps the external representation as canonical UUID strings. That preserves the existing JSON shape while removing the sequential allocation source.

## 2. Physical Analogy

A sequential identifier is like a coat-check ticket that says 101, 102, 103: a stranger can guess which ticket belongs to the next customer. A UUID v4 is like a randomly generated claim code printed from a very large ticket roll; knowing one code does not reveal the next code. The API still hands the customer a ticket, but the ticket is no longer a map of the entire coat room. PostgreSQL stores the same opaque claim code so a second application instance can create IDs without coordinating a counter.

## 3. Why & What

### Why this task exists

The current AppState ID allocation in [state.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:147) uses two process-local AtomicU64 counters. A fresh process starts again at one, and the values are exposed through session and task endpoints. PostgreSQL strategies currently use SERIAL and the strategy handlers still parse IDs as i64 in [strategies.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/strategies.rs:175).

### What the concept is

Use UUID version 4 values generated at the application boundary with Uuid::new_v4(). UUID v4 provides 122 random bits, making practical collision and enumeration attacks infeasible when IDs are treated as opaque references. The identifier is not an authorization control: later multi-tenant scoping must still check ownership.

### What breaks if we skip it

1. GET /v1/sessions/session-2 is easy to probe after observing session-1; a future ID becomes a discovery oracle.
2. Two replicas can independently allocate session-1 or task-1, creating collisions when state is persisted or shared.
3. Strategy identifiers remain numeric, so a client can iterate /v1/strategies/1, /2, /3; this conflicts with distributed-safe persistence.

## 4. Abstraction Level Map

| Level | What lives here | SmartTradeAI example | Task 2.2 impact |
|---|---|---|---|
| Application | Route handlers and domain state | create_session, enqueue_turn, strategy save | Primary: generate and carry IDs |
| Framework | HTTP extraction and serialization | Axum Path<String>, Serde JSON | Preserve route and JSON shape |
| Library | UUID and SQL clients | uuid::Uuid, sqlx::PgPool | Primary: generation and binding |
| Runtime | Async worker scheduling | Tokio turn worker | Carry opaque IDs unchanged |
| OS/network | Sockets and process memory | HTTP workers and container replicas | Avoid process-local counters |
| Hardware | Randomness source and CPU | OS CSPRNG used by UUID crate | Indirect secure entropy source |

## 5. Mermaid Diagrams

### Request sequence

~~~mermaid
sequenceDiagram
    participant U as Client
    participant X as Axum route
    participant S as AppState
    participant V as UUID v4 generator
    participant W as Turn worker
    participant D as PostgreSQL

    U->>X: POST /v1/sessions
    X->>S: allocate_session_id()
    S->>V: Uuid::new_v4()
    V-->>S: opaque session UUID
    S-->>X: store session in memory
    X-->>U: session_id UUID
    U->>X: POST /v1/sessions/{session_id}/turn
    X->>S: allocate_task_id()
    S->>V: Uuid::new_v4()
    V-->>S: opaque task UUID
    X->>W: TurnRequest(session UUID, task UUID)
    W->>D: persist strategy with UUID, when enabled
    D-->>W: UUID strategy_id
    W-->>U: task and event payloads retain UUID strings
~~~

### Component relationship and decision tree

~~~mermaid
graph TD
    Q[Entity ID request] --> T{Entity type?}
    T -->|Session| SG[Uuid::new_v4]
    T -->|Task| TG[Uuid::new_v4]
    T -->|Strategy| PG[Uuid::new_v4 before INSERT]
    SG --> K[Canonical lowercase UUID string]
    TG --> K
    PG --> K
    K --> J[JSON/API boundary]
    K --> DB[(PostgreSQL UUID-compatible column)]
    DB --> O{Ownership check?}
    O -->|No| E[Not sufficient for authorization]
    O -->|Yes| OK[Opaque ID plus tenant scope]
~~~

## 6. Data Flow Trace-Through

1. A client calls POST /v1/sessions.
2. Axum invokes create_session in [sessions.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/sessions.rs:19).
3. The handler asks [AppState](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/state.rs:157) for a new ID.
4. The allocator obtains UUID v4 entropy instead of incrementing AtomicU64.
5. The session is keyed by that UUID in memory; the same value is used by SSE/WebSocket event payloads.
6. A turn request creates another UUID for task_id, attaches it to TurnContext, and queues a TurnRequest in [turns.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/server/src/routes/turns.rs:65).
7. When generation succeeds, persist_strategy_postgres in [smarttrade_tools.rs](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs:1006) inserts an application-generated strategy UUID instead of receiving a SERIAL integer.
8. The API returns unchanged field names session_id, task_id, and strategy_id with non-sequential values.

## 7. Cognitive Model → Code Variable Mapping

| Cognitive stage | Mental model | Rust / SQL variable | Compiler or database enforcement |
|---|---|---|---|
| Analogy | Print a random claim code | Uuid::new_v4() | UUID crate uses OS-backed randomness |
| Constraints | Never use the ticket as permission | user_id and tenant checks | Query predicates and auth middleware |
| Lifetimes | Carry the same claim code through every desk | SessionId, TaskId, strategy_id | Serde and SQLx bind the exact value |
| Persistence | The ledger stores the claim code | id, session_id, task_id columns | PK/FK constraints and uniqueness |
| Compatibility | Keep the ticket field name | JSON session_id, task_id, strategy_id | Existing clients need no field rename |

## 8. Rust and Stack Context

- uuid is already declared in the workspace [Cargo.toml](C:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/services/c2-engine/rust/Cargo.toml:17) with v4 and serde; server and runtime already depend on it.
- AppState is Clone and shared across Axum handlers. UUID generation is cheap and does not need a shared mutable counter or new lock.
- Uuid can be serialized as a canonical string at the HTTP boundary. The internal aliases can remain string-compatible during the first migration.
- Existing Result-based SQL helpers should return invalid-ID errors rather than silently treating a malformed strategy path as a numeric miss.
- sqlx should bind UUIDs or canonical strings consistently with the chosen schema.

## 9. Five Alternative Approaches

| # | Alternative | Pros | Cons | Decision |
|---|---|---|---|---|
| 1 | UUID v4 generated by Rust, persisted as UUID/text | Simple, decentralized, existing crate, strong anti-enumeration | Requires schema and SQL helper migration | Recommended |
| 2 | UUID v7 generated by Rust | Sortable by creation time | New feature policy; ordering can leak timing | Reject for this focused task |
| 3 | Database gen_random_uuid defaults | Centralizes generation | Extension/default discipline; app and DB behavior diverge | Reject as primary generator |
| 4 | ULID/KSUID | Sortable and URL-friendly | New dependency and wire format | Reject: unnecessary dependency |
| 5 | Hashids over integers | Shorter public IDs | Reversible/guessable; exposes sequence structure | Reject: not cryptographically opaque |

## 10. Production Rationale & Consequences

### Why this is standard

Opaque, independently generated identifiers are a common cloud-native pattern because service replicas should not coordinate through one counter merely to allocate a resource name. UUIDs are supported directly by Rust, PostgreSQL, JSON serialization, and common tracing systems. Stateless processes also favor identifiers that do not depend on process lifetime or local counter state.

### Disaster scenarios if skipped

- A client lists one valid session and walks adjacent IDs, learning the existence and timing of other sessions. UUIDs reduce discovery, but Task 3.2 must still enforce tenant predicates.
- A rolling restart resets both counters. A new task can reuse an old task-1 while a durable worker or client still refers to the previous task, causing status confusion or overwrite.
- A strategy saved by PostgreSQL receives numeric ID 42, while local fallback receives local-UUID. Clients get different identifier semantics depending on deployment mode, and the SQL route rejects the UUID path because it parses only i64.

## Stage 1 Decision

Use application-generated UUID v4 identifiers for sessions, tasks, and strategies; migrate existing persisted rows with a transactional mapping; preserve API field names and JSON string semantics; and keep authorization/tenant checks separate from identifier secrecy.

