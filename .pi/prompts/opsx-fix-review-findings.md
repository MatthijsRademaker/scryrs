---
description: Continue the review session, fix findings, verify the result, and report the final worker outcome.
swarm: true
agent_types:
  - swarm-worker
---

You are continuing the immediately preceding review session.

This workflow state is the terminal worker outcome state. It must call `report_work_outcome` exactly once.

$ARGUMENTS

## Execution rules

1. Continue from the review session context produced by `opsx-code-review`.
2. If the review produced findings, fix them directly in the repository.
3. If the review produced an explicit **no findings** result, do not invent work. No-op except for required verification.
4. Keep changes scoped to the review findings and current task.
5. For behavioral changes, follow TDD: add or update a failing test first, make it pass, and confirm the test fails when the behavior is deliberately broken.

## Verification

Run the repository verification required for the changed code. Use `run_development_verification` and any additional task-specific checks needed to prove the fix is correct. Treat verification as required even when the review had no findings.

## Outcome

- If the work and verification succeed, call `report_work_outcome` with `finished` exactly once.
- If you find unresolved issues or verification failures you cannot fix in scope, call `report_work_outcome` with `needs_work` exactly once.
- If execution is blocked by an unrecoverable error, call `report_work_outcome` with `failed` exactly once.
- Do not emit assistant JSON as a substitute for the outcome tool.
