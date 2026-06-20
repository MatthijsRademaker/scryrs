## Why

scryrs Phase 1 (Deterministic Proxy Capture) requires reference hooks under `hooks/claude-code/` to complete the trace capture pipeline. Currently, the trace-hook-contract documentation and roadmap both state that the Claude Code reference hook is "forthcoming" and "does not exist in the repository yet." This task delivers the first checked-in Claude Code reference hook — a thin JavaScript transport that forwards Claude Code PreToolUse tool-event metadata into the existing `scryrs record` ingestion path without ever becoming a tool proxy.

## What Changes

- **New `hooks/claude-code/` directory** containing the reference hook source (`scryrs-hook.js`), a README with consumer-side installation instructions, and no committed `.claude/` configuration.
- **Hook implementation** as a Claude Code PreToolUse hook module in JavaScript (Claude Code's native hook format), intercepting Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, and WebFetch events and forwarding them as canonical TraceEvent JSONL to `scryrs record --stdin`.
- **PreToolUse contract limitation documented**: all events carry `outcome: Success` unconditionally since the PreToolUse hook fires before tool execution. No SessionStart/SessionEnd lifecycle events are emitted — the hook generates a per-process UUID v4 session_id for subject-bearing events only.
- **Fail-open behavior**: when `scryrs` is missing, rejects input, or exits non-zero, the hook writes a timestamped warning to a dedicated log file (`.scryrs/hooks/claude-code-warnings.log`) without altering the original tool's stdout, stderr, or exit status.
- **Documentation updates**: `.devagent/docs/docs/trace-hook-contract.md` updated to reflect that the Claude Code reference hook now exists (removing "forthcoming Phase 1 deliverable" language) and documents PreToolUse-specific limitations. `.devagent/docs/docs/roadmap.mdx` updated so the "Current Starting Point" section no longer claims reference hooks are absent.
- **New `scripts/hook-test`** integration test script that exercises the hook's JSON shaping, happy-path forwarding to `scryrs record --stdin`, and fail-open behavior without requiring the Rust toolchain.

## Impact

- **Affected specs**: New capability spec `claude-code-reference-hook`; cross-reference updates to `trace-hook-contract` docs.
- **Affected code**: New `hooks/claude-code/scryrs-hook.js` plus `hooks/claude-code/README.md`; new `scripts/hook-test`; no changes to any Rust crate.
- **Affected docs**: `.devagent/docs/docs/trace-hook-contract.md` (Reference Hooks section and PreToolUse limitation notes), `.devagent/docs/docs/roadmap.mdx` (Current Starting Point section).
- **No consumer-side config committed**: `.claude/` or similar consumer installation directories are never checked in.
- **No scryrs-visible surface change**: `scryrs record` behavior and TraceEvent schema are unchanged.