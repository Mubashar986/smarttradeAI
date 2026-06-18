# Backend Handoff v3 — LLM-Driven Orchestrator (no multi-user)

Date: 2026-06-13 (rev 3 — backtest focus, no multi-user)
From: smarttrade-program-lead
To: smarttrade-backend-engineer (+ smarttrade-ai-engineer for prompt/tool protocol)

## Why this rewrite

Rev 1 was written before `docs/PROJECT_BRIEF.md` existed. Rev 2 added
multi-user auth, which the user has now cut. v1 is single-user,
local, backtest-focused. The orchestrator's job is to drive
LLM -> MQL5 -> validate -> compile -> backtest -> viz -> report -> store
-> audit. Multi-user concerns are out.

This handoff also explicitly **depends on** the new
`docs/handoffs/2026-06-13-backtest-engine.md` and
`docs/handoffs/2026-06-13-reports-viz.md`. Read those first.

## Goal

A user with a valid single-user session can submit a single
`POST /v1/sessions/{id}/turn` and receive a `task_id` whose eventual
`payload.generation` contains an LLM-generated, statically-validated,
(compile-or-stub), backtested-with-rich-metrics, charted, explained,
and stored strategy. The audit trail records every step. The same
natural-language input run twice produces observably different drafts
(proves the LLM is in the loop, not the regex fallback).

## Scope

In:

- Wire `ConversationRuntime` (or `SmartTradeToolExecutor` — pick the
  cleaner one and justify in the handoff back) into
  `process_turn`. The regex functions stay as fallbacks when the
  LLM is unavailable or returns malformed tool calls. They must
  never be the primary path again.
- Implement the tool chain end-to-end on the LLM path:
  `classify_intent` -> `detect_ambiguity` -> clarification round
  -> `select_skeleton_type` -> `inject_skeleton` ->
  `run_static_analysis` (3-retry loop) -> `compile_mql5` (or
  `compile_skipped_stub`) -> **backtest** (see backtest-engine
  handoff) -> **report + charts** (see reports-viz handoff) ->
  `save_strategy` -> explain.
- Language routing: detect whether the user wants MQL5 or Pine
  Script. **v1 generates MQL5 only.** If the user asked for Pine,
  the orchestrator returns a clear "Pine Script generation is on
  the v2 roadmap; v1 produces MQL5" message and does not generate
  code. The audit log records the routing decision.
- Compile result honesty: `compile_mql5` stub returns
  `success: true` with a warning, which is misleading. The
  orchestrator must surface this as `status: compile_skipped_stub`
  SSE event AND set `task.payload.generation.compile_status =
  "STUB_SKIPPED"`. A user inspecting the task must be able to tell
  that no real compile happened.
- Single-user auth gate: a single shared password from
  `C2_SINGLE_USER_PASSWORD` env var gates the UI and the `/v1`
  routes. Login returns a JWT signed with `C2_JWT_SECRET`. If both
  env vars are unset, dev mode is open. The dev-JWT path stays for
  unit tests only. **No `users` table. No `user_id` column. No
  roles.**
- Audit trail: a new `audit_log` table recording
  `(timestamp, session_id, task_id, action, target, status, details)`
  for every state transition. The SRS already has
  `strategy_audit_log` for status transitions; we extend it to
  cover the full pipeline. No `user_id` column — single-user
  instance, timestamps + details are the proof.
- Plain-English explanation: a new `explain` tool that takes the
  generated strategy and produces a 3-5 sentence summary using the
  LLM (not a hardcoded template). Stored in
  `strategies.explanation`. Surfaced in the SSE `explanation`
  event.
- Integration tests in `crates/server/tests/`: at least one test
  that uses a `wiremock` / `httpmock` server to stand in for the
  LLM provider, drives the full chain (LLM -> compile -> backtest
  -> report), and asserts (a) the `saved_strategy_id` is
  non-empty, (b) the audit log contains the expected sequence,
  (c) two runs of the same input produce observably different
  drafts.

Out (forbidden moves):

- No live broker integration. No order placement. No real-money
  routing. The `compile_mql5` hook is the only "external" call.
