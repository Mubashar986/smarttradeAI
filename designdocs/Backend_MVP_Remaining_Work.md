# SmartTradeAI Backend MVP Remaining Work

Status: Working draft
Date: 2026-05-08
Audience: project owner, backend contributors, reviewers

## Purpose

This document turns the current repo understanding into a concrete backend work list.
The goal is not "perfect production architecture." The goal is a backend that feels like
a credible, reviewable MVP: stable enough for frontend integration, easy to demo, and
clear enough that reviewers can see where the system is complete vs still evolving.

## Current Backend Reality

What already exists in meaningful form:

- Rust workspace with clear crate roles:
  - `c2-engine` = binary bootstrap
  - `server` = inbound API and orchestration
  - `runtime` = conversation/runtime + SmartTrade processing
  - `api` = outbound provider integration
- Session + task model
- Realtime delivery via SSE and WebSocket
- Intent classification
- Strategy spec extraction
- Ambiguity / clarification flow
- Basic code generation path
- Static analysis path
- Strategy CRUD routes

What is still obviously incomplete or fragile:

- sessions/tasks are in memory
- task execution is in-process and non-durable
- route surface is inconsistent (`/sessions` vs `/v1/...`)
- explanation flow is only a stub
- provider / retrieval / compile / validation chain is not complete enough for a strong demo
- no clean frontend-ready contract yet
- no strong observability / audit / metrics story
- no "production-ish" persistence and lifecycle for session/task state

## MVP Standard We Should Target

For this project, "backend MVP" should mean:

1. A user can start a conversation from the frontend without seeing backend quirks.
2. A strategy request can be submitted, tracked, and resumed safely.
3. Session/task state survives process restart.
4. The backend exposes one canonical API flow for the frontend team.
5. Generated outputs have clear statuses: clarification needed, generated, validation passed/failed, saved.
6. Logs, errors, and task results are understandable enough for demos and reviewer questions.
7. The system has enough validation and safety boundaries that it does not look like a toy chatbot.

## Recommended Backend Workstreams

## 1. Canonical API Cleanup

Priority: High

Why it matters:
- The current API is usable for developer testing, but not clean enough as the long-term frontend contract.
- The frontend should not have to understand route overlap or create sessions manually in a visible way.

Work:
- Treat `/v1/...` as the canonical API surface.
- Keep legacy `/sessions...` routes only as temporary compatibility paths.
- Add or refine a frontend-first entrypoint:
  - either `POST /v1/chat`
  - or keep `POST /v1/sessions` + `POST /v1/sessions/{id}/turn`, but hide session creation in UI
- Add missing canonical inspection endpoints if needed:
  - `GET /v1/sessions`
  - `GET /v1/sessions/{id}`
- Decide whether SSE or WebSocket is the primary realtime contract for the MVP frontend.

Definition of done:
- One documented frontend path exists for "start chat -> send turn -> receive status/result".
- Legacy routes are marked as compatibility/debug routes, not canonical product routes.

## 2. Persistent Session / Message / Task Storage

Priority: High

Why it matters:
- Current in-memory storage is fine for local prototyping but weak for a serious MVP.
- A reviewer will quickly ask what happens on restart or crash.

Work:
- Move sessions, messages, and tasks into PostgreSQL.
- Suggested core tables:
  - `sessions`
  - `messages`
  - `tasks`
  - optional `task_events`
- Persist clarification state instead of keeping it only in memory.
- Keep the server API shape stable while swapping in repository-backed storage.

Definition of done:
- Restarting the backend does not lose active session/task history.
- Task lookup and session fetch work off persistent records.

## 3. Durable Background Task Execution

Priority: High

Why it matters:
- In-process `mpsc` worker flow is a good development step but not durable.
- Long-running generation/validation should survive more than one request loop.

Work:
- Replace or supplement in-process task execution with a durable queue/claim model.
- Best practical MVP path:
  - Postgres-backed job claiming
  - or Redis-backed queue if task fanout grows quickly
