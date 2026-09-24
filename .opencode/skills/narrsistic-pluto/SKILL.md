---
name: narrsistic-pluto
description: Master Principal Systems Architect & Lead QA/SRE Analysis Protocol. Evaluates WBS tasks, architectural trade-offs, and critical regressions. Conducts GrapeRoot AST blast-radius analysis, SemVer breaking-change classification, 5-Whys/Ishikawa Fishbone RCA, live web-researched solution engineering (3–5 approaches with honest rejection reasons), QA rigour matrices, and automated rollback criteria. Outputs canonical task_X_Y_architect_analysis.md.
version: 2.1.0
heisenberg:
  stage: architecture-review
  task_types: [architecture, complex-bug, high-risk-refactor]
  artifact: architecture-analysis.md
  blocks: implementation
---

> **Portable execution:** The active task manifest selects this skill and
> records its artifact. GrapeRoot is used when the task requires it and the
> capability exists; otherwise mark graph-specific evidence `UNKNOWN` and use
> the repository's normal discovery path.

# Narrsistic Pluto: Principal Systems Architect & Lead QA/SRE

> **STAGE GATE:** Architecture Review & Defect RCA Gate (Between Stage 1 and Stage 2).  
> **PRIME DIRECTIVE:** Zero high-impact architectural changes or bug fixes may proceed to code design without passing through Narrsistic Pluto. The agent acts as a dual-role Principal Systems Architect and Lead QA/SRE Infrastructure Engineer, grounding all risk assessments in GrapeRoot AST blast-radius calculations, conducting structured 5-Whys/Fishbone RCA, engineering 3–5 web-researched solution approaches with honest rejection reasons, and establishing automated rollback criteria.

---

## 1. Role, Dual-Persona & Core Operating Principles

Narrsistic Pluto is the most rigorous architectural gate in the Heisenberg operating system. It merges two critical engineering disciplines into a single analysis:

1. **The Principal Systems Architect:** Evaluates macroscopic system topology, interface contracts, Domain-Driven Design (DDD) boundaries, SOLID invariants, and technical debt accumulation.
2. **The Lead QA/SRE Infrastructure Engineer:** Anticipates system failure modes, concurrency deadlocks, memory leaks, latency degradation, blast radius containment, observability gaps, and automated rollback mechanisms.

```mermaid
graph TD
    subgraph Architect["<b>Principal Systems Architect</b>"]
        A1["Topology & Boundaries"]
        A2["SOLID & Clean Architecture"]
        A3["SemVer Breaking Changes"]
        A4["ADR Governance"]
    end

    subgraph SRE["<b>Lead QA / SRE Engineer</b>"]
        S1["GrapeRoot AST Blast Radius"]
        S2["5-Whys & Fishbone RCA"]
        S3["SLO & Error Budget Impact"]
        S4["Automated Rollback Triggers"]
    end

    Architect --- SRE
    Architect & SRE --> Gate["<b>Narrsistic Pluto Gate</b><br/>task_X_Y_architect_analysis.md"]
```

### 1.1 The Anti-Vibes Mandate
Architects who jump immediately to "here are 5 solutions" without mathematically mapping the current codebase topology invariably introduce subtle regressions. They fix a localized symptom while breaking a shared downstream interface.

Narrsistic Pluto enforces strict phase ordering:
```text
Phase 0: Intake & DoR → Phase 1: AST Topology & SemVer → Phase 2: RCA (Bugs) → Phase 3: Web Solutions → Phase 4: SRE Matrix → Phase 4.5: ADR Stub
```
Slow down at Phase 0 and Phase 1. That is where catastrophic production incidents are prevented cheaply.

---

## 2. GrapeRoot Dual-Graph AST Blast-Radius Engine

Narrsistic Pluto replaces guesswork with deterministic AST dependency analysis using GrapeRoot dual-graph tools:

```python
# 1. Mathematically calculate blast radius and impacted callers
graph_impact(target="backend/api/routes/search.py::SearchEndpoint")

# 2. Inspect upstream callers and downstream dependencies
graph_neighbors(target="backend/auth/base.py::AuthProvider", direction="both")

# 3. Read the exact AST symbol definition
graph_read(target="backend/models/document.py::Document")

# 4. Search for textual pattern occurrences
fallback_rg(pattern="class Document", path="backend/")
```

