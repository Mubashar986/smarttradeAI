# SmartTradeAI — Software Requirements Specification (SRS)

Revision: 2026-06-13
Author: SmartTradeAI Program Lead (generated)

## Table of contents
- 1. Introduction
- 2. Project overview
- 3. System architecture
- 4. Project structure (files)
- 5. Functional requirements (implemented vs remaining)
- 6. Data model
- 7. Interfaces / API
- 8. Non-functional requirements
- 9. Remaining work & recommendations
- 10. Deployment & verification
- 11. Risks & mitigations
- 12. References

---

## 1. Introduction

Purpose: This SRS captures what SmartTradeAI currently implements, how the system is structured, and what remains to reach the June-MVP goals. It is intended for: engineers, product owners, QA, and reviewers.

Scope: The scope covers the C2 orchestration engine that receives user strategy text and produces verified MQL5 Expert Advisor drafts, plus local persistence and developer tooling. Frontend/UI and some design artefacts are out of scope (not implemented) and are listed in Remaining Work.

Audience: Backend engineers, AI engineers, MQL5 specialists, DevOps, QA, and product.

## 2. Project overview

- Mission: Convert natural-language trading strategy requests into safe, verified MQL5 Expert Advisors and store/manage them for review and deployment.
- Primary user goal: Provide a concise natural-language → EA pipeline with verification steps (static analysis and compilation check) and safe defaults.

## 3. System architecture (high level)

- C2 Engine (Rust): HTTP/SSE/WebSocket server, session/turn orchestration, task queue and worker. Entrypoint: [services/c2-engine/rust/crates/c2-engine/src/main.rs](services/c2-engine/rust/crates/c2-engine/src/main.rs)
- Server routing & API: [services/c2-engine/rust/crates/server/src/lib.rs](services/c2-engine/rust/crates/server/src/lib.rs)
- Runtime & AI tools (intent classification, ambiguity detection, generation, static analysis, compile/save tools): [services/c2-engine/rust/crates/runtime/src/lib.rs](services/c2-engine/rust/crates/runtime/src/lib.rs)
- MQL5 skeletons / templates: [services/c2-engine/skeletons/](services/c2-engine/skeletons)
- Persistence plugin (smarttrade-mql5) + DB schema initializer: [services/c2-engine/plugins/smarttrade-mql5/db/init.sql](services/c2-engine/plugins/smarttrade-mql5/db/init.sql)
- System prompt & rules for generation pipeline: [services/c2-engine/CLAW.md](services/c2-engine/CLAW.md)
- Container & runtime infra: [services/c2-engine/Dockerfile](services/c2-engine/Dockerfile) and [services/c2-engine/docker-compose.yml](services/c2-engine/docker-compose.yml)
- Project coordination and program lead skills: `.codex/skills/smarttrade-program-lead/SKILL.md` (coordination rules and read-first docs)

## 4. Project structure (files and purpose)

- `.codex/skills/` — project roles & coordination skills (program lead, system architect, AI/back-end skills). Useful for handoffs and process rules.
- `services/c2-engine/` — main C2 orchestration service (Rust workspace, skeletons, plugins, Docker compose).
  - `services/c2-engine/rust/crates/runtime/` — AI runtime, tools, static-analysis, compile/save logic.
  - `services/c2-engine/rust/crates/server/` — HTTP routes, SSE, WebSocket, session/task management.
  - `services/c2-engine/skeletons/` — EA templates (e.g. `sma_crossover.mqh`).
  - `services/c2-engine/plugins/smarttrade-mql5/` — persistence plugin and DB init.
- `docs/` — diagram assets (draw.io activity/sequence/usecase diagrams).
- `designdocs/` — (this file) planned: ConOps, Interface Control Document, Verification Plan (some referenced design docs are not present under `designdocs/` yet).

## 5. Functional requirements (implemented vs remaining)

The numbering below is a suggested REQ ID that can be mapped into a traceability matrix.

