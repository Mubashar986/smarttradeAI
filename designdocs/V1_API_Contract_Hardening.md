# V1 API Contract Hardening

Status: Implemented
Date: 2026-05-08

## Change Item

Harden the canonical `/v1` backend API contract so the frontend can rely on a self-sufficient route surface instead of mixing canonical and legacy routes.

## Current-State Diagnosis

Before this change:

- `POST /v1/sessions` existed
- `POST /v1/sessions/{id}/turn` existed
- `GET /v1/tasks/{task_id}` existed
- `GET /v1/sessions/{id}/events` existed
- `GET /v1/ws/{id}` existed

But the canonical surface was incomplete because:

- `GET /v1/sessions` did not exist
- `GET /v1/sessions/{id}` did not exist
- callers had to mix:
  - `/v1/sessions` for creation
  - `/sessions` for list/details

That was awkward for frontend work and easy to misunderstand.

## Options Considered

## Option 1 — Keep the contract as-is

How it works:
- leave `/v1` partially defined
- let frontend mix `/v1` and legacy `/sessions...`

Why teams sometimes do this:
- no code changes
- fast in the short term

Why rejected here:
- keeps contract ambiguity alive
- frontend would need source-level knowledge of route overlap

## Option 2 — Make `/v1` self-sufficient and keep legacy routes for compatibility

How it works:
- add `GET /v1/sessions`
- add `GET /v1/sessions/{id}`
- keep `/sessions...` routes temporarily as compatibility/debug routes

Why teams commonly use it:
- clean canonical contract
- low migration risk
- lets older/debug flows continue while frontend standardizes

Why chosen:
- smallest change that removes frontend ambiguity
- does not force immediate legacy route removal

## Option 3 — Remove legacy routes immediately

How it works:
- add missing `/v1` routes
- delete `/sessions...` routes now

Why teams sometimes choose it:
- strong cleanup boundary
- fewer overlapping APIs

Why rejected here:
- higher breakage risk
- existing tests and manual workflows still use legacy endpoints

## Option 4 — Add a higher-level `POST /v1/chat` first

How it works:
- introduce a new frontend-first route before fixing route completeness

Why teams sometimes choose it:
- smoother UX
- less explicit session handling in clients

Why rejected for this change:
- valuable later, but it does not by itself solve the incomplete `/v1` inspection contract

## Option 5 — Hide route awkwardness only in the frontend

How it works:
- frontend uses a local adapter layer to combine legacy and v1 routes

Why teams sometimes choose it:
- backend unchanged

Why rejected here:
- pushes backend inconsistency into the frontend
- increases client-side accidental complexity

## Chosen Approach

Choose Option 2.

Implementation:
- make `/v1` self-sufficient for:
  - create session
  - list sessions
  - get session
  - submit turn
  - fetch task
  - subscribe to realtime updates
- keep legacy `/sessions...` routes available as compatibility/debug paths
- update tests so the canonical path is protected explicitly

## Realtime Decision for MVP

Primary frontend-facing realtime path:
- `GET /v1/sessions/{id}/events` via SSE

Secondary/optional path:
- `GET /v1/ws/{id}` via WebSocket

Reason:
- the current product flow is primarily one-way status/result delivery
- SSE is the simpler MVP transport

## Regression Risks Considered

- accidental change to JWT protection behavior on `/v1`
- breaking older `/sessions...` debug/manual flows
- route ambiguity remaining in tests or docs
- changing create/list/get semantics unintentionally

## Implemented Changes

In `server/src/lib.rs`:

- added `GET /v1/sessions`
- added `GET /v1/sessions/{id}`
- documented `/v1` as canonical and `/sessions...` as compatibility/debug routes
- updated server tests so canonical `/v1` list/get behavior is covered
- added a compatibility test for legacy session routes

## What This Change Does Not Solve Yet

- session/task persistence
- durable background execution
- explanation-path quality
- frontend integration contract document
- full legacy route deprecation

## Learning Notes

- A canonical API is not only "the routes we prefer." It must be complete enough that clients do not need fallback knowledge.
- For migrations, "canonical plus compatibility" is usually safer than "cleanup by deletion."
- Route hardening is often mostly about reducing ambiguity, not adding brand-new capability.
