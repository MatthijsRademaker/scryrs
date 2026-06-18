---
description: execute a saved implementation plan
agent: build
swarm: true
agent_types:
  - swarm-worker
---

$ARGUMENTS

You are executing a saved implementation plan. Read the full plan before making changes.

## Scope Discipline

- Treat the current task prompt, task ID, injected task context, and referenced saved plan as the complete execution scope.
- Do not pull in unrelated tasks, experiments, evaluations, backlog items, or adjacent cleanup unless the current task or plan explicitly requires them.
- Ground every edit, command, and verification step in the current task materials, saved plan, or repository files inspected specifically for this task.

## Before you start

- Read the plan in full first
- Inspect relevant code
- Resolve domain descriptions to concrete files

## Execution rules

- Execute phase by phase. Keep changes minimal. Follow existing conventions.
- Do not expand scope beyond the plan. Do not invent uninspected files, APIs, or patterns.

## Verification

- Run relevant tests, lint, typecheck, or build commands. Fix failures caused by your implementation. Do not claim verification passed unless commands actually ran successfully.

## Outcome & Output

After all edits, call `report_work_outcome` once with `finished`, `needs_work`, `failed`, or `skipped`.