- FR-001: Session lifecycle (create/list/get session) — IMPLEMENTED. See API in server `create_session`, `list_sessions`, `get_session` ([server lib](services/c2-engine/rust/crates/server/src/lib.rs)).
- FR-002: Turn submission and worker pipeline (submit turn → queued task → worker) — IMPLEMENTED. See `enqueue_turn` and worker spawn in `main.rs` and server. Supports multiple message types (intent/clarification/explain).
- FR-003: SSE and WebSocket session event streaming — IMPLEMENTED (`/v1/sessions/{id}/events`, `/v1/ws/{id}`).
- FR-004: Intent classification & ambiguity detection — IMPLEMENTED (`classify_intent`, `detect_ambiguity` in runtime tools).
- FR-005: Knowledge-base search (local skeleton fallback + Pinecone optional) — PARTIAL/IMPLEMENTED (local skeletons present; Pinecone integration supported via env but not necessarily configured).
- FR-006: Generate strategy code and inject into skeleton templates — IMPLEMENTED (`generate_strategy_code`, `inject_skeleton`). Templates available under `skeletons/`.
- FR-007: Static analysis of generated MQL5 — IMPLEMENTED (`run_static_analysis`); automated retry loop supported.
- FR-008: Compile-check integration (MetaEditor/C3) — PARTIAL (compile path implemented via `compile_mql5_with_config`, but requires `C3_COMPILER_URL`. If unset, runtime returns a stubbed compile result — dev-friendly but not a true compile).
- FR-009: Strategy persistence (local file + Postgres optional) — IMPLEMENTED (`persist_strategy_local` and `persist_strategy_postgres`). DB schema in `init.sql`.
- FR-010: Strategy management APIs (list/get/patch/delete) — IMPLEMENTED (`/v1/strategies` endpoints in server).
- FR-011: Dockerized runtime & compose (Redis + Postgres + C2) — IMPLEMENTED (`Dockerfile`, `docker-compose.yml`).
- FR-012: System prompt and generation safety rules — IMPLEMENTED (`CLAW.md` and runtime prompts in `prompt.rs`).

Remaining functional items:
- FR-013: Frontend UI that consumes `/v1` endpoints — NOT IMPLEMENTED (no `package.json` or UI code present).
- FR-014: Full Design Docs (ConOps, ICD, Verification Plan, Traceability Matrix) — MISSING (referenced by `smarttrade-program-lead` but files not present under `designdocs/`).
- FR-015: CI/CD pipeline & automated tests — PARTIAL/NOT IMPLEMENTED (no pipeline config found). Unit/integration tests in Rust workspace not visible or not run by CI here.
- FR-016: Secure secret management / secret rotation — ACTION REQUIRED (secrets exist in `.env`; rotate and move to vault).

## 6. Data model

- Primary persisted entity: `strategies` (see `services/c2-engine/plugins/smarttrade-mql5/db/init.sql`)
  - `id` (serial), `name`, `code` (text), `explanation`, `status` (DRAFT/GENERATED/etc), `session_id`, `user_id`, `pair`, `timeframe`, `created_at`, `updated_at`.

## 7. Interfaces / API (summary)

- Health: `GET /health`, `GET /healthz`, `GET /readyz` — quick service status.
- Sessions: `POST /v1/sessions` (create), `GET /v1/sessions` (list), `GET /v1/sessions/{id}` (details)
- Turn submission: `POST /v1/sessions/{id}/turn` — submit user turn (intent/clarification/explanation). Returns `task_id`.
- Task status: `GET /v1/tasks/{task_id}` — inspect queued/running/completed turn
- Events: `GET /v1/sessions/{id}/events` (SSE) and `GET /v1/ws/{id}` (WebSocket) — stream session events and generated artifacts.
- Strategies: `GET /v1/strategies`, `GET /v1/strategies/{id}`, `PATCH /v1/strategies/{id}`, `DELETE /v1/strategies/{id}`

