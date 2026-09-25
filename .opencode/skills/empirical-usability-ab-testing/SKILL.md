---
name: empirical-usability-ab-testing
description: >-
  Designs controlled usability experiments, formulates testable UX hypotheses, configures A/B testing
  variant assignments, calculates System Usability Scale (SUS) scores, and instruments frontend telemetry
  to measure user conversion, task completion time, and error rates with statistical rigor.
artifact: usability-experiment.md
version: 3.0.0
---

# Empirical Usability Evaluation, Controlled Experiments & A/B Testing

Design intuition and expert heuristics are only hypotheses until validated against empirical data from real users. Empirical evaluation in Human-Computer Interaction encompasses both qualitative observational lab testing and quantitative large-scale online experimentation (A/B testing and telemetry).

> **Heisenberg OS Operational Contract:**  
> When invoked for experiments, feature flag rollouts, or telemetry tasks, the agent outputs `.heisenberg/artifacts/<task-id>/usability-experiment.md` defining Null/Alternative hypotheses, SUS scoring targets, and telemetry schemas before deploying code.

---

## 1. Evaluation Taxonomy: Formative vs. Summative & Settings

When planning usability evaluations, select the appropriate methodology:

| Dimension | Formative Evaluation | Summative Evaluation |
| :--- | :--- | :--- |
| **Stage** | Mid-development (prototypes, MVP, iterative sprints). | Post-release or milestone sign-off (production). |
| **Purpose** | Diagnose friction points to inform rapid redesign ("chef tasting the soup"). | Benchmark performance against quantitative SLAs ("guests eating the soup"). |
| **Typical Metrics** | Think-aloud comments, error observations, confusion points. | Task success rate (%), time-on-task (seconds), SUS score (0–100), conversion rate. |

### Settings: Controlled vs. Natural
- **Controlled Settings (Lab Experiments):** High internal validity. Isolate independent variables (e.g., measuring exact millisecond task speed differences in an eye-tracking lab).
- **Natural Settings (Field Studies):** High ecological validity. Observe users using the software in their authentic, noisy, interrupt-driven environments.

---

## 2. Scientific Experiment Design: Variables & Hypotheses

Every experiment run by a development team must follow scientific rigor:

### Variables Framework
- **Independent Variable (IV):** The interface factor you manipulate (e.g., Single-page checkout vs. Multi-step checkout).
- **Dependent Variables (DV):** The measurable outcomes (e.g., Conversion Rate %, Time to Complete Task, Error Frequency).
- **Controlled Variables:** Factors held constant across both groups (e.g., network speed, product catalog, payment methods).

### Hypothesis Formulation Template
- **Null Hypothesis ($H_0$):** There is no significant difference in [Dependent Variable] between Design A and Design B.
- **Alternative Hypothesis ($H_1$):** Users on Design B will complete [Task] in significantly less time / with higher conversion than Design A.

### Subject Assignment Designs
1. **Between-Subjects (A/B Testing Standard):** Each participant experiences only *one* variant (either A or B). Eliminates learning bias; requires larger sample sizes.
2. **Within-Subjects (Lab Usability Standard):** Each participant completes tasks on *both* variant A and B. Requires smaller sample sizes; requires counterbalancing (alternating order A $\rightarrow$ B and B $\rightarrow$ A) to eliminate practice effects.

---

## 3. The System Usability Scale (SUS) Benchmark

The **System Usability Scale (SUS)** (Brooke, 1986) is the industry standard 10-item Likert questionnaire (1 = Strongly Disagree to 5 = Strongly Agree):

### The 10 Standard Questions:
1. I think that I would like to use this system frequently.
2. I found the system unnecessarily complex.
3. I thought the system was easy to use.
4. I think that I would need the support of a technical person to be able to use this system.
5. I found the various functions in this system were well integrated.
6. I thought there was too much inconsistency in this system.
7. I would imagine that most people would learn to use this system very quickly.
8. I found the system very cumbersome to use.
9. I felt very confident using the system.
10. I needed to learn a lot of things before I could get going with this system.

