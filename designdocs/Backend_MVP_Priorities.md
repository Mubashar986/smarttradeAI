# SmartTradeAI Backend MVP Priorities

Status: Working execution split
Date: 2026-05-08
Source: `Backend_MVP_Remaining_Work.md`

## Purpose

This document turns the broader backend backlog into a practical execution split.
The key question is not "what would be nice eventually?" The key question is:

> What backend work must be finished before frontend work can move safely and credibly?

The buckets below are designed to reduce delay and avoid getting trapped in endless backend work.

## Bucket 1 — Must Finish Before Frontend

These are the backend items that should be finished before serious frontend implementation starts.

## 1. Freeze the Canonical API Contract

Why now:
- The frontend must not be built against overlapping or uncertain route shapes.

Must-do:
- Declare `/v1/...` as canonical.
- Decide the primary request flow for the UI:
  - `POST /v1/sessions`
  - `POST /v1/sessions/{id}/turn`
  - `GET /v1/tasks/{task_id}`
  - realtime updates via one chosen transport
- Explicitly mark legacy `/sessions...` routes as compatibility/debug routes only.

Done when:
- Frontend developers have one recommended backend path and no route ambiguity.

## 2. Decide Primary Realtime Transport

Why now:
- The frontend should not implement both SSE and WebSocket unless there is a real reason.

Must-do:
- Pick one primary realtime contract for MVP.
- Recommended default: SSE unless bidirectional realtime behavior is truly required.
- Keep the other transport only as optional/advanced/debug.

Done when:
- Event subscription strategy for the frontend is explicit.

## 3. Harden the Session / Task / Turn Contract

Why now:
- Frontend work needs stable meanings for session, task, clarification, and result payloads.

Must-do:
- Define the canonical lifecycle for a turn:
  - request accepted
  - queued
  - running
  - clarification needed / explanation / generation / failed
- Clarify the contract between:
  - `message_type`
  - detected intent
  - `TaskResultType`
- Ensure result payloads are coherent enough for UI consumption.

Done when:
- A frontend can reliably render turn state without reverse-engineering source code.

## 4. Complete the Explanation Path Enough for MVP

Why now:
- If explanation is exposed in the product but returns a stub, the system feels unfinished.

Must-do:
- Replace the placeholder explanation path with something useful enough for demo/review.
- At minimum, explanation should summarize:
  - extracted strategy intent
  - generated strategy logic
  - high-level risk/trade behavior

Done when:
- Explanation requests return real value, not placeholder text.

## 5. Write the Frontend Integration Contract

Why now:
- This is the bridge between backend stability and frontend implementation.

Must-do:
- Document:
  - create/start chat flow
  - send turn flow
  - clarification loop behavior
  - task polling
  - realtime event names/payloads
  - strategy CRUD contract
- Include example request/response payloads.

Done when:
- A frontend can be implemented without repeatedly opening Rust source files.

## 6. Add Regression Tests for the Canonical Flow

Why now:
- Once frontend starts, backend behavior should stop drifting silently.

Must-do:
- Add tests for the canonical `/v1` path:
  - create session
  - submit turn
  - fetch task
  - clarification result
  - generation result
  - explanation result

Done when:
- Backend contract changes are caught before they break frontend work.

## Bucket 2 — Can Be Done In Parallel With Frontend

These are important, but they do not need to fully block initial frontend execution.

## 7. Persistent Session / Message / Task Storage

Why parallel:
- Very important for credibility, but frontend screens can begin while this is being implemented if the contract stays stable.

Recommended scope:
- Move sessions/messages/tasks into Postgres.
- Persist clarification state as well.

## 8. Durable Background Task Execution

Why parallel:
- A real upgrade, but frontend can still proceed if current single-process behavior is stable enough during development.

Recommended scope:
- Replace in-process-only worker flow with durable job execution.

## 9. Strategy Lifecycle Persistence and Status Model

Why parallel:
- Frontend can start with basic generated/clarifying flows while deeper lifecycle states are being refined.

Recommended scope:
- Formal statuses like:
  - draft
  - clarifying
  - generated
  - validation_failed
  - validated
  - approved
  - active

## 10. Validation Chain Completion

Why parallel:
- The frontend can ship initial UI around generation while validation layers improve beneath it.

Recommended scope:
- tighten static analysis
- add compile-stage result tracking
- expose staged validation states clearly

## 11. Observability / Logging / Auditability

Why parallel:
- Extremely useful, but not a hard prerequisite for first UI implementation.

Recommended scope:
- task lifecycle logging
- provider failure logging
- clarification round logs
- validation stage logs
- correlation IDs

## 12. RAG and Knowledge Base Completion

Why parallel:
- The frontend can still be built while backend generation quality improves.

Recommended scope:
- improve retrieval grounding
- make retrieval usage inspectable
- define fallback behavior

## 13. Public ID Upgrade

Why parallel:
- Important, but can happen behind a stable API contract if done carefully.

Recommended scope:
- move toward UUID/ULID public identifiers

## Bucket 3 — Later / Post-MVP Hardening

These are worthwhile, but should not block the MVP backend + frontend milestone.

## 14. Full File/Module Cleanup

Examples:
- split `server/src/lib.rs`
- split `runtime/src/smarttrade_tools.rs`

Why later:
- improves maintainability
- not required to begin frontend work if tests protect behavior

## 15. Full Multi-Instance Scaling Design

Examples:
- distributed locking
- cross-instance event fanout
- shared pub/sub design

Why later:
- important for scale, not first MVP

## 16. Advanced Provider Failover / Reliability Strategy

Examples:
- multi-provider fallback
- circuit breakers
- more advanced retry/routing logic

Why later:
- strong improvement, not first milestone

## 17. Full Safety Guard / MT5 Productionization

Examples:
- deep C4 integration
- full execution controls
- hardened live trade lifecycle

Why later:
- part of the larger platform, not the immediate frontend-enabling C2 MVP

## Recommended Delivery Boundary

The right stopping line before frontend is:

- one canonical `/v1` contract
- one primary realtime strategy
- stable session/task/result semantics
- explanation path good enough for demo
- frontend integration document
- regression tests for canonical flow

If those are done, the backend is ready enough for frontend work to start safely.

## Recommended Immediate Next Step

Take Bucket 1 and convert it into a short implementation checklist with owners/order:

1. canonical API freeze
2. realtime decision
3. session/task/turn contract hardening
4. explanation path completion
5. frontend integration doc
6. regression tests

That becomes the actual backend execution plan before frontend.
