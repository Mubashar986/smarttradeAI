# SmartTradeAI — System Design & Modeling Framework

> **What this is:** A step-by-step framework you follow BEFORE coding.  
> **Goal:** Reach 80%+ design clarity so coding becomes "filling in the blanks."  
> **How to use:** Complete each phase in order. Each phase has a checklist. Don't move forward until checked.

---

## How This Framework Works

```
Phase 1 → Phase 2 → Phase 3 → ... → Phase 9
  │                                      │
  └──── Each phase produces an ARTIFACT ──┘
        (a document, a diagram, a decision)
        
When all 9 artifacts exist → you are ready to code.
```

**Rules:**
- Complete phases **in order** — each one builds on the previous
- Each phase produces a **concrete artifact** (not just "understanding")
- If you can't complete a phase, it means the previous phase has gaps — go back
- Ask yourself after each phase: *"Could a new developer read this and understand what to build?"*

---

## Phase 1: System Context — "What is my system and what is NOT my system?"

### What It Is
A single diagram showing your system as ONE box, surrounded by every external thing it talks to. No internal details.

### Why It Matters
If you don't know what's inside vs. outside your boundary, you'll accidentally try to build things that aren't yours (like a broker, or MT5 itself), or forget to plan for things you depend on (like the LLM API going down).

### What to Produce

**Artifact: System Context Diagram**

For SmartTradeAI, your context diagram has:
- **Center box:** SmartTrade System
- **External actors (people):** Retail Trader, System Administrator
- **External systems:** LLM Provider (OpenAI), Pinecone, MetaTrader 5 Terminal, TradingView, Broker

For each connection, write:
```
[Actor/System] --what data flows--> [SmartTrade]
[SmartTrade] --what data flows--> [Actor/System]
```

Example:
```
Retail Trader  --"natural language strategy description"--> SmartTrade
SmartTrade     --"generated code, backtest report, explanations"--> Retail Trader

LLM Provider   --"generated text/code"--> SmartTrade  
SmartTrade     --"prompt with context"--> LLM Provider

SmartTrade     --"trade signals (JSON)"--> MetaTrader 5
MetaTrader 5   --"execution confirmations, market data"--> SmartTrade
```

### How to Research This
- Read your own proposal's Section 7.2 (System Architecture)
- List every external service mentioned in all 3 documents
- Ask: "Do I build this, or do I just connect to it?"

### ✅ Done Checklist
- [ ] Drew the context diagram (pen/paper or draw.io)
- [ ] Listed every external system with data flowing in BOTH directions
- [ ] Confirmed: "I am NOT building MT5, NOT building TradingView, NOT building the LLM"
- [ ] Every external dependency has a failure question answered: "What happens if X is unavailable?"

---

## Phase 2: Requirements Freeze — "What EXACTLY must my system do?"

### What It Is
Take the requirements from your SRS document and organize them into a **priority matrix** with clear pass/fail criteria. The goal is to separate "must have for demo" from "nice to have."

### Why It Matters
Your SRS has 15 functional requirements, 9 non-functional requirements, 13 data requirements. You CANNOT build all of them equally. You need to know which ones are **demo-critical** (your FYP evaluator will test these) and which are **stretch goals**.

### What to Produce

**Artifact: Prioritized Requirements Table**

| ID | Requirement | Priority | MVP? | Acceptance Test |
|----|------------|----------|------|-----------------|
| FR-01 | Accept natural language input | Critical | ✅ | User types strategy → system processes it |
| FR-02 | Clarification loop | Critical | ✅ | If SL missing → system asks for it |
| FR-04 | RAG-enhanced code generation | Critical | ✅ | Generated MQL5 code uses correct syntax |
| FR-07 | Historical backtesting | Critical | ✅ | Strategy produces profit/loss metrics |
| FR-09 | Risk control enforcement | Critical | ✅ | System injects SL if user doesn't provide |
| FR-12 | MT5 execution | Critical | ✅ | Signal reaches MT5 demo |
| FR-13 | Kill switch | Critical | ✅ | Button press halts all strategies |
| FR-06 | Cross-platform (Pine Script) | High | ❌ | Add after MQL5 works |
| FR-08 | Stress testing | High | ❌ | Add after basic backtest works |
| FR-14 | Audit logging | High | Partial | Basic logs, not full audit trail |
| FR-15 | Multi-user isolation | Medium | ❌ | Single-user for MVP |

### How to Research This
- Re-read SRS Chapter 3 (your doc3)
- For each FR, ask: *"If this is missing, can I still demo the system end-to-end?"*
- If yes → it's not MVP. If no → it IS MVP.

