---
name: task-modeling-goms-klm
description: >-
  Models user task workflows using Hierarchical Task Analysis (HTA) and calculates
  predictive task completion times using GOMS and the Keystroke-Level Model (KLM)
  to optimize user interfaces for speed and efficiency.
artifact: task-model-goms.md
version: 3.0.0
---

# Task Modeling, GOMS & KLM for UI Optimization

When developing power-user interfaces, internal admin dashboards, checkout flows, or high-volume data-entry applications, engineering decisions should not rely on subjective guesswork. Human task performance can be analyzed hierarchically and modeled mathematically.

> **Heisenberg OS Operational Contract:**  
> When invoked for workflow optimization or high-frequency data entry, the agent outputs `.heisenberg/artifacts/<task-id>/task-model-goms.md` computing KLM execution times ($K, P, H, M, R$), eliminating homing overhead, and sizing targets with Fitts's Law before finalizing designs.

---

## 1. Hierarchical Task Analysis (HTA)

**HTA** decomposes a high-level user goal into hierarchical subtasks and specifies the exact conditions ("Plans") under which those subtasks are executed.

### Core HTA Components
1. **Goal:** The ultimate objective the user wants to accomplish (e.g., *Goal 0: Order lunch via corporate portal*).
2. **Subtasks:** Atomic operations required to achieve the goal (e.g., *1. Select restaurant, 2. Choose meal, 3. Checkout*).
3. **Plans:** Logical rules stating order, conditions, or cycles (e.g., *Plan 1: Do 1.1, then 1.2. If coupon available, do 1.3, then 1.4*).

### HTA Representation Format
```markdown
Goal 0: Export monthly sales report to CSV
  1. Navigate to Sales Analytics screen
  2. Configure date range filter
     2.1 Click "Start Date" picker
     2.2 Select month/year
     2.3 Click "End Date" picker
     2.4 Select month/year
  3. Apply segment filters (Plan 3: Optional, do if analyzing sub-teams)
  4. Generate and download file
     4.1 Click "Export" dropdown
     4.2 Select "CSV Format"
     4.3 Wait for file download

Plan 0: Do 1 -> 2 -> (3 if needed) -> 4.
```

---

## 2. The GOMS Family of Models

GOMS models human procedural knowledge for routine, error-free tasks:
- **G - Goals:** What the user accomplishes (e.g., "Find customer record").
- **O - Operators:** Primitive actions performed (e.g., Press key, Click button, Read text).
- **M - Methods:** Well-learned sequence of operators to accomplish a goal (e.g., *Search by ID* vs. *Scroll through table*).
- **S - Selection Rules:** Rules guiding which method to choose when alternatives exist (e.g., *"If Customer ID is known, use Method A; else use Method B"*).

### Variations in the GOMS Family
- **CMN-GOMS:** Card, Moran, and Newell's original formulation with strict sequential execution and hierarchical call trees.
- **KLM (Keystroke-Level Model):** Simplified, single-level version focused strictly on physical keystrokes and mental operators.
- **CPM-GOMS (Cognitive-Perceptual-Motor):** Parallel execution model using Critical Path Method for highly practiced telephone operators and fighter pilots.

---

## 3. The Keystroke-Level Model (KLM)

The **Keystroke-Level Model** predicts the execution time $T_{\text{execute}}$ for expert users performing routine tasks:

$$T_{\text{execute}} = \sum T_K + \sum T_P + \sum T_H + \sum T_M + \sum T_R$$

### Standard KLM Operator Timings

