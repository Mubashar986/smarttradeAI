# SmartTradeAI Interface Control Document

Status: Draft
Date: 2026-06-02

## Purpose

This document freezes the system boundaries that frontend, backend, service
delivery, and future integrations must respect.

## Backend API Boundary

Canonical API namespace:

```text
/v1
```

Legacy/debug routes may exist, but new frontend or service tooling should use
only canonical /v1 routes.

## Canonical Routes

| Route | Method | Purpose | Status |
| --- | --- | --- | --- |
| /v1/sessions | POST | Create session | Implemented |
| /v1/sessions | GET | List sessions | Implemented |
| /v1/sessions/{id} | GET | Fetch session details | Implemented |
| /v1/sessions/{id}/turn | POST | Submit user turn | Implemented |
| /v1/sessions/{id}/events | GET | SSE event stream | Implemented |
| /v1/ws/{id} | GET | Secondary WebSocket stream | Implemented |
| /v1/tasks/{task_id} | GET | Fetch task status/result | Implemented |
| /v1/strategies | GET | List strategies | Implemented |
| /v1/strategies/{id} | GET | Fetch strategy | Implemented |
| /v1/strategies/{id} | PATCH | Update strategy | Implemented |
| /v1/strategies/{id} | DELETE | Soft-delete strategy | Implemented |

## Realtime Contract

Primary transport:

```text
GET /v1/sessions/{id}/events
```

Secondary transport:

```text
GET /v1/ws/{id}
```

Known session event names:

| Event | Meaning |
| --- | --- |
| snapshot | Initial session state. |
| message | User message appended. |
| assistant_reply | Assistant message appended. |
| turn_complete | Turn reached terminal success. |
| turn_error | Turn failed. |
| status | Human-readable task phase update. |
| clarification_question | Required strategy details are missing. |
| validation_feedback | Validation stage emitted result. |
| generated_code | Generated MQL5 content available. |
| error | Task-level error. |

## Turn Semantics

Request type:

```json
{
  "text": "Buy EURUSD when RSI crosses above 30...",
  "message_type": "intent",
  "context": {
    "user_id": "optional",
    "strategy_id": "optional"
  }
}
```

`message_type` is a client label/hint. Server-side classification remains the
semantic routing authority for MVP behavior.

## External Provider Boundary

LLM providers are configured through environment variables in Docker.
Do not rely on host toolchain credentials.

Important rule:

- Secrets must not be committed.
- If a real-looking key appears in composed config or logs, rotate it before
  demo or sharing.

## Storage Boundary

Current MVP storage behavior:

- Sessions and tasks: in-memory.
- Strategies: Postgres when DATABASE_URL exists, local fallback otherwise.

Production gap:

- Sessions, messages, task state, and clarification state need durable storage.

## Compiler Boundary

Current compile behavior:

- If C3_COMPILER_URL is unset, compile validation uses stub mode.
- Stub mode is not real MetaEditor compilation.

Client-facing rule:

- Do not say "compiled" unless MetaEditor or a real C3 compiler service has
  actually compiled the generated file.

## Service Delivery Boundary

For June, SmartTradeAI can support a service-led workflow:

1. Intake.
2. Clarify.
3. Generate.
4. Validate basic structure.
5. Human review.
6. Deliver `.mq5` plus explanation/disclaimer.

The system must not imply autonomous live trading readiness.
