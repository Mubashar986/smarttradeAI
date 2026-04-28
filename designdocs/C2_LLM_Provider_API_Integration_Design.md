```mermaid
flowchart LR
    ActorInfra([Infrastructure])
    subgraph FR16: Scalability
        UC1(FR16-01: Support Concurrent Users via Workers)
    end
    ActorInfra --> UC1
```