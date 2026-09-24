---
name: emotional-ux-interaction-design
description: >-
  Guides the design of emotionally engaging, delightful user interfaces by applying
  Norman’s 3 levels of emotional design (Visceral, Behavioral, Reflective), crafting
  expressive micro-interactions, and systematically identifying and removing frustrating,
  annoying UI traps and dark patterns.
artifact: emotional-ux-spec.md
version: 3.0.0
---

# Emotional Interaction & Expressive UX Design

Human decision-making and problem-solving are fundamentally coupled with emotion. When users feel anxious, their cognitive bandwidth narrows, making them prone to mistakes and quick to abandon a product. Conversely, when an interface feels responsive, calm, and delightful, users are more tolerant of minor difficulties and learn faster.

> **Heisenberg OS Operational Contract:**  
> When invoked for affective UI design or error handling tasks, the agent outputs `.heisenberg/artifacts/<task-id>/emotional-ux-spec.md` applying Norman's 3-level model (Visceral, Behavioral, Reflective) and validating zero frustrating anti-patterns before completion.

---

## 1. Don Norman's Three Levels of Emotional Processing

Every user evaluates software across three cognitive tiers simultaneously:

```
          ┌───────────────────────────┐
          │   Reflective Level        │  (Meaning, pride, self-image, long-term memory)
          ├───────────────────────────┤
          │   Behavioral Level        │  (Usability, performance, control, fluidity)
          ├───────────────────────────┤
          │   Visceral Level          │  (Sensory, rapid, aesthetic, gut emotional reaction)
          └───────────────────────────┘
```

### 1. Visceral Design (Look, Feel & Sensory Aesthetics)
- **What it is:** The subconscious, immediate physical reaction within milliseconds of viewing the screen.
- **UI Focus:** Visual harmony, typography hierarchy, crisp icons, subtle shadows, balanced color palettes, and polished micro-transitions.
- **Agent Rule:** High visceral appeal sets positive expectations, but can never compensate for broken behavioral usability.

### 2. Behavioral Design (Use, Performance & Feeling in Control)
- **What it is:** The practical experience of using the tool to achieve real goals efficiently.
- **UI Focus:** Immediate response times (<100ms), intuitive mental models, smooth scrolling, and reliable error recovery.
- **Agent Rule:** If a product is clunky, slow, or unpredictably crashes, users experience instant irritation regardless of pretty graphics.

### 3. Reflective Design (Identity, Meaning & Narrative)
- **What it is:** Conscious thought, post-experience rationalization, pride of ownership, and brand affinity.
- **UI Focus:** Customer service warmth, celebration of user milestones (e.g., "Project Complete! 🎉"), privacy respect, and ethical design.
- **Agent Rule:** Users remember how a product made them feel about *themselves*.

---

## 2. Expressive Interfaces & Tasteful Delight

Expressive interface elements communicate personality, state transitions, and empathy:

### Positive Expressive Techniques
1. **Dynamic Micro-Interactions:** Subtle button click depress effects, bouncy checkmark completions, and progress fills that make interfaces feel alive:
   ```css
   /* Tactile button micro-interaction */
   .btn-expressive {
     transition: transform 0.1s ease, box-shadow 0.1s ease;
   }
   .btn-expressive:active {
     transform: translateY(2px) scale(0.98);
     box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
   }
   ```
2. **Empathetic Error Tone:** Replace robotic or accusatory messages with supportive, solution-oriented copy.
3. **Empty States with Character:** Transform blank screens into welcoming launchpads with helpful starter templates.

---

## 3. Anthropomorphism & Lessons from History

Anthropomorphism involves attributing human qualities to software agents or user interfaces:
- **Historical Failures (Clippy & Microsoft Bob):** Popups that interrupt workflow with unsolicited cheerful greetings ("It looks like you're writing a letter!") induce immediate rage because they violate behavioral control and lack contextual intelligence.
- **Modern Principle:** Virtual agents and AI assistants must remain **humble, unobtrusive, and user-invoked**. Never interrupt a focused user unless responding to a direct query or mission-critical safety warning.

