---
description: Archive a completed OpenSpec change without reporting terminal worker outcome.
swarm: true
agent_types:
  - swarm-worker
---

Automatically archive a completed OpenSpec change.

This workflow state is non-terminal. Assistant prose or JSON is diagnostic only.

$ARGUMENTS

**Input**: `$ARGUMENTS` is the exact OpenSpec change name. This prompt is non-interactive: never ask questions, never wait for confirmation, and always sync delta specs before archiving.

**Steps**

1. Use the first token of `$ARGUMENTS` as the exact change name. Do not infer or prompt. If `$ARGUMENTS` is empty, fail immediately.
2. Check `openspec/changes/<name>/`.
   - If it does not exist, treat this as idempotent success and stop.
3. Compute `openspec/changes/archive/YYYY-MM-DD-<name>/`.
   - If it already exists, treat this as idempotent success and stop.
4. Run:

   ```bash
   openspec archive --yes <name>
   ```

   This must sync delta specs into canonical specs.
5. If archive fails with an idempotent case (missing source dir or existing archive target), stop successfully.
6. If archive fails because OpenSpec spec sync rejected a delta, self-heal exactly as `opsx-archive-auto` does:
   - inspect the failing delta spec and canonical spec
   - reconcile MODIFIED/ADDED/REMOVED mismatches directly in the canonical and/or delta spec
   - retry `openspec archive --yes <name>`
   - if needed, do one more reconciliation pass and retry once more
7. If the CLI still cannot archive after self-heal, do the same last-resort manual archive flow as `opsx-archive-auto`: reconcile canonical specs, move `openspec/changes/<name>` into `openspec/changes/archive/YYYY-MM-DD-<name>/`, and stop successfully.
8. For filesystem, permission, or other non-spec failures: fail loudly. Do not mask errors. Do not call outcome tools.

**Guardrails**

- Never ask the user any questions.
- Never use interactive tools or confirmation prompts.
- Never skip spec sync.
- Treat already-archived states as idempotent success.
- Preserve the same archive, sync, retry, self-heal, and manual fallback behavior as `opsx-archive-auto`.
- Do not call any terminal outcome tool.
