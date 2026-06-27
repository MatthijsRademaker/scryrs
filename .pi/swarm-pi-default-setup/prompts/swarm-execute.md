---
description: execute NeedsWork rework from current session context
agent: build
swarm: true
agent_types:
  - swarm-worker
---

You are executing review-comment rework from the current session context.

Use the current session context, gathered review feedback, and inspected repository state to implement the required changes directly.

## Scope Discipline

- Treat the current task prompt, task ID, injected task context, gathered review feedback, and current session context as the complete scope.
- Do not pull in unrelated tasks, experiments, evaluations, backlog items, or adjacent cleanup unless the current task explicitly requires them.
- Ground every edit, command, and verification step in the current task materials or repository files inspected specifically for this task.

## Before you start

- Inspect the relevant code
- Verify gathered feedback against the repository state
- Identify the concrete files and tests needed for the rework

## Execution rules

- Implement directly from the current session context; do not author or save a separate implementation plan.
- Keep changes minimal. Follow existing conventions.
- Do not expand scope beyond the gathered feedback and task materials.

## Verification

- Run relevant tests, lint, typecheck, or build commands. Fix failures caused by your implementation. Do not claim verification passed unless commands actually ran successfully.

## Outcome & Output

For behavioral changes: write a failing test first, confirm it fails for the right reason, implement the fix, then deliberately break the implementation and confirm the test catches it.

After all edits and verification, call `report_work_outcome` exactly once with `finished`, `needs_work`, `failed`, or `skipped`. This outcome tool writes the terminal artifact consumed by the swarm runtime. Do not emit assistant JSON as a substitute for the tool call.