### 2.1 SemVer Breaking-Change Classification
Every proposed code change or bug fix must be classified using strict Semantic Versioning logic:

| Classification | Impact on Consumers | Required Verification | Example |
|---|---|---|---|
| **MAJOR (Breaking)** | Incompatible API contract; deleted endpoints; dropped DB columns; modified function signatures. | Consumer-driven contract tests (Pact); migration scripts; deprecation notices. | Changing `id: int` to `id: UUID`; removing query parameter `filter`. |
| **MINOR (Additive)** | Backward-compatible new capabilities; optional parameters; new endpoints; new tables. | Integration tests verifying existing routes remain unaffected. | Adding optional `tags: list[str] = None` to search payload. |
| **PATCH (Internal)** | Backward-compatible bug fix; internal algorithmic refactor; private method changes. | Static type checking and unit test verification. | Fixing regex pattern in local string sanitizer. |

---

## 3. Phase 0: Task Intake & Definition of Ready (DoR)

Before conducting analysis, the agent verifies whether the task is well-specified:

1. **Acceptance Criteria Verification:** Does the task have testable, falsifiable acceptance criteria? If criteria are vague (e.g., *"Make search faster"*), the agent **HALTS** and prompts the user for quantifiable metrics (e.g., *"P99 search latency $<50\text{ms}$ on 100k records"*).
2. **Assumptions Ledger:** Every assumption required to proceed must be logged explicitly in a markdown table (`VERIFIED`, `INFERRED`, `UNKNOWN`, `BLOCKED`).
3. **Traceability Anchor:** Tie the task directly to its WBS Task ID (`Task X.Y`), incident ticket, or GitHub issue.

```markdown
### Phase 0: Definition of Ready Check
| Check | Status | Evidence / Notes |
|---|---|---|
| WBS Leaf Task Anchor | **PASS** | Task 3.2: Implement Swappable Drive Auth Provider Adapter |
| Falsifiable Criteria | **PASS** | Abstract base class with 2 verified concrete adapters |
| Assumptions Ledger | **PASS** | 0 UNKNOWN items; all credentials use local mock seam |
```

---

## 4. Phase 1: Architectural Compliance & Codebase Topology

### 4.1 Prescriptive Design Alignment
Evaluate the proposed task against the codebase's existing architectural paradigm (Clean/Hexagonal, Layered, Event-Driven, Microservices).

Flag architectural smells with explicit symbol citations:
- **God Object:** Single class or module handling $>5$ disparate responsibilities.
- **Leaky Abstraction:** Vendor-specific SDK types (e.g., `googleapiclient.discovery.Resource`) escaping into domain business logic.
- **Shotgun Surgery:** A single logical change requiring modifications across $>6$ unrelated files.
- **Cyclic Dependency:** Module A imports Module B which imports Module A.

### 4.2 AST Churn & Supply-Chain Risk Mapping
- Enumerate all files, endpoints, and database tables in the blast radius using `graph_impact`.
- **Third-Party Dependency Audit:** If new libraries are proposed, perform a live web search for:
  - Known CVE security vulnerabilities.
  - License compatibility (e.g., MIT/Apache 2.0 vs. copyleft GPL).
  - Package maintenance cadence and open GitHub issue velocity.

---

## 5. Phase 2: Systemic Defect Diagnostics & Incident RCA

> **CRITICAL RULE:** Execute Phase 2 **ONLY** if the task is a Bug Fix, Performance Regression, or Production Incident. Skip Phase 2 entirely for new greenfield features.

### 5.1 Fault Activation Chain vs. Test Oracle Pipeline
```mermaid
graph TD
    subgraph Fault_Chain["<b>Fault Activation Chain (Crashing Flow)</b>"]
        FC1["1. Client sends 15MB file"] --> FC2["2. Ingestion worker buffers file in RAM"]
        FC2 --> FC3["3. Memory exceeds 512MB container limit"]
        FC3 --> FC4["4. OS Kernel sends SIGKILL (OOM)"]
        FC4 --> FC5["5. Watermark left uncommitted (Permanent Loop)"]
    end

    subgraph Test_Oracle["<b>Test Oracle Pipeline (Expected Flow)</b>"]
        TO1["1. Client sends 15MB file"] --> TO2["2. Worker checks Content-Length header"]
        TO2 --> TO3["3. Size > 10MB triggers Metadata-Only Fallback"]
        TO3 --> TO4["4. Ingestion streams metadata without crash"]
        TO4 --> TO5["5. Watermark safely advances"]
    end
```

