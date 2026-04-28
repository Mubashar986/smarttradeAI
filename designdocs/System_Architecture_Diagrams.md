# SmartTradeAI — System Components & Architecture Diagrams

## First: What ARE the Main Components?

Before any diagram, let's answer the fundamental question. Your entire system has **6 main components**. Everything else is a sub-part of these 6.

Think of it like a human body:

| # | Component | What it does (one sentence) | Body analogy |
|---|-----------|---------------------------|--------------|
| **C1** | **Chat Interface** (Frontend) | Takes what the user says and shows what the system produces | 👁️ Eyes & mouth |
| **C2** | **Brain** (AI Engine) | Understands trading strategies, generates code, explains decisions | 🧠 Brain |
| **C3** | **Quality Lab** (Validation) | Tests if generated code actually works and is safe | 🔬 Immune system |
| **C4** | **Safety Guard** (Risk Sentinel) | Blocks dangerous trades before they reach the market | 🛡️ Pain reflex |
| **C5** | **Bridge** (Execution) | Connects to MetaTrader 5 and sends actual trade signals | 🤚 Hands |
| **C6** | **Memory** (Data Layer) | Stores everything — strategies, results, market data, logs | 🗄️ Memory |

**That's it. 6 components.** Everything in your 50+ component list from the technical review fits inside one of these 6 boxes.

### Where Each Sub-Component Lives

```
C1: Chat Interface
    ├── Chat input box
    ├── Code display panel
    ├── Backtest charts
    ├── Strategy controls (pause/stop)
    └── Kill switch button

C2: Brain (AI Engine)
    ├── Intent classifier ("what does the user want?")
    ├── Ambiguity detector ("is anything missing?")
    ├── RAG pipeline ("find relevant templates from knowledge base")
    ├── Code generator ("call LLM to write the code")
    └── Self-correction loop ("fix syntax errors, retry up to 3x")

C3: Quality Lab (Validation)
    ├── Syntax checker ("does the code compile?")
    ├── Look-ahead bias detector ("is it cheating with future data?")
    ├── Backtester ("run strategy on historical data")
    └── Stress tester ("what happens in a crash?")

C4: Safety Guard (Risk Sentinel)
    ├── Position size limiter ("don't bet too much")
    ├── Stop-loss enforcer ("must have a safety net")
    ├── Fat-finger filter ("catch absurd order sizes")
    ├── Frequency limiter ("no rapid-fire orders")
    ├── Drawdown monitor ("auto-stop if losing too much")
    └── Kill switch engine ("emergency shutdown")

C5: Bridge (Execution)
    ├── TCP socket server (Python side)
    ├── MT5 Expert Advisor (MQL5 side, runs inside MetaTrader)
    ├── Heartbeat monitor ("is the connection alive?")
    └── Safe-state protocol ("what to do when connection dies")

C6: Memory (Data Layer)
    ├── PostgreSQL (strategies, users, audit logs)
    ├── TimescaleDB (market price data)
    ├── Redis (task queue, cache, sessions)
    └── Pinecone (code templates & documentation embeddings)
```

---

## Diagram 1: System Context — What's Inside vs. Outside

This is the most fundamental diagram. It shows SmartTradeAI as **one single box** and everything external it connects to.

```mermaid
graph LR
    subgraph EXTERNAL["OUTSIDE YOUR SYSTEM"]
        USER["👤 Retail Trader"]
        ADMIN["👨‍💼 System Admin"]
        LLM["☁️ OpenAI API"]
        PINE["☁️ Pinecone"]
        MT5["💻 MetaTrader 5<br/>(Windows Terminal)"]
        BROKER["🏦 Broker<br/>(via MT5)"]
    end

    subgraph SYSTEM["🔲 SmartTrade AI System"]
        S["C1 + C2 + C3 + C4 + C5 + C6<br/>(All 6 components)"]
    end

    USER -->|"Strategy in plain English"| S
    S -->|"Generated code, backtest report,<br/>explanations, trade status"| USER
    
    ADMIN -->|"Monitor, configure,<br/>kill-switch"| S
    S -->|"System health, logs"| ADMIN

    S -->|"Prompt with context"| LLM
    LLM -->|"Generated code text"| S

    S -->|"Search query (vector)"| PINE
    PINE -->|"Relevant templates"| S

    S -->|"Trade signal (JSON)"| MT5
    MT5 -->|"Execution confirmation,<br/>market data"| S

    MT5 -->|"Orders"| BROKER
    BROKER -->|"Fills, prices"| MT5
```

### What This Tells You

- **You build:** The grey box (SmartTrade AI System)
- **You DON'T build:** OpenAI, Pinecone, MetaTrader 5, or the Broker
- **You CONNECT TO:** 4 external systems (each is a risk if it goes down)
- **Two types of users:** Trader (uses the product) and Admin (monitors the system)

