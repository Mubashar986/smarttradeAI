---
name: user-research-thematic-analysis
description: >-
  Extracts actionable software requirements, user pain points, and technical feature backlogs
  from raw qualitative data (user interview transcripts, customer feedback, support logs,
  and survey responses) using structured coding and thematic analysis.
artifact: thematic-backlog.md
version: 3.0.0
---

# User Research & Thematic Analysis for Software Agents

Before writing code for complex applications, software teams must understand actual user behavior and frustrations. Raw feedback—such as interview transcripts, GitHub issues, user feedback forms, or usability session recordings—contains qualitative noise that must be structured systematically.

> **Heisenberg OS Operational Contract:**  
> When invoked for user research or customer feedback synthesis, the agent outputs `.heisenberg/artifacts/<task-id>/thematic-backlog.md` converting raw qualitative research into validated software requirements and architectural backlogs.

---

## 1. Qualitative Data Gathering Methods Overview

When evaluating user research inputs, recognize the source method and its strengths:

| Method | Best For | Typical Data Output | Limitations / Bias |
| :--- | :--- | :--- | :--- |
| **Semi-Structured Interviews** | Exploring user mental models, deep pain points, and workflows. | Audio/video transcripts, verbatim quotes. | Small sample size, self-reporting bias. |
| **Surveys / Questionnaires** | Quantifying satisfaction, feature frequency, and demographic trends. | Likert scale ratings (1–5), short text responses. | Superficial reasoning, low response rates. |
| **Diary Studies** | Tracking user habits and context over days/weeks in situ. | Timestamped logs, photos, event notes. | High participant attrition over time. |
| **Observational Field Studies** | Observing actual user workarounds in natural environments. | Interaction timestamps, error frequency logs. | Hawthorne effect (users act differently when watched). |

---

## 2. Inductive vs. Deductive Qualitative Coding

When analyzing qualitative text, choose the appropriate coding strategy:
- **Inductive Coding (Bottom-Up):** The agent generates codes purely from what emerges in the raw text without preconceptions. Ideal for exploratory greenfield product discovery.
- **Deductive Coding (Top-Down):** The agent tests an existing theoretical framework (e.g., Nielsen’s 10 Heuristics or Norman’s Usability Principles) against the text, tagging quotes that match pre-existing categories.

---

## 3. The 6-Phase Thematic Analysis Workflow for Agents

When given interview transcripts or feedback logs, execute these 6 phases:

```
[Raw User Feedback Text] 
   └── 1. Familiarize & Ingest Corpus
   └── 2. Generate Atomic Codes (Assign meaningful tags to raw quotes)
   └── 3. Cluster into Candidate Themes
   └── 4. Review Themes against Entire Corpus for Cross-Validation
   └── 5. Formally Define & Name Themes
   └── 6. Synthesize Technical Epics & User Stories
```

### Phase 1: Familiarization
- Read the entire transcript corpus without filtering to grasp the user’s holistic context, goals, and tone.

### Phase 2: Generating Initial Codes
- Extract concise, descriptive labels ("codes") that capture single points of friction, delight, or user intent.
- Preserve verbatim user quotes as evidence for each code.

### Phase 3: Searching for Themes
- Cluster related codes into higher-level candidate themes that represent broader operational or cognitive patterns.

### Phase 4: Reviewing Themes
- Verify that every theme is supported by evidence across multiple participants (avoid single-user idiosyncrasies).
- Ensure themes do not overlap excessively.

### Phase 5: Defining and Naming Themes
- Write a 1-sentence conceptual definition for each theme summarizing what it means for the product.

### Phase 6: Producing the Requirements Backlog
- Convert each theme into functional requirements, technical epics, and acceptance criteria.

---

## 4. Standard Analysis Tables (Agent Output Format)

Agents performing thematic analysis should output these two structured tables:

### Table 1: Generated Codes & Evidence Matrix
| Interviewee / User | Raw User Statement (Verbatim Quote) | Generated Code | Category |
| :--- | :--- | :--- | :--- |
| Saira (Teacher) | *"The app said the bus would come in 5 min, but it arrived 20 min later."* | Inaccurate Real-Time Tracking | Reliability |
| Saira (Teacher) | *"I couldn't tell how full the bus was before it arrived."* | Lack of Crowd Information | Transparency |
| Imran (Office Worker)| *"Connecting metro to bus is tricky; I never know if the bus is still there."* | Fragmented Multi-Modal Routing | Navigation |
| Imran (Office Worker)| *"Notifications about delays come too late to adjust my travel."* | Delayed Alert Notifications | Timeliness |
| Nida (Student) | *"The interface is cluttered and hard to use while walking."* | Cluttered Mobile Interface | Usability |
| Nida (Student) | *"There is no feature focused on women's safety or SOS."* | Missing Safety Emergency Feature | Trust & Safety |
| Nida (Student) | *"Reviews from other passengers helped me choose safer routes."* | Community Feedback Value | Social Trust |

