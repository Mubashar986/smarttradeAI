# SmartTradeAI Anomaly Log

Status: Active
Date: 2026-06-02

## Severity

| Severity | Meaning |
| --- | --- |
| 1 | Blocks baseline, build, or safe demo. |
| 2 | Blocks MVP feature or creates misleading claim. |
| 3 | Important gap but workaround exists. |
| 4 | Cosmetic or documentation-only issue. |

## Open Anomalies

| ID | Severity | Title | Description | Status | Next Action |
| --- | --- | --- | --- | --- | --- |
| ANOM-001 | 2 | Docker config exposed real-looking API key | Earlier docker compose config expanded a real-looking Gemini key from environment. | Open | Rotate key before demo/sharing; avoid printing secrets. |
| ANOM-002 | 3 | Host cargo unavailable | Rust toolchain is intentionally Docker-based, but host cargo command failed. | Open | Verify through Docker, not host cargo. |
| ANOM-003 | 2 | Compile validation uses stub mode without C3 URL | Runtime can return success with source=stub when compiler is absent. | Open | Make UI/docs distinguish static/stub/real compile. |
| ANOM-004 | 2 | Explanation path is placeholder | Current server response says explanation path is still being wired. | Open | Implement MVP explanation based on spec/generation. |
| ANOM-005 | 2 | Sessions/tasks are in-memory | Restart loses session/task state. | Open | Document as MVP limitation or implement persistence later. |
| ANOM-006 | 3 | RAG/Pinecone not active in runtime | Search uses local skeleton fallback even if Pinecone key exists. | Open | Do not market as full RAG until retrieval evidence exists. |
| ANOM-007 | 3 | .omx is untracked runtime state | Runtime state exists but is not product code. | Open | Keep out of commits unless intentionally needed. |
| ANOM-008 | 2 | Docker engine not running | `docker compose build c2-engine` could not connect to Docker Desktop Linux engine. | Open | Start Docker Desktop/Linux engine, then rerun Docker build and tests. |

## Closed Anomalies

| ID | Title | Resolution |
| --- | --- | --- |
| ANOM-BASE-001 | No recoverable June branch existed | Created before-june and initial_mvp on 2026-06-02. |
