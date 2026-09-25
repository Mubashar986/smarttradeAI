---
name: heuristic-evaluation-audit
description: >-
  Conducts expert usability inspections and automated code audits on UI components,
  dashboards, forms, and web interfaces using Jakob Nielsen’s 10 Usability Heuristics
  and assigns prioritized 0–4 severity ratings with concrete code fixes.
artifact: heuristic-audit.md
version: 3.0.0
---

# Heuristic Usability Evaluation & UI Code Auditing

Usability testing with end users can be resource-intensive and time-consuming. **Heuristic Evaluation** (developed by Jakob Nielsen and Rolf Molich) is an expert inspection method that allows software engineers and coding agents to rapidly identify usability defects in interfaces and codebases *before* shipping to production.

> **Heisenberg OS Operational Contract:**  
> When invoked for UI audits, redesigns, or pre-release inspections, the agent outputs `.heisenberg/artifacts/<task-id>/heuristic-audit.md` auditing the codebase against Nielsen's 10 Heuristics, assigning 0–4 severity ratings, and providing concrete code remediations.

---

## 1. Nielsen's 10 Usability Heuristics (Reference Guide)

When reviewing any component or page layout, evaluate against these 10 criteria:

| # | Heuristic | Definition & UI Requirement | Code Anti-Pattern | Correct Implementation |
| :---: | :--- | :--- | :--- | :--- |
| **H1** | **Visibility of System Status** | System must keep users informed about what is going on through prompt, appropriate feedback. | Submitting a form with no button spinner or progress indicator. | Display loading spinners, upload progress percentages, and status toasts. |
| **H2** | **Match Between System & Real World** | System must speak the users' language with familiar concepts rather than internal system jargon. | Showing backend exception: `"Error: 0x80041010 SQLNullConstraint"`. | Show human message: `"We couldn't save your profile because phone number is empty."` |
| **H3** | **User Control & Freedom** | Users need a clearly marked "emergency exit" to undo mistakes without hassle. | Deleting a record immediately without an Undo button or cancel option. | Implement an Undo toast (`"Item deleted. [Undo]"`), back buttons, and `Esc` handlers. |
| **H4** | **Consistency & Standards** | Users should not wonder whether different words or icons mean the same thing across screens. | Using a trash can icon on one page and a red "X" icon on another for deletion. | Centralize design tokens and standard icon button components in a design system. |
| **H5** | **Error Prevention** | Prevent problems from occurring in the first place through smart constraints and confirmations. | Allowing users to submit invalid credit card formats, crashing on backend. | Implement input masks, formatters, and disable submit until valid. |
| **H6** | **Recognition Rather Than Recall** | Minimize user memory load by making elements, actions, and options visible. | Hiding keyboard shortcuts or required codes without autocomplete or hints. | Provide search autocompletion, recent history chips, and inline tooltips. |
| **H7** | **Flexibility & Efficiency of Use** | Accelerators for expert users while keeping basic flows simple for novices. | Requiring all users to click through 4 steps with no keyboard shortcuts. | Provide quick search (`Cmd+K`), keyboard navigation, and bulk-action selections. |
| **H8** | **Aesthetic & Minimalist Design** | Interfaces should not contain irrelevant or rarely needed information that clutters focus. | Cluttered dashboards packed with 50 unformatted metric cards. | Progressive disclosure: show essential metrics first with expandable drawers. |
| **H9** | **Recognize, Diagnose & Recover from Errors**| Error messages should be in plain language, precisely indicate the problem, and suggest a solution. | Displaying generic `"Submission failed"` banner at the top of a 30-field form. | Highlight the exact field in red with helper text: `"Password must contain 8+ characters."` |
| **H10**| **Help & Documentation** | Provide easy-to-search, contextual guidance focused on the user's specific task. | Linking to a generic 200-page manual PDF. | Add inline `?` helper tooltips and contextual drawer walkthroughs. |

---

## 2. Nielsen's Severity Rating Scale (0 to 4)

Every identified usability issue must be assigned a severity score based on **Frequency**, **Impact**, and **Persistence**:

| Rating | Severity Level | Definition | Action Requirement |
| :---: | :--- | :--- | :--- |
| **0** | **No Problem** | Not a usability issue; personal preference or aesthetic variation. | No action required. |
| **1** | **Cosmetic** | Minor inconvenience; does not impede task completion (e.g., slight font mismatch). | Fix only if extra development time permits. |
| **2** | **Minor** | Causes slight friction or brief hesitation; user easily recovers on their own. | Low priority; schedule for subsequent sprint. |
| **3** | **Major** | Significantly delays task completion, causes confusion, or frustrates users. | High priority; must be resolved before next release. |
| **4** | **Catastrophe** | Critical failure; causes data loss, complete blocker, or prevents goal achievement. | **Blocker;** emergency fix required immediately. |