- Add clear task lifecycle transitions:
  - queued
  - running
  - completed
  - failed
  - cancelled (recommended)
- Add retry policy for known transient failures.

Definition of done:
- A task remains recoverable even if the web process restarts.
- Failed and retried tasks are visible in task state.

## 4. Public ID Strategy

Priority: Medium-High

Why it matters:
- Incrementing public IDs are okay for local testing, but weak for persistence, multi-instance behavior, and public API stability.

Work:
- Replace public `session-1`, `task-1` style IDs with UUID or ULID.
- If needed, keep internal numeric DB keys separate from external public IDs.

Definition of done:
- Public API identifiers are stable, non-guessable, and restart-safe.

## 5. Clarification / Intent Contract Hardening

Priority: High

Why it matters:
- Right now `message_type` and classifier intent overlap.
- This is manageable today, but muddy for future clients and reviewers.

Work:
- Decide whether `message_type` is:
  - authoritative client intent
  - or only a hint
- Keep detected intent as a separate server-side concept.
- Make the clarification workflow explicit in task payloads and event payloads.
- Ensure frontend can resume clarification loops cleanly.

Definition of done:
- The contract clearly separates:
  - request kind
  - detected intent
  - task result type
- Clarification flows are easy to explain and test.

## 6. Explanation Path Completion

Priority: Medium

Why it matters:
- Explanation exists as an intent path, but the current response is still a stub.
- For demo/reviewer value, plain-English explanation is important.

Work:
- Replace the stub explanation path with a real explanation generator for:
  - generated strategy logic
  - extracted parameters
  - safety/risk implications
- Decide whether explanation is built from:
  - generated code
  - extracted spec
  - or both

Definition of done:
- Explanation requests return a real, useful explanation instead of placeholder text.

## 7. Strategy Lifecycle and Persistence Model

Priority: High

Why it matters:
- Generated artifacts need an explicit lifecycle or the system feels unfinished.

Work:
- Define backend statuses clearly, for example:
  - draft
  - clarifying
  - generated
  - validation_failed
  - validated
  - approved
  - active
- Persist generated code, explanation, extracted spec, analysis output, and strategy metadata.
- Make strategy save/update behavior predictable and documented.

Definition of done:
- Reviewers can see how a strategy moves from request to stored asset.

## 8. RAG and Knowledge Base Completion

Priority: Medium-High

Why it matters:
- The documented architecture expects retrieval-backed generation.
- Without enough retrieval grounding, the system risks looking like a plain prompt wrapper.

Work:
- Finalize knowledge base ingestion path for MQL5 docs/templates.
- Define namespaces and retrieval rules clearly.
- Make retrieval results inspectable in logs or debug payloads.
- Add fallback behavior when retrieval is unavailable.

Definition of done:
- The generation pipeline can explain what knowledge source or template context it used.

## 9. Validation Chain Completion

Priority: High

Why it matters:
- The project’s credibility depends on showing that generation is gated and checked.

Work:
- Keep current static analysis as Gate 1.
- Add or tighten compile integration as Gate 2.
- Clearly record validation stage results in task payloads/events.
- If C3 is not ready yet, still expose a staged validation model in backend status fields.

Definition of done:
- A generated strategy can be shown as:
  - generated only
  - statically validated
  - compile-validated
  - backtest-ready

## 10. Safety Guard Integration Boundary

Priority: Medium

Why it matters:
- Even if the full sentinel is not implemented yet, the backend should show where risk gating belongs.

Work:
- Define the backend contract between generated strategy output and safety validation.
- Add placeholders or interfaces for:
  - stop-loss enforcement
  - position size checks
  - kill switch / disable execution
- Make it explicit that generated code does not auto-authorize live execution.

Definition of done:
- The backend lifecycle clearly separates generation from execution approval.

## 11. Realtime Contract Simplification

