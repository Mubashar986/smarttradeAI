# Realtime Transport Decision

Status: Implemented
Date: 2026-05-08

## Change Item

Decide and document the primary realtime transport for the MVP frontend:

- SSE
- WebSocket
- or both in some defined primary/secondary form

## Why This Matters Now

- The frontend should not be built against two competing realtime contracts without a clear reason.
- The backend already exposes both:
  - `GET /v1/sessions/{id}/events` via SSE
  - `GET /v1/ws/{id}` via WebSocket
- Without a decision, frontend integration stays ambiguous.

## Current-State Diagnosis

What exists now:

- SSE route exists and works for:
  - snapshot
  - message
  - status
  - assistant reply
  - turn completion
- WebSocket route also exists and streams the same session event model.
- Both transports are fed by the same session broadcast event source.

What the product/docs currently imply:

- existing design docs mention WebSocket for frontend updates
- current backend implementation already supports SSE cleanly
- the current MVP flow is mainly one-way status/result delivery from backend to client

What feels awkward:

- two realtime transports exist, but the frontend does not yet have one declared as canonical
- this creates avoidable implementation uncertainty

## Questions To Answer Before Implementing

1. Does the MVP frontend need bidirectional realtime messaging over one persistent connection?
2. Is one-way server-to-client status streaming enough for the current product stage?
3. Which transport is simpler to document, test, and debug for first frontend integration?
4. What future capabilities would force WebSocket to become primary later?
5. Can we keep one transport primary and the other secondary without creating confusion?

## Candidate Options

### Option 1 — SSE as primary, WebSocket as secondary

How it works:
- frontend uses `/v1/sessions/{id}/events` as the default realtime path
- WebSocket remains available for advanced or future use

Why teams choose this:
- simpler browser integration for one-way updates
- lower complexity for MVP
- easier debugging with raw stream visibility

Potential downside:
- if later the UI needs true bidirectional realtime control, frontend may add WebSocket later

### Option 2 — WebSocket as primary, SSE as secondary

How it works:
- frontend uses `/v1/ws/{id}` as the default realtime path
- SSE remains as a fallback/debug path

Why teams choose this:
- one transport can handle richer future interaction
- aligns with docs that already mention WebSocket

Potential downside:
- more complexity now than the MVP may actually need
- current flow is not strongly bidirectional yet

### Option 3 — Keep both equal

How it works:
- no canonical choice
- frontend may use either

Why teams sometimes drift into this:
- avoids making a decision immediately

Why this is risky:
- keeps ambiguity alive
- increases frontend and testing complexity

## Final Decision

Choose:

- **SSE as primary**
- **WebSocket as secondary/optional**

Reason:
- current MVP flow is mostly one-way event delivery
- SSE is enough for:
  - task status
  - clarification prompts
  - generated-code notifications
  - explanation/generation completion
- WebSocket can stay available for future richer interaction without blocking frontend progress now

## Regression / Impact Concerns

- docs currently mention WebSocket in some places; those may need clarification
- if WebSocket is demoted to secondary, the backend route should still remain available
- frontend integration docs must clearly mark one transport as recommended

## Implemented Contract

For MVP frontend work:

- primary realtime contract:
  - `GET /v1/sessions/{id}/events` via SSE
- secondary/optional realtime contract:
  - `GET /v1/ws/{id}` via WebSocket

Implementation policy:

- no route deletion yet
- docs and examples should prefer SSE
- WebSocket remains available for future richer interaction

## Current Event Model

The current shared session event model contains 10 event types:

1. `snapshot`
2. `message`
3. `assistant_reply`
4. `turn_complete`
5. `turn_error`
6. `status`
7. `clarification_question`
8. `validation_feedback`
9. `generated_code`
10. `error`

These events are shared by both realtime transports because both SSE and WebSocket are fed from the same `SessionEvent` enum and session broadcast source.

## How Future Events Should Evolve

Adding future events such as compilation-related progress does **not** require a transport redesign.

In general, adding a new event means:

1. add a new `SessionEvent` variant
2. map it in `event_name()`
3. emit it from the relevant worker/runtime path
4. document it in the frontend contract
5. update UI handling/tests

Because SSE and WebSocket both stream the same session event model, a new event will naturally become available to both transports once added.

### Recommendation for compilation-related updates

There are two sensible ways to model future compile-stage progress:

#### Option A — reuse `validation_feedback`

Use:
- `event: validation_feedback`
- `stage: "compilation"`

Good when:
- compilation is treated as another validation stage
- payload shape can stay aligned with other validation reports

#### Option B — add a dedicated compilation event

Example:
- `compilation_feedback`

Good when:
- compile payloads become structurally different
- frontend needs compile-specific UI treatment

Current recommendation:
- prefer **Option A** first
- add a dedicated compile event only if compile data becomes meaningfully different from existing validation feedback

## Manual Verification

Manual verification should confirm:

- SSE remains healthy for canonical `/v1` flow
- WebSocket route still exists and is functional as secondary
- docs and integration notes point to one primary transport only

## Testing Steps

Run these manually after container rebuild/restart:

1. create a v1 session
2. open the SSE stream:
   - `curl.exe -N http://localhost:3000/v1/sessions/<session-id>/events`
3. submit a turn via:
   - `POST /v1/sessions/{id}/turn`
4. verify SSE receives:
   - `snapshot`
   - `message`
   - `status`
   - completion-related events depending on outcome
5. optionally verify WebSocket route still upgrades and streams session events

Expected result:
- frontend-facing docs and examples treat SSE as canonical
- live async progress remains observable without polling-only UX
