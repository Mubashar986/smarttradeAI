---
name: cognitive-walkthrough-evaluator
description: >-
  Evaluates the learnability of user onboarding, sign-up flows, and multi-step tasks for first-time
  users by systematically executing Wharton & Lewis's Cognitive Walkthrough method answering
  the 4 core psychological questions at each action step.
artifact: cognitive-walkthrough.md
version: 3.0.0
---

# Cognitive Walkthrough Learnability Evaluator

First-time users do not read instruction manuals—they explore interfaces by trying actions that seem promising, observing the system's reaction, and forming a mental model on the fly. 

The **Cognitive Walkthrough (CW)** method (Wharton, Rieman, Lewis, & Polson) is an HCI inspection technique specifically engineered to evaluate **Learnability**: Can a novice user successfully complete a task without prior training?

> **Heisenberg OS Operational Contract:**  
> When invoked for onboarding, multi-step wizards, or UI audits, the agent outputs `.heisenberg/artifacts/<task-id>/cognitive-walkthrough.md` answering the 4 Core Psychological Questions at every step before completing the task.

---

## 1. The Four Core Cognitive Walkthrough Questions

At each discrete step of an action sequence, the agent must interrogate the interface with these four questions:

```
[User Formulates Goal]
       │
       ▼
[Q1: Intent] ────────► Will the user try to achieve the right effect?
       │
       ▼
[Q2: Visibility] ────► Will the user notice that the correct action is available?
       │
       ▼
[Q3: Association] ───► Will the user associate the correct action with their goal?
       │
       ▼
[Q4: Feedback] ──────► Will the user see that progress is being made toward the task?
```

### Detailed Question Criteria:
- **Question 1 (Intent):** *Will the user try to achieve the right effect?*
  - Does the user’s mental goal at this moment correspond to the action the system expects? 
  - *Failure mode:* System assumes the user knows they must "Generate an API Key" before clicking "Connect Database".
- **Question 2 (Visibility):** *Will the user notice that the correct action is available?*
  - Is the button, input, or control immediately visible on screen, or is it hidden inside a collapsed hamburger menu or below the fold?
  - *Failure mode:* Primary action is hidden inside an obscure nested sub-menu.
- **Question 3 (Association / Signifier):** *Will the user associate the correct action with the effect they are trying to achieve?*
  - Do the label, iconography, and text match the user's natural language and mental model?
  - *Failure mode:* Button says *"Provision Instance"* when the user is looking for *"Create New Server"*.
- **Question 4 (Feedback):** *If the correct action is performed, will the user see that progress is being made toward solution of the task?*
  - Does the interface provide clear, unambiguous feedback confirming the action worked and signaling what to do next?
  - *Failure mode:* The screen remains static after clicking, leaving the user wondering if the click registered.

---

## 2. Prerequisites for Conducting a Cognitive Walkthrough

Before running a walkthrough, the agent must define four inputs:

1. **User Profile (Persona):** Document the user's prior knowledge, mental models, and technical background.
2. **Concrete Task:** Specify the exact goal (e.g., *"Upload an avatar image and crop it to fit profile"*).
3. **Interface Representation:** Mockups, HTML/JSX templates, or live staging URLs.
4. **Optimal Action Sequence:** The exact step-by-step path an experienced user would take.

---

## 3. Cognitive Walkthrough Matrix Template

Document the walkthrough using this structured Markdown matrix:

