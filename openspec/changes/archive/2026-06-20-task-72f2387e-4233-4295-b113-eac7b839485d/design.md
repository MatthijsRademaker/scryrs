## Context

scryrs Phase 1 requires reference hooks for Pi and Claude Code to demonstrate deterministic proxy capture. The Claude Code hook must intercept specific tool events and forward them into `scryrs record` using the canonical TraceEvent schema. The existing trace-hook-contract.md describes the full contract but marks the Claude hook as "forthcoming." The roadmap.md Current Starting Point lists reference hooks as absent.

**Constraint**: The task specifies a PreToolUse-only hook — the hook fires *before* the tool executes, not after. This creates a fundamental tension with the canonical TraceEvent schema, which requires every event to carry an explicit `outcome` (Success or Failure) that cannot be truthfully determined pre-execution.

**Constraint**: Claude Code's hook system uses JavaScript modules. The hook must run in the consumer's Claude Code environment where Node.js is available.

## Goals / Non-Goals

### Goals
- Ship reference source in `hooks/claude-code/` with no committed consumer-side `.claude/` configuration.
- Forward the nine listed Claude Code tool events (Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, WebFetch) into `scryrs record --stdin` using canonical TraceEvent shape.
- Preserve agent-visible tool behavior completely: no stdout/stderr/status rewriting, no proxying.
- Fail open when `scryrs` is missing, rejects input, or exits non-zero. Warnings go to a dedicated log file outside agent context.
- Update trace-hook-contract.md and roadmap.mdx to reflect the hook's existence.
- Provide automated verification via `scripts/hook-test`.

### Non-Goals
- Do not add consumer-specific Claude installation/config files under `.claude/` or elsewhere in this repo.
- Do not expose `scryrs` as a Claude-callable tool, MCP surface, or business-tool wrapper.
- Do not expand `scryrs record` into hotspot analysis, graph building, or new ingestion interfaces.
- Do not emit SessionStart or SessionEnd lifecycle events (PreToolUse cannot provide session lifecycle triggers).
- Do not change the canonical TraceEvent schema — the hook adapts to the contract, not vice versa.
- Do not broaden scope to Pi hook delivery, plugin-tier integrations, or rules-file fallback behavior.

## Decisions

