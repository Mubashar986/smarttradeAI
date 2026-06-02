# Session / Task / Turn Contract Hardening

Status: Pre-implementation review
Date: 2026-05-08

## Change Item

Harden the session / task / turn contract so the frontend can understand turn state, task outcomes, and clarification/generation/explanation flows without reverse-engineering Rust internals.

## Why This Matters Now

- The frontend is about to depend on task and turn semantics heavily.
- The current backend contract is usable, but parts of it are muddy:
  - `message_type` exists, but intent routing mostly depends on server-side classification
  - `TaskResultType` is coarse
  - task payload shapes differ significantly by outcome
  - some outcome labels and state meanings are not explicit enough for a frontend contract

## Current-State Diagnosis

### What exists now

The current model already has:

- `TurnMessageType`
  - `intent`
  - `clarification_response`
  - `explanation_request`
- `TaskStatus`
  - `queued`
  - `running`
  - `completed`
  - `failed`
- `TaskResultType`
  - `clarification`
  - `generation`
  - `explanation`
  - `error`

Tasks also expose:
- `task_id`
- `status`
- `result_type`
- `payload`

### What feels awkward

#### 1. `message_type` is not the main routing truth

Today:
- the client can send `message_type`
- legacy message flow hardcodes `message_type = intent`
- but actual semantic routing is mostly based on `classify_intent(user_message)`

So:
- `message_type` exists
- but detected intent is what really drives behavior

This can confuse frontend and future backend maintainers.

#### 2. `TaskResultType` is useful but still coarse

Right now:
- clarification
- generation
- explanation
- error

This is workable, but it compresses some meaning that the frontend may want more clearly.

Example:
- clarification needed
- clarification exhausted / draft saved
- generated and statically validated
- explanation response

Some of these are distinct product states, but not fully distinct contract states yet.

#### 3. Task payload shape is not uniform

Clarification payload includes:
- missing fields
- next question
- round/max rounds
- spec snapshot

Generation payload includes:
- spec
- classification
- generated code block
- analysis
- ready_for_compile

Explanation payload includes:
- classification
- message

This is understandable internally, but the frontend contract is still too implied.

#### 4. Status and result semantics are split across multiple fields

To understand a turn, a client may need to interpret:
- HTTP response status
- `TaskStatus`
- `TaskResultType`
- `payload.status`
- SSE `status` phase
- event types

That is too much contract knowledge to leave undocumented or loosely defined.

## Questions To Answer Before Implementing

1. Should `message_type` be authoritative, advisory, or only historical metadata?
2. What is the canonical frontend meaning of a turn state?
3. Which frontend-visible states deserve explicit contract names?
4. How much payload normalization is needed now vs later?
5. Can we harden the contract without a large refactor?

## Candidate Options

### Option 1 — Document the current contract only

How it works:
- keep all current code semantics
- explain the meaning in docs only

Why teams sometimes choose it:
- no code churn
- fastest path

Why this may be insufficient:
- some semantics are muddy enough that docs alone may not remove frontend confusion

### Option 2 — Minimal hardening with stable documented semantics

How it works:
- keep current major model types
- explicitly define:
  - what `message_type` means
  - what `TaskResultType` means
  - what `payload.status` values are valid per result type
  - which SSE `status.phase` values are contractually meaningful
- tighten docs/tests around this without a large schema redesign

Why teams commonly use it:
- strong improvement without broad rewrite
- good fit for MVP hardening

Potential downside:
- preserves some internal asymmetry for now

### Option 3 — Strong schema redesign now

How it works:
- redesign task/result payloads into a more explicit unified schema
- possibly rename fields and refactor event contract

Why teams sometimes choose it:
- cleaner long-term architecture

Why risky now:
- bigger change surface
- higher regression risk before frontend starts

### Option 4 — Make `message_type` authoritative

How it works:
- route turns primarily from `message_type`
- classifier becomes secondary support

Why teams sometimes choose it:
- cleaner client/server contract

Why risky now:
- current system logic is already classifier-centric
- likely larger behavioral shift than this MVP item needs

### Option 5 — Hide contract complexity only in the frontend adapter

How it works:
- frontend interprets all current combinations and normalizes them client-side

Why teams sometimes drift into this:
- backend unchanged

Why rejected in principle:
- pushes backend ambiguity into the UI layer

## Current Leaning

Current leaning is:

- choose **Option 2**

Meaning:
- keep the current server model largely intact
- harden and document the semantics
- add tests around the canonical meanings
- postpone deeper schema redesign until after frontend is unblocked

## Regression / Impact Concerns

- changing turn/task semantics can accidentally break existing tests
- changing `message_type` meaning too aggressively can create behavior drift
- frontend docs may become wrong if state meanings are not frozen carefully
- explanation path is still incomplete, so some contract hardening may be partial until item 4 is done

## Intended Implementation Direction

Likely scope:

1. Define a canonical turn lifecycle in docs:
   - accepted
   - queued
   - running
   - clarification_needed
   - generated
   - explanation_returned
   - failed

2. Define `message_type` as:
   - client-supplied request label / hint
   - not the sole semantic routing authority

3. Define frontend-meaningful result semantics for:
   - clarification
   - generation
   - explanation
   - error

4. Freeze a documented interpretation of task payload shape by result type.

5. Add or update regression tests for the canonical meanings.

## Intended Verification

Manual verification should confirm:

- a frontend can understand turn outcome from stable documented semantics
- clarification path remains understandable
- generation path remains understandable
- explanation path semantics are at least consistent, even if content quality is still improved later
- tests/docs align with actual route behavior
