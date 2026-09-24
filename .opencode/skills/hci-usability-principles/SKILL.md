---
name: hci-usability-principles
description: >-
  Audits, validates, and guides the implementation of UI/UX code according to foundational
  Human-Computer Interaction (HCI) usability goals, user experience goals, Norman's design
  principles (Affordance, Signifiers, Feedback, Mapping, Constraints, Consistency),
  Shneiderman's 8 Golden Rules, and the five core interaction types.
artifact: hci-interaction-spec.md
version: 3.0.0
---

# HCI Usability & Interaction Principles

> **Heisenberg OS Operational Contract:**  
> When invoked for a task with a UI surface, the agent generates `.heisenberg/artifacts/<task-id>/hci-interaction-spec.md` before editing product code. This artifact establishes concrete Norman affordance signifiers, 4-stage feedback state machines, defensive constraints, and WCAG AA accessibility compliance.

---

## 1. Core Framework: Usability vs. User Experience (UX) Goals

When designing or reviewing any user-facing code, evaluate both functional usability and experiential UX:

### Usability Goals (Objective & Quantifiable)
- **Effectiveness:** Does the feature support users in accomplishing their tasks accurately?
- **Efficiency:** How quickly and with how few steps can users perform their actions?
- **Safety:** Does the system protect users from dangerous conditions and unintended errors (e.g., destructive actions with undo)?
- **Utility:** Does the product provide the exact set of tools and operations users actually need?
- **Learnability:** Can a first-time user figure out how to operate the interface without training?
- **Memorability:** Can an occasional user return after weeks and remember how to use it?

### User Experience Goals (Subjective & Affective)
- **Desirable States:** Engaging, satisfying, enjoyable, helpful, motivating, aesthetically pleasing, empowering.
- **Undesirable States to Eliminate:** Annoying, frustrating, patronizing, boring, confusing, tedious, anxiety-inducing.

---

## 2. Norman's Fundamental Design Principles

Every component, interaction, or screen written by an agent must uphold Don Norman’s 6 core principles:

| Principle | Meaning & UI Requirement | Practical Code Example |
| :--- | :--- | :--- |
| **Affordance** | Physical or visual properties that indicate how an object can be used. | A button looks clickable (elevation, rounded borders, distinct contrast). |
| **Signifiers** | Explicit visual clues communicating *where* and *what* action to take. | A text field has an icon and placeholder: `"Search by title, author..."` |
| **Feedback** | Returning immediate, perceptible information about what action has been done. | Showing a spinner during async fetch, then green checkmark `"Saved!"`. |
| **Constraints** | Restricting the possible actions to prevent user errors. | Disabling the `"Submit"` button until required fields pass validation. |
| **Mapping** | Natural spatial or conceptual relationship between controls and their effects. | Slider moving right increases volume; reorder handle matches dragged card position. |
| **Consistency** | Using similar operations and elements for similar tasks across the entire app. | Standardizing destructive modals (red confirmation button always on the bottom right). |

### Perceived vs. Real Affordance
- *Real Affordance:* Physical capability (e.g., a physical touchscreen accepts touch anywhere).
- *Perceived Affordance:* What the user believes can be done based on visual cues (e.g., a card with shadow looks like a raised button). Always ensure perceived affordance matches real capability.

---

## 3. Shneiderman's 8 Golden Rules of Interface Design

In addition to Norman's principles, enforce Ben Shneiderman’s foundational Golden Rules:
1. **Strive for Consistency:** Uniform layouts, typography, terminology, and action sequences across all screens.
2. **Seek Universal Usability:** Cater to diverse user skill levels, physical abilities, and screen sizes.
3. **Offer Informative Feedback:** Every operator action must have system feedback (modest for frequent, prominent for rare).
4. **Design Dialogs to Yield Closure:** Sequences of actions should be organized into groups with beginning, middle, and end.
5. **Prevent Errors:** Make it impossible to submit erroneous input (e.g., date pickers rather than freeform text).
6. **Permit Easy Reversal of Actions:** Relieve anxiety by making actions non-destructive and easily reversible.
7. **Support Internal Locus of Control:** Make users the initiators of actions rather than feeling forced by system surprises.
8. **Reduce Short-Term Memory Load:** Humans hold only $7 \pm 2$ chunks of info; keep information accessible without memorization.

---

## 4. The Five Interaction Types

When choosing or implementing user workflows, categorize and implement the interaction appropriately:

1. **Instructing:** One-way command entry where users tell the system what to do (e.g., search bars, terminal commands, keyboard shortcuts).
2. **Conversing:** Two-way dialogue where system and user take turns (e.g., chat interfaces, interactive setup wizards).
3. **Manipulating:** Direct manipulation of visual objects in virtual space (e.g., drag-and-drop kanban boards, pinch-to-zoom images).
4. **Exploring:** Navigating through structured information or 3D environments (e.g., interactive dashboards, data drill-downs, virtual tours).
5. **Responding:** System prompts or initiates an event based on sensor/state changes and user responds (e.g., push notifications, safety alerts).

---

## 5. Agent Step-by-Step UI Implementation Workflow

When generating or refactoring UI components, follow this procedure:

### Step 1: Identify the Primary Task & Interaction Type
- Clarify what the user wants to achieve (e.g., "Download report", "Filter table").
- Match the interaction type (e.g., direct manipulation vs. command instruction).

### Step 2: Ensure Explicit Signifiers and Affordance
- Never display an action that looks like static text.
- Use explicit labels, icons with accessible `aria-label`, and interactive hover/focus states:
  ```css
  /* Good affordance & signifier */
  .primary-btn {
    cursor: pointer;
    transition: background-color 0.15s ease, transform 0.05s ease;
  }
  .primary-btn:hover { background-color: var(--color-primary-dark); }
  .primary-btn:active { transform: scale(0.98); }
  .primary-btn:focus-visible { outline: 2px solid var(--color-focus-ring); }
  ```

### Step 3: Implement Multi-Stage Feedback
Feedback must communicate **Pending**, **Success**, and **Failure** states explicitly. Never leave the user in ambiguity:
```tsx
// Anti-pattern: Button disables, nothing else changes
<button onClick={handleDownload}>Download</button>

// Recommended pattern: Explicit multi-stage feedback
const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');

return (
  <button 
    onClick={handleDownload} 
    disabled={status === 'loading'}
    aria-live="polite"
    className={`btn btn-${status}`}
  >
    {status === 'idle' && <span>📥 Download PDF</span>}
    {status === 'loading' && <span>⏳ Downloading...</span>}
    {status === 'success' && <span>✅ Downloaded to your folder</span>}
    {status === 'error' && <span>⚠️ Download Failed. Click to retry.</span>}
  </button>
);
```

### Step 4: Apply Defensive Constraints
- Prevent illegal input via input masks, formatters, and sensible defaults:
  - Date range: Ensure `Start Date` constraint enforces `endDate >= startDate`.
  - Number fields: Disallow negative numbers or non-numeric characters at keystroke level.
  - Form submission: Provide inline validation before submission rather than a batch of obscure errors afterward.

### Step 5: Preserve Internal and External Consistency
- **Internal Consistency:** Maintain identical padding, typography scales, iconography, button colors (e.g., Primary = Blue, Success = Green, Destructive = Red).
- **External Consistency:** Conform to established platform conventions (e.g., `Ctrl+Z` / `Cmd+Z` for undo, top-right `X` for close, search bar with magnifying glass).

---

## 6. Common UX Traps & Code Anti-Patterns

| Anti-Pattern | UX Violation | Agent Remediation |
| :--- | :--- | :--- |
| Silent Failure / Silent Success | Broken Feedback | Always provide immediate inline confirmation, toast alert, or state transition. |
| Greyed out button with no explanation | Broken Affordance / Constraints | Add tooltip or helper text explaining *why* the button is disabled and how to enable it. |
| Destructive action executed on single click | Broken Safety | Implement two-step confirmation (modal or undo banner with 5-second grace period). |
| Moving targets or layout shifts during load | Broken Mapping / Efficiency | Reserve layout skeleton dimensions (`min-height`, placeholder skeletons). |
| Ambiguous icon buttons without labels | Missing Signifiers | Always add visible text or `aria-label` / `title` attributes for tooltips. |
| Inverted mapping on toggles | Broken Mapping | Ensure toggle state explicitly displays whether it represents ON or OFF. |

---

## 7. Actionable Verification Checklist for Agents

Before completing any frontend or UI task, verify:
- [ ] **Affordance & Hover:** Do all interactive elements visually react to hover, focus, and active states?
- [ ] **Signifiers:** Is every icon's meaning instantly clear, backed by text or accessible labels?
- [ ] **Feedback:** Does every asynchronous operation (fetch, upload, save) display visual feedback within 100ms?
- [ ] **Error Prevention:** Are destructive actions guarded by confirmation or non-blocking undo?
- [ ] **Constraints:** Are invalid inputs prevented before hitting backend validation?
- [ ] **Efficiency:** Can power users navigate and execute common actions using keyboard shortcuts (`Enter`, `Esc`, `Tab`)?
- [ ] **Accessibility:** Do contrast ratios pass WCAG AA (minimum 4.5:1 for body text)?
- [ ] **Closure:** Do multi-step tasks offer clear completion milestones with informative summary screens?
