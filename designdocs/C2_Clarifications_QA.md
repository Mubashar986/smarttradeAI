# SmartTradeAI — C2 Clarifications & Market Comparison (Q&A)

> **Role:** AI / Inference Systems Architect
> **Date:** 2026-03-14
> **Purpose:** Answer clarification questions about C2 components, explain why they exist, how to implement them, and how similar ideas appear in industry systems.

---

## 1. Semantic Router

### 1.1 What it is (simple definition)
A **Semantic Router** is a small decision layer that classifies user input into a predefined intent label (e.g., `STRATEGY_CREATION`, `REFINEMENT`, `CLARIFICATION_RESPONSE`, `EXPLANATION_REQUEST`) and routes the request to the correct downstream workflow.

### 1.2 Why we need it
- Prevents sending every input into the expensive generation pipeline.
- Keeps the system deterministic and safe by ensuring the right workflow is used.
- Enables fast handling of clarifications and explanations without re-running generation.

### 1.3 Best 3 implementation approaches
1. **Rules + keyword matching (deterministic)**
   - Fast, cheap, and stable.
   - Best for MVP and safety-critical paths.
2. **Lightweight classifier (small model)**
   - Train or prompt a small model to classify intent into a fixed enum.
   - Use confidence thresholds; fall back to rules if uncertain.
3. **LLM-based routing (structured output)**
   - Prompt the LLM to output a JSON enum and parse it.
   - More flexible for complex inputs but should be treated as untrusted and validated.

### 1.4 How it works in real agent systems (market examples)
- **Microsoft Multi-Agent Reference Architecture** explicitly describes a **Semantic Router** pattern with a classifier + LLM fallback for intent-based routing. citeturn0search1turn0search6
- **NVIDIA LLM Router Blueprint** implements intent-based routing and model selection for agent-like pipelines. citeturn0search0
- **LLM-based routing pattern** is documented as a standard agentic design pattern. citeturn0search5

### 1.5 How Cursor, v0, and Codex handle it (based on public docs)
- **Cursor:** Public docs emphasize an “Agent mode” that plans and executes tasks, but do not publish a specific semantic router design. citeturn2search2turn2search3
- **v0:** Public docs describe a UI generation model and API, but no explicit semantic router architecture is documented. citeturn2search0turn2search1
- **Codex:** OpenAI describes Codex as a coding agent that accepts tasks and runs them in isolated environments. No public semantic router architecture is described. citeturn1search0turn1search6turn1search7

---

## 2. Ambiguity Detection

### 2.1 What it is
A **deterministic check** that detects missing or vague parameters required for safe, executable code (e.g., entry condition, exit condition, stop loss, timeframe, instrument).

### 2.2 Why we need it
- Prevents unsafe assumptions (e.g., missing stop-loss).
- Reduces downstream failures in generation and validation.
- Enables a structured, auditable clarification loop (grey-box transparency).

### 2.3 Best 3 implementation approaches
1. **Checklist-based rules (deterministic)**
   - Required field checklist; if missing, ask a targeted question.
2. **Schema extraction + validation**
   - Extract a `StrategySpec` JSON, validate required fields.
3. **Hybrid extraction (LLM + rules)**
   - LLM extracts fields, deterministic validator checks completeness.

### 2.4 Market usage
Public docs for commercial agents rarely label “ambiguity detection,” but the pattern is common in **clarification-first** agentic systems (ask for missing info before action). Patterns are described in multi-agent reference architectures and routing/clarification workflows. citeturn0search1

### 2.5 Cursor / v0 / Codex behavior
No public documentation explicitly describes an ambiguity detection module in these tools. They typically rely on user prompting and follow-up questions, but the internal mechanism is not documented. citeturn2search0turn1search0

---

## 3. Clarification Loop

### 3.1 What it is
A **stateful Q&A loop** that pauses the pipeline, asks targeted questions, merges answers into the structured spec, and resumes when complete.

### 3.2 Why we need it
- Ensures deterministic strategy specs.
- Prevents silent injection of unsafe defaults.
- Improves explainability and user trust.

### 3.3 Best 3 implementation approaches
1. **Explicit state machine (recommended)**
   - Store pending questions in Redis; resume when answers arrive.
2. **Conversational memory with validation gate**
   - Store conversation context, re-validate missing fields on each turn.
3. **LLM-guided clarification**
   - LLM proposes next question, deterministic validator approves it.

### 3.4 Who manages it in SmartTradeAI
- **C2 (AI Engine)** manages the clarification loop.
- **Clarification Loop Manager** tracks pending questions and merges answers.
- **C1** only displays the question and returns user answers.

### 3.5 Market behavior
Clarification-style workflows are common in agentic architectures; multi-agent reference patterns explicitly describe branching and clarification stages. citeturn0search1

---

## 4. Pinecone: What, Why, Who Queries, and Where to Store Templates

### 4.1 What is Pinecone
Pinecone is a managed **vector database** used for semantic retrieval in RAG systems. It stores embeddings and supports fast similarity search. citeturn0search4

