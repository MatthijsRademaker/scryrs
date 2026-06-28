# scryrs reference trace hook for Pi

A **transport-only** Pi extension. Pi loads an in-process module (there is no
subprocess hook for Pi the way Claude Code has), so this shim cannot be deleted —
but it is reduced to pure transport. It registers `session_start` and
`tool_result`, resolves the `session_id` from Pi's `SessionManager`, serializes
the **raw** harness event, and hands it to the native `scryrs hook pi --file
<path>` command. **All tool→`TraceEvent` translation lives in the Rust
`scryrs-adapter-harness` crate**, not in this file.

Note: Pi's `exec()` opens child stdin as `/dev/null`, so the shim writes each
raw event to a temp file under a unique `/tmp/scryrs-pi-*/` directory and calls
`scryrs hook pi --file <path>` (rather than piping on stdin). Temp files are
cleaned up after each invocation, regardless of success or failure.

## Install

```bash
scryrs init --agent pi
```

This installs the slimmed `index.ts` to `.pi/extensions/pi-trace/index.ts`. Then
reload Pi with `/reload` (or restart). You can also install globally by copying
this directory to `~/.pi/agent/extensions/pi-trace/`.

Once installed, every invocation of a tracked Pi tool forwards its raw event to
`scryrs hook pi`, which translates and persists it into `.scryrs/scryrs.db` in
the working directory. Bash tracing is opt-in via `SCRYRS_DEBUG`.

## Requirements

`scryrs` must be on `PATH`. If it is missing or errors, the shim logs via
`console.error` and continues (fail-open) — no tool result is ever modified or
blocked.

## What the shim does (and does not do)

- **Does**: register `session_start`/`tool_result`, resolve `session_id`,
  capture `process.cwd()` for each event, serialize the raw event (with
  `session_id` and `cwd` injected), call `scryrs hook pi --file <tmp>`,
  clean up, fail open.
- **Does not**: map tool names to event types, maintain a tracked-tools
  whitelist, gate Bash, make HTTP calls, or know the `TraceEvent` schema.
  All of that moved to the Rust `pi` adapter, which is the single source of
  truth. The `cwd` field enables the hook path to resolve `scryrs.json`
  remote configuration from the target project root rather than the harness
  process working directory.

## Tool → event mapping (performed by the Rust `pi` adapter)

The native `scryrs hook pi` command applies this mapping. Bash is captured only
when `SCRYRS_DEBUG` is set to a non-empty value.

| Pi tool name | TraceEvent type | Key field | Notes |
|---|---|---|---|
| `read` | `FileOpened` | `path` ← `input.path` | |
| `ast_grep_search` | `SearchRun` | `query` ← `input.query` | |
| `edit` | `EditMade` | `target` ← `input.path` | |
| `write` | `EditMade` | `target` ← `input.path` | No `WriteMade` variant; a write is semantically an edit |
| `lsp_navigation` | `SymbolInspected` (ok) / `FailedLookup` (`isError`) | `name`/`subject` ← `input.symbol` | Branch on `isError` |
| `bash` (debug-only) | `CommandExecuted` | `command` ← `input.command` | Only when `SCRYRS_DEBUG` is non-empty |

Missing input fields fall back to `"unknown"`. Because `tool_result` fires
post-execution, `outcome` reflects the event's `isError` (`Failure` vs
`Success`). Events for any other tool produce no `TraceEvent` (pass-through).

## Session demarcation

The `session_id` is resolved from Pi's built-in `SessionManager.getSessionId()`
during `session_start` and injected into every forwarded event. A `SessionStart`
`TraceEvent` is produced (the shim forwards a session_start marker, which the
adapter maps to `SessionStart`). All tool events within that session carry the
same `session_id`.

`SessionEnd` is **deferred**: Pi's `session_shutdown` fires during process exit
where the shim may not reliably complete a subprocess call.

## Debug logging

`SCRYRS_DEBUG` is opt-in. When set to a non-empty value the shim emits bounded
`[scryrs]` breadcrumbs on stderr and enables Bash capture. The native `scryrs
hook pi` command additionally emits `[scryrs-hook]` breadcrumbs under the same
flag. Keep it off in normal runs.

## Fail-open guarantees

- The `scryrs hook pi --file` call runs inside a try/catch with a 5s timeout.
- Temp-file I/O errors are caught and logged; the shim returns without throwing.
- Any failure (missing binary, non-zero exit, timeout) is logged via
  `console.error`; the temp file is removed in a `finally` block.
- The `tool_result` handler always returns `undefined` — the original tool
  `content`, `details`, and `isError` pass through unchanged.

## Automated verification

```bash
scripts/verify-trace-capture --pi-only
```

This builds the real `scryrs` binary and (1) drives `scryrs hook pi --file` with
crafted raw events to verify the mapping (including `isError`→`Failure` and the
`lsp_navigation` branches), and (2) loads this `index.ts` via `tsx` with a mock
Pi runtime to verify it delegates to `scryrs hook pi --file` and fails open.
