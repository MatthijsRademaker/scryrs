## 1. Publish rewrite-tool compatibility policy

- [x] Update `.devagent/docs/docs/trace-hook-contract.md` to state that `CommandExecuted.payload.command` records the command string observed by the hook at capture time and that scryrs never rewrites, normalizes, or canonicalizes commands in hooks.
- [x] Update `hooks/pi/README.md` to explain Pi's `tool_result` capture point, the observed-command semantics for rewritten Bash input, and the requirement to present any unverified propagation behavior as a limitation rather than a guarantee.
- [x] Update `hooks/claude-code/README.md` to explain PreToolUse capture, hook-order dependence beside rewrite tools, and that `CommandExecuted.payload.command` is not guaranteed to preserve original agent intent.

## 2. Add Claude Code rewrite-compatibility regression coverage

- [x] Extend `scripts/verification/claude-code-e2e.mjs` with a Bash fixture whose input command is already RTK-prefixed (for example `rtk ls -la`).
- [x] Add a compound-command Bash fixture covering the task pattern `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`.
- [x] Assert that the persisted `CommandExecuted.payload.command` matches the observed input string exactly, the hook stays silent on stdout/stderr, and non-Bash tool coverage remains unchanged.

## 3. Add Pi rewrite-compatibility regression coverage

- [x] Extend `scripts/verification/pi-hook-e2e.mjs` with simulated `tool_result` Bash inputs that already contain RTK-prefixed commands.
- [x] Add a compound-command fixture covering rewritten subcommands in a single Bash command string.
- [x] Assert that the hook persists the observed command string as-is, returns `undefined`, and leaves the original `ToolResultEvent` input unchanged.

## 4. Preserve Phase 1 scope boundaries

- [x] Simulate rewrite-tool behavior through direct fixture inputs only; do not install RTK or add any RTK dependency.
- [x] Do not modify `crates/scryrs-types`, `crates/scryrs-core`, hotspot scoring, or the `CommandExecutedPayload` schema in this change.
- [x] If empirical harness behavior cannot be proven during implementation, document the limitation explicitly instead of claiming deterministic original-versus-rewritten capture.