### SUS Score Calculation Algorithm ($0 - 100$):
$$\text{SUS} = 2.5 \times \left[ \sum_{i \in \{1,3,5,7,9\}} (R_i - 1) + \sum_{j \in \{2,4,6,8,10\}} (5 - R_j) \right]$$
- *Industry Average Score:* **68.0**. A score $\ge 80.3$ ranks in the top 10% (Grade A).

---

## 4. Client-Side Telemetry & Event Instrumentation

Agents building frontend components must instrument essential user interaction events. Capture these four standard interaction telemetry types:

```typescript
// Telemetry event schema
interface UXTelemetryEvent {
  eventName: 'click' | 'view' | 'error' | 'task_complete';
  componentId: string;
  userId: string;
  variant: 'control_A' | 'treatment_B';
  timestamp: number;
  metadata?: Record<string, string | number | boolean>;
}
```

### Critical Telemetry Signals to Instrument
1. **Time on Task:** Record delta between `task_start` and `task_complete`.
2. **Rage Clicks:** Detect when a user clicks the same element $\ge 3$ times within 1 second (indicates broken feedback or unresponsive UI).
3. **Form Abandonment:** Track the last focused input field before the user unloads or navigates away.
4. **Dead Clicks:** Track clicks on non-interactive elements that users mistakenly perceived as buttons (broken affordance).

---

## 5. Production A/B Testing Implementation Pattern (React / TypeScript)

When adding an A/B test, assign variants deterministically using a hash of the user ID to ensure a user never switches variants between page reloads:

```tsx
import React, { useMemo } from 'react';

// Simple deterministic hash to allocate 50/50 A/B split
function getExperimentVariant(userId: string, experimentId: string): 'control' | 'treatment' {
  let hash = 0;
  const str = `${userId}:${experimentId}`;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash) % 2 === 0 ? 'control' : 'treatment';
}

export function CheckoutExperience({ userId }: { userId: string }) {
  const variant = useMemo(() => getExperimentVariant(userId, 'exp_checkout_redesign_2026'), [userId]);

  const handleComplete = (orderId: string) => {
    // Dispatch experiment conversion telemetry
    analytics.track('checkout_completed', {
      experimentId: 'exp_checkout_redesign_2026',
      variant,
      userId,
      orderId,
      timestamp: Date.now(),
    });
  };

  return (
    <div>
      {variant === 'control' ? (
        <MultiStepCheckoutWizard onComplete={handleComplete} />
      ) : (
        <SinglePageFastCheckout onComplete={handleComplete} />
      )}
    </div>
  );
}
```

---

## 6. Experiment Analysis & Pitfalls to Avoid

When analyzing results:
1. **Statistical Significance ($p < 0.05$):** Do not declare a winner until the sample size meets minimum power calculations (typically $n \ge 1,000$ per variant for small conversion deltas).
2. **The "Peeking" Problem:** Never stop an experiment prematurely just because variant B appears ahead on Day 2; wait for the full test duration to account for weekday/weekend variance.
3. **Novelty Effect:** Returning users may initially struggle with a new UI simply because it changed, not because it is worse. Allow a burn-in stabilization period.

---

## 7. Actionable Verification Checklist for Agents

Before deploying an experiment or instrumenting evaluation metrics:
- [ ] **Testable Hypothesis:** Are the Null ($H_0$) and Alternative ($H_1$) hypotheses explicitly stated with measurable DVs?
- [ ] **Deterministic Assignment:** Does the user consistently see the exact same variant across all sessions and page reloads?
- [ ] **Telemetry In Place:** Are task start, completion, abandonment, and error events properly logged with timestamps?
- [ ] **SUS Rubric Configured:** Is the 10-question SUS benchmark ready for survey post-tests?
- [ ] **No Confounding Changes:** Does the experiment isolate a single clear variable rather than bundling 10 unrelated UI changes?
- [ ] **Fallback Default:** If telemetry fails or feature flags timeout, does the application safely fall back to the control baseline without breaking?
