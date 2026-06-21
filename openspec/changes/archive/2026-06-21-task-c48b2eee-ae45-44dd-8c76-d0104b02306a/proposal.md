## Why

scryrs currently records a single `CommandExecuted.payload.command` string, but rewrite tools like RTK can mutate Bash commands before execution. Without a defined compatibility policy, traces become ambiguous across the two supported harnesses because Pi observes post-execution `tool_result` inputs while Claude Code captures PreToolUse inputs in a hook-order-dependent pipeline.

## What Changes

- Lock Phase 1 compatibility to a **record-what-the-hook-observes** policy: `CommandExecuted.payload.command` stores the command string visible at the hook boundary, with no normalization, reverse-rewrite logic, or schema expansion.
- Extend the Pi, Claude Code, and trace-hook contract documentation to explain rewrite-tool co-installation semantics, including Pi's `tool_result` capture point and Claude Code's hook-order caveat.
- Add cross-harness regression coverage using simulated RTK-style inputs only: a simple `rtk ls -la` command plus a compound command such as `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`.
- Keep scope limited to documentation and verification. Do not add an RTK dependency, do not modify hook execution semantics, and do not change the TraceEvent schema or hotspot scoring in this increment.

## Impact

- Users get an explicit rewrite-tool compatibility policy without weakening scryrs' non-interference boundary.
- Verification gains regression coverage for rewritten Bash commands on both supported harnesses.
- Rewritten and unre-written commands remain distinct hotspot subjects for now; canonicalization is explicitly deferred to Phase 2.