# Traceability Workflow

Use this workflow to keep requirements, design, code, tests, and evidence connected.

## Requirement Handling

1. Search `designdocs/Requirements_Traceability_Matrix.md` for a matching REQ ID.
2. Reuse an existing REQ ID when the behavior already exists in the matrix.
3. Propose a new REQ ID only when the behavior is materially new.
4. Mark status honestly:
   - `Planned`: not implemented or not confirmed.
   - `Partial`: some code exists, but contract or verification is incomplete.
   - `Implemented`: code exists and contract is documented.
   - `Verified`: evidence has been collected.

## Architecture-To-Implementation Handoff

```text
REQ ID:
Design artifact:
Code touchpoint:
Interface touchpoint:
Verification target:
Status before:
Expected status after:
Owner:
```

## Verification Alignment

Each requirement should point to at least one proof type:

- Source inspection for docs, contracts, or small static changes.
- Unit test for pure logic and helpers.
- Integration/API test for backend flows.
- SSE/WebSocket/manual runtime check for realtime behavior.
- Docker build/test output for build claims.
- Manual review for service copy, disclaimers, and portfolio artifacts.

## Common Traceability Mistakes

- Marking `Verified` when only implementation exists.
- Creating a new requirement for a behavior already covered by an existing one.
- Updating code without naming the affected interface.
- Forgetting downstream consumers such as frontend, QA, docs, or client deliverables.