### 4.2 Why we need it
- Grounds the LLM with authoritative MQL5 templates and docs.
- Reduces hallucination risk.
- Enables scalable retrieval with namespaces. citeturn0search4turn0search7

### 4.3 Who queries Pinecone
- The **RAG pipeline inside C2** (Vector Search Client) queries Pinecone.
- C1 and C3 never query Pinecone directly.

### 4.4 Where to store pre-validated templates
- Primary store: **Pinecone namespaces** (e.g., `mql5_docs`, `risk_templates`). citeturn0search7
- Secondary fallback: **local cached “golden templates”** on disk for downtime.

### 4.5 Alternatives to Pinecone (if needed)
- **pgvector (PostgreSQL)**
- **Milvus**
- **Weaviate**
- **Qdrant**
- **OpenSearch kNN**
- **Redis Vector**

(These are valid substitutes if you want self-hosted or open-source options.)

---

## 5. Prompt Assembly

### 5.1 What it is
A **prompt construction layer** that combines:
- System rules
- Retrieved templates
- Structured intent
- Mandatory constraints
- Code skeleton

This yields a single, structured prompt sent to the LLM.

### 5.2 How many prompts are needed
You typically need **2 prompt types**:
1. **Generation prompt** → produces code + explanation
2. **Correction prompt** → uses compiler/static errors to fix the code

### 5.3 How prompts are added
- Prompts are assembled dynamically by the **Context Assembler** based on the retrieved context and the current task state.

---

## 6. Skeleton Injection

### 6.1 Why inject into a skeleton
- Prevents the LLM from inventing invalid program structure.
- Guarantees required MQL5 scaffolding exists (`OnInit`, `OnTick`, etc.).
- Reduces hallucination surface area.

### 6.2 Why not generate full code directly
Direct full generation increases syntax errors and non-functional code. Skeleton injection constrains the LLM to fill only the logic portions, which is safer and more reliable.

---

## 7. Human-Readable Explanation

### 7.1 What it is
A **plain-English mapping** from user intent to code logic (e.g., “When SMA(50) crosses SMA(200), we open a buy.”)

### 7.2 Why it matters
- Builds trust (grey-box requirement).
- Supports academic evaluation and traceability.

### 7.3 Market usage
Many AI coding tools generate explanations, but the exact mechanism varies and is often not disclosed. Codex is designed to provide traceable evidence of actions, but explanation policy is not explicitly documented as a standalone feature. citeturn1search0turn1search6

---

## 8. Context Assembler vs Prompt Assembler

### 8.1 Are they the same?
Yes. In your architecture, **Context Assembler** is the **Prompt Assembler**. It takes retrieval results + intent + rules and builds the final prompt.

### 8.2 Market patterns
RAG pipelines explicitly include a “prompt construction / augmentation” stage between retrieval and generation. citeturn0search2turn0search3

---

## 9. Static Analyzer and Compiler Loop

### 9.1 Why we need a static analyzer
- Fast, deterministic pre-filter before costly compilation.
- Catches basic structural issues early.

### 9.2 Where it belongs
- **Static Analyzer belongs in C2** (fast, deterministic).
- **Compilation belongs in C3** (authoritative, slower).
- **Compiler feedback loop is managed by C2** but execution happens in C3.

### 9.3 Best static analysis approaches
1. **Regex + structural checks** (MVP)
2. **Lightweight parser** (more accurate)
3. **Use MetaEditor output as the authoritative validator** (slow but final)

---

## 10. “Stages” in Architectural Style (Why We Need Them)

The **stages** describe how the AI Engine is structured for safety and determinism:
1. **Perception (deterministic)** → understand intent
2. **Generation (probabilistic)** → produce code
3. **Correction (deterministic)** → validate and loop

This staged design is common in agentic systems and RAG pipelines, where deterministic gates protect downstream steps from untrusted outputs. citeturn0search1turn0search2turn0search3

---

## Appendix: Public Links Mentioned

- Microsoft Multi-Agent Reference Architecture (Semantic Router pattern): https://microsoft.github.io/multi-agent-reference-architecture/docs/reference-architecture/Patterns.html
- NVIDIA LLM Router Blueprint: https://build.nvidia.com/nvidia/llm-router
- Agentic Design Patterns (LLM-based routing): https://agentic-design.ai/patterns/routing/llm-based-routing/
- Pinecone RAG overview: https://www.pinecone.io/solutions/rag/
- Pinecone namespaces: https://docs.pinecone.io/docs/namespaces
- v0 Model API docs: https://vercel.com/docs/v0/api
- v0 usage (Vercel Academy): https://docs.vercel.com/academy/ai-sdk/ui-with-v0
- Cursor Agent changelog: https://www.cursor.com/en/changelog/agent-is-ready-and-ui-refresh
- Cursor Agent planning docs: https://docs.cursor.com/agent/planning
- OpenAI Codex overview: https://openai.com/index/introducing-codex/
- OpenAI Codex cloud docs: https://platform.openai.com/docs/codex/overview
- OpenAI Codex help center: https://help.openai.com/en/articles/11369540/

