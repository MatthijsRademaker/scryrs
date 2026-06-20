## 1. Hook Source Implementation

- [ ] 1.1 Create `hooks/claude-code/` directory with `scryrs-hook.js` (JavaScript module implementing PreToolUse hook)
- [ ] 1.2 Implement tool-name whitelist for the nine Claude Code tools: Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, WebFetch
- [ ] 1.3 Implement tool-to-event family mapping (Read→FileOpened, Bash→CommandExecuted, Grep/Glob/WebSearch→SearchRun, Edit/Write/NotebookEdit→EditMade, WebFetch→DocRetrieved)
- [ ] 1.4 Implement canonical TraceEvent JSON construction per crate `scryrs-types` schema, including `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome: Success`
- [ ] 1.5 Implement session ID generation: prefer Claude Code-provided session identifier if available; otherwise generate UUID v4 per process lifetime
- [ ] 1.6 Implement `scryrs record --stdin` subprocess invocation via `child_process.spawn`, piping newline-delimited JSON
- [ ] 1.7 Implement fail-open behavior: catch missing `scryrs` binary, process errors, and non-zero exits; write ISO-8601 timestamped warnings to `.scryrs/hooks/claude-code-warnings.log`
- [ ] 1.8 Implement multi-line command payload escaping for JSONL line integrity (collapse/replace embedded newlines in Bash commands)
- [ ] 1.9 Ensure hook stdout is valid JSON per Claude Code hook contract (return success to Claude regardless of scryrs outcome)
- [ ] 1.10 Verify no stdout/stderr rewriting — original tool output passes through unmodified

## 2. Hook README

- [ ] 2.1 Create `hooks/claude-code/README.md` with consumer-side installation instructions for Claude Code's hook system
- [ ] 2.2 Document PreToolUse limitations: outcome is always Success (pre-execution signal, not post-execution truth); no SessionStart/SessionEnd lifecycle events; session IDs are per-process UUID v4
- [ ] 2.3 Document fail-open behavior: warnings go to `.scryrs/hooks/claude-code-warnings.log`; scryrs failures never block tool execution
- [ ] 2.4 Document tool-to-event mapping table for integrator reference
- [ ] 2.5 Document prerequisites: `scryrs` must be on consumer's PATH; Claude Code must have hooks enabled
- [ ] 2.6 Explicitly note that consumer-side `.claude/` config is NOT stored in this repo

## 3. Integration Tests

- [ ] 3.1 Create `scripts/hook-test` shell script that exercises hook behavior without requiring Rust toolchain
- [ ] 3.2 Test JSON shaping: verify hook outputs valid canonical TraceEvent JSON for each of the nine tool types
- [ ] 3.3 Test happy-path forwarding: verify hook pipes events to `scryrs record --stdin` successfully
- [ ] 3.4 Test fail-open (missing binary): verify hook returns success when `scryrs` is not on PATH
- [ ] 3.5 Test fail-open (non-zero exit): verify hook returns success when `scryrs record` exits non-zero
- [ ] 3.6 Test transparency: verify hook does not alter simulated tool stdout, stderr, or exit status

## 4. Documentation Updates

- [ ] 4.1 Update `.devagent/docs/docs/trace-hook-contract.md` Reference Hooks section: remove "forthcoming Phase 1 deliverable" language for Claude Code hook; state that the reference hook now exists at `hooks/claude-code/`
- [ ] 4.2 Add PreToolUse limitation notes to trace-hook-contract.md: document that PreToolUse-only hooks emit `outcome: Success` unconditionally and cannot provide SessionStart/SessionEnd lifecycle events
- [ ] 4.3 Add fail-open warning channel documentation to trace-hook-contract.md: document `.scryrs/hooks/claude-code-warnings.log` as the dedicated warning log file
- [ ] 4.4 Update `.devagent/docs/docs/roadmap.mdx` Current Starting Point: remove claim that reference hooks remain "forthcoming" or absent; update to reflect that `hooks/claude-code/` exists
- [ ] 4.5 Verify consistency: ensure roadmap.mdx, trace-hook-contract.md, and README.md do not contradict each other about Claude hook existence