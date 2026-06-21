## Context

- The Pi reference hook records Bash commands from `event.input.command` on `tool_result`, so it captures whatever command string is present at that post-execution boundary.
- The Claude Code reference hook records Bash commands from `tool_input.command` on PreToolUse, so co-installed rewrite hooks can change what scryrs sees depending on hook order and platform forwarding behavior.
- `CommandExecutedPayload` currently contains only `command: String`, and typed event persistence means hook-only extra fields are not a reliable way to preserve both original and effective commands.
- `score_events` groups hotspots by exact `event.subject()` values, so `ls -la` and `rtk ls -la` remain separate subjects today.
- Existing cross-harness verification covers ordinary Bash commands but not RTK-prefixed or compound rewritten commands.

## Goals

- Define Phase 1 semantics for rewritten Bash commands without violating scryrs' non-interference boundary.
- Verify that scryrs accepts RTK-prefixed and compound rewritten Bash commands in both Pi and Claude Code verification flows.
- Document co-installation expectations and limitations for rewrite hooks.

## Non-Goals

- Do not invoke `rtk rewrite` from scryrs or add any RTK dependency.
- Do not change the `CommandExecutedPayload` schema to preserve both original and effective commands.
- Do not add command canonicalization, hotspot regrouping, or shell parsing in this increment.
- Do not alter tool stdout, stderr, exit status, or execution semantics.
- Do not register scryrs as a callable tool, MCP surface, or shell proxy.

## Decisions

1. **Record what the hook observes.**
   - `CommandExecuted.payload.command` is the command string visible at the hook's capture point.
   - Hooks do not normalize, canonicalize, or reconstruct original agent intent.
2. **Keep implementation scope to documentation and verification.**
   - Extend the existing Pi and Claude Code verification fixtures with direct RTK-style command inputs.
   - Do not modify Rust crates, event storage, or hook transport behavior for this change.
3. **Document harness-specific semantics conservatively.**
   - Pi documentation must explain that capture happens from `tool_result` and records whatever command string is present there.
   - Claude Code documentation must explain that PreToolUse hook order determines whether scryrs sees original or rewritten input.
   - Unverified harness behaviors must be documented as limitations rather than guarantees.
4. **Defer Phase 2 concerns.**
   - Command canonicalization for hotspot grouping and any dual-field original/effective schema work remain out of scope.

## Conflict Resolution

- The room aligned on keeping the current single-string schema for Phase 1 and resolving the original-versus-effective command ambiguity by recording the observed command string only.
- Open questions about Pi mutation propagation and Claude Code updated-input forwarding are handled as documentation caveats unless empirical verification is added during implementation.

## Risks

- Claude Code may not forward `updatedInput` between PreToolUse hooks, or may change hook-order behavior; documentation must avoid treating that as a stable guarantee unless verified.
- Pi propagation of `tool_call` input mutations into `tool_result` remains an empirical assumption; documentation must avoid overclaiming until it is verified.
- Hotspot subjects remain fragmented between rewritten and non-rewritten commands until Phase 2 canonicalization work exists.

## Traceability

- `task:c48b2eee-ae45-44dd-8c76-d0104b02306a`
- `dossier:2026-06-21T06:21:40.868Z`
- `decision:1-swarm-architect-recommendation`
- `decision:1-swarm-lead-dev-recommendation`
- `decision:1-swarm-reviewer-recommendation`
- `round:1:agent:swarm-architect`
- `round:1:agent:swarm-lead-dev`
- `round:1:agent:swarm-reviewer`