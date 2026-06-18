# SmartTradeAI — Project Brief

Date: 2026-06-13 (rev 2 — backtest focus, no multi-user)
Status: Source of truth for project scope. Supersedes the academic abstract, the previous brief, and any prior designdocs.
Owner: smarttrade-program-lead

## Purpose

Help traders turn natural-language trading ideas into safer, reviewable,
testable automated trading strategies, with backtesting as the primary
deliverable.

## Vision

A single-user, local-first strategy-development workspace. The user types
a strategy in plain English; the system produces MQL5 code, validates it,
compiles it to `.ex5`, runs a backtest with rich metrics, generates a
visualizable report, and stores the full decision trail. No accounts, no
roles, no tenant boundaries, no broker integration, no live trading.

## Problem statement

Traders have strategy ideas but cannot reliably turn them into testable
automated systems. Even when a draft is produced, the trader cannot
easily see whether the strategy is safe, what it would have done on
historical data, or how its parameters behave across the search space.
SmartTradeAI turns the natural-language idea into a backtested, charted,
reportable artifact the trader can actually evaluate.

## Target user

A single trader-developer who:
- can describe a strategy in plain English,
- can read MQL5 (or wants to learn),
- wants a fast loop from idea -> backtested report,
- is the only person using this instance,
- is not running real money through it (v1).

Multi-user / multi-tenant / role-based access is explicitly **out of
scope for v1**. The instance is single-user. Adding accounts is a v2
problem, and the schema keeps room for it without forcing a refactor.

## Core user goal

A user says what they want in normal trading language. Example:

> Create a moving-average crossover strategy for EURUSD on the H1
> timeframe with a 50-pip stop-loss.

SmartTradeAI guides the request into a complete strategy specification,
produces a MQL5 draft, validates it, compiles it to `.ex5`, runs a
backtest with rich metrics on historical OHLCV, generates charts and a
report, and stores the full decision trail.

## Canonical flow (v1)

```
natural-language input
   -> intent classification
   -> ambiguity check + clarification round (max 5)
   -> MQL5 strategy generation
   -> static analysis (with 3-retry loop)
   -> compile to .ex5 via C3 (or explicit stub-skipped)
   -> backtest against historical OHLCV
        (bundled canned dataset by default; user CSV upload supported)
   -> risk + performance metrics
   -> equity curve + drawdown chart + trade markers
   -> monthly returns heatmap
   -> plain-English explanation
   -> PDF + JSON report export
   -> store with full audit trail
   -> downloadable bundle: .ex5 + .mq5 + report + audit
```

## In scope (v1)

- Natural-language intake, clarification, intent classification (LLM-driven).
- MQL4/5 code generation. v1 generates MQL5 only. Pine Script is detected
  in the language-routing step and answered with a clear "Pine Script
  generation is on the v2 roadmap; v1 produces MQL5" message.
- Static analysis (brackets, required functions, deprecated MQL4 calls,
  stop-loss presence, #property directives) with a real retry loop.
- MQL5 compilation to `.ex5` via `C3_COMPILER_URL`. A stub result is
  never reported as a successful compile; it is a first-class
  `compile_status: STUB_SKIPPED` audit entry.
- Backtest engine with:
  - input: generated MQL5 + historical OHLCV (bundled EURUSD H1 + user CSV)
  - output: PnL, max drawdown, win rate, trade count, Sharpe ratio,
    Sortino ratio, profit factor, recovery factor, average win / loss,
    max consecutive wins / losses, average holding period, exposure time
  - determinism: same input + same OHLCV = same metrics
- Visualization:
  - equity curve (line chart)
  - drawdown chart (underwater equity)
  - trade markers on the price chart (entry / exit arrows)
  - monthly returns heatmap
  - PnL distribution histogram
- Report export:
  - PDF report (strategy spec + code + metrics + charts + audit)
  - JSON export (machine-readable, complete)
  - Bundle zip: `.ex5` + `.mq5` + PDF report + audit JSON
- Parameter sweep / optimization:
  - user defines a parameter grid (e.g. fast_ma 10-100 step 10, slow_ma
    50-300 step 50)
  - backtest runs all combinations
  - top N configurations by an objective metric (Sharpe, total return,
    profit factor) reported with a heatmap
- Plain-English explanation (LLM, 3-5 sentences).
- Single-user auth: a single shared password (env var) gates the UI and
  the `/v1` routes. No user table, no roles, no tenant column. JWT
  signed with the existing `C2_JWT_SECRET`. Optional: if the env is
  unset, dev mode is open (current behavior).
- Audit trail of every action (intake, classify, clarify, generate,
  static analysis, compile, backtest, save, explain, export). The
  audit log is the only persisted state besides the strategy itself.
  No `user_id` column — the instance is single-user, but the
  timestamps and details are the proof.

## Out of scope (deferred from the academic abstract or explicitly cut)

- Multi-user, multi-tenant, role-based access. v1 is single-user.
- Live trading, broker integrations, order placement, real-money
  routing. The system stops at `.ex5` + backtest report. A human
  copies the `.ex5` into MT5 manually.
- Python webhook + signal server for live MT4/5 execution.
- Pine Script code generation (detected, redirected to MQL5 for v1).
- Real-time market data feeds. Backtests use bundled or uploaded
  historical data only.
- External notifications (email, push, desktop). In-app SSE/WS only.
- Cloud deployment. v1 is local.
- Mobile and desktop apps. Web only.
- Walk-forward validation. v2.
- Multi-dataset robustness scoring. v2.
- Strategy side-by-side comparison view. v2.
- Version history / diff between regenerations. v2.

## Success criteria (v1, evidenced)

1. A user with a valid session can submit a natural-language MQL5
   request and receive a generated MQL5 draft that:
   - passes static analysis with zero errors,
   - is compiled by C3 (or has `compile_status: STUB_SKIPPED` if no
     `C3_COMPILER_URL` is set, and the user can see the difference),
   - has been backtested on the bundled EURUSD H1 dataset with all
     the v1 metrics populated,
   - has charts (equity, drawdown, trade markers, monthly heatmap,
     PnL histogram) rendered in the UI and embedded in the PDF,
   - has a 3-5 sentence LLM-generated explanation,
   - is downloadable as a bundle zip (`.ex5` + `.mq5` + PDF + JSON).
2. The same natural-language request, run twice, produces two
   observably different drafts (proves the LLM, not the regex
   fallback, is in the loop).
3. A parameter sweep over a 2-axis grid (e.g. fast_ma × slow_ma)
   produces a heatmap and a top-N table.
4. Every step is recorded in the audit trail with timestamp and
   details.
5. The single-user password gate keeps unauthenticated callers out
   of `/v1` routes.
6. The system has zero live-trading surface. There is no code path
   that places an order, talks to a broker, or accepts a live data
   feed. (Static review confirms this.)

## Forbidden claims (until the v1 success criteria are evidenced)

- Do not claim the LLM is wired in until criterion 2 is demonstrated
  on a recorded run.
- Do not claim "ready for traders" until criteria 1, 3, 4, 5, 6 are
  demonstrated end-to-end with a real session, real provider key,
  and a real `.ex5` artifact (or an explicit stub-skipped audit
  entry).
- Do not claim "live trading" or "broker integration" in any
  user-facing artifact. The system does not and will not place
  orders.
- Do not reintroduce the multi-user / role / tenant complexity that
  was removed in this revision.
