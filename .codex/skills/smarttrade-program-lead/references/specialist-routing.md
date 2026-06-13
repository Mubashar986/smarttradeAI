# Specialist Routing

Use this map when the project lead must decide where work belongs.

## Owner Map

| Owner | Route work here when the task is about | Typical handoff consumer |
| --- | --- | --- |
| `smarttrade-program-lead` | Scope, sprint goal, next task, role routing, risk triage, handoff review, resume planning | Any specialist |
| `smarttrade-systems-architect` | Requirements, ConOps, ICD, traceability, architecture, cross-domain decisions | Backend, frontend, AI, QA |
| `smarttrade-backend-engineer` | Rust C2 backend, `/v1` API, sessions, tasks, SSE/WebSocket, strategy CRUD, persistence | Frontend, QA, security/devops |
| `smarttrade-ai-engineer` | Intent classification, clarification, strategy extraction, prompt chains, template selection, explanation | Backend, MQL5, QA |
| `smarttrade-mql5-engineer` | MQL5 templates, EA skeletons, risk modules, MQL4/MQL5 correctness, MetaEditor compile behavior | AI, QA, docs |
| `smarttrade-qa-verifier` | Tests, Docker verification, regression checks, evidence, anomaly log, release readiness | Program lead |
| `smarttrade-risk-safety-reviewer` | Trading safety, disclaimers, no-profit-claim policy, unsafe client requests, legal/ethical boundaries | Product, docs, frontend |
| `smarttrade-frontend-engineer` | Chat UI, strategy form, code viewer, explanation panel, realtime status, saved strategy screens | QA, program lead |
| `smarttrade-security-devops` | Docker, secrets, env vars, auth, deployment, logs, CI/CD | Program lead, QA |
| `smarttrade-product-strategist` | Market, user segment, pricing, service offer, positioning, roadmap | Growth, docs |
| `smarttrade-growth-operator` | Landing page, demo video, outreach, first users, feedback loop | Product, docs |
| `smarttrade-docs-portfolio-writer` | README, case studies, resume bullets, LinkedIn posts, PDFs, handoff docs | Program lead |

## Routing Rules

- Route to one owner when the work can be isolated.
- Keep the program lead as owner when the task changes scope, priority, risk, or project truth.
- Require architecture review before changing public contracts, data model, realtime semantics, or safety claims.
- Require QA verification before claiming "done" across domains.
- Require risk/safety review before any trading-result, live-readiness, or client-facing claim.

## Cross-Chat Request Format

```text
Requesting owner:
Target owner:
Context:
Decision or artifact needed:
Files/contracts involved:
Deadline or urgency:
What not to change:
Return handoff to:
```
