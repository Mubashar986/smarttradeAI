---
name: graperoot-dual-graph-mandate
description: The immutable, zero-tolerance operational contract governing GrapeRoot dual-graph AST intelligence, token conservation, blast radius verification, and agent self-configuration.
version: 2.0.0
---

# Rule 01: The GrapeRoot Dual-Graph Master Mandate

> **Portable policy note (v3):** This rule describes Heisenberg's enhanced
> graph-enabled mode. In repositories using `AGENTS.md` and
> `.heisenberg/policy.json`, GrapeRoot is required only when the active task
> manifest sets `requires_graperoot: true`. Otherwise use normal repository
> analysis, record graph-specific evidence as `UNKNOWN`, and do not fabricate
> tool availability.

> **STATUS: HARD OPERATIONAL LAW — ZERO TOLERANCE**  
> Any violation of this document constitutes a catastrophic protocol breach. Agents that violate these mandates must be terminated and their session restarted.

---

## 1. Core Philosophy: Why This Mandate Exists

Standard coding agents waste **60% to 80% of conversation tokens and user credits** wandering through directories, running blind terminal grep commands, and ingesting massive 800-line files. This causes context window saturation, severe hallucination, and regressive code edits.

**Heisenberg OS eliminates this entirely.** Heisenberg OS operates exclusively through a **dual-graph AST context optimization engine** (GrapeRoot) running on `http://127.0.0.1:8080/mcp`.

---

## 2. The 8 Immutable Laws of Dual-Graph Execution

### LAW 1: Pre-Flight Self-Configuration & Tool Palette Lock
- **The Mandate:** Before writing code, answering technical queries, or inspecting directories, the agent **MUST** verify that the GrapeRoot tools (`graph_retrieve`, `graph_read`, `graph_neighbors`, `graph_impact`, `graph_continue`, `graph_register_edit`, `graph_add_memory`) are present in its active tool palette.
- **The Lock:** If these tools are **NOT** present:
  1. The agent is **STRICTLY FORBIDDEN** from modifying any file or proposing code solutions.
  2. The agent **MUST HALT IMMEDIATELY** and configure its own editor using the **Universal MCP Configuration Registry** in Section 3.
  3. The agent must verify the connection by running:
     ```bash
     python Heisenberg/bin/doctor.py --json
     ```
  4. Only when `doctor.py` returns `"healthy": true` may the agent proceed with the user's task.

### LAW 2: Absolute Ban on Blind Terminal Grep & Raw Search
- **The Mandate:** The agent is **STRICTLY PROHIBITED** from executing raw terminal search commands:
  - Banned: `grep`, `egrep`, `Select-String`, `findstr`, raw `ripgrep` via shell, or blind directory dumps.
- **The Dual-Graph Mechanism:**
  - All codebase exploration, architectural discovery, and symbol location **MUST** be performed via:
    ```json
    graph_retrieve(query="<semantic query or symbol name>")
    ```
- **Fallback Exception:** `fallback_rg` is permitted **ONLY** if `graph_retrieve` returns zero results and the target text is confirmed to be an un-indexed string or comment outside the AST.

### LAW 3: Surgical Reads Only (`file::symbol` Anchor Enforcement)
- **The Mandate:** Dumping whole files (>100 lines) into the conversation context is a **DIRECT FAILURE**.
- **The Dual-Graph Mechanism:**
  - When inspecting a function, method, class, interface, or route, the agent **MUST** use symbol anchor notation:
    ```json
    graph_read(file="src/routes/auth.ts", anchor="src/routes/auth.ts::handleLogin")
    ```
  - Ingesting entire files is permitted **ONLY** for configuration files $< 80$ lines (e.g., `package.json`, `tsconfig.json`).

### LAW 4: Mandatory Pre-Edit Blast Radius Verification
- **The Mandate:** The agent is **STRICTLY FORBIDDEN** from modifying any code file (via `replace_file_content`, `write_to_file`, or sed/patch) without first calculating its downstream caller impact.
- **The Dual-Graph Mechanism:**
  - Before writing or modifying code, the agent **MUST** run:
    ```json
    graph_impact(changed_files=["path/to/target_file.ts"])
    ```
  - In its response, the agent **MUST explicitly list**:
    1. Direct callers and consumers of the modified functions.
    2. Upstream interfaces or types affected.
    3. Potential regression risks identified by the dependency graph.
  - If `graph_impact` identifies impacted callers, the agent **MUST** account for them in its plan before modifying the target file.

### LAW 5: Immediate Post-Edit Graph Synchronization
- **The Mandate:** Every file modification creates an immediate delta in the workspace AST. Leaving a turn without synchronizing the action graph degrades memory for subsequent turns and subagents.
- **The Dual-Graph Mechanism:**
  - In the **EXACT SAME TURN** that a file is created, modified, or deleted, the agent **MUST** call:
    ```json
    graph_register_edit(
      files=["path/to/modified_file.ts"],
      summary="Concise 1-sentence imperative description of changes"
    )
    ```