---

## Diagram 2: Container/Service Architecture — What Actually Runs

This shows every **running process** (Docker container or application) and how they communicate.

```mermaid
graph TB
    subgraph CLIENT["User's Browser"]
        FE["C1: React Frontend<br/>──────────<br/>Chat UI, Code Viewer,<br/>Charts, Kill Switch"]
    end

    subgraph DOCKER["Docker Compose Environment"]
        API["C2a: FastAPI Backend<br/>──────────<br/>API Gateway, Auth,<br/>Agent Orchestrator"]
        
        WORKER["C2b + C3: Celery Workers<br/>──────────<br/>AI Generation Tasks,<br/>Backtesting Tasks"]

        DB["C6a: PostgreSQL<br/>+ TimescaleDB<br/>──────────<br/>Strategies, Users,<br/>Market Data, Logs"]
        
        REDIS["C6b: Redis<br/>──────────<br/>Task Queue,<br/>Cache, Sessions"]
    end

    subgraph EXTERNAL_SERVICES["External Cloud Services"]
        LLM["OpenAI API"]
        PINECONE["Pinecone<br/>Vector DB"]
    end

    subgraph WINDOWS_HOST["Windows Host Machine"]
        BRIDGE["C5a: TCP Socket Server<br/>(Python process)"]
        MT5APP["C5b: MetaTrader 5<br/>+ Expert Advisor"]
    end

    FE -->|"HTTPS REST<br/>+ WebSocket"| API
    API -->|"Task Queue<br/>(Redis Protocol)"| REDIS
    REDIS -->|"Task Pickup"| WORKER
    WORKER -->|"SQL Queries"| DB
    API -->|"SQL Queries"| DB
    WORKER -->|"Prompt"| LLM
    WORKER -->|"Vector Search"| PINECONE
    API -->|"Trade Commands"| BRIDGE
    BRIDGE -->|"JSON over TCP<br/>Port 5555"| MT5APP
    MT5APP -->|"Confirmations"| BRIDGE
    BRIDGE -->|"Status Updates"| API

    style CLIENT fill:#e1f5fe
    style DOCKER fill:#f3e5f5
    style EXTERNAL_SERVICES fill:#fff3e0
    style WINDOWS_HOST fill:#e8f5e9
```

### What This Tells You

| Service | Technology | Runs Where | Port |
|---------|-----------|-----------|------|
| Frontend | React (Nginx) | Docker | 3000 |
| Backend API | FastAPI | Docker | 8000 |
| Celery Workers | Python + Celery | Docker | — (no port) |
| Database | PostgreSQL + TimescaleDB | Docker | 5432 |
| Redis | Redis | Docker | 6379 |
| Socket Bridge | Python script | Windows host | 5555 |
| MT5 Terminal | MetaTrader 5 + EA | Windows host | — |

**Key insight:** 5 services run in Docker. 2 things run on Windows (MT5 + the bridge script). This is why the MT5 deployment topology is a critical decision.

---

## Diagram 3: Backend Component Architecture — Inside the Brain

This zooms INTO the FastAPI Backend + Celery Workers to show the internal modules.

```mermaid
graph TB
    subgraph API_LAYER["API Layer (FastAPI)"]
        ROUTER["Router<br/>Endpoints"]
        AUTH["JWT Auth<br/>Middleware"]
        RATE["Rate<br/>Limiter"]
        WS["WebSocket<br/>Manager"]
    end

    subgraph ORCHESTRATION["Orchestration Layer"]
        ORCH["Agent<br/>Orchestrator"]
        LIFECYCLE["Strategy<br/>Lifecycle<br/>Manager"]
        TASKPROD["Celery Task<br/>Producer"]
    end

    subgraph AI_ENGINE["AI Engine (C2)"]
        CLASSIFY["Intent<br/>Classifier"]
        AMBIGUITY["Ambiguity<br/>Detector"]
        RAG["RAG<br/>Pipeline"]
        CODEGEN["Code<br/>Generator"]
        COMPILER["Self-Correction<br/>Loop"]
    end

    subgraph VALIDATION["Quality Lab (C3)"]
        SYNTAX["Syntax<br/>Checker"]
        BIAS["Look-Ahead<br/>Bias Detector"]
        BACKTEST["Backtester"]
        STRESS["Stress<br/>Tester"]
        SCORE["Robustness<br/>Scorer"]
    end

    subgraph SAFETY["Safety Guard (C4)"]
        SENTINEL["Risk<br/>Sentinel"]
        KILL["Kill Switch<br/>Engine"]
    end

    ROUTER --> AUTH --> RATE --> ORCH
    WS --> ORCH
    ORCH --> LIFECYCLE
    ORCH --> TASKPROD
    ORCH --> CLASSIFY
    CLASSIFY --> AMBIGUITY
    AMBIGUITY -->|"All clear"| RAG
    AMBIGUITY -->|"Missing info"| WS
    RAG --> CODEGEN
    CODEGEN --> COMPILER
    COMPILER --> SYNTAX
    SYNTAX -->|"Pass"| BACKTEST
    SYNTAX -->|"Fail"| CODEGEN
    BACKTEST --> STRESS
    STRESS --> SCORE
    SCORE -->|"Pass"| LIFECYCLE
    LIFECYCLE -->|"Deploy"| SENTINEL
    SENTINEL -->|"Safe"| BRIDGE_OUT["→ To Bridge (C5)"]
    KILL -->|"Emergency"| BRIDGE_OUT

    style API_LAYER fill:#e3f2fd
    style ORCHESTRATION fill:#fce4ec
    style AI_ENGINE fill:#f3e5f5
    style VALIDATION fill:#e8f5e9
    style SAFETY fill:#fff8e1
```