- No Pine Script generation in v1. Routing detects, redirects to
  MQL5 with a clear message, and does not generate code.
- No multi-user, no roles, no tenant, no user_id, no
  per-user isolation. (See
  `docs/handoffs/2026-06-13-multi-user-auth.md` for the
  decision log and the v2 reference matrix.)
- No Python webhook or signal server. The system stops at `.ex5`
  artifact + backtest result + report. A human copies the `.ex5`
  into MT5 manually.
- No real-time market data. Backtests use bundled or uploaded
  historical data only.
- No external notifications. SSE/WS in-app only.
- No frontend work in this handoff. Backend emits events;
  visualizations and the UI are the reports-viz handoff and a
  separate frontend workstream.
- No designdoc recreation in `designdocs/`.
- No dependency upgrades, no Rust edition bump.

## Read first

- `docs/PROJECT_BRIEF.md` (the source of truth)
- `docs/handoffs/2026-06-13-backtest-engine.md` (the backtest
  engine you must drive)
- `docs/handoffs/2026-06-13-reports-viz.md` (the report + viz
  pipeline you must emit events for)
- `services/c2-engine/CLAW.md` (system prompt + pipeline contract)
- `services/c2-engine/rust/crates/server/src/lib.rs` (esp.
  `process_turn`, `run_turn_worker`, `enqueue_turn`,
  `SessionEvent` variants)
- `services/c2-engine/rust/crates/runtime/src/conversation.rs`
  (the runtime to wire in)
- `services/c2-engine/rust/crates/runtime/src/smarttrade_tools.rs`
  (the `SmartTradeToolExecutor` and the standalone tool functions)
- `services/c2-engine/rust/crates/api/src/lib.rs` + `providers/`
  (LLM client surface; `ClawApiClient` is the multi-provider
  router)
- `services/c2-engine/rust/crates/server/src/auth.rs` (current
  dev-JWT path; needs single-user-password extension)
- `services/c2-engine/.env.example` (provider env vars; do not
  change)
- `services/c2-engine/skeletons/` (5 existing MQL5 templates)
- `services/c2-engine/plugins/smarttrade-mql5/db/init.sql` (DB
  schema; extend, do not break)
- `designdocs/SmartTradeAI_SRS.md` (the only designdoc in scope)
- `.codex/skills/smarttrade-program-lead/references/handoff-contract.md`
  (return-shape template)

## Interfaces affected

- `POST /v1/auth/login` — new. Body: `{password}`. Returns
  `{token, expires_at}`. Argon2-hashed password comparison
  against `C2_SINGLE_USER_PASSWORD_HASH`. JWT signed with
  `C2_JWT_SECRET`.
- `POST /v1/sessions` — same request/response shape. Internally
  stamps the session with the principal.
- `POST /v1/sessions/{id}/turn` — request gains a `target` field
  with values `mql5` | `pine` | `unspecified`. If `unspecified`
  and the user message does not contain a clear target, the
  clarification round asks. If `pine`, the orchestrator returns
  a clear "Pine Script generation is on the v2 roadmap; v1
  produces MQL5" message and does NOT generate code. If `mql5`,
  proceed as today.
- `GET /v1/tasks/{task_id}` — `payload.generation` gains:
  `target` (`mql5` or `pine_unsupported_in_v1`),
  `compile_status` (`COMPILED` | `STUB_SKIPPED` | `FAILED`),
  `compile_artifact_path` (the `.ex5` path or `None`),
  `backtest_result` (PnL, max DD, win rate, Sharpe, Sortino,
  profit factor, recovery factor, avg win/loss, max consec
  wins/losses, avg holding period, exposure time, trade count),
  `equity_curve_points` (array of `{t, equity}` for charting),
  `trade_markers` (array of `{t, side, price}` for the price
  chart),
  `monthly_returns` (array of `{year, month, return_pct}` for
  the heatmap),
  `pnl_distribution` (array of `{bucket, count}` for the
  histogram),
  `parameter_sweep` (optional, populated if the request asked
  for a sweep; see reports-viz handoff),
  `report_pdf_path` (path to the generated PDF, or `None` if
  report generation is deferred to a worker),
  `explanation` (3-5 sentences).
