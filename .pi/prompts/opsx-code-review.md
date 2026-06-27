---
description: Review the current implementation diff against task and OpenSpec context.
swarm: true
agent_types:
  - swarm-worker
---

Review the current implementation before final worker completion.

This workflow state is non-terminal.

$ARGUMENTS

Use the provided task and OpenSpec context, inspect the repository directly, and perform a fresh code review.

## Required review actions

1. Inspect the current branch diff with `git diff` and `git diff --stat`.
2. Review the changed files against:
   - the current task request / task description
   - relevant OpenSpec context available in the repository
   - surrounding code contracts and tests
3. Look for correctness issues, spec mismatches, missing verification, broken routing, stale docs, or prompt/workflow contract drift.
4. Produce actionable findings with file paths and concrete defects.
5. If you find nothing to fix, say so explicitly with a clear **no findings** result.

## Output requirements

- Keep the review scoped to the current task.
- Separate confirmed findings from uncertainties.
- Prefer a short numbered findings list.
- If there are no findings, state `No findings.` explicitly.
- Do not implement fixes in this state.
- Do not call any terminal outcome tool.