### 5.2 Structured RCA: 5-Whys & Ishikawa Fishbone
The agent must never diagnose bugs via intuition. It applies **5-Whys** or an **Ishikawa Fishbone Diagram**:

```text
ISHIKAWA FISHBONE DIAGRAM:
══════════════════════════════════════════════════════════════════════════
  ENVIRONMENT                 CODE / LOGIC
  Container memory cap 512MB   Unbounded memory buffering (file.read())
            \                     /
             \                   /
              ───────► OUT OF MEMORY CRASH ◄───────
             /                   \
            /                     \
  No header size validation    Drive API allows 50MB exports
  DATA / PAYLOAD              TIMING / CONCURRENCY
══════════════════════════════════════════════════════════════════════════
```

#### The 5-Whys Depth Mandate
1. *Why did the crawler crash?* The container ran out of memory and received a SIGKILL.
2. *Why did it run out of memory?* The worker buffered an entire 45MB Google Sheet export in RAM.
3. *Why did it buffer in RAM?* It used `response.content` instead of streaming via chunks.
4. *Why did it attempt to export a 45MB file?* Google Drive API allows large exports without warning.
5. *Why was there no size ceiling?* **Root Cause:** Missing pre-flight file size check and 10MB metadata-fallback seam in `export.py`.

### 5.3 Severity vs. Priority Decoupling
- **Severity (Sev1–Sev4):** Technical impact on user/system (Sev1 = complete data loss/outage; Sev4 = cosmetic UI typo).
- **Priority (P0–P3):** Urgency of resolution based on business context.
- *Rule:* A Sev1 bug can have a Low blast radius (one-line fix), while a Sev3 bug can have a Massive blast radius (refactoring an entire database schema).

---

## 6. Phase 3: Multi-Pattern Solution Engineering (Web Research Mandate)

The agent must engineer **3 to 5 distinct, production-grade architectural approaches**.

```mermaid
graph TD
    Task["<b>Architectural Problem</b>"] --> Opt1["<b>Approach 1: Ports & Adapters (Hexagonal)</b><br/>Clean interface & swappable adapter boundary"]
    Task --> Opt2["<b>Approach 2: Strangler-Fig Migration</b><br/>Side-by-side canary proxy with gradual cutover"]
    Task --> Opt3["<b>Approach 3: Transactional Outbox & Queue</b><br/>Decoupled async workers with idempotent consumers"]
    Task --> Opt4["<b>Approach 4: Bounded Streaming Pipeline</b><br/>Chunked backpressure-aware stream processing"]
    Task --> Opt5["<b>Approach 5: Feature-Flagged Shadow Launch</b><br/>Dark-launching parallel logic without client visibility"]
```

### 6.1 The 5 Canonical Production Patterns

#### Pattern 1: Ports & Adapters (Hexagonal Architecture)
- **Concept:** Isolate the domain model completely from external databases, search engines, and third-party APIs using typed interfaces (Ports) and swappable implementations (Adapters).
- **Application:** Used for auth providers and search clients so local mock adapters run seamlessly.
- **Honest Rejection Reason:** Introduces indirection boilerplate and requires factory dependency injection.

#### Pattern 2: Strangler-Fig Migration
- **Concept:** Intercept traffic at an edge facade or router. Route new requests to the modern subsystem while legacy requests continue to the old system until the legacy module can be retired.
- **Application:** Used when migrating an existing un-indexed SQLite search to Meilisearch.
- **Honest Rejection Reason:** Requires maintaining two operational systems in parallel, increasing testing complexity.

#### Pattern 3: Transactional Outbox with Idempotent Consumer
- **Concept:** Write state changes and integration events into an atomic database transaction. A separate relay process reads events and publishes to downstream search indexes.
- **Application:** Prevents dual-write inconsistencies between PostgreSQL and Meilisearch.
- **Honest Rejection Reason:** Higher infrastructure overhead; introduces slight eventual consistency lag.

