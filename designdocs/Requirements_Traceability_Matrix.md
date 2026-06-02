# Requirements Traceability Matrix

Status: Draft
Date: 2026-06-02

## Legend

| Status | Meaning |
| --- | --- |
| Planned | Not implemented or not confirmed. |
| Partial | Some implementation exists. |
| Implemented | Code exists and contract is documented. |
| Verified | Evidence has been collected. |

## MVP Requirements

| ID | Requirement | Design Artifact | Current Code Touchpoint | Verification | Status |
| --- | --- | --- | --- | --- | --- |
| REQ-C2-001 | System shall create a conversation session through canonical /v1 API. | Interface_Control_Document.md | services/c2-engine/rust/crates/server/src/lib.rs | API integration test | Implemented |
| REQ-C2-002 | System shall list and fetch sessions through canonical /v1 API. | V1_API_Contract_Hardening.md | services/c2-engine/rust/crates/server/src/lib.rs | API integration test | Implemented |
| REQ-C2-003 | System shall accept a user turn and return a task id. | Interface_Control_Document.md | server/src/lib.rs | API integration test | Partial |
| REQ-C2-004 | System shall expose task status by task id. | Session_Task_Turn_Contract_Hardening.md | server/src/lib.rs | API integration test | Partial |
| REQ-C2-005 | System shall stream session events over SSE as primary realtime transport. | Realtime_Transport_Decision.md | server/src/lib.rs | SSE manual/integration test | Partial |
| REQ-C2-006 | System shall classify strategy-related user intent. | C2_AI_Engine_Architecture.md | runtime/src/smarttrade_tools.rs | Unit tests | Partial |
| REQ-C2-007 | System shall extract required strategy fields from user text. | SmartTradeAI_ConOps.md | runtime/src/smarttrade_tools.rs | Unit tests | Partial |
| REQ-C2-008 | System shall ask clarification questions when required fields are missing. | SmartTradeAI_ConOps.md | runtime/src/smarttrade_tools.rs, server/src/lib.rs | Worker flow test | Partial |
| REQ-C2-009 | System shall generate MQL5 from controlled templates when fields are complete. | C2_AI_Engine_Architecture.md | runtime/src/smarttrade_tools.rs, services/c2-engine/skeletons | Unit and integration tests | Partial |
| REQ-C2-010 | System shall run basic static validation on generated MQL5. | Verification_Plan.md | runtime/src/smarttrade_tools.rs | Unit tests | Partial |
| REQ-C2-011 | System shall explain generated strategy logic in plain English. | SmartTradeAI_ConOps.md | runtime/src/smarttrade_tools.rs, server/src/lib.rs | Manual review/test | Planned |
| REQ-C2-012 | System shall save generated strategies and expose strategy CRUD. | Interface_Control_Document.md | server/src/lib.rs, db/init.sql | API integration test | Partial |
| REQ-C2-013 | System shall clearly label compile validation as stubbed unless a real compiler is configured. | Verification_Plan.md | runtime/src/smarttrade_tools.rs | Unit/manual inspection | Partial |
| REQ-C2-014 | System shall include disclaimers in generated service deliverables. | SmartTradeAI_ConOps.md | To be defined | Manual inspection | Planned |
| REQ-C2-015 | System shall preserve a recoverable baseline before new MVP work. | NASA_Lite_Engineering_Plan.md | git branches before-june, initial_mvp | git log/status | Verified |

## Service Requirements

| ID | Requirement | Verification | Status |
| --- | --- | --- | --- |
| REQ-SVC-001 | Service offer shall avoid profit guarantees. | Review landing/offer copy | Planned |
| REQ-SVC-002 | Each deliverable shall state prototype/educational use. | Deliverable checklist | Planned |
| REQ-SVC-003 | Each client strategy shall have structured parameters before code generation. | Intake checklist | Planned |
| REQ-SVC-004 | Client-facing validation claims shall distinguish compile/static validation from profitability. | Review copy and output | Planned |

## Open Traceability Gaps

1. Docker-based test evidence has not been collected yet.
2. Explanation path exists as a placeholder and needs a real MVP implementation.
3. Sessions and tasks are still in-memory.
4. Compile validation is stubbed without C3 compiler URL.
5. RAG/Pinecone is currently local skeleton fallback.
