---
description: Automatically archive a completed OpenSpec change. Non-interactive — suitable for swarm worker automation.
swarm: true
agent_types:
  - swarm-worker
---

Automatically archive a completed OpenSpec change.

$ARGUMENTS

**Input**: `$ARGUMENTS` is the exact OpenSpec change name (typically the task ID). This prompt is non-interactive: it never asks questions, never waits for confirmation, and always syncs delta specs before archiving.

**Steps**

1. **Extract change name**

   Use the first token of `$ARGUMENTS` as the exact change name. Do NOT prompt, infer, or ask for selection. If `$ARGUMENTS` is empty, fail immediately.

2. **Check if change directory exists**

   Verify `openspec/changes/<name>/` exists on disk.

   - If the directory does not exist: the change has likely already been archived. Treat this as successful idempotent completion — emit `{"outcome":"finished"}` and stop.
   - If the directory exists: continue.

3. **Check if archive target already exists**

   Compute the archive target: `openspec/changes/archive/YYYY-MM-DD-<name>/` using the current date.

   - If the archive target already exists: treat as idempotent success. The change was already archived under today's date. Emit `{"outcome":"finished"}` and stop.
   - If the archive target does not exist: continue.

4. **Archive using openspec CLI**

   Run the archive command:

   ```bash
   openspec archive --yes <name>
   ```

   This command:
   - Always syncs delta specs into canonical specs (default behavior).
   - Skips interactive confirmation prompts (`--yes`).
   - Moves the change directory to `openspec/changes/archive/YYYY-MM-DD-<name>/`.

   **If the command succeeds:** emit `{"outcome":"finished"}` and stop.

   **If the command fails:** proceed to step 5 (self-heal).

5. **Self-heal openspec failures**

   If the error is an idempotent case, report finished immediately:
   - Change directory missing (already archived) → emit `{"outcome":"finished"}`.
   - Archive target already exists → emit `{"outcome":"finished"}`.

   If the error is a **spec sync failure** (mentions MODIFIED, ADDED, REMOVED,
   requirement headers, spec validation, or "not found"), self-heal:

   **5a. Parse the error**

   Identify which spec capability failed (e.g. `hello-world-integration`),
   the operation that failed (MODIFIED/ADDED/REMOVED), and the header text
   that caused the mismatch.

   **5b. Read the conflicting specs**

   ```bash
   cat openspec/changes/<name>/specs/<capability>/spec.md
   cat openspec/specs/<capability>/spec.md
   ```

   **5c. Fix the mismatch**

   | Failure type | Self-heal action |
   |---|---|
   | MODIFIED failed (header not found in canonical) | Add the missing requirement header to the canonical spec at the correct position. Copy the MODIFIED entry's body from the delta spec as the requirement body. |
   | ADDED failed (requirement already exists in canonical) | Remove the conflicting ADDED entry from the delta spec. The requirement already exists — treat as idempotent. |
   | REMOVED failed (requirement not found in canonical) | Remove the REMOVED entry from the delta spec. The requirement is already absent — treat as idempotent. |
   | General merge/sync failure | Read both specs in full. Manually reconcile: apply the delta spec's intended changes to the canonical spec by writing a merged version. |

   When adding a missing header to a canonical spec:
   - Place it in the correct section (under `## Requirements`)
   - Use the exact header text from the error message or delta spec
   - Add a reasonable placeholder body if the delta spec doesn't provide one
   - Write the updated canonical spec atomically

   **5d. Retry archive**

   ```bash
   openspec archive --yes <name>
   ```

   If it succeeds → emit `{"outcome":"finished"}` and stop.

   **5e. Second retry (if first self-heal failed)**

   Re-read both specs and the fresh error. Fix any remaining mismatches.
   Retry one more time.

   If the second retry succeeds → emit `{"outcome":"finished"}`.

   **5f. Manual archive (last-resort fallback)**

   If both openspec CLI retries failed, archive manually while still syncing specs:

   1. Read each delta spec file at `openspec/changes/<name>/specs/<capability>/spec.md`
   2. For each, manually apply ADDED/MODIFIED/REMOVED entries to the canonical
      spec at `openspec/specs/<capability>/spec.md`. Write the reconciled spec.
   3. Move the change directory to archive:
      ```bash
      date=$(date +%Y-%m-%d)
      mkdir -p openspec/changes/archive
      mv openspec/changes/<name> openspec/changes/archive/${date}-<name>/
      ```
   4. Emit `{"outcome":"finished"}`.

   If even manual archive fails (filesystem error, permission error):
   report the error verbatim and stop without emitting a finished outcome.

   **5g. Non-spec failures (filesystem, permissions)**

   If the error is NOT about spec sync (e.g. disk full, permission denied,
   git lock conflict), report the error verbatim and stop without emitting
   a finished outcome. Do NOT attempt self-heal for non-spec failures.

**Output**

On successful or idempotent completion, emit exactly:

```json
{"outcome":"finished"}
```

On unexpected failure, report the error and stop without emitting the finished outcome.

**Guardrails**

- Never ask the user any questions. Never use interactive tools (AskUserQuestion, confirmation prompts, etc.).
- Never skip sync. Always sync delta specs — either via openspec CLI or manually.
- Treat already-archived and missing-source-directory as idempotent success.
- Do not check artifact or task completion status — proceed regardless.
- Do not offer sync choices or any interactive path.
- If the change directory does not exist when we first check it, the change is already archived — treat this as success.
- Self-heal openspec spec mismatches before giving up. Only fail on non-spec errors.