---

## 4. The Anatomy of Annoying Interfaces (What to Eliminate)

Negative emotional states—such as frustration, anger, and cognitive fatigue—arise from predictable engineering anti-patterns:

| Source of Annoyance | Psychological Impact | Concrete Agent Remediation |
| :--- | :--- | :--- |
| **System crashes / Unsaved work lost** | Anger, acute sense of betrayal and wasted labor. | Implement continuous auto-saving to `localStorage` and draft recovery buffers. |
| **Too many steps before discovering error** | Resentment, cognitive exhaustion. | Provide immediate inline form validation rather than waiting for server submission. |
| **Intrusive anthropomorphism / "Clippy"** | Irritation, feeling patronized. | Avoid unsolicited talking mascots; provide quiet, non-blocking contextual hints instead. |
| **Autoplaying audio or video** | Startle response, embarrassment in public spaces. | Keep media strictly muted by default; require explicit user initiation. |
| **Blaming / Accusatory language** | Defensiveness, feeling incompetent. | Rephrase error messages to take the blame off the user (see examples below). |
| **Confirm-shaming / Dark patterns** | Cynicism, distrust of brand integrity. | Ensure opt-out choices are neutral (e.g., *"No thanks"*, never *"No, I hate saving money"*). |

---

## 5. Copywriting Refactoring: Blaming vs. Empathetic UX Copy

Agents writing UI text and modal dialogs should always apply the empathetic tone pattern:

```markdown
# Anti-Pattern: Accusatory, Technical, Blaming
❌ "Error 400: You entered an illegal date format! Invalid parameter 'birthdate'."
❌ "User Error: Password too weak. Try again."
❌ "Failed. You cannot delete this directory because it is not empty."

# Recommended Pattern: Empathetic, Clear, Solution-Oriented
✅ "Please enter your date of birth as DD/MM/YYYY (e.g., 25/12/1995)."
✅ "To protect your account, please add at least 1 number or special symbol to your password."
✅ "This folder contains 4 active files. Would you like to view them or delete all contents together?"
```

---

## 6. Designing Calming, Non-Frustrating Error States in Code

```tsx
// Example: Empathetic error boundary component
export function GracefulErrorView({ error, onRetry }: { error: Error; onRetry: () => void }) {
  return (
    <div className="max-w-md mx-auto p-6 my-8 text-center bg-slate-50 border border-slate-200 rounded-xl shadow-sm">
      <div className="text-4xl mb-3" aria-hidden="true">🌱</div>
      <h3 className="text-lg font-bold text-slate-800 mb-1">
        We hit a temporary bump
      </h3>
      <p className="text-sm text-slate-600 mb-4">
        Your data is completely safe. We had trouble loading your recent project activity.
      </p>
      <div className="flex justify-center gap-3">
        <button
          onClick={onRetry}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition"
        >
          Try Again
        </button>
        <button
          onClick={() => window.location.href = '/support'}
          className="px-4 py-2 border border-slate-300 text-slate-700 rounded-lg hover:bg-slate-100 transition"
        >
          Contact Support
        </button>
      </div>
    </div>
  );
}
```

---

## 7. Actionable Verification Checklist for Agents

Before completing any frontend or copywriting task:
- [ ] **Visceral Polish:** Are spacing, typography, colors, and border radii visually cohesive?
- [ ] **Behavioral Fluidity:** Does every interaction respond in under 100ms with smooth visual feedback?
- [ ] **Blameless Copy:** Are all error messages and validation texts polite, clear, and non-accusatory?
- [ ] **Zero Disruption:** Are there zero autoplaying media, modal popups without escape keys, or aggressive interruptions?
- [ ] **Data Safety:** Is user work cached or auto-saved so a tab reload or crash loses zero input?
- [ ] **Ethical Transparency:** Is the interface completely free of dark patterns, hidden checkboxes, or deceptive wording?
- [ ] **Celebration Milestones:** Are long workflows concluded with a subtle, uplifting completion animation or banner?
