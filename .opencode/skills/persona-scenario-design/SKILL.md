---
name: persona-scenario-design
description: >-
  Designs behavioral user personas, designates primary personas, and authors context scenarios
  and user journey walkthroughs to guide application architecture, UI layout, and end-to-end
  test cases.
artifact: personas-and-scenarios.md
version: 3.0.0
---

# Persona Modeling & Scenario-Based Design

When engineering software, teams often build for the "elastic user"—an imaginary person whose skills and desires shift conveniently to justify whatever technical feature a developer wants to write. 

> **Heisenberg OS Operational Contract:**  
> When invoked for user modeling, journey mapping, or E2E specification, the agent outputs `.heisenberg/artifacts/<task-id>/personas-and-scenarios.md` defining primary personas, behavioral spectra, and context scenarios before coding.

---

## 1. Persona Taxonomy

Never treat all users identically. Classify personas into three explicit tiers:

| Persona Type | Definition | System Impact |
| :--- | :--- | :--- |
| **Primary Persona** | The user whose goals are most critical to the system's core function. If the software fails this user, it fails completely. | Directly dictates UI layout, primary navigation, default views, and performance SLAs. (Design for exactly ONE primary persona per interface). |
| **Secondary Persona** | Users whose needs are largely satisfied by the primary persona’s interface, but require a few supplementary tools. | Accommodated via advanced settings, secondary tabs, or role-based permission toggles. |
| **Anti-Persona** | A user archetype the system is explicitly **not** designed to support (e.g., fraudulent actors, unsupported legacy hardware users). | Guides defensive security, error boundaries, and explicit architectural exclusions. |

---

## 2. Clustering via Behavioral Variables (Not Demographics)

Demographic traits (age, gender, location) rarely determine software usage. Always cluster users along **Behavioral Variables**:

### Common Behavioral Variable Spectrums
1. **Information Frequency & Granularity:** High-level summary dashboards $\longleftrightarrow$ Real-time granular event streams.
2. **Primary Operational Focus:** Active intervention (doer) $\longleftrightarrow$ Strategic oversight (manager) $\longleftrightarrow$ Passive consumer (recipient).
3. **Technical Aptitude:** Scripting/CLI power-user $\longleftrightarrow$ Guided WYSIWYG beginner.
4. **Interaction Cadence:** Daily continuous multi-hour usage $\longleftrightarrow$ Once-a-month occasional reporting.
5. **Error Tolerance:** Mission-critical zero-fault $\longleftrightarrow$ Exploratory, forgiving experimentation.

---

## 3. Step-by-Step Persona Specification Template

When creating a persona, output this structured Markdown document:

```markdown
### Persona Profile: [Name], [Age] — [Professional Role]

- **Role & Background:** [Brief professional summary and responsibilities]
- **Persona Status:** Primary | Secondary | Anti-Persona
- **Behavioral Variables:**
  - *Frequency of Use:* [Daily / Weekly / Occasional]
  - *Technical Proficiency:* [Novice / Intermediate / Power User]
  - *Primary Focus:* [Intervention / Oversight / Execution]

#### 🎯 User Goals
- **End Goals (What they want to accomplish):**
  - [e.g., Immediately acknowledge critical alerts to prevent patient deterioration]
  - [e.g., Log bedside actions in under 10 seconds without navigation lag]
- **Experience Goals (How they want to feel):**
  - [e.g., Confident that no high-risk emergency is buried under false alarms]
  - [e.g., Free from cognitive fatigue during high-stress 12-hour shifts]

#### ⚠️ Pain Points & Frustrations
- [e.g., Auditory alert fatigue from nuisance alarms]
- [e.g., Complex multi-step menus while wearing surgical gloves]
```

---

## 4. Multiple Persona Archetypes: Clinical Stroke Monitoring System

To understand how multiple personas interact within a single domain, examine this triage matrix:

```markdown
### Primary Persona: Ms. Eisha (40 yrs, Registered Ward Nurse)
- **Role:** Immediate and acute patient intervention.
- **Focus:** Operational intervention (receives distress alerts, triages bedside emergencies).
- **Core Requirement:** Sub-second alert latency, high-visibility visual banners, 1-tap alarm silence.

### Secondary Persona: Mr. Ijaz (50 yrs, Hospital Administrator)
- **Role:** Facility operations, regulatory staffing compliance, resource utilization.
- **Focus:** High-level oversight (weekly bed occupancy, average nurse response latency).
- **Core Requirement:** Aggregated historical reporting, PDF export, low-frequency access.

### Tertiary / Subject: Mr. Aslam (75 yrs, Recovering Stroke Patient)
- **Role:** Passive patient monitored by off-the-body sensors.
- **Focus:** Recovery and comfort (does not operate software UI).
- **Core Requirement:** Silent non-intrusive room sensing, zero complex manual inputs.
```