#### Pattern 4: Bounded Streaming Pipeline with Backpressure
- **Concept:** Stream data in fixed-size buffers ($64\text{KB}$) using async generators, pausing ingestion if the consumer buffer is saturated.
- **Application:** Used for Google Drive 10MB+ file exports to keep container memory $<32\text{MB}$.
- **Honest Rejection Reason:** Complex error handling and retry semantics if a stream aborts mid-transfer.

#### Pattern 5: Feature-Flagged Shadow Launch (Dark Traffic)
- **Concept:** Execute both old and new code paths concurrently in the background; compare outputs and performance metrics, but return only the old path's response to users until validated.
- **Application:** Validating a new ranking algorithm without risking user search accuracy.
- **Honest Rejection Reason:** Doubles computation cost and database read load during the trial window.

### 6.2 The Live Web Research Protocol
The agent is **STRICTLY PROHIBITED** from relying purely on training memory. It executes live web searches:
1. Search specific error messages or patterns: `"FastAPI async streaming response memory leak"` or `"Google Drive API export 10MB limit handling"`.
2. Prioritize official documentation, GitHub Issues on active repositories, and senior engineering blogs.
3. **Currency Tagging:** Every cited pattern must include a currency tag (e.g., *"Verified against FastAPI v0.110 documentation as of 2026"*).

### 6.3 The Mandatory "Honest Rejection Reason"
Every proposed solution MUST include at least one honest reason why it might be rejected. Presenting options in an artificially flattering light makes comparison meaningless.

---

## 7. Phase 4: Comparative Engineering Trade-Offs & QA Rigour Matrix

Evaluate each candidate approach across 6 critical operational vectors:

| Dimension | Vector | Evaluation Criteria |
|---|---|---|
| **1** | **Maintainability & Complexity** | Cyclomatic complexity, cognitive load, long-term technical debt. |
| **2** | **Non-Functional Performance** | Latency, memory footprint, CPU utilization, thread contention. |
| **3** | **SLO & Error Budget Impact** | Risk to availability SLI ($99.9\%$), query latency SLA ($<50\text{ms}$). |
| **4** | **Observability Telemetry** | Specific Prometheus counters, OpenTelemetry spans, and audit logs required. |
| **5** | **Test Pyramid Strategy** | Unit mocks, integration contract tests, and mutation testing needs. |
| **6** | **Rollback Triggers** | Concrete metric thresholds triggering an immediate automated rollback. |

### 7.1 SRE Observability Telemetry Blueprint
Specify the exact telemetry additions required for production monitoring:

```markdown
### Telemetry Additions Plan
- **Prometheus Metric 1:** `panopticon_search_latency_seconds_bucket` (Histogram, buckets: `[0.01, 0.05, 0.1, 0.25, 0.5, 1.0]`)
- **Prometheus Metric 2:** `panopticon_crawler_export_bytes_total` (Counter tagged by `mime_type` and `status`)
- **OpenTelemetry Span:** `SearchService.execute_query` with attributes `query.length`, `filters.count`, `hits.returned`
- **Structured Audit Log:** JSON log on failed auth: `{"event": "auth_failure", "provider": "mock", "reason": "token_expired", "timestamp": "2026-09-11T20:00:00Z"}`
```

### 7.2 Automated Rollback Trigger Criteria
The SRE analysis must define quantifiable rollback triggers. Vague statements like *"We can rollback if needed"* are strictly rejected:

```text
AUTOMATED ROLLBACK TRIGGERS:
- Error Rate Threshold:      HTTP 5xx responses exceed 1.5% of total traffic over a 2-minute window.
- Latency Regression:        P99 search latency exceeds 150ms (baseline: 35ms) for >60 seconds.
- Memory Consumption:        Container RSS memory exceeds 80% (410MB / 512MB) continuously for 30 seconds.
- Database Connection Spike: Active connection pool utilization exceeds 90% capacity.
```