- `GET /v1/audit?session_id=...` — new endpoint. Returns the
  audit log entries for a session, in time order, paginated 50
  per page.
- `GET /v1/strategies/{id}/bundle` — new endpoint. Streams a
  zip bundle: `.ex5` + `.mq5` + PDF report + audit JSON.
- No DB schema break. `audit_log` is a new table. `strategies`
  gains `target`, `compile_status`, `compile_artifact_path`,
  `backtest_metrics` (JSONB), `equity_curve_points` (JSONB),
  `trade_markers` (JSONB), `monthly_returns` (JSONB),
  `pnl_distribution` (JSONB), `parameter_sweep` (JSONB),
  `report_pdf_path`, `explanation` (already exists).

## Acceptance criteria

1. With a real provider key set, a single `POST /v1/sessions` +
   `POST /v1/sessions/{id}/turn` with a complete MQL5 spec
   produces:
   - `task_id` returned 202.
   - SSE stream shows at least: `message`,
     `status: classified`, `status: target_routed`,
     `status: generating`, `generated_code`,
     `validation_feedback: static_analysis`,
     `status: compile_succeeded` (or `compile_skipped_stub`),
     `status: backtest_complete`, `status: charts_ready`,
     `status: report_ready`, `status: explained`,
     `status: saved`, `turn_complete`.
   - `GET /v1/tasks/{task_id}` returns all the v1 metrics in
     `payload.generation.backtest_result` and all the chart
     data in the corresponding fields.
2. The same input, run twice, produces two drafts with observably
   different wording in the generated code.
3. A submission with `target: "pine"` returns the v2-redirect
   message and does not generate code. The audit log records the
   routing decision.
4. A submission with `target: "unspecified"` triggers a
   clarification round asking which target the user wants.
5. With `C2_SINGLE_USER_PASSWORD_HASH` set,
   `POST /v1/sessions` without a Bearer token returns 401.
6. `GET /v1/strategies/{id}/bundle` returns a zip with the four
   expected files.
7. `cargo test --workspace` passes locally and in CI on
   `initial_mvp`.

## Verification required

- `cargo fmt --all -- --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `cargo test --workspace` green.
- `cargo build --release` green.
- A recorded `curl` + `curl -N` transcript (with real provider
  responses, keys redacted) showing the full SSE event sequence.
- The two-run diff proving the LLM is in the loop.
- The Pine-redirect transcript proving the routing decision is
  honored.
- The bundle zip contents (file listing + sizes).
- CI run on `initial_mvp` branch is green with the transcripts
  pasted in the run summary.

## Forbidden claims until evidence arrives

- Do NOT say "the LLM is wired in" until the two-run diff is in
  the handoff back.
- Do NOT say "the pipeline is closed" until a saved
  `strategy_id` and a populated `backtest_result` appear in
  `task.payload.generation`.
- Do NOT say "the report is generated" until the bundle zip
  contains a real PDF.
- Do NOT claim "tests pass" until the green CI run link is in
  the handoff.

## Handoff back to program lead

Use the `Handoff Back To Program Lead` template from
`.codex/skills/smarttrade-program-lead/references/handoff-contract.md`.
Include the SSE transcript, the two-run diff, the Pine-redirect
transcript, the bundle zip listing, the CI run URL, the diff
summary, and the SRS updates you made.

## Open questions for program lead

- **Static-analysis retry re-prompt**: should the retry loop
  re-prompt the LLM with the previous errors verbatim, or
  summarized? My read: verbatim, with `error_count` and the top
  3 issues. If you disagree, say so before the work starts.
- **Parameter sweep location**: parameter sweep can live in
  this orchestrator (one more tool call after generation) or in
  a separate workstream that the user invokes explicitly. My
  read: in the orchestrator, with the request body accepting an
  optional `parameter_sweep` field. Cheaper and the user
  doesn't need to learn a second workflow.
- **Report generation blocking vs deferred**: PDF generation can
  block the SSE stream (simple, slow) or run in a background
  worker (fast SSE, eventual PDF). My read: background worker
  with a `status: report_pending` -> `status: report_ready`
  sequence. UI shows a "generating report" state.