| Operator | Action Description | Standard Benchmark Time |
| :---: | :--- | :---: |
| **$K$** | **Keystroke / Keypress / Mouse Click** (Single key or mouse button down-up) | **$0.20\text{ s}$** (Expert) / **$0.28\text{ s}$** (Average) |
| **$P$** | **Pointing** (Moving mouse cursor to target area via Fitts's Law) | **$1.10\text{ s}$** |
| **$H$** | **Homing** (Switching hand position between keyboard and mouse) | **$0.40\text{ s}$** |
| **$M$** | **Mental Preparation** (Cognitive pause, decision, visual verification) | **$1.35\text{ s}$** (or $1.20\text{ s}$) |
| **$R(t)$** | **System Response Time** (System processing time where user must pause) | **$t\text{ s}$** (measured) |

### Fitts's Law for Pointing ($P$)
Pointing time $MT$ depends mathematically on the distance to the target ($D$) and the width of the target ($W$):
$$MT = a + b \log_2\left(\frac{2D}{W}\right)$$
*Implication:* Buttons located at the edges or corners of screen boundaries have infinite effective width ($W = \infty$), making them infinitely faster to click.

### KLM Mental Preparation ($M$) Placement Rules
To calculate accurate times, apply the standard heuristics for placing mental operators ($M$):
1. **Rule 1 (Task Initiation):** Place an $M$ before initiating any discrete chunk or sub-operation.
2. **Rule 2 (Cognitive Retrieval):** Place an $M$ before retrieving a value from memory (e.g., password, SKU).
3. **Rule 3 (Decision / Visual Verification):** Place an $M$ before selecting between multiple options or verifying feedback.
4. **Rule 4 (Suppression):** Do **NOT** place an $M$ between consecutive typing keystrokes in a single automated string (e.g., typing a familiar word has only one initial $M$, followed by continuous $K$'s).

---

## 4. Comparing UI Alternatives with KLM: A Practical Example

### Task: Delete an item from a table
Compare two UI designs for an admin dashboard:

#### Design A: Traditional Multi-Click Dropdown with Confirmation Modal
```markdown
1. Move hand to mouse:                                    H = 0.40s
2. Mental check: Locate the three-dot "Actions" button:    M = 1.35s
3. Point mouse to three-dot icon:                         P = 1.10s
4. Click three-dot icon:                                  K = 0.20s
5. Mental check: Locate "Delete" option in menu:          M = 1.35s
6. Point mouse to "Delete":                               P = 1.10s
7. Click "Delete":                                        K = 0.20s
8. System renders confirmation modal:                     R = 0.25s
9. Mental check: Verify modal title:                      M = 1.35s
10. Point to "Confirm Delete" modal button:               P = 1.10s
11. Click "Confirm Delete":                               K = 0.20s
---------------------------------------------------------------------
Total Estimated Execution Time (Design A):               8.65 seconds
```

#### Design B: Inline Action with Non-Blocking Undo Toast
```markdown
1. Move hand to mouse:                                    H = 0.40s
2. Mental check: Locate inline trash can icon:             M = 1.35s
3. Point mouse to trash can icon:                         P = 1.10s
4. Click trash can (item immediately vanishes + toast):   K = 0.20s
---------------------------------------------------------------------
Total Estimated Execution Time (Design B):               3.05 seconds
```
> **Result:** Design B achieves a **64.7% reduction** in task completion time and eliminates two modal interruptions.

---

## 5. Coding Agent Optimization Strategies

When optimizing UI code, apply these KLM-minimizing patterns:

### Strategy 1: Eliminate Homing ($H$) via Keyboard First Design
Support continuous keyboard workflows without requiring users to reach for the mouse:
```tsx
// Enable keyboard navigation for search dropdown
useEffect(() => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      searchRef.current?.focus(); // Eliminates H (0.40s) + P (1.10s)
    }
  };
  window.addEventListener('keydown', handleKeyDown);
  return () => window.removeEventListener('keydown', handleKeyDown);
}, []);
```

### Strategy 2: Reduce Mental Operators ($M$) via Autocomplete & Smart Defaults
Prefill known form values and provide autocompletion to replace cognitive recollection ($M$) and typing ($K$) with single-stroke selection:
```tsx
// Autocomplete reduces M (recalling ID) + typing multiple K's
<input 
  list="frequent-recipients" 
  placeholder="Type name or email..." 
  autoFocus 
/>
```

---

## 6. Actionable Verification Checklist for Agents

When implementing or evaluating task workflows:
- [ ] **HTA Decomposition:** Has the complex task been broken down into subtasks with explicit execution plans?
- [ ] **KLM Benchmark:** Is the critical path modeled with $K$, $P$, $H$, $M$, $R$ calculations?
- [ ] **Homing Reduction:** Can repetitive data-entry forms be operated purely with `Tab`, `Arrow`, and `Enter` without mouse homing?
- [ ] **Target Ergonomics:** Are frequent click targets appropriately sized according to Fitts's Law?
- [ ] **Shortcuts:** Are standard accelerator keys (`Cmd+S`, `Cmd+K`, `Escape`) implemented for power users?
- [ ] **Latency ($R$):** Are background operations executed optimistically so users do not spend time blocked on system response ($R$)?
