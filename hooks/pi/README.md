# scryrs reference trace hook for Pi

Transport-only Pi extension that maps post-execution tool events to the canonical
[TraceEvent][scryrs-types] schema and delegates ingestion to `scryrs record --file <path>`.

Note: Pi's `exec()` uses `stdio: ["ignore", "pipe", "pipe"]` — stdin cannot be
written to.  The hook writes each event to a temp file under a unique
`/tmp/scryrs-pi-*/` directory and invokes `scryrs record --file <path>` instead
of `--stdin`.  Temp files are cleaned up after each ingestion, regardless of
success or failure.

## Install

Copy this directory (`hooks/pi/`) to one of Pi's trusted extension locations:

- `~/.pi/agent/extensions/pi-trace/` — global, available in all Pi sessions
- `.pi/extensions/pi-trace/` — project-local, only for this project

Then reload Pi with `/reload` (or restart).

Once installed every invocation of the six tracked Pi tools produces a
corresponding entry in `.scryrs/scryrs.db` in the working directory.

## Requirements

`scryrs` must be on `PATH`.  If it is missing the hook continues silently
(fail-open) — no tool result is ever modified or blocked.

## Debug logging

Debug output is **opt-in** through `SCRYRS_DEBUG` and is intended for local
development and trace-pipeline troubleshooting.

- `SCRYRS_DEBUG=1` — emit bounded `[scryrs]` hook breadcrumbs and echo bounded
  `[scryrs-record]` child-process breadcrumbs.
- `SCRYRS_DEBUG=wire` — include sanitized previews of observed input fields such
  as `command`, `path`, `query`, and `symbol`.
- `SCRYRS_DEBUG=raw` — include a capped raw event preview for deeper local
  debugging.

All debug lines stay single-line and bounded. With `SCRYRS_DEBUG` unset the
hook stays quiet apart from existing fail-open warnings/errors.

**Warning:** `wire` and especially `raw` mode can expose observed tool inputs.
Do not enable them in sessions where those inputs may contain secrets or file
contents you would not want echoed to stderr.

Example:

```bash
SCRYRS_DEBUG=1 pi
SCRYRS_DEBUG=wire node scripts/verification/pi-hook-e2e.mjs
```

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

## Rewrite-tool compatibility

### Capture point: `tool_result`

The Pi hook captures Bash commands from `event.input.command` on `tool_result` — the post-execution event. Whatever command string the harness presents on `tool_result` is recorded as `CommandExecuted.payload.command` exactly as observed.

This means:

- If an upstream rewrite extension (e.g., RTK) transforms the `tool_call` input **and** the harness propagates that mutation into `tool_result`, scryrs records the rewritten form.
- If the harness does **not** propagate `tool_call` mutations, scryrs records whatever command string `tool_result` carries — which may be the original agent-entered command.

### Limitations

- **Mutation propagation is not verified.** Whether Pi propagates `tool_call` input mutations through to `tool_result` is an empirical question. Until confirmed, the observed-command behavior described above is a **limitation statement**, not a verified guarantee.
- **Rewrite prefixes are persisted as-is.** Commands like `rtk ls -la` appear verbatim in traces. scryrs does not strip prefixes, normalize, or reconstruct original intent.
- **Compound commands remain single trace events.** A command like `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/` is recorded as one `CommandExecuted` event, not split into subcommands.

## Fail-open guarantees

- The entire `scryrs record --file` subprocess call runs inside a try-catch.
- Temp file I/O errors are caught separately — the hook logs and returns
  before even attempting the subprocess call.
- A 5-second timeout prevents hung subprocesses from delaying the agent turn.
- Any failure — missing binary, non-zero exit, timeout, temp-file I/O error —
  is logged via `console.error` with enough context to identify the tracing gap.
- Temp files are always cleaned up in a `finally` block.
- The `tool_result` handler **always** returns `undefined`.  The original tool
  `content`, `details`, and `isError` are passed through unchanged.

## Session demarcation

The `session_id` is resolved from Pi's built-in `SessionManager.getSessionId()`
during the `session_start` lifecycle event.  This ensures alignment with Pi's
session identity — the same UUID that appears in the session file path
(`~/.pi/agent/sessions/--<path>--/<timestamp>_<uuid>.jsonl`) and the session
header entry (`{"type":"session","id":"..."}`).

A `SessionStart` TraceEvent is emitted during Pi's `session_start` lifecycle
event.  All tool events within that session carry the same `session_id`.

### `SessionEnd` is **deferred**

`SessionEnd` is not emitted.  Pi's `session_shutdown` fires during process exit
where the hook may not reliably complete a subprocess call.  This is a known
limitation; a follow-up task will add `SessionEnd` when Pi lifecycle handling
is better understood.

Each downstream event still carries a consistent `session_id`, so consumers
can attribute all events to a session even without an explicit end marker.

### Transport: `--file` (not `--stdin`)

The hook writes each event as a single-line JSONL file under a unique temp
directory (`/tmp/scryrs-pi-*/`), then delegates ingestion via `scryrs record
--file <path>`.  This works around Pi's `exec()` implementation which opens
child process stdin as `/dev/null` (`stdio: ["ignore", ...]`).

Temp files are deleted in a `finally` block after each ingestion attempt.

## Automated verification

Run the cross-harness verification entrypoint to exercise the Pi hook against
the real `scryrs record --stdin` binary in a Docker-backed environment:

> **Note:** The cross-harness test uses `--stdin` directly via shell pipe.
> The Pi-specific `--file` transport is only relevant when running inside
> the actual Pi process.

```bash
scripts/verify-trace-capture --pi-only
```

This builds the real `scryrs` binary and verifies:

- SessionStart lifecycle event emission and persistence
- Tool event capture for all six tracked Pi tools with canonical
  `TraceEvent` envelope shape
- Debug-disabled quiet behavior (`[scryrs]` and `[scryrs-record]` absent)
- Debug-enabled breadcrumbs for hook load, session start, tracked/untracked
  tools, missing-field fallback, record send/result, and exec-failure paths
- Handler returns `undefined` for all events (non-interference)
- Failure propagation: failing `lsp_navigation` produces `FailedLookup`
  with `outcome.result: 'Failure'` while original error payload is unchanged
- Fail-open: handler returns `undefined` when scryrs is missing;
  `console.error` reports the failure
- Unlisted tools are silently ignored

Run the full cross-harness suite (Claude Code + Pi) with:

```bash
scripts/verify-trace-capture
```

## Manual verification (deprecated)

After installation, verify the hook works:

1. Install the hook into Pi and restart.
2. Invoke each of the six tracked tools (`read`, `bash`, `ast_grep_search`,
   `lsp_navigation`, `edit`, `write`).
3. Confirm `.scryrs/scryrs.db` contains a `SessionStart` event followed by
   the corresponding tool events.
4. Move `scryrs` off `PATH` and invoke a tracked tool — confirm the tool result
   is unchanged and `console.error` reports the scryrs failure.

[scryrs-types]: ../../crates/scryrs-types/src/lib.rs