### Table 2: Grouped Themes & Architectural Implications
| Overarching Theme | Supporting Codes | Conceptual Interpretation | Direct Technical / Architectural Action |
| :--- | :--- | :--- | :--- |
| **1. Real-Time Reliability & Predictive Precision** | Inaccurate tracking, delayed alerts, app freezes | Commuters experience severe stress from stale data and missing live telemetry. | Integrate WebSocket real-time bus GPS telemetry; calculate server-side traffic congestion deltas. |
| **2. Multi-Modal Navigation Integration** | Fragmented routes, fare estimates, bus-metro sync | Users want unified transit planning across disparate transport authorities. | Implement Dijkstra/A* multi-modal routing engine combining metro, bus, and rideshare APIs. |
| **3. High-Mobility Usability & Accessibility** | Cluttered UI, difficult while walking, small buttons | Interactions occur on the go with one hand, distractions, and bright lighting. | Redesign mobile layout: bottom-sheet navigation, $\ge 48\text{px}$ touch targets, single-tap route favorites. |
| **4. Trust, Safety & Social Reassurance** | Missing safety feature, crowd level visibility, reviews | Personal safety and comfort dictate transit adoption, especially for women. | Build hardware-trigger SOS button, background live location sharing, and crowdsourced crowd ratings. |

---

## 5. Converting Themes into Agile User Stories

Once themes are identified, formulate production-ready User Stories:

```markdown
### Feature Epic: Multi-Modal Route Integration (Derived from Theme 2)

- **User Story:**
  - *As a* daily multimodal commuter (Imran),
  - *I want* the app to generate coordinated routes that synchronize metro arrivals with connecting local bus departures,
  - *So that* I do not get stranded at intermediate stations.

- **Acceptance Criteria (Gherkin format):**
  - **Given** the user selects an origin and destination requiring both metro and bus,
  - **When** the route suggestions are displayed,
  - **Then** the interface must show combined estimated fares, total travel duration, and connection wait times.
  - **And** if the connecting bus is delayed by > 3 minutes, an alert must trigger automatically.
```

---

## 6. Automated Code Analysis Helper: Transcript Keyword & Code Parser

Agents can write simple scripts to assist with large text datasets:

```python
# Helper to analyze sentiment and cluster frequent pain-point n-grams
from collections import Counter
import re

def extract_friction_ngrams(transcripts, min_len=2):
    stop_words = {'the', 'and', 'to', 'a', 'of', 'in', 'i', 'it', 'was', 'my', 'that'}
    phrases = []
    for text in transcripts:
        words = [w.lower() for w in re.findall(r'\b[a-zA-Z]{3,}\b', text) if w.lower() not in stop_words]
        for i in range(len(words) - min_len + 1):
            phrases.append(' '.join(words[i:i+min_len]))
    return Counter(phrases).most_common(10)
```

---

## 7. Anti-Patterns in User Research Analysis

| Anti-Pattern | Description | Agent Correction |
| :--- | :--- | :--- |
| **Premature Solutioneering** | Jumping directly into writing code without coding transcripts. | Always produce the coding table first to prove feature necessity. |
| **Confirmation Bias** | Only recording quotes that validate pre-existing assumptions. | Actively look for dissenting opinions, edge cases, and user workarounds. |
| **Vague Theme Naming** | Using useless generic labels like *"App Issues"* or *"Good Stuff"*. | Use descriptive names: *"Unpredictable Synchronization Lags"* or *"One-Handed Navigation Ergonomics"*. |
| **Ignoring Context** | Treating an in-office desk response the same as an in-transit response. | Factor in PACT context (movement, stress, environmental distractions). |

---

## 8. Actionable Verification Checklist for Agents

Before completing a research synthesis:
- [ ] **Traceability:** Does every proposed feature trace directly back to at least two user quotes in Table 1?
- [ ] **Distinct Themes:** Are themes distinct and non-redundant (aim for 3 to 6 overarching themes)?
- [ ] **Actionability:** Does each theme lead to a concrete technical or UI deliverable in Table 2?
- [ ] **Edge Cases:** Have safety concerns, accessibility impediments, or performance bottlenecks been flagged?
- [ ] **User-Centric Language:** Are epics framed around user goals rather than purely system capabilities?
- [ ] **Acceptance Criteria:** Are user stories accompanied by executable Gherkin scenarios?
