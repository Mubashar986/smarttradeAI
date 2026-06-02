# SmartTradeAI Risk Register

Status: Active
Date: 2026-06-02

| ID | Risk | Severity | Likelihood | Mitigation | Early Warning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| RISK-001 | User expects trading profit guarantees. | High | High | Use strict service wording and disclaimers. Refuse profit-guarantee clients. | Client asks "what win rate" or "will this pass prop firm". | Project owner |
| RISK-002 | Generated MQL5 compiles poorly or mixes MQL4/MQL5. | High | Medium | Use controlled templates, static checks, and human review. | Repeated compile errors on simple templates. | Engineering |
| RISK-003 | Stub compile mode is mistaken for real validation. | High | Medium | Label stub source clearly; require MetaEditor evidence for compile claims. | Output says success with source=stub. | Engineering |
| RISK-004 | Sessions/tasks are lost on restart. | Medium | High | Document as MVP gap; add persistence later. | Demo restart loses task history. | Engineering |
| RISK-005 | Secrets leak through env/config/logs. | High | Medium | Rotate exposed keys; never commit .env; avoid printing secrets. | Compose config or logs show real-looking API keys. | Security |
| RISK-006 | Overbuilding public SaaS delays revenue/demo. | High | High | Use productized-service engine first. Cut auth/payments/marketplace. | Building login/payment before one working demo. | Product |
| RISK-007 | Legal/ethical liability from live trading use. | High | Medium | Prototype disclaimer, no live-ready claims, demo-test recommendation. | Client asks for managed account or live deployment. | Product |
| RISK-008 | Frontend builds against legacy API routes. | Medium | Medium | Use only /v1 in docs and UI. | New code calls /sessions legacy routes. | Engineering |
| RISK-009 | Docker build/test path is unclear. | Medium | Medium | Verify through Docker and write exact commands in handoff. | Tests only run on host cargo. | Engineering |
| RISK-010 | Support scope explodes on first clients. | Medium | High | One revision included; fixed deliverables; decline vague changes. | Client sends repeated "small tweak" requests. | Service |
| RISK-011 | User/project owner cannot resume after exams. | High | Medium | Keep handoff current and branch isolation clean. | Next action is unclear or branch is dirty. | Engineering |
| RISK-012 | RAG/Pinecone claim exceeds implementation. | Medium | Medium | Say local skeleton fallback unless real retrieval evidence exists. | Marketing says "RAG-backed" without logs. | Product |

## Highest Priority Risks

1. RISK-003: Stub validation confusion.
2. RISK-005: Secrets hygiene.
3. RISK-006: Overbuilding.
4. RISK-011: Resume safety.
