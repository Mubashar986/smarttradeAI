# C2 Session / Conversation API Follow-ups

Status: Deferred design follow-up
Date: 2026-05-03

## Concerns Captured During Repo Walkthrough

- The current `session` resource behaves more like a conversation/thread container than an auth session.
- The current developer flow is two-step:
  1. `POST /sessions` to create a session
  2. `POST /v1/sessions/{id}/turn` to submit the first turn
- That flow is workable for backend testing, but awkward for end-user UX because the user should be able to start by sending the first message directly.
- Session state is currently in memory, so sessions reset on service restart.
- There are two overlapping message-entry routes:
  - `POST /sessions/{id}/message`
  - `POST /v1/sessions/{id}/turn`
- The older `/sessions/{id}/message` route still enqueues async turn processing, but it hides the created `task_id` by returning `204 No Content`.
- The richer `/v1/sessions/{id}/turn` route exposes `task_id` and queued status, which is a better fit for clients that need progress tracking.

## Why This Feels Awkward

- The term `session` is overloaded because JWT/auth session ideas exist elsewhere in the system.
- The user should not need to know a session ID exists.
- The first meaningful action is "send a prompt", but the API currently makes "create a session handle" the visible first step.

## Improvement Options

### Option 1: Frontend-Hidden Session Creation

Keep the backend contract as-is, but let the UI create a session automatically on first input and store the returned ID locally.

Good for:
- fast UI integration
- minimal backend change

Tradeoff:
- backend API still exposes the awkward workflow directly

### Option 2: Single-Call Chat Entry Point

Add a higher-level endpoint such as `POST /v1/chat` that:

1. creates a session if needed
2. submits the first turn
3. returns both `session_id` and `task_id`

Good for:
- cleaner product-facing API
- easier client implementations

Tradeoff:
- introduces overlapping entry points that must be documented clearly

### Option 3: Rename Toward Conversation/Thread

Keep `session` for backward compatibility, but introduce public naming like:

- `conversation_id`
- `thread_id`
- `/v1/conversations`

Good for:
- clearer mental model
- less confusion with auth/session terminology

Tradeoff:
- migration and compatibility work

## Suggested Direction

Near term:
- keep the current low-level endpoints for developer testing
- hide session creation in the frontend

Later:
- add a higher-level single-call chat endpoint
- consider renaming public resources from `session` to `conversation` or `thread`
- consider deprecating `/sessions/{id}/message` in favor of one canonical async turn route

## Future Architecture Follow-ups

### Persistence and Scaling

- Replace in-memory `sessions` and `tasks` maps with persistent storage before multi-instance scaling.
- Prefer Postgres as the source of truth for:
  - sessions / conversations
  - messages
  - tasks
  - task results / status metadata
- Decide whether clarification round tracking should also move out of memory into persistent task/session metadata.
- Revisit per-session turn serialization once multiple server instances exist:
  - distributed lock
  - DB claim/ownership model
  - queue partitioning by session/conversation

### Public ID Strategy

- Replace process-local incrementing public IDs like `session-1` and `task-1` before production exposure.
- Evaluate UUID or ULID for public API identifiers.
- If relational storage is added, consider keeping:
  - internal numeric primary keys
  - external stable public IDs for API use

### Background Processing Model

- Current in-process channel-based worker model is fine for single-process development but not durable.
- Evaluate future task execution options:
  - Postgres-backed job claiming
  - Redis-backed queue
  - dedicated message broker only if operational complexity is justified
- Decide whether SSE/WebSocket events should later be driven from:
  - the application process only
  - shared pub/sub
  - persisted task/event state

### AppState Evolution

- Reduce `AppState` from being the main in-memory source of truth toward being a holder for:
  - repositories / storage clients
  - queue clients
  - shared config
  - coordination services
- Keep the current `AppState` shape only as a development-stage control plane, not the long-term production architecture.

### Runtime File Decomposition

- `runtime/src/smarttrade_tools.rs` is carrying a coherent domain responsibility, but it is too large for one long-term file.
- Consider later decomposition into focused modules such as:
  - `intent.rs`
  - `spec.rs`
  - `ambiguity.rs`
  - `generation.rs`
  - `analysis.rs`
  - `compile.rs`
  - `persistence.rs`
  - `knowledge_base.rs`
- Keep the crate-level public API stable through `runtime/src/lib.rs` re-exports even if the internal files are split.

## Open Questions

- Should one user have many conversations/threads in parallel?
- Should session state become persistent before UX improvements are added?
- Is `session` naming acceptable internally even if the public API changes?
- Should `/sessions/{id}/message` remain as a compatibility wrapper, or be removed after frontend adoption of `/v1/sessions/{id}/turn`?
