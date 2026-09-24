---
name: rust-learning-extraction
description: Extracts Rust programming concepts from the current implementation task and creates a dedicated learning document. Maps cognitive thinking patterns to code variables, explains ownership, lifetimes, Arc, Option, and async patterns using physical analogies.
---

# Rust Learning Extraction Skill

Use this skill when the user asks to learn Rust concepts from the implementation they are working on. This skill transforms the current task's code changes into a structured learning document.

## Workflow

1. **Identify Learning Opportunities:**
   - Review the code changes made in the current task.
   - Identify Rust-specific patterns that are educational:
     - Ownership and borrowing (`&`, `&mut`, moves)
     - Lifetimes (`'a`, `'static`)
     - Smart pointers (`Arc`, `Box`, `Rc`)
     - Error handling (`Result`, `Option`, `?` operator)
     - Async/await and `tokio` runtime
     - Traits and trait objects (`dyn Trait`)
     - Pattern matching (`match`, `if let`, `ref`)
     - Module system (`mod`, `use`, `pub`)

2. **Map Cognitive Models to Code:**
   - Use the 3-stage cognitive framework:
     - **Stage 1: Analogy** — Map the concept to a physical-world analogy.
     - **Stage 2: Constraints** — Identify the resource limits the code enforces.
     - **Stage 3: Lifetimes** — Explain ownership and state management.
   - Show how each mental model translates into a specific variable or type declaration.

3. **Create the Learning Document:**
   - Save as `task_X_Y_rust_learnings.md` in the artifact directory.

### Required Sections

```markdown
# Rust Learning — [Task Title]

## 1. Concepts Encountered
(List each Rust concept with a brief explanation)

## 2. Cognitive Model → Code Mapping
| Mental Concept | Rust Code | How the Compiler Enforces It |
|---------------|-----------|------------------------------|
| Finite Boundaries | `.max_connections(20)` | Blocks the 21st thread... |

## 3. Deep Dive Examples
(For each concept, show the exact code from our codebase with line-by-line annotations)

## 4. Practice Exercises
(Optional: suggest small modifications the user could try to reinforce learning)
```

## Key Principles
- **Use Physical Analogies:** Phone lines, receptionists, post offices, traffic lights.
- **Reference Our Codebase:** Every example should come from SmartTradeAI's actual code, not generic textbook examples.
- **Explain the "Why":** Don't just show syntax. Explain why Rust forces you to write it this way and what bugs it prevents.
