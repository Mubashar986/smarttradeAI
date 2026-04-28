# SmartTradeAI — Component Overviews (For Prompts)

Copy the relevant section into any new chat prompt.

---

## C1: Chat Interface (Frontend)

**Job:** User types strategy, sees generated code, backtest results, and controls strategies.  
**Tech:** React, WebSocket, Chart.js  
**Input:** User keystrokes, button clicks  
**Output:** Rendered code, charts, status updates, notifications  
**Key parts:** Chat input box, code viewer (syntax highlighted), backtest charts (equity curve, drawdown), strategy controls (pause/stop/deploy), kill switch button  
**Talks to:** C2 (sends user text via REST, receives updates via WebSocket)  
**Requirements:** FR02-01, FR09-02, FR09-03  
**Use Cases:** UC02 (input side), UC10, UC11, UC12, UC13

---

## C2: AI Engine (Brain)

**Job:** Takes natural language strategy → generates compilable MQL5 code using RAG + LLM.  
**Tech:** FastAPI, LangChain, OpenAI API, Pinecone  
**Input:** User's strategy text (string)  
**Output:** Generated MQL5 code (string) + plain-English explanation  
**Key parts:** Intent classifier, ambiguity detector + clarification loop, RAG pipeline (Pinecone vector search → context assembly → prompt building), LLM code generator, self-correction loop (Stage 1: Python static analysis + Stage 2: MetaEditor compilation, max 5 retries)  
**Talks to:** C1 (receives text, sends code back), Pinecone (retrieves templates), OpenAI (generates code), C3 (passes code for compilation), C6 (saves strategies to DB)  
**Requirements:** FR02, FR03, FR04, FR06  
**Use Cases:** UC02, UC03, UC05

---

## C3: Quality Lab (Validation + Backtesting)

**Job:** Compiles MQL5 code using MetaEditor, backtests the compiled EA on historical data, calculates performance metrics.  
**Tech:** Docker, Wine, MetaEditor (compilation), MetaTrader 5 Strategy Tester (backtesting), Python (metrics)  
**Input:** Generated MQL5 code file  
**Output:** Compiled .ex5 file, backtest results (profit, drawdown, win rate, Sharpe), PDF report  
**Key parts:** Docker container spawner (Wine + MetaEditor), compilation log parser, MT5 Strategy Tester runner, performance metrics calculator (profit factor, max DD, win rate, Sharpe, trade count), PDF report generator  
**Talks to:** C2 (receives code, returns compile errors), C6 (reads market data from TimescaleDB, writes results to PostgreSQL)  
**Requirements:** FR05, FR07, FR08, FR10  
**Use Cases:** UC04, UC06, UC07, UC08

---

## C4: Safety Guard (Risk Sentinel)

**Job:** Intercepts every trade order and blocks it if it violates safety rules. Provides kill switch.  
**Tech:** Python middleware  
**Input:** Trade order (symbol, volume, SL, TP, type)  
**Output:** PASS (forward to bridge) or REJECT (log + notify user)  
**Key parts:** Position size limiter (volume × price ≤ equity × max_risk%), stop-loss enforcer (must have SL, inject default if missing), fat-finger filter (reject if volume > max lots), frequency limiter (reject if < 1s since last order), drawdown monitor (auto kill-switch if daily DD > 5%), kill switch engine (freeze queue → cancel all → close all → terminate)  
**Talks to:** C2 (receives validated orders), C5 (forwards safe orders), C6 (writes audit logs)  
**Requirements:** FR04-02 (risk in code), FR09-01  
**Use Cases:** UC14, UC15 (partially)

---

## C5: Execution Bridge (MT5 Connector)

**Job:** Maintains persistent TCP socket connection to MetaTrader 5 and sends/receives trade commands.  
**Tech:** Python `socket`, MQL5 `SocketCreate()`, MetaTrader5 Python library  
**Input:** Validated trade order (JSON)  
**Output:** Execution confirmation or failure (JSON)  
**Key parts:** TCP socket server (port 5555, JSON protocol), command serializer (ORDER_SEND, CANCEL_ALL, CLOSE_POSITION), MT5 Expert Advisor (MQL5 socket client in OnTimer), heartbeat manager (PING every 5s, 3 misses = failure), safe-state protocol (on failure: alert user, block signals, reconnect loop)  
**Talks to:** C4 (receives safe orders), MT5 terminal (sends JSON commands via TCP), C6 (writes execution logs), C1 (sends status via WebSocket)  
**Deployment:** MT5 runs on Windows host. Socket server runs on same host. Backend in Docker reaches it via host.docker.internal.  
**Requirements:** FR07-01 (backtest execution), FR09-01  
**Use Cases:** UC06 (execution side)

---

## C6: Data Layer (Memory)

**Job:** Stores all persistent data — strategies, users, market data, task queues, vector embeddings.  
**Tech:** PostgreSQL + TimescaleDB, Redis, Pinecone  
**Stores:**  
- PostgreSQL: users, strategies (with status enum), backtest_results, audit_logs  
- TimescaleDB: OHLCV market data (hypertable, partitioned by time+symbol)  
- Redis: Celery task queue, rate limiter state, session cache  
- Pinecone: MQL5 documentation embeddings, code template embeddings (namespace-isolated)  
**Talks to:** Every other component reads from or writes to C6  
**Requirements:** FR01, FR03, FR04-03, FR10  
**Use Cases:** All (data persistence underlies everything)