### LAW 6: Compounding Action Memory Discipline
- **The Mandate:** Context memory must compound across turns and subagents without bloating the context store.
- **The Dual-Graph Mechanism:**
  - The agent **MUST** record architectural decisions, completed tasks, watermarks, and blockers via:
    ```json
    graph_add_memory(
      type="decision | task | next | fact | blocker",
      content="One crisp sentence, strictly 15 words maximum.",
      files=["relevant/file.ts"],
      tags=["auth", "pkce"]
    )
    ```
  - **HARD RESTRICTION:** The agent is **STRICTLY FORBIDDEN** from editing `.dual-graph/context-store.json` directly. All writes must go through `graph_add_memory` to ensure automatic health pruning.

### LAW 7: Zero Terminal Testing Policy (Credit & Usage Protection)
- **The Mandate:** To protect the user's credits, tokens, and system stability, the agent is **STRICTLY FORBIDDEN** from running automated terminal test commands on its own initiative:
  - Banned: `npm test`, `pytest`, `cargo test`, `go test`, `playwright`, `cypress`, `vitest`, `npm run build`.
- **The Dual-Graph Mechanism:**
  - Code verification must be performed via **Dual-Graph Static Analysis**:
    1. Trace interface types and return values via `graph_read`.
    2. Traverse caller and callee edges via `graph_neighbors`.
    3. Verify downstream consumer integrity via `graph_impact`.
  - Terminal test commands may be executed **ONLY IF** the user explicitly types the words `"run test"` in their prompt.

### LAW 8: Terminal Command Error Lock (Zero Blind Retries)
- **The Mandate:** If ANY command executed in the terminal fails or returns a non-zero exit code:
  - The agent **MUST HALT ALL EXECUTION IMMEDIATELY**.
  - No trying alternative random commands.
  - No silent retries.
  - No pretending the error did not happen.
- **The Recovery Flow:**
  1. Capture complete command output and error traces.
  2. Log the failure in `.agents/state/issues.md` (or `Heisenberg/state/issues.md`).
  3. Execute a rigorous 5-Whys Root Cause Analysis (RCA) before proposing any fix.

---

## 3. Universal MCP Configuration Registry for AI Coding Agents

If the agent boots into a workspace and detects that GrapeRoot is not registered in its host environment, the agent **MUST find its host client below, inject the exact JSON block, and connect to `http://127.0.0.1:8080/mcp`**:

| AI Coding Client | Configuration File Location | Injection Payload |
|---|---|---|
| **Cursor** | Windows: `%APPDATA%\Cursor\User\mcp.json`<br>macOS: `~/Library/Application Support/Cursor/User/mcp.json`<br>Linux: `~/.config/Cursor/User/mcp.json`<br>Project: `<root>/.cursor/mcp.json` | See Payload A |
| **Windsurf (Codeium)** | Windows: `%USERPROFILE%\.codeium\windsurf\mcp_config.json`<br>macOS/Linux: `~/.codeium/windsurf/mcp_config.json` | See Payload A |
| **Cline (VS Code)** | Windows: `%APPDATA%\Code\User\globalStorage\saoudrizwan.claude-dev\settings\cline_mcp_settings.json`<br>Global: `~/.cline/data/settings/cline_mcp_settings.json`<br>macOS: `~/Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/...` | See Payload A |
| **Roo Code** | Windows: `%APPDATA%\Code\User\globalStorage\rooveterinaryinc.roo-cline\settings\cline_mcp_settings.json`<br>macOS: `~/Library/Application Support/Code/User/globalStorage/rooveterinaryinc.roo-cline/...` | See Payload A |
| **Claude Code (CLI)** | Global: `~/.claude.json`<br>Project: `<root>/.mcp.json` | See Payload A |
| **Claude Desktop** | Windows: `%APPDATA%\Claude\claude_desktop_config.json`<br>macOS: `~/Library/Application Support/Claude/claude_desktop_config.json` | See Payload B (Stdio Bridge) |
| **Antigravity 2.0** | Global: `~/.gemini/antigravity-cli/mcp_config.json` | See Payload A |
| **Universal Workspace** | `<workspace_root>/.mcp.json` | See Payload A |

### Payload A: Streamable HTTP (Standard for Cursor, Windsurf, Cline, Roo, Claude Code, Antigravity)
```json
{
  "mcpServers": {
    "graperoot": {
      "type": "streamableHttp",
      "url": "http://127.0.0.1:8080/mcp",
      "autoApprove": [
        "graph_retrieve",
        "graph_read",
        "graph_neighbors",
        "graph_impact",
        "graph_continue"
      ]
    }
  }
}
```

### Payload B: Stdio Bridge (For Claude Desktop & Stdio-Only Clients)
```json
{
  "mcpServers": {
    "graperoot": {
      "command": "graperoot",
      "args": ["mcp"]
    }
  }
}
```

---

## 4. Lifecycle Execution Checklist

Before closing any task, the agent must check off this list:
- [ ] Session booted with `graph_continue` or `graph_retrieve` (Zero blind file dumps).
- [ ] Targeted files inspected using `graph_read(file::symbol)`.
- [ ] Blast radius calculated using `graph_impact` before touching code.
- [ ] Code modifications registered immediately via `graph_register_edit`.
- [ ] Key architectural decisions logged via `graph_add_memory`.
- [ ] Zero unauthorized terminal tests run (Credit protection preserved).
- [ ] Verified system health via `python Heisenberg/bin/doctor.py --json`.
