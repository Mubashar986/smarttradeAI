# Heisenberg OS for Claude Code

Read and follow [AGENTS.md](AGENTS.md) first. It is the shared policy for all
supported agent hosts.

Use the path-scoped rules in `.claude/rules/` when they apply. Select skills
from `.heisenberg/skills.json` according to the active task manifest; do not
load the whole skill library into every session. Record artifacts and evidence
under `.heisenberg/artifacts/<task-id>/` before declaring completion.
