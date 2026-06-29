# SmartTradeAI Custom Agent Instructions

For every task selected from the roadmap, the agent must strictly execute the following four-stage lifecycle before making any code modifications.

---

## Stage 1: Understanding Artifact
Create a detailed, conceptual documentation file containing:
1.  **Why & What:** Explain why we are doing the task, what it is, and how it works conceptually.
2.  **Diagrams:** Use diagrams to visualize the core concepts.
3.  **Language/Stack Context:** Explain how the concept is implemented in Rust (`sqlx`, `tokio`, etc.).
4.  **Alternatives:** List and explain at least five (5) alternative approaches.
5.  **Rationale:** Detail why this is standard in production systems, why we chose it for SmartTradeAI, the problems it solves, and the negative consequences if we do not implement it.

---

## Stage 2: Design Artifact
Create a codebase-specific design documentation file containing:
1.  **Impact Analysis:** Analyze all files and modules in the workspace that will be affected by this task.
2.  **Regression Analysis:** Detail potential regression risks on existing features and test suites.
3.  **Quality Metrics:** Propose Rust-specific design patterns that preserve code-level metrics (avoiding duplication, maintaining loose coupling, and keeping lifetimes correct).
4.  **Design Diagram:** Render a diagram illustrating the new module connections and data flow.

---

## Stage 3: Implementation Plan Artifact & Approval
Create an step-by-step implementation plan:
1.  **Step-by-Step Changes:** Detail exact code changes line-by-line.
2.  **Commands for User Execution:** Provide clear shell commands so the user can test or verify execution on their end.
3.  **Explicit STOP:** The agent must **STOP and wait for the user's explicit approval** before writing code to the workspace.

---

## Stage 4: Testing & Completion Artifact
Define a rigorous testing protocol:
1.  **Edge Case Matrix:** Outline at least ten (10) specific test cases and edge cases with their expected outputs.
2.  **Copy-Pasteable Terminal Commands:** You **must** provide exact, copy-pasteable PowerShell/Terminal commands for the user to execute and verify each test case (e.g. env settings, process execution, curl calls, docker inspections, and SQL checks).
3.  **Verification Steps:** Detail how the user can run tests and verify the changes.
4.  **Test Analysis:** Once the user shares test results, the agent must analyze any errors and repeat the cycles (Design/Plan/Test) until 100% correct.
5.  **Completion Report:** Once resolved, document a final completion report.
