# Backend Verification

Use the strongest available proof for the claim being made.

## Verification Matrix

| Claim | Preferred proof |
| --- | --- |
| Route exists | Source inspection plus API/integration test |
| Request returns task id | API integration test |
| Task status is correct | API integration test with known task |
| SSE emits event | SSE integration/manual runtime evidence |
| Strategy CRUD works | API integration test |
| Runtime helper works | Unit test |
| Docker build passes | `docker compose build c2-engine` output |
| Workspace tests pass | `docker compose run --rm rust-dev cargo test --workspace` output |

## Evidence Format

```text
Evidence YYYY-MM-DD:
- Command or method:
- Result:
- Interpretation:
- Remaining gap:
```

## Docker Blocker Handling

If Docker is blocked, do not stop at "could not test." Record:

- exact command
- exact error
- whether it appears environmental, permission-related, network-related, or code-related
- what source-level verification was still completed

## Common Backend Risks

- Frontend builds against legacy or invented routes.
- Sessions/tasks vanish on restart because state is in memory.
- Stub compile validation is mistaken for real compiler evidence.
- Provider keys leak through env, compose config, or logs.
- Tests pass for helpers while API behavior remains unverified.
