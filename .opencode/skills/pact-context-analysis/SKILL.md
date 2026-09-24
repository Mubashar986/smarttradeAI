---
name: pact-context-analysis
description: >-
  Performs structured PACT (People, Activities, Contexts, Technologies) analysis
  to scope system requirements, analyze user environments, and guide architecture
  and UI design decisions before writing code.
artifact: pact-analysis.md
version: 3.0.0
---

# PACT Context & Requirements Analysis

The **PACT Framework** (People, Activities, Contexts, Technologies) is a foundational Human-Computer Interaction methodology for scoping and designing software. Systems do not exist in isolation: **people** undertake **activities** within specific **contexts** using interactive **technologies**. If any one of these four elements changes, the entire interaction changes.

> **Heisenberg OS Operational Contract:**  
> When invoked during greenfield scoping, architecture review, or mobile layout planning, the agent outputs `.heisenberg/artifacts/<task-id>/pact-analysis.md` examining all four PACT dimensions before choosing software architectures or interface patterns.

---

## 1. The Four PACT Dimensions

### Dimension 1: People (Who are the users?)
Analyze the physical, cognitive, and social diversity of the intended user base:
- **Physical Capabilities:** Vision (color blindness, visual acuity), hearing, motor dexterity (fine finger tapping vs. large click areas), speech, and age-related physical factors.
- **Cognitive Characteristics:** Memory capacity, spatial cognition, mental models, attention span, tech literacy, domain expertise (novice vs. power user).
- **Social & Cultural Differences:** Primary language, localization, cultural conventions (date formats, color connotations), accessibility requirements (screen readers, keyboard navigation).

### Dimension 2: Activities (What tasks will users perform?)
Analyze the nature and demands of the tasks the system supports:
- **Temporal Aspects:** Frequency (daily vs. occasional), duration, urgency, bursty vs. sustained interaction, peak vs. off-peak loads.
- **Cooperation & Collaboration:** Is this a single-user task or multi-user collaborative workflow (concurrent edits, permissions, handoffs)?
- **Task Complexity:** Well-defined procedural steps (e.g., checkout flow) vs. open-ended exploratory tasks (e.g., analytics dashboard).
- **Safety Criticality:** What are the consequences of error? (Trivial inconvenience vs. data loss vs. clinical/financial emergency).

### Dimension 3: Contexts (Where and under what conditions does interaction occur?)
Analyze the surroundings in which the software will actually run:
- **Physical Environment:** Lighting (sunlight glare vs. dark rooms), ambient noise, mobility (walking, transit commute, stationary desk), weather, hands-busy conditions.
- **Social Environment:** Privacy requirements (confidential medical/banking records vs. public displays), co-presence (nearby observers, shared workstations), interruptions.
- **Organizational Environment:** Corporate compliance, shift work handovers, internet firewall restrictions, IT support constraints.

### Dimension 4: Technologies (What hardware & software platforms are used?)
Analyze the technical stack and constraints of the user's setup:
- **Input Technologies:** Touchscreen, mouse/keyboard, voice commands, hardware barcodes/scanners, wearable biometric sensors.
- **Output Technologies:** Small mobile screen (360px), desktop monitor (4K), audio chimes, tactile/haptic vibrations, e-ink displays.
- **Connectivity & Hardware Constraints:** Unstable 3G/4G connectivity, offline-first requirements, low-end CPU, battery life preservation.

---

## 2. Comparative PACT Case Studies

To understand how PACT drives radically divergent architectural decisions, compare two systems:

| Dimension | Public Transit Commuter App (CityCommute) | Clinical Stroke Monitoring Station |
| :--- | :--- | :--- |
| **People** | Broad public, varied tech literacy, non-native speakers, rushing. | Experienced registered nurses, high cognitive load, clinical vocabulary. |
| **Activities** | Route lookup, live bus checking, quick ticketing, delay alerts. | Continuous physiological monitoring, acute risk triage, dosage logging. |
| **Context** | Outdoor streets, glaring sunlight, crowded buses, walking, high noise. | Busy hospital ward, nighttime low-light, sterile gloved hands, high stress. |
| **Technology** | Mid-range smartphones, erratic cellular LTE, battery conservation. | Wall-mounted 4K displays, bedside IoT sensors, dedicated local ethernet. |
| **Tech Choice** | **Offline-first PWA, IndexedDB caching, massive touch targets.** | **WebSocket low-latency feeds, high-contrast audio-visual alarms, 1-tap mute.** |

---

## 3. PACT to Technical Architecture Mapping

When an agent completes a PACT analysis, translate findings directly into technical decisions:

| PACT Finding | Architectural Decision | Code Implementation Strategy |
| :--- | :--- | :--- |
| Commuters on moving trains (low signal) | **Offline-First Storage** | Implement Service Workers, IndexedDB, and optimistic UI updates with background sync. |
| In-field workers with dirty hands/gloves | **Target Size & Voice Input** | Touch targets $\ge 48\text{px} \times 48\text{px}$, high-contrast UI, Web Speech API integration. |
| High-stress emergency alerts (nurses) | **Immediate Attention & Audio Cues** | Web Audio API priority chime, persistent notification banner, high-visibility red badge. |
| Complex multi-step regulatory forms | **State Persistence & Autosave** | LocalStorage / debounce saving, step-by-step wizard with progress indicator. |
| Mixed elderly and young audience | **Scalable Typography & Contrast** | Responsive `rem` units, WCAG AAA contrast ratio ($\ge 7:1$), explicit signifiers. |
| Outdoor high-glare environments | **Adaptive Theme & High Contrast** | Media queries for `prefers-contrast` and CSS variables for high brightness. |

---

## 4. Agent Step-by-Step PACT Analysis Procedure

When instructed to design, architect, or analyze a project, execute this 4-step workflow:

### Step 1: Draft the PACT Matrix
Populate the 4 dimensions using this markdown table format:

```markdown
### PACT Matrix: [System Name]

| Dimension | Key Characteristics | Implications for Design & Architecture |
| :--- | :--- | :--- |
| **People** | [User traits, cognitive levels, accessibility needs] | [Input formats, readability, error tolerance] |
| **Activities** | [Primary user jobs, frequency, criticality] | [Latency needs, caching, confirmation flows] |
| **Contexts** | [Physical surroundings, distractions, privacy] | [Dark mode, glanceable UI, session timeout] |
| **Technologies**| [Devices, network bandwidth, display sizes] | [Responsive breakpoints, bundle size, offline sync] |
```

### Step 2: Identify Critical Constraints & Environmental Media Queries
Incorporate CSS and hardware capabilities that adapt to the context:
```css
/* Accommodate PACT Context: High Contrast & Outdoor Glare */
@media (prefers-contrast: more) {
  :root {
    --text-primary: #000000;
    --border-action: 3px solid #000000;
  }
}

/* Accommodate PACT Context: Reduce motion for sensitive users */
@media (prefers-reduced-motion: reduce) {
  * {
    animation-duration: 0.01ms !important;
    transition-duration: 0.01ms !important;
  }
}
```

### Step 3: Implement Context-Aware Component Architecture
```tsx
// Example: Glanceable, High-Contrast Commuter Button for Walking Users
export function QuickActionButton({ label, icon, onAction }: { label: string; icon: string; onAction: () => void }) {
  return (
    <button
      onClick={onAction}
      className="min-h-[56px] min-w-[56px] w-full px-6 py-4 bg-blue-700 text-white font-bold text-lg rounded-xl shadow-lg active:scale-95 active:bg-blue-900 focus:outline-none focus:ring-4 focus:ring-blue-400 flex items-center justify-center gap-3 transition"
      aria-label={label}
    >
      <span className="text-2xl" aria-hidden="true">{icon}</span>
      <span>{label}</span>
    </button>
  );
}
```

---

## 5. Practical Reference Case: Clinic Stroke Monitoring System

```markdown
- **People:** 
  - Primary User: Ms. Eisha (Ward Nurse, 40 yrs, high clinical experience, heavy cognitive load).
  - Subject: Mr. Aslam (Patient, 75 yrs, hemiparesis, passive monitoring).
- **Activities:** 
  - Real-time patient distress detection (dysphagia, fall, seizure).
  - Time-critical intervention logging.
- **Context:** 
  - Busy nursing station, multi-tasking with paperwork, audible alarms, fluorescent lighting.
- **Technologies:** 
  - Central desktop dashboard, off-the-body room sensors, ceiling speaker alarms.
- **Design Decisions:** 
  - High-priority color-coded alert banner (Red = Critical, Yellow = Advisory).
  - 1-click alert acknowledgement to silence audio.
  - 15-second iconographic summary rather than dense medical jargon.
```

---

## 6. Actionable Verification Checklist for Agents

When reviewing any codebase or feature plan through PACT:
- [ ] **People:** Has accessibility (WCAG AA/AAA) and keyboard navigation been accommodated?
- [ ] **Activities:** Are frequent tasks reachable in 1–2 clicks, and dangerous actions guarded?
- [ ] **Contexts:** Does the interface adapt to high/low lighting (dark/light theme) and glanceable consumption?
- [ ] **Technologies:** Does the application perform efficiently on lower-spec hardware and throttled networks?
- [ ] **Coherence:** If any PACT dimension changes (e.g., migrating from desktop to mobile), have the interactions been re-evaluated?
- [ ] **Environmental Media Queries:** Are responsive breakpoints and accessibility media queries configured?
