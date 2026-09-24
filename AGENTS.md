# Heisenberg OS Agent Contract

This file is the portable entrypoint for agents working in a repository that
adopts Heisenberg OS. It is intentionally short. Detailed guidance remains in
`core/` and `skills/`; do not load every skill by default.

For a host without GrapeRoot, the portable policy in this file and
`.heisenberg/policy.json` takes precedence over older enhanced-mode wording in
the detailed runbooks. Those runbooks describe the stronger graph-enabled path;
they must not make an unavailable capability appear available or block ordinary
repository work by themselves.

## Bootstrap

1. Find the repository root and read `.heisenberg/policy.json`.
2. Run `python scripts/heisenberg_guard.py validate --workspace .` when the
   runtime permits it. Report a missing optional capability; do not pretend it
   is available.
3. Read the active task manifest at
   `.heisenberg/tasks/<task-id>.json`, if one is supplied or already active.
4. Load only the skills listed in `.heisenberg/skills.json` for that task type.
   Read each selected `SKILL.md` in full before creating its artifact. Record
   the selected skill IDs, source paths, versions, and evidence in the receipt.

## Task and artifact gates

- Do not change product code until an active task manifest exists and every
  artifact listed in `required_before_edit` is present.
- Store generated task artifacts under `.heisenberg/artifacts/<task-id>/`.
- Artifacts must name their task ID, selected skills, evidence, and files in
  scope. A required artifact is stale after a scoped code edit until its
  receipt is refreshed.
- `cs-domain-learning` is MANDATORY for all tasks. Every task must understand
  and document the underlying computer science first principles, data structures,
  invariants, and system semantics in `cs-concepts.md` before editing product code.
- For a task with `ui_surface`, read `.heisenberg/ui-workflow.json` before
  selecting a style or editing code. Use every required foundation skill—including
  HCI foundation skills (`pact-context-analysis`, `hci-usability-principles`,
  `task-modeling-goms-klm`, `heuristic-evaluation-audit`) and The Muses (Picasso,
  Escher, Vermeer)—and select no more than one compatible style skill. A style skill
  never replaces Picasso, Escher, Vermeer, HCI usability standards, data contracts,
  accessibility, or verification.

## Evidence and safety

- Prefer repository evidence and available tools. Mark unsupported or
  unverified capabilities as `UNKNOWN` rather than inventing proof.
- GrapeRoot is an optional capability unless the active task manifest marks it
  required. If available, use it for focused impact analysis; if unavailable,
  use the project's normal static-analysis and review path.
- Do not overwrite user changes, use destructive Git commands, alter global
  agent configuration, or install software without explicit user permission.
- Match verification to risk. Run configured checks when authorized and report
  exactly what was or was not verified.

## Completion

Before claiming a task is complete, run the policy validator, refresh the
receipt, and report changed files, selected skills, artifacts, verification,
and any remaining risks. Local rules and hooks help, but protected CI is the
cross-agent enforcement boundary.
