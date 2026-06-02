# SmartTradeAI NASA-Lite Engineering Plan

Status: Active control document
Date: 2026-06-02
Branch: initial_mvp

## Purpose

This document defines the engineering operating system for SmartTradeAI.
It is inspired by NASA-style systems and software engineering practices:
requirements first, controlled interfaces, traceability, verification,
risk management, anomaly tracking, and recoverable baselines.

This is a lightweight process for a solo project, not a full safety-critical
compliance program.

## Mission

Build a credible June MVP for SmartTradeAI:

User describes a trading strategy in plain English, the system asks for missing
details, generates an MQL5 Expert Advisor draft from controlled templates,
performs basic validation, explains the code, saves the strategy, and makes the
result usable for a service-led delivery workflow.

## Non-Goals

- No profit guarantees.
- No live trading execution.
- No managed trading accounts.
- No public SaaS billing system in June.
- No strategy marketplace.
- No automated Pine-to-MQL5 converter until the core workflow is proven.
- No backtest profitability claims.

## Engineering Principles

1. Baseline before change.
2. Requirements before implementation.
3. Interfaces are contracts, not suggestions.
4. AI output is untrusted until validated.
5. Trading outcomes are not software correctness.
6. Every claim needs evidence.
7. Small changes beat heroic rewrites.
8. Recovery path must be obvious.

## Controlled Branches

| Branch | Purpose |
| --- | --- |
| main | Original repository line. Do not use for new June work. |
| before-june | Recovery baseline captured before June MVP work. |
| initial_mvp | Active June MVP engineering branch. |

## Required Control Artifacts

| Artifact | Purpose |
| --- | --- |
| SmartTradeAI_ConOps.md | Defines users, operating modes, and mission flow. |
| Requirements_Traceability_Matrix.md | Maps requirements to design, code, and tests. |
| Interface_Control_Document.md | Freezes API and external boundaries. |
| Risk_Register.md | Tracks product, technical, and safety risks. |
| Verification_Plan.md | Defines how each claim is proven. |
| Anomaly_Log.md | Tracks bugs, gaps, and unresolved issues. |
| June_MVP_Handoff.md | Current status and next safe action. |

## Change Protocol

Every non-trivial change follows this sequence:

1. Identify requirement ID.
2. Identify affected interface.
3. Identify risk and rollback path.
4. Add or confirm regression coverage.
5. Make a small scoped change.
6. Run verification.
7. Record evidence in the handoff.
8. Update traceability if the requirement status changed.

## Verification Levels

| Level | Use When | Evidence |
| --- | --- | --- |
| Inspection | Docs, contracts, small static changes | Reviewed files and line references |
| Unit | Pure logic or helper behavior | Test output |
| Integration | API, storage, worker, Docker | Docker/test logs |
| Manual Demo | End-to-end user-visible flow | Steps, expected result, screenshot/video if available |

## June Readiness Gates

| Gate | Exit Criteria |
| --- | --- |
| Baseline Ready | before-june exists and current state is recoverable. |
| Build Ready | Docker-based backend build/test path is known. |
| Contract Ready | /v1 API and SSE primary flow are documented. |
| MVP Ready | Strategy intake to generated MQL5 plus explanation works for demo. |
| Service Ready | Offer, disclaimer, demo, and delivery checklist exist. |

## Current Decision

For the next 60 days, SmartTradeAI is a productized-service engine, not a public
SaaS. The Rust C2 backend remains the project core, but first revenue can use a
human-in-the-loop workflow around the system.

## References

- NASA NPR 7150.2: software engineering requirements and lifecycle discipline.
- NASA Systems Engineering Handbook: stakeholder expectations, technical
  requirements, system design, product realization, verification, and validation.
- NASA-STD-8739.8: software assurance and evidence discipline.
