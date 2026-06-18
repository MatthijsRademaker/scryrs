---
description: Validate one exact OpenSpec change. Non-interactive — suitable for swarm automation.
swarm: true
agent_types:
  - swarm-worker
  - swarm-lead-dev
  - swarm-room-controller
---

Validate one exact OpenSpec change.

$ARGUMENTS

**Input**: In swarm context, `DEV_SWARM_TASK_ID` is authoritative and the change name MUST be `task-<DEV_SWARM_TASK_ID>`. Outside swarm, use the first token of `$ARGUMENTS` as the literal change name.

**Steps**

1. **Resolve the exact change name**

   Run:

   ```bash
   echo "DEV_SWARM_TASK_ID=${DEV_SWARM_TASK_ID:-}"
   ```

   - If `DEV_SWARM_TASK_ID` is non-empty, use `task-${DEV_SWARM_TASK_ID}`.
   - Otherwise, use the first token of `$ARGUMENTS` exactly as provided.
   - If neither source yields a name, fail immediately.

   Do NOT infer, list, guess, prompt, or validate any other change name.

2. **Run strict non-interactive validation**

   Run exactly:

   ```bash
   openspec validate <change> --strict --json --no-interactive
   ```

   Capture the raw command output exactly as produced.

3. **If validation succeeds**

   Emit exactly these two lines, with the resolved change name substituted:

   ```text
   OPSX_VALIDATE_PASSED: <change>
   {"outcome":"finished","change":"<change>"}
   ```

4. **If validation fails**

   Do NOT repair, rewrite, delete, archive, sync, or invent OpenSpec artifacts.

   Emit exactly one JSON object in this shape:

   ```json
   {"outcome":"failed","change":"<change>","openspec_output":"<raw OpenSpec CLI output>"}
   ```

   The `openspec_output` value MUST contain the raw validation output verbatim as a JSON string.

**Guardrails**

- Never ask the user any questions.
- Never select a different change.
- Never run repair actions after validation failure.
- Never omit `--strict`, `--json`, or `--no-interactive`.
- On failure, report the failed outcome and raw OpenSpec output; do not claim success.