---

## 5. Scenario-Based Design: Context Scenarios

A **Context Scenario** is a day-in-the-life narrative illustrating how a persona applies the proposed software to achieve a goal in their real-world environment.

### Narrative Architecture of a Context Scenario
1. **Setting:** Physical location, ambient conditions, device, and cognitive state.
2. **Trigger:** The motivating event that initiates the interaction.
3. **Action Flow:** The sequence of steps the user takes with the system.
4. **System Response:** How the software reacts (feedback, state change, guidance).
5. **Outcome:** The successful resolution of the goal and emotional relief.

### Real-World Example: Acute Dysphagia Intervention
```markdown
### Context Scenario: Acute Dysphagia Intervention
- **Persona:** Ms. Eisha (Primary Persona, Staff Nurse, 40 yrs).
- **Setting:** Central nursing station, busy afternoon shift, completing paper charts.
- **Trigger:** Room audio sensors and biometric cameras detect abnormal swallowing struggle in Room 312.
- **Action Flow & System Response:**
  1. The central dashboard highlights Room 312 in bold red with an urgent chime: *"Critical Dysphagia Risk Detected"*.
  2. The screen displays a 15-second iconographic summary showing persistent throat-clearing and zero liquid intake.
  3. Ms. Eisha taps the notification card; the system marks the alert as "Acknowledged by Eisha" and automatically pins the patient chart.
  4. Ms. Eisha grabs a portable suction tool, rushes to Room 312, provides bedside airway clearance, and taps "Set Status: Nil by Mouth" on the tablet.
- **Outcome:** Aspiration pneumonia is averted within 90 seconds. Patient chart is updated with zero manual paperwork delay.
```

---

## 6. Converting Scenarios to End-to-End (E2E) Test Specs

Agents should translate narrative scenarios directly into executable test suites (Playwright/Cypress/Gherkin):

```gherkin
Feature: Acute Dysphagia Alert & Clinical Intervention

  Scenario: Staff Nurse acknowledges critical dysphagia alert
    Given Ms. Eisha is logged into the Nurse Dashboard at "/dashboard"
    When the system receives a "DYSPHAGIA_RISK_HIGH" event for patient "Room-312"
    Then an urgent visual alert card should appear within 500ms
    And an auditory chime with priority "HIGH" should play
    When Ms. Eisha clicks the "Acknowledge & View Chart" button on the alert card
    Then the alert status should transition to "ACKNOWLEDGED"
    And the patient record should display an open clinical intervention log
    And the auditory chime should cease immediately
```

---

## 7. Common Persona & Scenario Pitfalls

| Pitfall | Why It Fails | Agent Correction |
| :--- | :--- | :--- |
| **The "Everyone" Persona** | Trying to please every user type in a single UI screen leads to bloated menus. | Restrict each interface layout to exactly **one Primary Persona**. |
| **Demographic Stereotyping** | Assuming age or gender dictates tech usage without behavioral evidence. | Cluster users based strictly on task goals and behavioral variables. |
| **Specifying Implementation in Scenarios** | Saying *"User clicks button `#submit-btn` at coordinates (200, 300)"*. | Keep context scenarios focused on user intent; save CSS selectors for test scripts. |
| **Ignoring the Physical Setting** | Writing scenarios assuming quiet desks when users are walking outdoors. | Always include the environmental setting (glare, noise, interruptions). |

---

## 8. Actionable Verification Checklist for Agents

When creating or referencing personas and scenarios:
- [ ] **One Primary Persona:** Is there an unambiguous Primary Persona designated for this product workflow?
- [ ] **Justified Selection:** Is the primary persona justified by the system's core value proposition?
- [ ] **Behavioral Clustering:** Are users grouped by operational behaviors rather than generic demographics?
- [ ] **Goal-Driven:** Are user goals framed as end-state accomplishments rather than UI tasks?
- [ ] **Realistic Context:** Do scenarios account for environmental constraints, distractions, and stress?
- [ ] **Code Traceability:** Can each major UI component and E2E test be traced directly back to a scenario step?