Priority: Medium

Why it matters:
- SSE and WebSocket both exist, but frontend MVP should not rely on unnecessary complexity.

Work:
- Choose one primary realtime transport for frontend MVP:
  - SSE if updates are one-way
  - WebSocket only if bidirectional realtime control is actually needed
- Keep the other path as optional/debug/advanced transport if useful.
- Standardize event payloads and event names.

Definition of done:
- Frontend team has one recommended realtime integration path.

## 12. Observability, Logging, and Auditability

Priority: High

Why it matters:
- Reviewers and teammates will ask "what happened?" more often than "what code ran?"

Work:
- Log task lifecycle transitions
- Log provider calls and failures
- Log clarification rounds
- Log retrieved knowledge context
- Log validation stage results
- Add correlation IDs tied to session/task IDs
- Add structured audit records for important workflow events

Definition of done:
- A failed generation can be diagnosed without guessing through raw source code.

## 13. Testing and Contract Protection

Priority: High

Why it matters:
- The backend has enough moving parts now that route cleanup or persistence changes can easily cause regressions.

Work:
- Add regression tests for canonical `/v1` flow
- Add tests for clarification path
- Add tests for explanation path
- Add tests for strategy CRUD lifecycle
- Add tests for persistence-backed task/session behavior
- Add tests around event payload contracts

Definition of done:
- The core backend workflow can be refactored without fear of silent API breakage.

## 14. Backend Documentation for Frontend Integration

Priority: High

Why it matters:
- A frontend can move quickly only if the backend contract is boring and explicit.

Work:
- Write a frontend integration contract doc covering:
  - create/start chat flow
  - submit turn
  - poll task
  - subscribe to realtime updates
  - clarification handling
  - strategy fetch/update/delete
- Include example request/response payloads.

Definition of done:
- Frontend implementation can proceed without repeatedly reading Rust source.

## 15. File and Module Cleanup

Priority: Medium

Why it matters:
- The current code works, but `server/src/lib.rs` and `runtime/src/smarttrade_tools.rs` are carrying too much in one place.

Work:
- Split `server/src/lib.rs` into smaller files by responsibility:
  - state
  - task models
  - routes
  - realtime
  - worker
  - strategy store
- Split `runtime/src/smarttrade_tools.rs` into focused domain modules.
- Preserve public crate surfaces through crate root re-exports.

Definition of done:
- The codebase is easier to review, explain, and safely extend.

## Suggested MVP Build Order From Here

If the goal is "credible backend MVP first, then frontend":

1. Canonical API cleanup
2. Persistent sessions/messages/tasks
3. Durable task execution
4. Frontend integration contract doc
5. Explanation path completion
6. Validation chain completion
7. Observability/logging
8. Tests for the canonical flow
9. Realtime contract simplification
10. File decomposition cleanup

If the goal is "frontend can start now while backend hardens in parallel":

1. Freeze `/v1` as canonical
2. Hide session creation in frontend
3. Write frontend integration doc
4. Build frontend against current backend
5. In parallel, implement persistence and task durability

## Not Required For MVP Right Now

These are valuable, but should not block the first serious demo:

- full multi-provider failover strategy
- perfect horizontal scaling
- advanced chart/image-based strategy input
- multi-tenant admin controls
- complete MT5 production deployment hardening
- final modular refactor of every large file

## Review Questions To Keep Asking

- Can a frontend team use this backend without reading source code?
- Can a session/task survive restart?
- Can we explain task status and failure clearly?
- Can we demonstrate that generation is gated, not blindly trusted?
- Is there one canonical path through the backend, or are we still carrying overlapping flows?
- If a reviewer asks "what happens after code is generated?", do we have a believable answer?

## Bottom Line

The backend already contains meaningful C2 infrastructure, but it still needs work in persistence,
task durability, API cleanup, explanation/validation completion, and observability before it feels
like a strong MVP backend for peer review or frontend-first product development.