---

## 3. Step-by-Step Usability Audit Procedure for Agents

When requested to review or audit a screen or component:

### Step 1: Component Walkthrough
Examine every possible interactive state:
- Default / Empty State
- Hover / Focus / Active States
- Pending / Async Loading State
- Partial Error / Validation State
- Empty Data vs. High-Volume Data State

### Step 2: Compile the Usability Audit Matrix
Document defects using this standardized Markdown table:

```markdown
### Usability Inspection Audit Report: [Page / Component Name]

| ID | Screen / Element | Heuristic Violated | Severity (0–4) | Observed Usability Defect | Concrete Code Remediation |
|---|---|---|:---:|---|---|
| 01 | Checkout Button | **H1: Visibility of Status** | **3 (Major)** | Clicking button causes 3s freeze with zero spinner; users click multiple times. | Add loading spinner, disable button on click, set `aria-busy="true"`. |
| 02 | Delete Customer Modal | **H3: User Control & Freedom** | **4 (Catastrophe)** | Clicking "Delete" permanently drops record immediately without confirmation or undo. | Add two-step confirmation modal with red warning badge and 10s undo banner. |
| 03 | SKU Search Input | **H6: Recognition over Recall** | **2 (Minor)** | Requires users to know exact 12-digit internal alphanumeric SKU code. | Implement fuzzy autocomplete search supporting product names and categories. |
| 04 | Form Error Banner | **H9: Error Recovery** | **3 (Major)** | Red alert says "Server error 500" when email already exists in DB. | Highlight email field specifically: "This email is already registered. Log in instead." |
```

### Step 3: Implement Code Fixes Directly
Convert audit findings directly into code refactors:
```tsx
// Fully compliant with H1 (Status), H3 (Control), H5 (Prevention), H9 (Error Recovery)
function SaveButton({ onSave }: { onSave: () => Promise<void> }) {
  const [state, setState] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const [errorMessage, setErrorMessage] = useState('');

  const handleClick = async () => {
    setState('saving');
    try {
      await onSave();
      setState('saved');
      setTimeout(() => setState('idle'), 2500);
    } catch (err: any) {
      setState('error');
      setErrorMessage(err.message || 'Network disconnected. Check your connection.');
    }
  };

  return (
    <div className="flex flex-col gap-1">
      <button 
        onClick={handleClick} 
        disabled={state === 'saving'}
        aria-busy={state === 'saving'}
        className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 transition flex items-center justify-center gap-2"
      >
        {state === 'saving' && <span className="animate-spin">⏳</span>}
        {state === 'saving' && 'Saving changes...'}
        {state === 'saved' && '✅ Changes Saved!'}
        {state === 'idle' && 'Save Changes'}
        {state === 'error' && '⚠️ Retry Save'}
      </button>
      {state === 'error' && (
        <span role="alert" className="text-sm text-red-600 font-medium">
          {errorMessage}
        </span>
      )}
    </div>
  );
}
```

---

## 4. Automated CI/CD Usability Linter Rule Schema

Agents can configure automated checks in PR pipelines to enforce heuristics:
```json
{
  "rules": {
    "require-aria-labels-on-icon-buttons": "error",
    "require-form-loading-indicator": "error",
    "disallow-raw-error-stack-rendering": "error",
    "require-explicit-undo-on-destructive-endpoints": "error",
    "enforce-minimum-touch-target-size": 48
  }
}
```

---

## 5. Actionable Verification Checklist for Agents

Before completing a UI inspection:
- [ ] **Heuristic Coverage:** Have all 10 heuristics been evaluated across the target UI?
- [ ] **Severity Scored:** Does every identified defect have a justified severity rating (0 to 4)?
- [ ] **H1 & Feedback:** Are all async network states accompanied by explicit loading and completion indicators?
- [ ] **H3 & Undo:** Can all destructive or modifying actions be undone or cancelled?
- [ ] **H5 & Error Prevention:** Are validation rules enforced before submitting to avoid server rejections?
- [ ] **H9 & Error Messages:** Are error descriptions written in plain, human-friendly English with recovery guidance?
- [ ] **Catastrophe Check:** Are there any Severity 4 issues remaining unresolved?
- [ ] **Accessibility Contrast:** Do all text elements pass WCAG AA contrast standards?
