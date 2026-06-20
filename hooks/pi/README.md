# scryrs reference trace hook for Pi

Transport-only Pi extension that maps post-execution tool events to the canonical
[TraceEvent][scryrs-types] schema and delegates ingestion to `scryrs record --stdin`.

## Install

Copy this directory (`hooks/pi/`) to one of Pi's trusted extension locations:

- `~/.pi/agent/extensions/pi-trace/` — global, available in all Pi sessions
- `.pi/extensions/pi-trace/` — project-local, only for this project

Then reload Pi with `/reload` (or restart).

Once installed every invocation of the six tracked Pi tools produces a
corresponding entry in `.scryrs/events.jsonl` in the working directory.

## Requirements

`scryrs` must be on `PATH`.  If it is missing the hook continues silently
(fail-open) — no tool result is ever modified or blocked.

## Tracked tools

| Pi tool name | TraceEvent type | Payload type | Key field | Notes |
|---|---|---|---|---|
| `read` | `FileOpened` | `FileOpenedPayload` | `path` ← `event.input.path` | |
| `bash` | `CommandExecuted` | `CommandExecutedPayload` | `command` ← `event.input.command` | |
| `ast_grep_search` | `SearchRun` | `SearchRunPayload` | `query` ← `event.input?.query ?? "unknown"` | Defensive access (see below) |
| `edit` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` | |
| `write` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` | No `WriteMade` variant exists (see below) |
| `lsp_navigation` | `SymbolInspected` (success) / `FailedLookup` (failure) | `SymbolInspectedPayload` / `FailedLookupPayload` | `name`/`subject` ← `event.input?.symbol ?? "unknown"` | Conditional on `event.isError` (see below) |

Events for any other tool are silently ignored.

## Mapping decisions

### `write` → `EditMade`

`scryrs-types` does not define a `WriteMade` payload variant.  The `write` Pi
tool creates or overwrites a file — semantically an edit — so it maps to the
existing `EditMade` family (`target: filePath`).  This was confirmed with the
project architect.

### `lsp_navigation` conditional mapping

When `event.isError` is **false** the hook emits a `SymbolInspected` TraceEvent
(`payload.name`).  When `event.isError` is **true** it emits a `FailedLookup`
TraceEvent (`payload.subject`) with outcome `Failure`.  Both paths use the same
input field extraction.

### `ast_grep_search` / `lsp_navigation` assumed input fields

The hook assumes the following input field names:

- `ast_grep_search`: `query`
- `lsp_navigation`: `symbol`

These are accessed defensively (`event.input?.query`, `event.input?.symbol`).
If a field is missing the hook uses the fallback value `"unknown"` and logs a
`console.warn`.  **Consumers should verify these field names against their
Pi tool definitions** — Pi releases may rename tool arguments.

## Fail-open guarantees

- The entire `scryrs record --stdin` subprocess call runs inside a try-catch.
- A 5-second timeout prevents hung subprocesses from delaying the agent turn.
- Any failure — missing binary, non-zero exit, timeout, I/O error — is logged
  via `console.error` with enough context to identify the tracing gap.
- The `tool_result` handler **always** returns `undefined`.  The original tool
  `content`, `details`, and `isError` are passed through unchanged.

## Session demarcation

A unique `session_id` (UUID v4) is generated when the extension loads.  A
`SessionStart` TraceEvent is emitted during Pi's `session_start` lifecycle
event.  All tool events within that session carry the same `session_id`.

### `SessionEnd` is **deferred**

`SessionEnd` is not emitted.  Pi's `session_shutdown` fires during process exit
where the hook may not reliably complete a subprocess call.  This is a known
limitation; a follow-up task will add `SessionEnd` when Pi lifecycle handling
is better understood.

Each downstream event still carries a consistent `session_id`, so consumers
can attribute all events to a session even without an explicit end marker.

## Manual verification

After installation, verify the hook works:

1. Install the hook into Pi and restart.
2. Invoke each of the six tracked tools (`read`, `bash`, `ast_grep_search`,
   `lsp_navigation`, `edit`, `write`).
3. Confirm `.scryrs/events.jsonl` contains a `SessionStart` event followed by
   the corresponding tool events.
4. Move `scryrs` off `PATH` and invoke a tracked tool — confirm the tool result
   is unchanged and `console.error` reports the scryrs failure.

[scryrs-types]: ../../crates/scryrs-types/src/lib.rs