### D1: Implementation language — JavaScript
**Decision**: The reference hook is authored as a JavaScript module (Claude Code's native hook format).
**Rationale**: Claude Code's hook system requires JavaScript modules. This is an external runtime constraint stronger than the repository's Rust+Shell bias. The hook remains thin transport code — JSON shaping and a `child_process.spawn` call to `scryrs record --stdin`.
**Alternatives considered**: POSIX shell script (JSON-fragile, error-prone quoting; rejected because Claude Code hooks expect JavaScript); Rust binary (requires consumer compilation; rejected as heavy for a reference transport hook).

### D2: PreToolUse outcome gap — emit Success unconditionally
**Decision**: All events carry `outcome: Success` unconditionally. Document this as a known v0 limitation: these are pre-execution metadata signals, not post-execution outcomes.
**Rationale**: The canonical TraceEvent schema requires `outcome` on every event. A PreToolUse-only hook cannot determine the real outcome. Fabricating speculative outcomes is worse than accepting the limitation transparently. The outcome gap is documented in the hook README and trace-hook-contract.md.
**Alternatives considered**: (a) Extend schema to allow nullable/pending outcome — rejected because it would change the shared contract that other hooks depend on. (b) Expand scope to include PostToolUse hook — rejected because the task explicitly specifies PreToolUse only.

### D3: No session lifecycle events
**Decision**: The hook generates a per-process UUID v4 session_id for all events but does not emit SessionStart or SessionEnd lifecycle events.
**Rationale**: Claude Code PreToolUse hooks provide no session-open or session-close trigger. Faking lifecycle boundaries would mislead downstream consumers. Every subject-bearing event still carries a valid session_id.

### D4: Fail-open warning channel — dedicated log file
**Decision**: Warnings are written to `.scryrs/hooks/claude-code-warnings.log` (relative to the consumer's project root) with ISO-8601 timestamps. They are NEVER written to stderr, which Claude Code captures as tool stderr.
**Rationale**: Writing to stderr would violate the transparency requirement (original tool stderr unchanged). A dedicated log file is out-of-band from the agent-visible tool context.
**Alternatives considered**: `logger`/syslog (not universally available); `/tmp/scryrs-claude-hook.log` (not scoped to the project, could collide across consumers).

### D5: Tool-to-event family mapping
**Decision**: The nine Claude Code tools map to canonical scryrs event families as follows:

| Claude Code tool | TraceEventType | Payload field |
|---|---|---|
| Read | FileOpened | `path`: file path from tool input |
| Bash | CommandExecuted | `command`: command string from tool input |
| Grep | SearchRun | `query`: search pattern from tool input |
| Glob | SearchRun | `query`: glob pattern from tool input |
| Edit | EditMade | `target`: file path from tool input |
| Write | EditMade | `target`: file path from tool input |
| NotebookEdit | EditMade | `target`: file path from tool input |
| WebSearch | SearchRun | `query`: search term from tool input |
| WebFetch | DocRetrieved | `doc_ref`: URL from tool input |

Each event carries `tool_name` set to the original Claude Code tool name (e.g., `"read"`, `"bash"`).

### D6: Ingestion mode — stdin pipe only
**Decision**: The hook pipes newline-delimited JSON to `scryrs record --stdin`. No file-mode or alternate path.
**Rationale**: This matches the hook-contract's primary mode. The subprocess-per-event overhead is acceptable since the hook runs as a non-blocking observer (Claude Code invokes hooks alongside, not instead of, tool execution).

### D7: Session ID generation
**Decision**: Session IDs are UUID v4, generated once per hook process lifetime. If Claude Code provides a session-scoped stable identifier (e.g., environment variable), the hook prefers that; otherwise it generates its own.
**Rationale**: Events within one Claude Code session should share the same session_id for downstream correlation. Per-process UUID is the minimum viable contract.

### D8: Test infrastructure
**Decision**: A new `scripts/hook-test` shell script exercises the hook independently from `cargo test`. It tests (a) JSON shaping correctness, (b) happy-path forwarding to `scryrs record --stdin`, and (c) fail-open behavior when scryrs is missing.
**Rationale**: The existing `scripts/test` runs only `cargo test --workspace` inside a Rust Docker image and cannot verify JavaScript hook code. A separate shell-based integration test path keeps testing simple and dependency-light.

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Claude Code hook API surface may change (tool names, metadata shape) | Medium | Hook is thin transport only — minimal breakage surface. Tool name whitelist is a single map. |
| PreToolUse events may not contain sufficient metadata (file paths, commands, queries) from Claude Code's hook API | Medium | The hook extracts best-effort metadata from available fields. If fields are absent, payload fields are empty strings rather than crashing. |
| Multi-line Bash command payloads break JSONL newline-delimited contract | Medium | The hook collapses newlines in command strings to spaces or escapes them before serialization. |
| Subprocess-per-tool overhead may compound latency on rapid tool sequences | Low | Claude Code invokes hooks as observers, not proxies. Additive latency is bounded by fork+exec time. |
| Shell-based integration tests may be brittle across environments | Low | `scripts/hook-test` uses Docker for isolation, same as `scripts/test`. |

## Traceability

- **Task**: 72f2387e-4233-4295-b113-eac7b839485d — Trace Foundation 04 — Build Claude Code reference trace hook
- **Dossier**: 2026-06-20T16:47:14.217Z — Exploration dossier identifying contract gap, placement, and risks
- **Decisions**: 1-swarm-architect-recommendation (outcome gap, session lifecycle, warning channel, shell vs JS, tool mapping); 1-swarm-lead-dev-recommendation (JS implementation, outcome as Success, tool mapping, test path, docs updates)
- **Artifact snapshot**: initial (task-72f2387e-4233-4295-b113-eac7b839485d, openspec/changes)
- **Consulted sources**: crates/scryrs-types/src/lib.rs, .devagent/docs/docs/trace-hook-contract.md, .devagent/docs/docs/cli-v0-contract.md, .devagent/docs/docs/roadmap.mdx, scripts/test, openspec/specs/trace-event-schema/spec.md, openspec/specs/scryrs-record-endpoint/spec.md, openspec/specs/trace-hook-contract/spec.md