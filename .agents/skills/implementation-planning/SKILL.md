---
name: implementation-planning
description: Creates a Stage 3 Implementation Plan Artifact. Details exact line-by-line code changes, provides copy-pasteable shell commands, and enforces an explicit STOP for user approval before any code is written.
---

# Implementation Planning Skill

This is Stage 3 of the SmartTradeAI task lifecycle. After the concept is understood (Stage 1) and the design is mapped (Stage 2), this skill produces a surgical, reviewable plan of every code change — down to the exact line numbers and diff previews.

---

## Core Principles (Non-Negotiable)

1. **Never write code without explicit user approval.** The plan must end with a hard STOP. The user must say "Approve" or "Proceed" before any file in the workspace is modified.
2. **Show diffs, not code blocks.** Every change must be presented in `diff` format (`+` for additions, `-` for deletions) so the user can review exactly what will change without comparing two separate blocks.
3. **Order changes by dependency.** If `state.rs` defines a struct that `strategies.rs` uses, `state.rs` must be changed first. Document the execution order explicitly.
4. **Check for new dependencies.** If the change requires a new crate (e.g., `uuid`, `argon2`), the plan must include the `Cargo.toml` modification as the very first step.
5. **User runs all commands.** Never execute docker or cargo commands directly. Provide copy-pasteable PowerShell commands.

---

## Required Document Structure

Save as `task_X_Y_implementation_plan.md` in the artifact directory.
Set `RequestFeedback = true` in the artifact metadata.

### Section 1: Change Summary Card
A quick-reference card at the top of the document:

```markdown
| Property | Value |
|----------|-------|
| Files Modified | 3 |
| Files Created | 0 |
| Files Deleted | 0 |
| Lines Added | ~25 |
| Lines Removed | ~8 |
| New Dependencies | None |
| Estimated Complexity | Low (single-function insertion) |
| Estimated Time | ~10 minutes |
| Risk Level | 🟢 Low |
```

### Section 2: Dependency Check
Before any code changes, verify if new crate dependencies are needed:

```markdown
#### New Crate Dependencies
- **None required** for this task.

OR:

#### New Crate Dependencies
1. Add `uuid = { version = "1", features = ["v4"] }` to `crates/server/Cargo.toml`
2. Add `argon2 = "0.5"` to `crates/server/Cargo.toml`
```

If dependencies are needed, this MUST be the first change applied, because all subsequent code changes depend on successful compilation with the new crates.

### Section 3: Execution Order
List the files in the exact order they must be modified:

```markdown
#### Execution Order
1. `Cargo.toml` — Add new dependencies (if any)
2. `state.rs` — Define new struct fields (upstream)
3. `main.rs` — Initialize new fields at boot
4. `strategies.rs` — Use new fields in route handlers (downstream)
```

Explain WHY this order matters (e.g., "strategies.rs imports from state.rs, so state.rs must compile first").

### Section 4: Step-by-Step Code Changes
For each file, show the exact change in diff format:

```markdown
#### Step 1: [MODIFY] [main.rs](file:///absolute/path/to/main.rs)
**What:** Add environment validation at boot.
**Where:** After line 24 (after `TcpListener::bind`), before the `database_url` fetch.
**Why:** Enforce fail-fast in production mode.

` ` `diff
     let listener = TcpListener::bind(&address)
         .await
         .unwrap_or_else(|error| panic!("failed to bind {address}: {error}"));
+    let app_env = std::env::var("APP_ENV").ok();
+    let is_production = app_env.as_deref()
+        .map(|s| s.eq_ignore_ascii_case("production"))
+        .unwrap_or(false);
+
     let database_url = std::env::var("DATABASE_URL").ok().filter(|s| !s.is_empty());
+
+    if is_production && database_url.is_none() {
+        tracing::error!("FATAL: DATABASE_URL is missing under production profile!");
+        std::process::exit(1);
+    }
` ` `
```

Rules for this section:
- Show 2-3 lines of unchanged context above and below the change (like `git diff`).
- Use `+` for added lines, `-` for removed lines, and a space for unchanged context.
- Specify the exact line number range where the change occurs.
- Explain the "Why" for non-obvious changes.

### Section 5: Verification Commands
Provide copy-pasteable PowerShell commands the user can run after approving and applying the changes:

```markdown
#### Compilation Check
` ` `powershell
cd "c:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI\services\c2-engine"
docker compose --profile dev run --rm rust-dev cargo check
` ` `

#### Unit Test Check
` ` `powershell
docker compose --profile dev run --rm rust-dev cargo test
` ` `

#### Quick Smoke Test
` ` `powershell
docker compose up c2-engine --build -d
Invoke-RestMethod -Uri "http://localhost:3000/v1/strategies" -Method Get | ConvertTo-Json -Depth 5
` ` `
```

### Section 6: Rollback Instructions
If the changes cause unexpected issues:

```markdown
#### Rollback
` ` `powershell
cd "c:\Users\Abdul Jabbar Metlo\Desktop\smarttradeAI"
git diff --stat HEAD   # See what changed
git checkout -- .      # Revert all uncommitted changes
# OR if already committed:
git revert HEAD        # Create a revert commit
` ` `
```

### Section 7: STOP Gate

```markdown
---

## ⛔ STOP! USER REVIEW REQUIRED

This plan modifies **[N] files** with **[M] lines** of changes.

Please review the diff previews above carefully. If everything looks correct:
- Reply with **"Approve"** or click **"Proceed"** to begin implementation.
- Reply with feedback if you want changes to the plan.

**No code will be written until you approve.**
```

---

## Workflow Checklist

Before presenting the plan to the user, verify:

- [ ] Summary card filled in (files, lines, complexity, risk)
- [ ] New crate dependencies checked and listed if needed
- [ ] Execution order specified with reasoning
- [ ] Every change shown in diff format with context lines
- [ ] "Why" explanation provided for non-obvious changes
- [ ] Verification commands are copy-pasteable PowerShell
- [ ] Rollback instructions provided
- [ ] STOP gate present at the end
- [ ] `RequestFeedback = true` set in artifact metadata