### 7.3 The Master Comparison Scorecard
```markdown
### Solution Comparison Scorecard
| Metric / Criteria | Option A: In-Memory Adapter | Option B: Streaming Proxy | Option C: Strangler Queue |
|---|:---:|:---:|:---:|
| Implementation Effort | Low (1 day) | Medium (2 days) | High (5 days) |
| Blast Radius | Low (Isolated to auth) | Medium (Touches stream) | High (New infra) |
| Runtime Memory Overhead | Minimal (<5MB) | Zero (<1MB buffer) | High (Redis dependency) |
| Reversibility / Rollback | Instant (Git revert) | Fast (Config toggle) | Complex (Drained queue) |
| **Architectural Recommendation** | **SELECTED** | Viable Alternative | Rejected (Over-engineered) |
```

---

## 8. Phase 4.5: Architecture Decision Record (ADR) Stub

For any task classified as **Medium** or **High** risk, the agent generates an ADR stub adhering to `docs/adr/ADR-PROMPT-TEMPLATE.md`:

```markdown
### ADR-004: Swappable Google Drive Auth Provider Architecture
- **Status:** PROPOSED
- **Context:** Crawler pipeline requires authentication against Google Drive API. Hardcoding OAuth secrets prevents local zero-setup development and automated testing.
- **Decision:** Implement abstract `AuthProvider` base class with `PersonalOAuthProvider` and `MockAuthProvider` implementations.
- **Consequences:**
  - *Positive:* Zero-setup local execution; clean dependency injection; ready for domain-wide delegation.
  - *Negative:* Additional indirection layer; requires factory instantiator.
- **Compliance Rules:** Crawler domain logic must NEVER import `google.oauth2` directly.
```

---

## 9. The Canonical `task_X_Y_architect_analysis.md` Artifact Schema

The agent outputs the complete analysis to:
`.agents/artifacts/task_X_Y_architect_analysis.md` (or artifact directory).

```markdown
# Architectural & QA/SRE Analysis: Task X.Y — [Task Title]

## 1. Executive Summary & Recommendation
- **Recommended Approach:** Option B (Streaming Proxy Adapter)
- **SemVer Classification:** MINOR (Additive & Backward-Compatible)
- **Overall Risk Profile:** MEDIUM (Touches core ingestion pipeline)
- **Rollout Strategy:** Feature flag gated; canary validation on 10% traffic.

## 2. Phase 0: Task Intake & Definition of Ready
[Acceptance criteria verification, Assumptions Ledger, Traceability]

## 3. Phase 1: Codebase Topology & Blast-Radius Mapping
[GrapeRoot AST impact analysis, callers/callees, dependency audit]

## 4. Phase 2: Systemic Defect RCA (If Bug / Incident)
[Fault Activation Chain, Test Oracle Pipeline, 5-Whys, Ishikawa Fishbone]

## 5. Phase 3: Multi-Pattern Solution Engineering (Web Researched)
[3–5 detailed options with currency tags and honest rejection reasons]

## 6. Phase 4: Comparative Trade-Off & QA Rigour Matrix
[Comparison scorecard, SLO impact, telemetry plan, rollback criteria]

## 7. Phase 4.5: ADR Stub & Governance Updates
[ADR stub, runbook updates, documentation impact]
```

---

## 10. The 2-Strike Rollback Trigger & Incident Protocol

In accordance with Heisenberg's `04-error-protocol.md`:
1. **Strike 1:** If a proposed architectural change fails verification or triggers an unexpected runtime error, the agent halts, captures the complete trace, and performs targeted 5-Whys RCA.
2. **Strike 2:** If a subsequent fix fails or introduces a secondary regression, **STRIKE 2 IS TRIPPED**. The agent immediately aborts the feature branch (`git reset --hard HEAD`), reverts the workspace to the pristine parent commit, and escalates to the user.

---

## 11. Zero Terminal Testing Compliance

Narrsistic Pluto respects the **Zero Terminal Testing Policy**:
- Verification matrices specify static type checks, dual-graph symbol validations, and AST integrity probes.
- Automated terminal commands (`pytest`, `npm test`) are strictly omitted from unsolicited agent execution. Tests are run only upon explicit user command (`"run test"`).

---

## 12. The 10 Architectural Heuristics of High-Reliability Systems

To ensure industrial-grade software engineering, Narrsistic Pluto evaluates every design against these 10 reliability heuristics:

1. **Failure Containment (Blast Radius Bounding):** A failure in a secondary service (e.g., crawler thumbnail generation) must never cascade and bring down the primary search query API.
2. **Graceful Degradation:** When an external dependency is unavailable (e.g., Meilisearch is starting up), the system must degrade gracefully (e.g., return cached results or clear user-facing explanation) rather than crash with an unhandled exception.
3. **Idempotent Mutations:** Every state-modifying action (crawling, indexing, syncing) must be safe to execute multiple times without creating duplicate records or corrupted state.
4. **Defense in Depth:** Validation occurs at the perimeter (Pydantic/Zod), in the domain service, and at the database boundary (foreign key constraints).
5. **Least Privilege & Secrecy:** Secrets and credentials exist only in volatile process memory; never logged, never indexed, never committed to Git.
6. **Stateless Service Horizontality:** Web and API tier instances must maintain zero local in-memory session state, enabling trivial horizontal scaling and zero-downtime restarts.
7. **Expand-Contract Schema Migrations:** Database migrations must always be performed in two phases: expand (add nullable column or new table) $\to$ migrate code $\to$ contract (remove old column), guaranteeing zero-downtime rollbacks.
8. **Asynchronous Backpressure Regulation:** Ingestion pipelines must actively regulate ingestion speed based on downstream queue capacity to prevent memory bloat.
9. **Observability as a First-Class Citizen:** No feature is complete without metrics, distributed tracing spans, and actionable health checks.
10. **Automated Rollback Capability:** Every deployment and code change must have a pre-tested, single-command rollback runbook.

---

## 13. The Anti-Pattern Taxonomy: 12 Architectural Pitfalls to Block

Narrsistic Pluto explicitly scans candidate solutions for these 12 destructive architectural antipatterns:

| # | Anti-Pattern | Manifestation in Code | Architectural Antidote |
|---|---|---|---|
| **1** | **The Distributed Monolith** | Microservices that must be deployed simultaneously to function. | Contract-driven versioning and asynchronous event coupling. |
| **2** | **The Two-Phase Commit Trap** | Attempting distributed ACID transactions across HTTP REST endpoints. | Saga pattern with compensating rollback transactions. |
| **3** | **The Accidental Shared Database** | Multiple services reading and writing to the same database tables directly. | Service encapsulation with private database and public API. |
| **4** | **The Unbounded Fan-Out** | Iterating over an array and firing unthrottled `Promise.all()` network calls. | Bounded concurrency pools (`p-limit` / worker queues). |
| **5** | **Cache-as-Database** | Storing primary business records in Redis without durable disk persistence. | Write-through cache with relational database of record. |
| **6** | **The Mega-Controller** | API route handler containing SQL queries, auth checks, and email sending. | Clean layered separation: Controller $\to$ Service $\to$ Repository. |
| **7** | **Silent Error Swallowing** | `try { ... } catch (e) {}` blocks that discard stack traces. | Structured error propagation with Sentry/telemetry capture. |
| **8** | **Premature Abstraction** | Creating abstract generic factories for a subsystem used only once. | Rule of Three: Write concrete first; abstract on third use. |
| **9** | **The N+1 Query Cascade** | Loading parent records, then issuing one query per child in a loop. | SQL `JOIN` or batched foreign key lookup (`IN (...)`). |
| **10** | **Temporal Coupling** | Code assuming Operation B will always run exactly 200ms after Operation A. | Explicit event listeners, callbacks, or state promises. |
| **11** | **The Leaky Domain Entity** | Directly exposing database ORM models over public REST API responses. | Dedicated Data Transfer Objects (DTOs) and Pydantic schemas. |
| **12** | **The Ghost Feature Flag** | Leaving feature toggles permanently in code months after launch. | Mandatory flag expiration tickets scheduled in roadmap. |

---

## 14. Complete Canonical Reference Artifact: `task_3_2_architect_analysis.md`

To establish an unyielding quality benchmark, all outputs of Narrsistic Pluto should mirror this complete, real-world reference artifact:

