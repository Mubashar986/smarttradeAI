# API Handoff Contract

Use this reference when backend work affects frontend, AI, QA, security, or docs.

## Contract Fields

```text
Route or event:
Method or transport:
Producer:
Consumer:
Request shape:
Response shape:
Success states:
Failure states:
Persistence behavior:
Authentication or security notes:
Compatibility notes:
Verification evidence:
```

## Canonical Backend Surfaces

- `/v1/sessions` for create/list sessions.
- `/v1/sessions/{id}` for session details.
- `/v1/sessions/{id}/turn` for user turn submission and task creation.
- `/v1/sessions/{id}/events` for primary SSE stream.
- `/v1/ws/{id}` for secondary WebSocket stream.
- `/v1/tasks/{task_id}` for task status/result.
- `/v1/strategies` and `/v1/strategies/{id}` for strategy listing, fetch, update, and soft delete.

Check `designdocs/Interface_Control_Document.md` before changing any route, event, or payload.

## Handoff Consumers

- Frontend needs exact route, payload, loading/error states, and event names.
- AI engineer needs tool invocation expectations and result shape.
- QA needs test setup, expected responses, and known gaps.
- Security/devops needs auth, secret, environment, logging, and deployment implications.
- Program lead needs status, evidence, risks, and next owner.