### ✅ Done Checklist
- [ ] Every FR, NFR has a priority (Critical / High / Medium / Low)
- [ ] MVP scope is defined (≤ 8-10 requirements)
- [ ] Each MVP requirement has a ONE-SENTENCE acceptance test
- [ ] Confirmed with supervisor: "This MVP scope is acceptable for demo"

---

## Phase 3: Use Case Modeling — "What can each user actually DO?"

### What It Is
A list of every distinct action a user can perform, described as a step-by-step scenario — not technology, not code, just *"user does X, system does Y."*

### Why It Matters
Use cases are the bridge between requirements (what the system must do) and design (how it does it). Each use case becomes one testable workflow. If you can't write the use case, you don't understand the requirement yet.

### What to Produce

**Artifact: Use Case Descriptions** (one per use case)

Write each use case in this format:

```
USE CASE: UC-01 — Create Strategy from Natural Language
ACTOR: Retail Trader
PRECONDITION: User is logged in
TRIGGER: User types a strategy description

MAIN FLOW:
1. User enters strategy description in chat
2. System classifies the intent as STRATEGY_CREATION
3. System checks for missing parameters
4. IF parameters missing → system asks clarification question
5. User provides clarification
6. System retrieves relevant code templates from knowledge base
7. System generates MQL5 code using LLM + templates
8. System validates generated code (syntax check)
9. IF syntax errors → system auto-corrects (up to 3 tries)
10. System displays generated code + plain-English explanation to user
11. System saves strategy as DRAFT

ALTERNATIVE FLOWS:
- 4a. No parameters missing → skip to step 6
- 9a. Code fails after 3 retries → show error, ask user to simplify

POSTCONDITION: Strategy exists in DRAFT status with generated code
```

**Your MVP use cases:**
- UC-01: Create Strategy from Natural Language
- UC-02: Review Generated Strategy
- UC-03: Run Backtest on Strategy
- UC-04: Approve and Deploy Strategy
- UC-05: Emergency Kill Switch
- UC-06: Export Strategy Code

### How to Research This
- Read your own SRS sections 3.3 (User Requirements) and 3.4 (Functional Requirements)
- For each UR, ask: *"What does the user click/type/see at each step?"*
- Walk through it mentally like you're the user sitting at the screen

### ✅ Done Checklist
- [ ] 6-8 use cases written in the format above
- [ ] Each use case has: Actor, Precondition, Trigger, Main Flow, Alternative Flows, Postcondition
- [ ] Every MVP functional requirement maps to at least one use case
- [ ] You can verbally walk someone through each use case without hesitation

---

## Phase 4: Domain Model — "What are the core THINGS in my system?"

### What It Is
A list of the key **entities** (nouns) in your system and how they relate to each other. Not database tables yet — just the business concepts.

### Why It Matters
Before you can design databases, APIs, or classes, you need to know the "vocabulary" of your system. If two developers use different words for the same thing, you get bugs.

### What to Produce

**Artifact: Domain Entity List with Relationships**

```
ENTITIES:
- User (has login credentials, owns strategies)
- Strategy (has raw intent, generated code, status, robustness score)
- Conversation (a session of user messages and system responses)
- ClarificationQuestion (a specific question asked by the system)
- BacktestResult (metrics from running a strategy on historical data)
- TradeSignal (a command to buy/sell sent to MT5)
- AuditEvent (a logged system action)
- MarketData (OHLCV price history for an instrument)

RELATIONSHIPS:
- User OWNS many Strategies
- Strategy BELONGS TO one User
- Strategy HAS one Conversation
- Conversation CONTAINS many ClarificationQuestions
- Strategy HAS many BacktestResults
- Strategy GENERATES many TradeSignals
- TradeSignal CREATES one AuditEvent
- BacktestResult USES MarketData
```

### How to Research This
- Read through your use cases from Phase 3
- Underline every **noun** — those are your candidate entities
- Ask: *"Is this a thing the system needs to remember?"* — If yes, it's an entity

### ✅ Done Checklist
- [ ] 8-12 domain entities identified
- [ ] Each entity has 3-5 key attributes listed
- [ ] Relationships between entities mapped (one-to-many, many-to-many)
- [ ] No two entities mean the same thing (no duplicates/synonyms)

---

## Phase 5: Architecture Definition — "How is my system organized into layers and services?"

### What It Is
Define the major layers, what technology each layer uses, and how they communicate. This is where you make the BIG technology decisions.

