# Human-Computer Interaction (HCI) Agent Skills Suite

This directory contains **9 modular, production-ready Skill definitions (`SKILL.md`)** derived from the complete Human-Computer Interaction course materials, lecture slide decks, examination solution rubrics, and the 5th Edition textbook *Interaction Design: Beyond Human-Computer Interaction* (Sharp, Preece, Rogers).

These skills transform any general-purpose coding agent (Google Antigravity, Cursor, Claude Code, GitHub Copilot Workspace, etc.) into an expert **Human-Centered Software Engineer**.

---

## 🧭 Skills Catalog & Navigation

| # | Skill Directory | Focus Area | Primary HCI Methodology | When a Coding Agent Should Use It |
|---|---|---|---|---|
| **1** | [**`hci-usability-principles`**](./hci-usability-principles/SKILL.md) | Foundations & Principles | Norman's 6 Principles, Shneiderman's 8 Rules, 5 Interaction Types | When designing new UI components, forms, buttons, or navigation systems. |
| **2** | [**`pact-context-analysis`**](./pact-context-analysis/SKILL.md) | Discovery & Scoping | PACT Framework (People, Activities, Contexts, Technologies) | When planning project architecture, responsive layouts, offline sync, or accessibility. |
| **3** | [**`user-research-thematic-analysis`**](./user-research-thematic-analysis/SKILL.md) | Qualitative Research | Braun & Clarke Thematic Analysis, Qualitative Coding | When synthesizing user interviews, customer support tickets, or feedback forms into backlogs. |
| **4** | [**`persona-scenario-design`**](./persona-scenario-design/SKILL.md) | User Modeling | Cooper Goal-Directed Design, Context Scenarios | When defining user stories, primary personas, user journeys, and E2E test specs. |
| **5** | [**`task-modeling-goms-klm`**](./task-modeling-goms-klm/SKILL.md) | Performance Modeling | HTA, GOMS, Keystroke-Level Model (KLM), Fitts's Law | When optimizing high-frequency workflows, data entry forms, and keyboard shortcuts for speed. |
| **6** | [**`heuristic-evaluation-audit`**](./heuristic-evaluation-audit/SKILL.md) | Expert Usability Auditing | Nielsen’s 10 Usability Heuristics, 0–4 Severity Ratings | When reviewing PRs, auditing legacy screens, or identifying usability bugs before release. |
| **7** | [**`cognitive-walkthrough-evaluator`**](./cognitive-walkthrough-evaluator/SKILL.md) | Learnability Evaluation | Wharton & Lewis's 4 Cognitive Walkthrough Questions | When designing first-time user onboarding, sign-up flows, or self-service wizards. |
| **8** | [**`emotional-ux-interaction-design`**](./emotional-ux-interaction-design/SKILL.md) | Affective Interaction | Norman’s 3 Levels (Visceral, Behavioral, Reflective), Anti-Annoyance | When writing error messages, designing micro-interactions, empty states, and avoiding dark patterns. |
| **9** | [**`empirical-usability-ab-testing`**](./empirical-usability-ab-testing/SKILL.md) | Empirical Testing & Metrics | Controlled Experiments, A/B Testing, SUS Scoring, Telemetry | When formulating UX hypotheses, configuring A/B feature flags, and instrumenting clickstream telemetry. |

---

## 🚀 How to Use These Skills in Heisenberg OS

In Heisenberg OS, these skills are registered in `.heisenberg/skills.json` and enforced by `scripts/heisenberg_guard.py`:
- **UI Surfaces:** Tasks with `ui_surface` route through `.heisenberg/ui-workflow.json` and require HCI artifacts (e.g. `hci-interaction-spec.md`, `heuristic-audit.md`, `pact-analysis.md`) before product code edits.
- **Task Types:** Use dedicated task manifests:
  - `ui-audit`: `heuristic-evaluation-audit` + `cognitive-walkthrough-evaluator`
  - `user-research`: `pact-context-analysis` + `user-research-thematic-analysis` + `persona-scenario-design`
  - `workflow-optimization`: `task-modeling-goms-klm` + `concept-to-code-bridge`
- **Agent Prompts:** Trigger directly via prompt cheat sheet in `QUICKSTART.md`.

---

## 📊 Summary of Source Materials Used
- **Course:** CSC356 - Human Computer Interaction (COMSATS University Islamabad)
- **Textbook:** *Interaction Design: Beyond Human-Computer Interaction* (5th Edition, Wiley, 2019)
- **Reference Papers:** Card, Moran & Newell (KLM/GOMS); Nielsen & Molich (Heuristic Evaluation); Wharton et al. (Cognitive Walkthrough); Braun & Clarke (Thematic Analysis); Brooke (System Usability Scale).
