# Architecture Output Template

Use this when producing a full architecture artifact.

## System Overview

Rewrite the idea clearly:

```text
The system helps [user] solve [problem] by [core workflow], producing [desired outcome].
```

State assumptions and constraints.

## Architecture Diagram (text)

Use a text diagram like:

```text
User
  -> UI Layer
  -> Application/API Layer
  -> AI Orchestration Layer
  -> Model/Tool Layer
  -> Data Layer
  -> Monitoring/Evaluation Layer
  -> User Result or Downstream Action
```

Adapt it to the actual system.

## Component Explanation

For each component, explain:

- Responsibility.
- Why it exists.
- What happens if it is removed.
- Main failure modes.

## Technology Choices

Compare:

```text
Option A: Simple/cheap approach
Option B: Production approach
Option C: Research/advanced approach
```

For AI/model choices, evaluate:

- Capability.
- Accuracy.
- Latency.
- Cost.
- Hardware.
- Deployment.
- Licensing.
- Scalability.

Choose the smallest option that fits the constraints.

## Data Flow

Explain:

- What data enters the system.
- Where it is stored.
- How it is processed.
- How it is retrieved.
- How it is versioned.
- How quality is checked.
- What data should not be stored.

## AI Pipeline

Cover:

- Prompting or orchestration.
- RAG, if needed.
- Fine-tuning, if justified.
- Tool use, if needed.
- Agent roles, if agents are justified.
- Validation and evals.
- Human review boundaries.
- Fallback behavior.

## Infrastructure Plan

Cover:

- Deployment target.
- CPU/GPU needs.
- Memory.
- Queues/workers.
- Storage.
- Scaling strategy.
- Monitoring.
- Cost controls.
- Training infrastructure, if any.
- Inference infrastructure.

## Risks

List:

- Product risks.
- Data risks.
- Security risks.
- Model risks.
- Reliability risks.
- Cost risks.
- Scaling risks.

For each major risk, include a mitigation.

## Roadmap

Use stages:

```text
Phase 1: MVP
Phase 2: Growing system
Phase 3: Production scale
```

Each phase should include build scope, verification, and what not to build yet.

## Future Improvements

Include improvements that are useful later but not required now, such as:

- More advanced models.
- Fine-tuning.
- Multi-agent workflows.
- Dedicated eval dashboards.
- Self-hosted inference.
- Advanced observability.
- Enterprise security controls.
