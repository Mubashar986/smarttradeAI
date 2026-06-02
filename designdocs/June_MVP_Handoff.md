# June MVP Handoff

Status: Active
Date: 2026-06-02
Active branch: initial_mvp

## Current Branch State

Recovery branch:

```text
before-june
```

Active work branch:

```text
initial_mvp
```

Baseline commit:

```text
8547789 Preserve restart baseline before June MVP work
```

## What Has Been Done

- Created `before-june` branch.
- Captured the pre-June dirty code/docs state in a baseline commit.
- Created `initial_mvp` branch from that baseline.
- Preserved `.omx` as untracked runtime/session state.
- Added NASA-lite engineering control documents.

## Current Strategic Decision

SmartTradeAI should be built first as a productized-service engine, not as a
public SaaS.

First service angle:

```text
Strategy intake -> clarification -> template MQL5 draft -> validation evidence
-> plain-English explanation -> human review -> delivery
```

## Build Boundary For June

In scope:

- /v1 backend contract hardening.
- SSE-first realtime workflow.
- Strategy clarification flow.
- Template-based MQL5 generation.
- Basic static validation.
- Plain-English explanation.
- Strategy save/list/fetch.
- Demo-ready internal UI or delivery workflow.

Out of scope:

- Live MT5 execution.
- Profit guarantees.
- Public SaaS billing.
- Full backtesting engine.
- Pine conversion automation.
- Marketplace.
- Advanced scaling.

## Known Technical Gaps

- Docker-based Rust test run has not yet been verified in this session.
- Host cargo is absent, but this is expected because toolchain is Docker-based.
- Compile validation is stubbed unless `C3_COMPILER_URL` is configured.
- Explanation path still needs a useful MVP implementation.
- Sessions/tasks are in memory.
- RAG currently falls back to local skeleton templates.
- Secrets hygiene needs attention before demo/sharing.

## Next Safe Technical Step

Verify the Docker backend baseline:

```powershell
docker compose config
docker compose build c2-engine
docker compose run --rm rust-dev cargo test --workspace
```

Run from:

```text
C:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine
```

If Docker build fails due network or permission, record the exact failure in
this file and continue with source-level verification.

## Evidence Log

Evidence 2026-06-02:

- Command: `git log --oneline --decorate -5`
- Result: `8547789` is HEAD of both `initial_mvp` and `before-june`.
- Interpretation: branch recovery point exists.
- Remaining gap: Docker build/test evidence pending.

Evidence 2026-06-02:

- Command: `docker compose config`
- Result: Compose configuration parsed successfully, but Docker reported a
  local config access warning and expanded a real-looking Gemini key from the
  environment.
- Interpretation: Compose syntax is valid, but secrets hygiene must be fixed
  before demo or sharing.
- Remaining gap: Rotate/replace exposed-looking key and avoid printing composed
  secret-bearing config in public artifacts.

Evidence 2026-06-02:

- Command: `docker compose build c2-engine`
- Result: first attempt failed due Docker config/buildx permission; elevated
  attempt then failed because Docker Desktop Linux engine was not running.
- Interpretation: build is blocked by local Docker environment, not proven to
  be a Rust code failure.
- Remaining gap: start Docker Desktop/Linux engine and rerun image build plus
  Docker-based Rust tests.

## Resume Note For Project Owner

If you return after exams and want your pre-MVP state:

```powershell
git switch before-june
```

If you want to continue MVP work:

```powershell
git switch initial_mvp
```

Do not build new features on `main`.