```markdown
### Cognitive Walkthrough: [Task Name]
- **Target User:** First-time developer onboarding (Junior Engineer).
- **Goal:** Connect a remote GitHub repository to the project.

| Step # | Intended Action | Q1: Intent? | Q2: Visible? | Q3: Associated? | Q4: Feedback? | Failure Story & Remediation |
| :---: | :--- | :---: | :---: | :---: | :---: | :--- |
| **1** | Click "Add Integration" | YES | YES | YES | YES | Button is prominent blue on welcome dashboard. |
| **2** | Select "OAuth Token" | **NO** | YES | **NO** | -- | **Failure Story:** User expects to click "Log in with GitHub", but system asks for a raw "Personal Access Token". <br>**Fix:** Replace PAT field with 1-click "Connect with GitHub OAuth" button. |
| **3** | Click "Grant Access" | YES | YES | YES | **NO** | **Failure Story:** Screen stays blank for 4 seconds during OAuth handshake with zero loading banner.<br>**Fix:** Add full-screen spinner: *"Authenticating with GitHub..."*. |
```

---

## 4. Practical Example: Academic Research Paper Download Flow

### Context
Alex, a first-time university student, needs to download an academic paper for a research assignment.

#### Action Sequence Walkthrough
- **Step 1: Locate search bar and enter keywords.**
  - *Q1 (Intent):* **YES** (Alex knows he needs to search).
  - *Q2 (Visibility):* **YES** (Large search input centered on screen).
  - *Q3 (Association):* **YES** (Placeholder text reads *"Search by keyword, author, or title"*).
  - *Q4 (Feedback):* **YES** (Results list renders within 300ms).
- **Step 2: Filter results by publication year (2015–2024).**
  - *Q1 (Intent):* **YES** (Wants recent articles).
  - *Q2 (Visibility):* **YES** (Left sidebar labeled *"Filter by Date"*).
  - *Q3 (Association):* **YES** (*"Start Year"* and *"End Year"* input boxes).
  - *Q4 (Feedback):* **YES** (List re-sorts and active filter chips appear).
- **Step 3: Download the selected article PDF.**
  - *Q1 (Intent):* **YES** (Wants to save paper locally).
  - *Q2 (Visibility):* **YES** (Button labeled *"Download PDF"*).
  - *Q3 (Association):* **YES** (Iconography and text are unambiguous).
  - *Q4 (Feedback):* ❌ **CRITICAL FAILURE (NO)**.
    - *What happened:* Button showed a small spinner for 2s, then returned to normal blue. Nothing else changed.
    - *Why it failed:* Alex did not see that the file had downloaded to his background folder. Confused, he clicked the button two more times, generating duplicate downloads.
    - *Agent Remediation:*
      ```tsx
      // Code remediation for Step 3 Feedback failure
      const handleDownload = async () => {
        setIsDownloading(true);
        await triggerBrowserDownload(pdfUrl);
        setIsDownloading(false);
        // Explicit feedback answering Q4 with a clear confirmation banner
        toast.success(
          "Paper successfully downloaded to your Downloads folder!", 
          { duration: 4000, icon: '📄' }
        );
      };
      ```

---

## 5. Streamlined Cognitive Walkthrough (CW-Lite) for Agile Teams

In fast-paced sprints, perform the streamlined CW-Lite variant focusing on two critical questions per action:
1. *Will the user know what to do at this step?* (Merges Q1, Q2, Q3)
2. *If the user does the right thing, will they know they did it and that they are making progress?* (Q4)

---

## 6. Actionable Verification Checklist for Agents

When using this skill to review or build first-time user flows:
- [ ] **Action Sequence Defined:** Is every discrete click, keystroke, and decision documented?
- [ ] **All 4 Questions Answered:** Are Q1, Q2, Q3, and Q4 evaluated at each individual step?
- [ ] **Failure Stories Documented:** For every "NO" answer, is there an explicit narrative explaining *why* the user failed?
- [ ] **Code Fix Applied:** Has the UI been refactored (copy simplified, buttons made prominent, feedback added) to turn every "NO" into a "YES"?
- [ ] **Zero Jargon:** Are technical system internals replaced with terminology the novice persona natively understands?
- [ ] **Next Step Affordance:** Does the post-action feedback tell the user what step to take next?
- [ ] **Progress Indicators:** In multi-step wizards, is the active step clearly numbered (e.g., "Step 2 of 4")?
