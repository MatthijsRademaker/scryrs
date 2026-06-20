## Context

Trace Foundation 02 delivered `scryrs record --stdin|--file` as the deterministic JSONL ingestion endpoint that validates, persists, and reports on `TraceEvent` input. Trace Foundation 05 documented the canonical hook contract covering non-interference rules, fail-open guarantees, event family mappings, session demarcation, and the `scryrs.json` manifest boundary. This task (Trace Foundation 03) bridges those two foundations by delivering the actual Pi-side transport layer: a TypeScript extension that converts Pi tool events into `TraceEvent` JSONL and delegates to `scryrs record --stdin`.

The hook must be thin, transport-only, and reference-grade — consumers copy it into their own Pi extension location rather than having this repo wire it in automatically.

## Goals / Non-Goals

### Goals

1. **Transport-only hook**: The hook filters tool events, extracts minimal metadata, shapes JSON, and shells out. No business logic, analysis, or agent-facing surface.
2. **Fail-open by construction**: scryrs failures never block or modify the agent-visible tool result.
3. **Correct tool-to-event mapping**: Each of the six named Pi tools maps to the canonical TraceEvent family with the correct payload fields.
4. **Session demarcation (SessionStart only)**: A unique `session_id` is generated on extension load and a `SessionStart` TraceEvent is emitted. SessionEnd is deferred to a follow-up task.
5. **No tool registry contamination**: scryrs is never registered as a Pi tool via `pi.registerTool()` or any equivalent API.
6. **Reference-only**: Source lives in `hooks/pi/` with companion install/mapping docs. No consumer-side `.pi/extensions/` config is committed.

### Non-Goals

- Do NOT emit `SessionEnd` events — Pi `session_shutdown` lifecycle handling requires further investigation and is deferred.
- Do NOT add consumer-specific `.pi/extensions/` installation or configuration files to this repository.
- Do NOT modify `scryrs record`, `scryrs-types`, the TraceEvent schema, hotspot logic, or any Rust crate.
- Do NOT capture large tool outputs, edit diffs, or document bodies — trace payloads are minimal metadata only.
- Do NOT batch events or implement IPC pooling — each `tool_result` fires one subprocess call (sufficient for reference correctness).

## Decisions

### D1: Listen on `tool_result` (post-execution), never `tool_call` (pre-execution)

**Evidence**: Pi extension docs state that `tool_call` errors **block** the tool (fail-safe), while `tool_result` errors are logged and the agent continues. The trace-hook-contract requires non-interference — scryrs must never rewrite tool output, exit status, or semantics.

**Decision**: The hook subscribes exclusively to `tool_result`. The handler returns `undefined` (no return value) so Pi passes the original tool result through unchanged.

### D2: Map `write` to `EditMade` payload family

**Evidence**: `scryrs-types` defines no `WriteMade` payload variant. Both `EditMade` and `FileOpened` have candidate fields for file paths, but `EditMade` (`target: string`) carries the semantic intent of _modifying_ a file (the `write` tool writes file contents), and the trace-hook-contract describes `EditMade` as "when an agent edits a file". The lead-dev specifically confirmed this mapping.

**Decision**: `write` → `EditMadePayload { target: filePath }`.

### D3: Conditional `lsp_navigation` → `SymbolInspected` or `FailedLookup`

**Evidence**: The trace-hook-contract specifies `FailedLookup` for failed symbol lookups and `SymbolInspected` for successful inspections. The reviewer required this conditional mapping be documented and implemented.

**Decision**: If `event.isError` is `true`, emit `FailedLookupPayload { subject: navTarget }` with `Outcome::Failure`. Otherwise emit `SymbolInspectedPayload { name: navTarget }` with `Outcome::Success`.

### D4: Defensive field access for `ast_grep_search` and `lsp_navigation` inputs

**Evidence**: The input schemas for these Pi tools are not defined in this repository. Both the architect and lead-dev flagged this as a risk and recommended defensive access with documented fallback values.

**Decision**: Access input properties via optional chaining (`event.input?.query`, `event.input?.symbol`). Document the assumed field names in the README with a note that consumers should verify against their Pi tool definitions. Use a fallback value (`"unknown"`) when expected fields are absent, accompanied by a `console.warn`.