All endpoints and the middleware are implemented in: [services/c2-engine/rust/crates/server/src/lib.rs](services/c2-engine/rust/crates/server/src/lib.rs)

Auth: Optional JWT middleware enabled when env `C2_JWT_SECRET` is set. If unset, endpoints are open for dev use.

## 8. Non-functional requirements

- Performance: The service is async (Tokio + Axum) and supports concurrent session turns and SSE streaming. Concurrency is bounded by the runtime and the single-worker model used for turn processing.
- Scalability: Horizontal scaling requires externalizing state (session store) or sharding task queues; current design keeps session state in-memory.
- Reliability: Persistence to Postgres is available; local-file fallback exists for quick dev proofing.
- Security: Secrets currently in `.env` should be removed; JWT support available but disabled by default.
- Maintainability: Clear separation of `server`, `runtime`, and `skeletons`; code is organized as a Rust workspace.

## 9. Remaining work & prioritized recommendations

Priority: High
- Add/secure frontend that targets `/v1` contracts (owner: frontend) — builds the user-facing flow. Required for demo.
- Create `designdocs/` artifacts referenced by program lead: `SmartTradeAI_ConOps.md`, `Interface_Control_Document.md`, `Verification_Plan.md`, `Requirements_Traceability_Matrix.md` (owner: program lead / architect). These guide scope and verification.
- Remove/rotate hardcoded provider keys from `.env` and integrate secret management (owner: security/devops).

Priority: Medium
- Enable real compile integration: provide `C3_COMPILER_URL` pointing to a MetaEditor/C3 service (owner: backend + devops). Current runtime supports this hook.
- Add CI pipeline and tests (unit + integration): run `cargo test`, lint, and compile stub flows in CI (owner: QA/DevOps).

Priority: Low
- Add Pinecone or vector DB RAG indexing and knowledge-base ingestion (owner: AI engineer).
- Expand skeleton library and template coverage; add example strategies and meta docs (owner: MQL5 specialist).

## 10. Deployment & verification

To run locally (developer flow):

```bash
cd services/c2-engine
docker compose up --build
```

Verify basics:

1. Check health:

```bash
curl http://localhost:3000/health
```

2. Create a session and submit a turn (example):

```bash
curl -s -X POST http://localhost:3000/v1/sessions | jq
# take session_id from response, then:
curl -s -X POST -H "Content-Type: application/json" \
  -d '{"text":"Create an EA: SMA 50 crosses SMA 200 on EURUSD H1, stop loss 50 pips"}' \
  http://localhost:3000/v1/sessions/session-1/turn | jq
```

3. Watch SSE events:

```bash
curl -N http://localhost:3000/v1/sessions/session-1/events
```

Notes:
- For full compile verification, set `C3_COMPILER_URL` env to a compilation service; otherwise compilation will return a stubbed message indicating compilation is skipped.

## 11. Risks & mitigations

- Risk: Secrets leaked in `.env` — Mitigation: rotate keys, add `.env` to `.gitignore`, use vault.
- Risk: In-memory session state prevents safe multi-instance scaling — Mitigation: externalize session store (Redis) or use sticky session strategy.
- Risk: Automated compile integration could run untrusted code — Mitigation: sandbox compile service, restrict network egress, and run in ephemeral container.

## 12. References

- Engine prompt & safety rules: [services/c2-engine/CLAW.md](services/c2-engine/CLAW.md)
- Server & API: [services/c2-engine/rust/crates/server/src/lib.rs](services/c2-engine/rust/crates/server/src/lib.rs)
- Runtime tools & generation: [services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs](services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs)
- Skeletons: [services/c2-engine/skeletons/](services/c2-engine/skeletons)
- Database schema: [services/c2-engine/plugins/smarttrade-mql5/db/init.sql](services/c2-engine/plugins/smarttrade-mql5/db/init.sql)

---

End of SRS (draft). Ask the program lead to accept, or request edits to the scope/format.