### Why It Matters
Architecture decisions are the HARDEST to change later. Choosing PostgreSQL vs. MongoDB, FastAPI vs. Django, Backtrader vs. vectorbt — if you change these mid-project, you rewrite entire subsystems.

### What to Produce

**Artifact: Architecture Decision Record (ADR)**

For EACH major decision, write:

```
DECISION: Database Technology
CONTEXT: We need to store strategies, users, audit logs (relational) 
         and time-series market data
OPTIONS CONSIDERED:
  1. MongoDB — flexible schema, but weak for relational queries
  2. PostgreSQL + TimescaleDB — strong relational + time-series extension
  3. PostgreSQL + separate InfluxDB — two databases to maintain
DECISION: PostgreSQL + TimescaleDB extension
REASON: Single database engine, relational for strategies/users, 
        TimescaleDB extension for OHLCV. Fewer moving parts.
CONSEQUENCE: Must learn PostgreSQL + TimescaleDB hypertables
```

**Decisions you MUST freeze:**

| Decision | Recommended Choice | Freeze? |
|----------|-------------------|---------|
| Backend framework | FastAPI | 🔒 Freeze |
| Frontend framework | React | 🔒 Freeze |
| Database | PostgreSQL + TimescaleDB | 🔒 Freeze |
| Cache / Queue | Redis | 🔒 Freeze |
| Vector DB | Pinecone | 🔒 Freeze |
| Task queue | Celery | 🔒 Freeze |
| LLM Provider | OpenAI (primary) | 🔒 Freeze |
| Backtesting | Backtrader | 🔒 Freeze |
| Containerization | Docker Compose | 🔒 Freeze |
| AI framework | LangChain / LangGraph | 🔒 Freeze |