### D5: Session demarcation — `SessionStart` only, `SessionEnd` deferred

**Evidence**: The trace-hook-contract requires explicit `SessionStart`/`SessionEnd` lifecycle events bounding every session. However, the task acceptance criteria do not test `SessionEnd`, Pi `session_shutdown` fires on process exit where the hook may not reliably emit, and both architect and lead-dev recommended deferring `SessionEnd`. The reviewer's objection was reclassified as non-blocking.

**Decision**: Generate a unique `session_id` (UUID) on extension load via `crypto.randomUUID()`. Emit `SessionStart` during `session_start` Pi event. Do not emit `SessionEnd`. This gives downstream consumers a session-scoped identifier for all events while deferring closure semantics.

### D6: Subprocess timeout of 5 seconds

**Evidence**: The hook must guarantee fail-open behavior — a hung scryrs process must not delay the agent turn. Pi `pi.exec()` supports a `timeout` option in milliseconds.

**Decision**: Set `timeout: 5000` (5 seconds) on every `pi.exec('scryrs', ...)` call. If scryrs exceeds this timeout, Pi kills the subprocess and the catch block handles the resulting error.

## Full tool-to-event mapping

| Pi tool name | TraceEvent type | Payload type | Key field | Notes |
|---|---|---|---|---|
| `read` | `FileOpened` | `FileOpenedPayload` | `path` ← `event.input.path` | |
| `bash` | `CommandExecuted` | `CommandExecutedPayload` | `command` ← `event.input.command` | |
| `ast_grep_search` | `SearchRun` | `SearchRunPayload` | `query` ← `event.input?.query ?? "unknown"` | Defensive access |
| `edit` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` | |
| `write` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` | No WriteMade variant exists |
| `lsp_navigation` (success) | `SymbolInspected` | `SymbolInspectedPayload` | `name` ← `event.input?.symbol ?? "unknown"` | Defensive access |
| `lsp_navigation` (failure) | `FailedLookup` | `FailedLookupPayload` | `subject` ← `event.input?.symbol ?? "unknown"` | Outcome: Failure |

## Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **ast_grep_search / lsp_navigation input field names unverified** — assumed field names (`query`, `symbol`) may not match actual Pi tool schemas | Medium | Document assumptions in README; use optional chaining with fallback values and `console.warn` on misses |
| **Parallel tool execution interleaving** — `tool_result` events fire in tool completion order, not source order | Low | Each event is independently serialized and dispatched; fire-and-forget subprocess model is unaffected by ordering |
| **scryrs not on PATH** — `pi.exec('scryrs', ...)` fails | Low | Caught by fail-open try-catch; traced failure is logged to `console.error`; agent tool execution continues normally |
| **SessionEnd never emitted** — downstream consumers cannot detect session completion from the trace stream alone | Low | Documented as a deferred concern; the `session_id` spans all events for attribution; a follow-up task will add SessionEnd |
| **Documented Pi tool schemas may diverge** — future Pi releases may rename tool input fields | Low | README explicitly documents the assumed field names and instructs consumers to verify against their Pi version |

## Traceability

| Source | Reference |
|---|---|
| Task definition | `task:e6fdee54-7420-49cf-a72c-d3101433411a` |
| Exploration dossier | `dossier:2026-06-20T16:32:21.353Z` |
| Architect recommendation (round 1) | `round:1:agent:swarm-architect` |
| Lead-dev recommendation (round 1) | `round:1:agent:swarm-lead-dev` |
| Reviewer recommendation (round 1) | `round:1:agent:swarm-reviewer` |
| Canonical TraceEvent schema | `crates/scryrs-types/src/lib.rs` |
| CLI ingestion contract | `crates/scryrs-cli/src/lib.rs` |
| Trace hook contract (canonical) | `.devagent/docs/docs/trace-hook-contract.md` |
| CLI v0 contract | `.devagent/docs/docs/cli-v0-contract.md` |
| Pi extensions API docs | `@earendil-works/pi-coding-agent/docs/extensions.md` |
| Trace event schema spec | `openspec/specs/trace-event-schema/spec.md` |
| scryrs record endpoint spec | `openspec/specs/scryrs-record-endpoint/spec.md` |
| Trace hook contract spec | `openspec/specs/trace-hook-contract/spec.md` |