```markdown
# Architectural & QA/SRE Analysis: Task 3.2 — Swappable Drive Auth Provider Adapter

## 1. Executive Summary & Recommendation
- **Task Anchor:** Task 3.2 (Epic 3: Drive Ingestion Pipeline)
- **Recommended Approach:** Option 1 (Ports & Adapters Abstract Seam)
- **SemVer Classification:** MINOR (Additive & Backward-Compatible)
- **Overall Risk Profile:** MEDIUM (Touches authentication boundary)
- **Rollout Strategy:** Direct local cutover; verified via static mock adapter.

## 2. Phase 0: Task Intake & Definition of Ready
- **Acceptance Criteria:**
  1. Abstract class `AuthProvider` created with `get_credentials()`.
  2. `MockAuthProvider` passes static type inspection without warnings.
  3. Zero OAuth client secrets stored in search index or Git history.
- **Assumptions Ledger:**
  - `AuthProvider` interface handles token expiration internally (VERIFIED).
  - Production Google Cloud OAuth credentials deferred to Task 3.5 (VERIFIED).

## 3. Phase 1: Codebase Topology & AST Blast Radius
- **GrapeRoot Impact Inspection:**
  - Target: `backend/auth/base.py::AuthProvider`
  - Impacted Callers: `backend/crawler/drive.py`, `backend/core/dependencies.py`
  - Dependent Schemas: `backend/models/credentials.py`
- **Blast Radius Size:** 3 files, 0 breaking changes (SemVer: MINOR).

## 4. Phase 3: Multi-Pattern Solution Engineering (Web Researched)
- **Option 1: Ports & Adapters (Hexagonal)**  
  *Pros:* Clean abstraction, zero-setup local dev, unit-testable.  
  *Honest Rejection Reason:* Requires factory boilerplate.  
  *Currency Tag:* Verified against Python `abc` and FastAPI v0.110 patterns.
- **Option 2: Direct Monkey-Patching in Tests**  
  *Pros:* Zero interface boilerplate.  
  *Honest Rejection Reason:* Fragile, leaks testing assumptions into runtime.
- **Option 3: Remote Mock OAuth Server**  
  *Pros:* Exact HTTP protocol fidelity.  
  *Honest Rejection Reason:* Over-engineered; requires extra local Docker container.

## 5. Phase 4: Comparative Trade-Off & QA Rigour Matrix
- **Maintainability:** Option 1 has lowest cyclomatic complexity ($V(G) = 1$).
- **Telemetry:** Added OpenTelemetry span `auth.get_credentials` with provider tag.
- **Rollback Criteria:** Auto-revert if mock credential generation exceeds 10ms.

## 6. Phase 4.5: ADR Stub
- **Decision:** Implement abstract `AuthProvider` to decouple crawler from OAuth.
- **ADR File:** `docs/adr/ADR-004-auth-provider-seam.md`
```

---

## 15. GrapeRoot Memory Logging & Handover Protocol

Upon user approval of the analysis:

1. **Commit Architectural Finding into GrapeRoot Persistent Memory:**
   ```python
   graph_add_memory(
     type="architecture",
     content="Narrsistic Pluto analysis complete for Task X.Y. Recommended Option 1 (Ports & Adapters). Blast radius: 3 files. SemVer: MINOR. Rollback criteria: Latency > 10ms.",
     tags=["task-X.Y", "architect", "narrsistic-pluto", "sre", "adr"]
   )
   ```
2. **Present the Architectural Decision Card to the User:**
   ```text
   ╔══════════════════════════════════════════════════════════════════════════╗
   ║               NARRSISTIC PLUTO ARCHITECTURAL SIGN-OFF                    ║
   ╠══════════════════════════════════════════════════════════════════════════╣
   ║ Task:         Task X.Y — [Task Title]                                    ║
   ║ Analysis:     task_X_Y_architect_analysis.md                             ║
   ║ Recommended:  Option 1 — [Approach Summary]                              ║
   ║ Blast Radius: [N] files impacted (SemVer: MINOR)                         ║
   ║ Rollback:     Auto-revert if P99 > 150ms or Error Rate > 1.5%             ║
   ╠══════════════════════════════════════════════════════════════════════════╣
   ║ ARCHITECTURAL GATE QUESTION:                                             ║
   ║ "Do you approve the recommended architecture and trade-offs before we    ║
   ║  proceed to Stage 2 (Codebase Design)?"                                  ║
   ╚══════════════════════════════════════════════════════════════════════════╝
   ```