### How to Research This
- For each decision, search: `"[Option A] vs [Option B] for [your use case]"`
- Read 2-3 comparison articles, focus on **trade-offs**, not hype
- Ask: *"Which one has more tutorials and community support?"* (matters when you're learning)

### ✅ Done Checklist
- [ ] All 10 technology decisions have written ADRs
- [ ] No unresolved "OR" in the tech stack (e.g., no more "Backtrader or vectorbt")
- [ ] The MongoDB vs PostgreSQL conflict from your proposal is resolved
- [ ] You can explain WHY you chose each technology in one sentence

---

## Phase 6: Component Design — "What are the modules inside each layer?"

### What It Is
Break each layer from Phase 5 into its internal components. Define what each component does, what it receives, and what it produces.

### Why It Matters
This is the blueprint developers read to know which module to create, which file to write, and which team member owns what. Without this, two developers will build overlapping code.

### What to Produce

**Artifact: Component Specification Table**

Use the Level 3 decomposition from the technical review, but add INPUT/OUTPUT:

```
COMPONENT: AmbiguityDetector
LAYER: Cognitive Engine (SS3.2)
INPUT: Structured intent from SemanticRouter
OUTPUT: Either "all clear" signal OR a ClarificationQuestion object
LOGIC:
  - Check: Does intent have entry conditions? 
  - Check: Does intent have exit conditions?
  - Check: Is stop-loss defined?
  - Check: Is timeframe specified?
  - Check: Is instrument specified?
  - If ANY check fails → generate question for first missing parameter
DEPENDS ON: SemanticRouter (upstream), ClarificationLoopManager (downstream)
```

**Do this for your ~15 most important components** (not all 50+). Focus on:
1. SemanticRouter
2. AmbiguityDetector
3. ContextAssembler (prompt builder)
4. LLMCodeGenerator
5. CompilerLoop
6. BacktestRunner
7. RobustnessScorer
8. RiskSentinel
9. ExecutionBridge
10. HeartbeatManager

### How to Research This
- Re-read section 4 of the technical review (System Decomposition)
- For each component ask: *"What goes IN? What comes OUT? What's the logic in between?"*

### ✅ Done Checklist
- [ ] 10-15 component specs written with INPUT/OUTPUT/LOGIC
- [ ] Every component has at most 2 upstream dependencies and 2 downstream
- [ ] No component does "everything" — if it has >5 responsibilities, split it
- [ ] You can trace a user request from component to component on paper

---

## Phase 7: Behavior Modeling — "How does the system behave over time?"

### What It Is
Three specific diagram types that model HOW the system behaves, not just what it contains.

### Why It Matters
Static component diagrams show you the **pieces**. Behavior diagrams show you the **motion** — the order of events, the decisions, the loops. Without these, developers guess at "what happens next."

### What to Produce

**Artifact 7a: Strategy Lifecycle State Machine**

```
States: DRAFT → GENERATING → CLARIFYING → GENERATED → 
        VALIDATING → VALIDATED → APPROVED → DEPLOYING → 
        ACTIVE → PAUSED → HALTED → TERMINATED

Transitions (examples):
  DRAFT → GENERATING:     when user submits strategy
  GENERATING → CLARIFYING: when ambiguity detected
  CLARIFYING → GENERATING:  when user answers question
  GENERATING → GENERATED:  when code passes syntax check
  GENERATED → VALIDATING:  automatic (after generation)
  VALIDATING → VALIDATED:  when robustness score ≥ threshold
  VALIDATING → GENERATED:  when robustness score < threshold (retry)
  VALIDATED → APPROVED:    when user clicks "Approve"
  APPROVED → ACTIVE:       when user clicks "Deploy"
  ACTIVE → HALTED:         when kill-switch triggered
  ANY → TERMINATED:        when user clicks "Delete"

INVALID transitions (must be blocked):
  DRAFT → ACTIVE (cannot skip validation)
  GENERATING → ACTIVE (cannot skip validation)
  HALTED → ACTIVE (must re-validate)
```

**Artifact 7b: Activity Diagram — Strategy Creation Workflow**

Draw using flowchart notation:

```
START
  → User enters strategy text
  → System sanitizes input
  → System classifies intent
  → [Decision] Is it a new strategy?
      YES → System checks for missing parameters
            → [Decision] Parameters complete?
                YES → Retrieve templates from Pinecone
                NO  → Ask clarification question
                      → Wait for user response
                      → Merge answer → loop back to check
            → Assemble prompt
            → Call LLM
            → Inject into skeleton
            → Static analysis
            → [Decision] Syntax OK?
                YES → Save as GENERATED → proceed to backtest
                NO  → [Decision] Retry count < 3?
                    YES → Feed error back to LLM → loop
                    NO  → Mark FAILED → notify user
      NO → (handle refinement or explanation)
END
```

**Artifact 7c: Sequence Diagram — Key Scenarios**

Write at least 2 sequence diagrams:
1. **Happy path:** User creates strategy → gets code → backtests → deploys
2. **Error path:** LLM fails → retry → still fails → user notified

### How to Research This
- Search: `"state machine diagram tutorial"` — understand states, transitions, guards
- Search: `"UML activity diagram tutorial"` — understand decision diamonds, parallel bars
- Search: `"UML sequence diagram tutorial"` — understand lifelines, messages, alt fragments
- Use draw.io (free) or Mermaid syntax (text-based, works in markdown)

### ✅ Done Checklist
- [ ] Strategy lifecycle state machine drawn with ALL states and transitions
- [ ] At least 2 activity diagrams (strategy creation, kill-switch activation)
- [ ] At least 2 sequence diagrams (happy path, one error path)
- [ ] No state has unclear "what happens next" — every transition is labeled

---

## Phase 8: Data Design — "What exactly gets stored and where?"

### What It Is
Turn your domain model (Phase 4) into actual database tables with columns, types, and relationships.

### Why It Matters
Bad data design causes bugs that are extremely painful to fix. If you store strategy status as a free-text string instead of an enum, you'll get `"active"`, `"Active"`, `"ACTIVE"`, and `"actve"` in your database.

### What to Produce

**Artifact: Database Schema**

```
TABLE: users
  - id: UUID (PK)
  - email: VARCHAR(255) UNIQUE NOT NULL
  - password_hash: VARCHAR(255) NOT NULL
  - created_at: TIMESTAMP WITH TIME ZONE

TABLE: strategies
  - id: UUID (PK)
  - user_id: UUID (FK → users.id)
  - intent_raw: TEXT NOT NULL
  - spec_json: JSONB
  - code_mql5: TEXT
  - code_pine: TEXT
  - robustness_score: FLOAT
  - status: ENUM('draft','generating','clarifying','generated',
                  'validating','validated','approved','active',
                  'paused','halted','terminated')
  - created_at: TIMESTAMP WITH TIME ZONE
  - updated_at: TIMESTAMP WITH TIME ZONE

TABLE: backtest_results
  - id: UUID (PK)
  - strategy_id: UUID (FK → strategies.id)
  - net_profit: FLOAT
  - max_drawdown: FLOAT
  - win_rate: FLOAT
  - sharpe_ratio: FLOAT
  - total_trades: INT
  - robustness_score: FLOAT
  - created_at: TIMESTAMP WITH TIME ZONE

TABLE: audit_logs
  - id: BIGSERIAL (PK)
  - strategy_id: UUID (FK → strategies.id)
  - event_type: ENUM('signal_gen','order_sent','risk_reject',
                      'kill_switch','validation_pass','validation_fail')
  - payload: JSONB
  - created_at: TIMESTAMP WITH TIME ZONE

HYPERTABLE: market_data (TimescaleDB)
  - time: TIMESTAMP WITH TIME ZONE (PK part)
  - symbol: VARCHAR(20) (PK part)
  - timeframe: VARCHAR(10)
  - open: FLOAT
  - high: FLOAT
  - low: FLOAT
  - close: FLOAT
  - volume: FLOAT
```

### ✅ Done Checklist
- [ ] All domain entities from Phase 4 have corresponding tables
- [ ] Every column has a type, nullable/not-null, and constraints
- [ ] All ENUM values are explicitly listed (not "string")
- [ ] Foreign key relationships match the domain model
- [ ] Sensitive data (passwords, API keys) marked for encryption

---

## Phase 9: Deployment Design — "How does it actually run?"

### What It Is
Define your Docker Compose topology — what containers exist, what ports they use, how they connect, and how MT5 fits in.

### Why It Matters
The MT5 Windows dependency is the single biggest deployment risk. If you don't figure this out now, you'll discover it doesn't work when you're 3 months into coding.

### What to Produce

**Artifact: Docker Compose Service Map**

```
SERVICES:
  frontend:
    - Nginx serving React build
    - Port: 3000 → 80
    
  backend:
    - FastAPI application
    - Port: 8000
    - Depends on: db, redis
    
  celery_worker:
    - Celery workers (GenAI + Quant)
    - No exposed port
    - Depends on: redis, db
    
  db:
    - PostgreSQL 16 + TimescaleDB
    - Port: 5432
    - Volume: pg_data
    
  redis:
    - Redis 7
    - Port: 6379
    - Volume: redis_data

NETWORK: smart_trade_net (bridge)

MT5 TOPOLOGY (CRITICAL DECISION):
  Option A: MT5 runs on Windows HOST machine
            Backend connects to it via localhost TCP socket
            → Simpler, works guaranteed
            
  Option B: MT5 runs inside Docker with Wine
            → More portable, but may not work

  DECISION: Start with Option A. 
            MT5 on host, Docker services on same host.
            Backend uses host.docker.internal to reach MT5.
```

### ✅ Done Checklist
- [ ] docker-compose.yml structure designed (services, ports, volumes, networks)
- [ ] MT5 deployment approach decided and documented
- [ ] Environment variables listed (API keys, DB credentials, LLM keys)
- [ ] You can draw the network diagram on a whiteboard

---

## Phase Dependency Map

```
Phase 1 (Context)
    ↓
Phase 2 (Requirements Freeze)
    ↓
Phase 3 (Use Cases)
    ↓
Phase 4 (Domain Model) ←────┐
    ↓                        │
Phase 5 (Architecture) ──────┤ These three inform each other.
    ↓                        │ Iterate between them if needed.
Phase 6 (Components) ────────┘
    ↓
Phase 7 (Behavior: States, Sequences, Activities)
    ↓
Phase 8 (Data Design)
    ↓
Phase 9 (Deployment Design)
    ↓
    ✅ READY TO CODE
```

---

## Research Methodology: How to Learn What You Need

When you hit something you don't know during any phase, follow this exact process:

### Step 1: Name the Gap
> *"I don't understand how to draw a state machine diagram"*

### Step 2: Search for a MINIMAL explanation (10 min max)
> Search: `"state machine diagram explained simple example"`  
> Read ONE article or watch ONE short video (< 10 min)

### Step 3: Apply it to SmartTradeAI immediately
> Don't do a generic exercise. Draw YOUR strategy lifecycle states right away.

### Step 4: Validate with a question
> Ask yourself: *"If I show this to my supervisor, can they understand it?"*  
> If yes → move on. If no → your diagram is too vague, add detail.

### Common Research Queries by Phase

| Phase | What to search |
|-------|---------------|
| 1 | `"C4 system context diagram example"` |
| 2 | `"MoSCoW prioritization requirements example"` |
| 3 | `"use case description template example"` |
| 4 | `"domain model class diagram tutorial"` |
| 5 | `"architecture decision record template"` |
| 6 | `"component diagram C4 model example"` |
| 7 | `"UML state machine diagram tutorial"`, `"sequence diagram tutorial"` |
| 8 | `"database schema design tutorial PostgreSQL"` |
| 9 | `"docker compose multi-service example"` |

**Tool for diagrams:** Use [draw.io](https://draw.io) (free, browser-based). Export as PNG for your FYP report.
