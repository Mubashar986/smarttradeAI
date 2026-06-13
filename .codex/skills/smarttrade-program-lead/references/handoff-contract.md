# Handoff Contract

Use these templates to keep separate specialist chats synchronized.

## Task Brief To A Specialist Chat

```text
Owner:
Mission:
Task:
Why it matters:
Read first:
In scope:
Out of scope:
Interfaces affected:
Acceptance criteria:
Verification required:
Forbidden moves:
Expected handoff:
```

## Specialist Handoff Back To Program Lead

```text
Owner:
Task:
Status:
Files changed:
Interfaces affected:
Decisions made:
Evidence:
Known gaps:
Risks or anomalies:
Needs from other roles:
Next recommended owner:
```

## Evidence Log Entry

```text
Evidence YYYY-MM-DD:
- Command or method:
- Result:
- Interpretation:
- Remaining gap:
```

## Handoff Review Checklist

- Does the handoff say what actually changed?
- Does it name affected interfaces and downstream consumers?
- Does it include evidence, not just confidence?
- Does it distinguish implemented, verified, blocked, and planned?
- Does it expose any scope, safety, security, or trading-claim risk?
- Does it say who should act next?

## Program State Update

Recommend updating project state when any of these changed:

- Requirement status changed.
- Interface contract changed.
- Verification evidence was collected.
- New anomaly or risk appeared.
- A blocker was resolved.
- The next safe action changed.