### The Flow in Plain English

1. **User request enters** through the Router → gets checked by Auth → Rate Limiter
2. **Orchestrator** takes over, classifies what the user wants
3. If info is missing → **Ambiguity Detector** asks clarification via WebSocket
4. When ready → **RAG** retrieves templates → **Code Generator** calls LLM → **Self-Correction** fixes errors
5. Generated code goes to **Syntax Checker** → **Backtester** → **Stress Tester** → gets a **Robustness Score**
6. If approved for deployment → **Risk Sentinel** validates the trade → sends to **Bridge**

---

## Diagram 4: End-to-End Data Flow — How Input Becomes Output

This shows how the user's text **transforms** step by step into executed trades.

```mermaid
graph LR
    subgraph STEP1["Step 1: INPUT"]
        A["'Buy EURUSD when<br/>50 SMA crosses above<br/>200 SMA, 1% risk'"]
    end

    subgraph STEP2["Step 2: UNDERSTAND"]
        B["Structured Intent:<br/>─────────────<br/>action: BUY<br/>pair: EURUSD<br/>entry: SMA(50) > SMA(200)<br/>risk: 1%<br/>exit: ❌ MISSING<br/>SL: ❌ MISSING"]
    end

    subgraph STEP3["Step 3: CLARIFY"]
        C["System asks:<br/>'What exit condition?'<br/>'What stop-loss distance?'<br/>─────────────<br/>User answers:<br/>'Exit when SMA crosses back'<br/>'50 pip stop-loss'"]
    end

    subgraph STEP4["Step 4: GENERATE"]
        D["Complete MQL5 Code<br/>─────────────<br/>OnTick() function<br/>SMA indicators<br/>OrderSend() calls<br/>Stop-loss logic<br/>Risk calculation"]
    end

    subgraph STEP5["Step 5: VALIDATE"]
        E["Backtest Report<br/>─────────────<br/>Net Profit: +12.3%<br/>Max Drawdown: -8.1%<br/>Win Rate: 58%<br/>Sharpe: 1.4<br/>Score: 72/100 ✅"]
    end

    subgraph STEP6["Step 6: EXECUTE"]
        F["Trade Signal to MT5<br/>─────────────<br/>BUY EURUSD<br/>0.05 lots<br/>SL: 1.0850<br/>TP: 1.0950"]
    end

    A --> B --> C --> D --> E --> F

    style STEP1 fill:#e3f2fd
    style STEP2 fill:#f3e5f5
    style STEP3 fill:#fff3e0
    style STEP4 fill:#e8f5e9
    style STEP5 fill:#fce4ec
    style STEP6 fill:#e0f2f1
```

### What Each Step Produces

| Step | Input | Process | Output | Component |
|------|-------|---------|--------|-----------|
| 1 | Raw English text | — | Raw text | C1 (Frontend) |
| 2 | Raw text | Intent classification | Structured JSON with gaps | C2 (Classifier) |
| 3 | Gaps in intent | Q&A with user | Complete structured spec | C2 (Clarification) |
| 4 | Complete spec + templates | LLM + RAG + Self-correction | Compiled MQL5 code | C2 (Generator) |
| 5 | MQL5 code + market data | Backtest + stress test | Robustness score | C3 (Lab) |
| 6 | Validated code + user approval | Sentinel check + bridge | Trade on MT5 | C4 + C5 |

---

## Diagram 5: Strategy Lifecycle — The Heart of the System

This is the **most important diagram for your project**. It shows every state a strategy can be in and what moves it between states.

