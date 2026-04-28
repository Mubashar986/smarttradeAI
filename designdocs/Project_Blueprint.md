# SmartTradeAI — Project Blueprint

> **One document. Everything decided. What to do next.**  
> Last updated: 2026-03-14

---

## What Is This System? (Plain English)

A trader types a strategy in English → the system writes the trading code → tests it on historical data → and if it's safe, runs it on MetaTrader 5.

**The 5 steps:**

```
Step 1: UNDERSTAND — User types strategy, system asks clarifying questions if anything is missing
Step 2: GENERATE  — System uses RAG + LLM to write MQL5 code, auto-fixes syntax errors (up to 3 tries)
Step 3: VALIDATE  — System backtests the code + stress tests it, gives a score out of 100
Step 4: GUARD     — Risk Sentinel checks position size, stop-loss, lot limits before any trade
Step 5: EXECUTE   — Bridge sends the signal to MetaTrader 5 via TCP socket
```

---

## The 6 Components (Nothing More)

| # | Name | One-Line Job | Tech |
|---|------|-------------|------|
| **C1** | Chat Interface | User types strategy, sees code + results | React |
| **C2** | AI Engine | Understands intent, generates code via RAG+LLM | FastAPI, LangChain, OpenAI |
| **C3** | Quality Lab | Backtests code, stress tests, gives robustness score | Backtrader |
| **C4** | Safety Guard | Blocks dangerous trades (position size, stop-loss, kill switch) | Python middleware |
| **C5** | Execution Bridge | Sends signals to MT5 via persistent TCP socket | Python sockets + MQL5 EA |
| **C6** | Data Layer | Stores strategies, market data, logs, vector embeddings | PostgreSQL, TimescaleDB, Redis, Pinecone |

---

## Frozen Decisions (Do NOT Revisit)

| Decision | Choice | Done? |
|----------|--------|-------|
| Backend framework | FastAPI | ✅ |
| Frontend | React | ✅ |
| Database | PostgreSQL + TimescaleDB | ✅ |
| Cache & queue | Redis + Celery | ✅ |
| Vector DB | Pinecone | ✅ |
| LLM provider | OpenAI | ✅ |
| AI framework | LangChain / LangGraph | ✅ |
| Backtesting library | Backtrader | ✅ |
| MT5 connectivity | Custom TCP socket bridge (not webhooks) | ✅ |
| Deployment | Docker Compose (MT5 stays on Windows host) | ✅ |

---

## Build Order — What to Work On and When

> [!IMPORTANT]
> **Work in this exact order. Each step gives you something demonstrable. Don't jump ahead.**

### 🔴 Step 1: RIGHT NOW — The Minimal Talking System

**Goal:** User types a strategy → system calls LLM → shows generated MQL5 code.

**What you build:**
- FastAPI backend with ONE endpoint: `POST /api/v1/generate`
- It receives text, calls OpenAI API, returns generated MQL5 code
- Simple React page: text input box + code display panel
- Docker Compose: `frontend` + `backend` + `redis` containers

**What you DON'T build yet:** No RAG, no backtesting, no MT5, no Sentinel, no database.

**Why this first:** If this doesn't work, nothing else matters. This proves the core loop.

---

### 🟡 Step 2: Add RAG (Make the Code Better)

**Goal:** Before calling LLM, retrieve relevant MQL5 templates from Pinecone.

**What you add:**
- Pinecone setup with `mql5_docs` namespace
- Embed 20-30 key MQL5 code snippets and docs
- `ContextAssembler` that builds: user intent + retrieved templates + skeleton → prompt
- Code skeleton injection (pre-validated MQL5 structure)

**Why second:** Without RAG, the LLM hallucinates MQL5 syntax ~30-40% of the time. RAG fixes that.

---

### 🟡 Step 3: Add Validation (Make Sure the Code Works)

**What you add:**
- Static syntax checker (Python AST or regex-based for MQL5)
- Self-correction loop (if syntax error → feed error back to LLM → retry, max 3x)
- PostgreSQL to store strategies with status (DRAFT → GENERATED → VALIDATED)

**Why third:** Now you can tell the user "this code is verified" instead of "hope it works."

---

### 🟢 Step 4: Add Backtesting

**What you add:**
- Backtrader integration
- TimescaleDB with sample OHLCV data (EURUSD, 1-2 years)
- Basic backtest: run strategy on historical data → show equity curve + metrics
- Robustness scorer (Win Rate, Max Drawdown, Sharpe)

---

### 🟢 Step 5: Add Safety + MT5 Bridge

**What you add:**
- Risk Sentinel middleware (position size, stop-loss, fat-finger checks)
- Kill switch
- TCP socket bridge to MT5 Expert Advisor
- Heartbeat protocol (5s ping/pong)

---

### 🔵 Step 6: Polish

- Stress testing (volatility shock, black swan replay)
- Clarification loop (ask user when parameters are missing)
- Audit logging
- Pine Script support (if time allows)
- UI polish (charts, explanations, diff viewer)

---

## Documents We've Produced

| Document | What it is | Location |
|----------|-----------|----------|
| Technical Review | Strengths, gaps, risks analysis of the system | [SmartTradeAI_Technical_Review.md](file:///c:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/SmartTradeAI_Technical_Review.md) |
| System Architecture Diagrams | 5 key diagrams with component definitions | [System_Architecture_Diagrams.md](file:///c:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/System_Architecture_Diagrams.md) |
| Chapter 4 — System Design | Full FYP chapter with all diagrams and use cases | [Chapter4_System_Design.md](file:///c:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/Chapter4_System_Design.md) |
| Design Framework | 9-phase methodology for system design | [System_Design_Framework.md](file:///c:/Users/Abdul%20Jabbar%20Metlo/Desktop/smarttradeAI/System_Design_Framework.md) |
