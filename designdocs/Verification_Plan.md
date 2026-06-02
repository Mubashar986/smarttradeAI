# SmartTradeAI Verification Plan

Status: Draft
Date: 2026-06-02

## Purpose

This plan defines how SmartTradeAI proves engineering claims before they appear
in handoff notes, demo scripts, or client-facing language.

## Verification Rule

No claim without evidence.

Examples:

- "Route exists" requires source inspection or API test.
- "Build passes" requires Docker/test output.
- "Compiled" requires real MetaEditor/C3 compiler evidence.
- "Validated" must specify which validation level ran.

## Baseline Verification

| Check | Command/Method | Expected Result | Status |
| --- | --- | --- | --- |
| Branch recovery point | git log --decorate | before-june and initial_mvp at baseline commit | Passed |
| Worktree isolation | git status --short --branch | initial_mvp active; only .omx untracked | Passed |
| Docker config | docker compose config | Compose parses without schema errors | Prior partial evidence |
| Docker build | docker compose build c2-engine | Image builds | Blocked: Docker engine not running |
| Rust tests | Docker-based cargo test | TBD | Blocked until Docker engine runs |

## Backend Verification Targets

| ID | Flow | Evidence |
| --- | --- | --- |
| VER-BE-001 | Health endpoint returns ok. | curl/http response |
| VER-BE-002 | Create/list/get /v1 session. | API test |
| VER-BE-003 | Submit turn returns task id. | API test |
| VER-BE-004 | Incomplete strategy emits clarification. | API/SSE test |
| VER-BE-005 | Complete strategy emits generated_code and validation_feedback. | API/SSE test |
| VER-BE-006 | Strategy CRUD works through /v1. | API test |

## Runtime Verification Targets

| ID | Function | Evidence |
| --- | --- | --- |
| VER-RT-001 | classify_intent recognizes strategy creation. | Unit test |
| VER-RT-002 | extract_strategy_spec captures required fields. | Unit test |
| VER-RT-003 | detect_ambiguity asks first missing field. | Unit test |
| VER-RT-004 | generate_strategy_code injects controlled skeleton. | Unit test |
| VER-RT-005 | run_static_analysis catches missing functions/SL issues. | Unit test |
| VER-RT-006 | compile_mql5 reports stub mode clearly when compiler absent. | Unit test |

## Service Verification Targets

| ID | Deliverable | Evidence |
| --- | --- | --- |
| VER-SVC-001 | Sample generated EA has disclaimer comment. | File inspection |
| VER-SVC-002 | Sample explanation states what was and was not validated. | Document inspection |
| VER-SVC-003 | Demo shows no profit claim. | Script review |
| VER-SVC-004 | Offer copy refuses profit guarantees. | Copy review |

## Docker Verification Plan

Preferred path:

```powershell
docker compose config
docker compose build c2-engine
docker compose run --rm rust-dev cargo test --workspace
```

If Docker permissions or network access block image build, record:

- exact command
- exact error
- whether it is environment, network, permission, or code-related

## Evidence Log Format

Use this format in June_MVP_Handoff.md:

```text
Evidence YYYY-MM-DD:
- Command:
- Result:
- Interpretation:
- Remaining gap:
```