```mermaid
stateDiagram-v2
    [*] --> DRAFT: User starts typing

    DRAFT --> GENERATING: User submits strategy
    GENERATING --> CLARIFYING: Missing parameters detected
    CLARIFYING --> GENERATING: User provides answer
    GENERATING --> GENERATED: Code passes syntax check
    GENERATING --> FAILED: 3 retries exhausted

    GENERATED --> VALIDATING: Auto-submit to backtester
    VALIDATING --> VALIDATED: Score ≥ threshold
    VALIDATING --> GENERATED: Score too low (retry allowed)
    VALIDATING --> FAILED: Critical failure in backtest

    VALIDATED --> APPROVED: User clicks "Approve"
    APPROVED --> ACTIVE: User clicks "Deploy"
    
    ACTIVE --> PAUSED: User clicks "Pause"
    PAUSED --> ACTIVE: User clicks "Resume"
    
    ACTIVE --> HALTED: Kill switch OR drawdown > 5%
    PAUSED --> HALTED: Kill switch
    
    HALTED --> VALIDATED: User requests re-validation
    
    DRAFT --> TERMINATED: User deletes
    GENERATED --> TERMINATED: User deletes
    VALIDATED --> TERMINATED: User deletes
    HALTED --> TERMINATED: User deletes
    FAILED --> TERMINATED: User deletes

    FAILED --> DRAFT: User wants to retry from scratch
```

### State Definitions

| State | What it means | What the user sees |
|-------|--------------|-------------------|
| **DRAFT** | User is composing their strategy | Chat interface, typing |
| **GENERATING** | LLM is writing the code | Loading spinner, "Generating..." |
| **CLARIFYING** | System needs more info | Clarification question in chat |
| **GENERATED** | Code is ready, not yet tested | Code viewer with syntax highlighting |
| **VALIDATING** | Backtest + stress test running | Progress bar, "Testing your strategy..." |
| **VALIDATED** | Tests passed, awaiting approval | Backtest report + "Approve" button |
| **APPROVED** | User approved, ready to deploy | "Deploy" button active |
| **ACTIVE** | Live on MT5, executing trades | Green status, live metrics |
| **PAUSED** | Temporarily stopped | Yellow status, "Resume" button |
| **HALTED** | Emergency stopped | Red status, kill-switch was triggered |
| **FAILED** | Could not generate or validate | Error message, "Try Again" button |
| **TERMINATED** | Permanently deleted | Removed from dashboard |

### Rules (CRITICAL — Must Be Enforced in Code)

> **You can NEVER go from DRAFT → ACTIVE.** The validation gate is mandatory.  
> **You can NEVER go from HALTED → ACTIVE.** Must re-validate first.  
> **TERMINATED is permanent.** No resurrection.

---

## Component Relationship Matrix

This table shows exactly HOW each component connects to every other component:

| | C1 Frontend | C2 Brain | C3 Lab | C4 Guard | C5 Bridge | C6 Memory |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| **C1 Frontend** | — | REST + WS | — | WS (kill switch) | — | — |
| **C2 Brain** | WS (updates) | — | Sends code to test | — | — | Reads/writes strategies |
| **C3 Lab** | WS (results) | Returns score | — | — | — | Reads market data, writes results |
| **C4 Guard** | WS (alerts) | Receives orders | — | — | Forwards safe orders | Writes audit logs |
| **C5 Bridge** | WS (trade status) | — | — | Receives orders | — | Writes execution logs |
| **C6 Memory** | — | Serves data | Serves data | Serves data | Serves data | — |

### Key Relationships Explained

1. **C1 → C2:** Frontend sends user text to Brain via REST API. Brain sends updates back via WebSocket.
2. **C2 → C3:** Brain sends generated code to Lab for testing. Lab returns pass/fail + score.
3. **C2 → C6:** Brain saves strategies to PostgreSQL, retrieves templates from Pinecone.
4. **C3 → C6:** Lab reads historical market data from TimescaleDB for backtesting.
5. **C4 → C5:** Guard validates orders and forwards safe ones to Bridge. This is the ONLY path to MT5.
6. **C5 → MT5:** Bridge maintains persistent TCP socket connection with the Expert Advisor.
7. **C6 serves everyone:** Every component reads from or writes to the data layer.

---

## Summary: The 5 Diagrams and What Each Answers

| # | Diagram | Question It Answers |
|---|---------|-------------------|
| 1 | System Context | What's my system vs. what's external? |
| 2 | Container Architecture | What services run and how do they communicate? |
| 3 | Backend Components | What's inside the backend and in what order? |
| 4 | Data Flow | How does user text become a trade? |
| 5 | Strategy Lifecycle | What states can a strategy be in and what triggers transitions? |
