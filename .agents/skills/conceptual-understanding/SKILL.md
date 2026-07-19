---
name: concept-to-code-bridge
description: Creates a Stage 1 Understanding Artifact that bridges abstract systems concepts to concrete Rust code. Generates multiple visual diagrams, uses physical analogies, maps cognitive thinking patterns to code variables, traces data flows end-to-end, and explains abstraction levels. Designed for a developer who learns best through visuals, analogies, and "why" explanations.
---

# Concept-to-Code Bridge Skill

This is the most important skill in the SmartTradeAI workflow. It is always the **first stage** before any code is written. Its purpose is to transform an abstract engineering concept into something the developer can see, feel, and understand at a gut level before touching the code.

---

## Core Principles (Non-Negotiable)

1. **Visuals are mandatory, not optional.** Generate at least 2 images using `generate_image`:
   - One **architecture infographic** showing the system-level concept.
   - One **data flow diagram** showing how a single request travels through the system.

2. **Physical analogies are mandatory.** Every concept must be mapped to a real-world object (phone lines, receptionists, bouncers, post offices, traffic lights, hotel check-in desks). The user learns through spatial reasoning, not abstract definitions.

3. **Never explain "what" without "why".** For every technical decision, explain:
   - Why does this exist?
   - What problem does it solve?
   - What breaks if we skip it?

4. **Reference our codebase, not textbooks.** Every example must point to an actual file, function, or variable in the SmartTradeAI workspace using clickable file links.

---

## Required Document Structure

Save as `task_X_Y_understanding.md` in the artifact directory.

### Section 1: Visual Architecture (Top of Document)
- Generate a high-quality infographic using `generate_image` and embed it at the top.
- This image should be the first thing the user sees when opening the document.

### Section 2: The Physical Analogy
- Open with a 3-5 sentence analogy that maps the concept to a physical-world scenario.
- Example: "Opening a database connection is like calling someone on the phone. If you hang up and redial 100 times, you waste 90% of your time waiting for the phone to ring."
- This section primes the developer's spatial reasoning before introducing technical details.

### Section 3: Why & What
- **Why are we doing this task?** (Business motivation)
- **What is the concept?** (Technical definition in plain language)
- **What breaks if we skip it?** (Concrete consequences with scenarios)

### Section 4: Abstraction Level Map
Explain where this concept sits in the system's abstraction hierarchy:

```markdown
| Level | What Lives Here | Our Example |
|-------|----------------|-------------|
| Application | Route handlers, business logic | `strategies.rs` |
| Framework | HTTP server, middleware | Axum `Router` |
| Library | Connection pool, query builder | `sqlx::PgPool` |
| Runtime | Async task scheduler | `tokio` |
| OS | TCP sockets, file descriptors | Linux kernel |
| Hardware | Network card, RAM | Server NIC |
```

Mark which level(s) the current task operates at.

### Section 5: Mermaid Diagrams
Include at least TWO diagrams:
1. A **`sequenceDiagram`** tracing a single user request from PowerShell → Axum → Pool → PostgreSQL → Response.
2. A **`graph TD`** showing the decision tree or component relationships.

### Section 6: Data Flow Trace-Through
- Generate a second image using `generate_image` showing the data flow.
- Walk through one complete request step-by-step, numbering each hop:
  1. User sends `GET /v1/strategies`
  2. Axum extracts `State<AppState>`
  3. Handler checks `state.pool`
  4. Pool lends a connection
  5. SQL query executes
  6. Rows deserialize into structs
  7. JSON response returns to user
  8. Connection returns to pool

### Section 7: Cognitive Model → Code Variable Mapping
Use the 3-stage framework to map human thinking to Rust code:

```markdown
| Cognitive Stage | Mental Model | Rust Variable | Compiler Enforcement |
|----------------|-------------|---------------|---------------------|
| 1. Analogy | "Keep the phone line open" | `PgPool` | Wraps `Arc<PoolInner>` for zero-copy sharing |
| 2. Constraints | "Max 20 lines at once" | `.max_connections(20)` | Blocks 21st thread in async queue |
| 3. Lifetimes | "Who holds the receiver?" | `pool.clone()` | Arc increment, auto-drop on scope exit |
```

### Section 8: Language/Stack Context (Rust)
- How is this concept implemented using our stack (`sqlx`, `tokio`, `axum`)?
- What Rust-specific patterns apply (`Arc`, `Option`, `Result`, `async/await`)?
- Show the actual function signatures from our codebase with clickable links.

### Section 9: Five Alternative Approaches
| # | Alternative | Pros | Cons |
|---|-------------|------|------|
| 1 | ... | ... | ... |
| 2 | ... | ... | ... |
| 3 | ... | ... | ... |
| 4 | ... | ... | ... |
| 5 | ... | ... | ... |

### Section 10: Production Rationale & Consequences
Split into two clear sub-sections:

**Why This Is Standard:**
- Reference industry standards (Twelve-Factor App, cloud-native patterns).
- Name real companies/systems that use this pattern.

**What Happens If We Skip This (Disaster Scenarios):**
- Describe at least 2 concrete failure scenarios with user impact.
- Example: "If we skip connection pooling, a burst of 500 users will fork 500 PostgreSQL processes, exhausting server RAM and crashing the database. All active trading sessions will be lost."

---

## Workflow Checklist

Before marking the Understanding Artifact as complete, verify:

- [ ] At least 2 images generated and embedded
- [ ] Physical analogy included
- [ ] Abstraction level table filled in
- [ ] At least 2 Mermaid diagrams (sequence + flowchart)
- [ ] Data flow trace-through with numbered steps
- [ ] Cognitive model → code variable mapping table
- [ ] 5 alternatives compared
- [ ] At least 2 disaster scenarios described
- [ ] All code references use clickable file links to our workspace