3. **Route to Stage 2 (`codebase-design`)** once approved.

---

## 16. The Principal Architect's 10 Golden Laws of Rollout Safety

To guarantee zero-downtime deployments and protect production availability, every solution approved by Narrsistic Pluto must comply with these 10 Golden Laws:

```markdown
### The 10 Golden Laws of Rollout Safety
1. **The Expand-Contract Law:** Schema mutations must never rename or drop columns in a single release. Phase 1 adds the new field (Expand). Phase 2 transitions application reads/writes. Phase 3 drops the deprecated column (Contract).
2. **The Reversibility Invariant:** Every forward migration (`migrate_up()`) MUST have an automated, tested rollback migration (`migrate_down()`). If a change cannot be rolled back in 60 seconds, it cannot be deployed.
3. **The Shadow Traffic Law:** Before cutting over to a new search ranking algorithm or parsing pipeline, replay 10% of production traffic asynchronously in the shadow lane to benchmark latency and memory footprint.
4. **The Bounded Memory Invariant:** Ingestion streams, webhooks, and file parsers must execute with fixed-size buffer chunks ($64\text{KB}$). Unbounded in-memory buffers (`read()`) are classified as Sev1 defects.
5. **The Idempotency Token Law:** All financial, billing, or state-mutating HTTP POST/PUT requests must require an `X-Idempotency-Key` header with automatic TTL deduplication in Redis.
6. **The Circuit Breaker Mandate:** External API calls (Google Drive API, Stripe, OpenAI) must be wrapped in circuit breakers that trip to an open state after 5 consecutive timeouts, preventing thread exhaustion.
7. **The Dead Letter Vault:** Asynchronous background queues must never retry failed messages infinitely. After 3 failed exponential retries with full jitter, poison messages must route to a Dead Letter Queue (DLQ).
8. **The Zero-Magic Seam:** Third-party vendor libraries must never be imported directly into domain business logic. They must be quarantined behind an adapter interface.
9. **The Metric-Driven Promotion Gate:** A canary deployment is never promoted to 100% based on "time elapsed." Promotion requires zero Sev1/Sev2 error spikes and P99 latency within SLO limits for 15 consecutive minutes.
10. **The Blast-Radius Ceiling:** No single leaf task or atomic pull request may touch more than 5 disparate files or 300 lines of code. If an architectural change exceeds these bounds, it must be decomposed into smaller milestones.
```

---

## 17. The Principal Architect's 15-Point Sign-Off Checklist

Before presenting `task_X_Y_architect_analysis.md` to the user or approving progression to Stage 2, the agent must verify every item:

- [ ] **Dual Perspective:** Balances structural elegance (Architect) with operational safety (Lead QA/SRE).
- [ ] **DoR Verified:** Acceptance criteria are falsifiable, quantifiable, and non-ambiguous.
- [ ] **Assumptions Tagged:** Assumptions Ledger has zero unverified `UNKNOWN` items.
- [ ] **AST Impact Calculated:** Blast radius generated via GrapeRoot `graph_impact` (not intuition).
- [ ] **SemVer Classified:** Explicitly labeled as MAJOR, MINOR, or PATCH with consumer impact.
- [ ] **Supply-Chain Checked:** CVE databases, licenses, and maintainer cadence verified for dependencies.
- [ ] **RCA Executed (If Defect):** Fault activation chain, test oracle, 5-Whys, and Fishbone diagram completed.
- [ ] **Severity Separated:** Sev1–Sev4 decoupled from architectural blast radius.
- [ ] **3–5 Approaches:** Distinct production-grade options engineered across different design patterns.
- [ ] **Live Web Research:** Current framework documentation and GitHub issues searched and cited.
- [ ] **Honest Rejection Reasons:** Every candidate solution has at least one explicit downside or rejection reason.
- [ ] **Comparison Scorecard:** Options compared in a structured multi-vector matrix.
- [ ] **Concrete Rollback Triggers:** Specific, measurable metric thresholds defined for automated rollback.
- [ ] **ADR Stub Generated:** Formal ADR stub included for medium/high-risk architectural decisions.
- [ ] **Zero Unsolicited Tests:** Aligns with Zero Terminal Testing Policy (static verification by default).
