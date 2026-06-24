# scryrs Claude Code Reference Hook

Claude Code integration is the **native `scryrs hook claude-code` subcommand** —
not a JavaScript file. Claude Code spawns it as a `command` hook on `PreToolUse`
and pipes the event JSON on stdin. There is no `.mjs`, no node runtime, and no
hook source file in this directory. Tool→event translation lives once, in the
Rust `scryrs-adapter-harness` crate.

## How It Works

On every `PreToolUse`, Claude Code runs `scryrs hook claude-code` and pipes the
event JSON to it on stdin. The command:

1. **Reads** `session_id`, `cwd`, `tool_name`, and `tool_input` from the payload.
2. **Translates** the tool invocation into a canonical `TraceEvent` (in the Rust
   `claude-code` adapter).
3. **Persists** the event into the trace store under the payload `cwd`
   (`<cwd>/.scryrs/scryrs.db`) via the same canonical store `scryrs record` uses.
4. **Stays invisible** — it writes nothing to stdout and always exits 0, so the
   original tool runs unchanged. It is a pure observer.

Intercepted tools (default, observer-first): `Read`, `Grep`, `Glob`, `Edit`,
`Write`, `NotebookEdit`, `WebSearch`, `WebFetch`. **`Bash` is captured only when
`SCRYRS_DEBUG` is set to a non-empty value.**

## Prerequisites

- **`scryrs` must be on your `PATH`.** Claude Code invokes `scryrs hook
  claude-code` as a command hook. If `scryrs` is missing, Claude Code's own
  missing-command handling lets the tool proceed (fail-open).
- **Claude Code must have hooks enabled** (built in; configured via
  `.claude/settings.json`).

## Installation

Run the installer from your project directory:

```bash
scryrs init --agent claude-code
```

This create-or-merges `.claude/settings.json` with the native command hook,
preserving any unrelated keys and existing hooks. Re-running is idempotent.

The block it writes (or merges) is:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "scryrs hook claude-code" }
        ]
      }
    ]
  }
}
```

> **Important:** `.claude/` is your consumer-side configuration. It is **never
> committed** to the scryrs repository.

After installing, ensure `scryrs` is on your `PATH` and restart your Claude Code
session for the hook to take effect.

### Verify

```bash
scripts/verify-trace-capture --claude-only
```

This builds the real `scryrs` binary and drives `scryrs hook claude-code` with a
`PreToolUse` payload on stdin in a Docker-backed environment (no host Node hook).
It verifies the canonical mapping for all eight default tools, Bash debug-gating,
event persistence under the payload `cwd`, empty stdout, and fail-open behavior.

## Tool-to-Event Mapping

Tool names are matched as documented **PascalCase** (never lowercased).

### Default intercepted tools

| Claude Code Tool | TraceEvent `event_type` | Payload Field | Source field |
|---|---|---|---|
| `Read` | `FileOpened` | `path` | `file_path` |
| `Grep` | `SearchRun` | `query` | `pattern` |
| `Glob` | `SearchRun` | `query` | `pattern` |
| `Edit` | `EditMade` | `target` | `file_path` |
| `Write` | `EditMade` | `target` | `file_path` |
| `NotebookEdit` | `EditMade` | `target` | `notebook_path`/`file_path` |
| `WebSearch` | `SearchRun` | `query` | `query`/`searchTerm` |
| `WebFetch` | `DocRetrieved` | `doc_ref` | `url`/`website` |

### Debug-only tool

| Claude Code Tool | Capture mode | TraceEvent `event_type` | Payload Field |
|---|---|---|---|
| `Bash` | debug-only (`SCRYRS_DEBUG`) | `CommandExecuted` | `command` |

Each event carries `tool_name` set to the original PascalCase Claude Code tool
name. Embedded newlines in payload values are collapsed to a visible ` ⏎ `
marker so each serialized event occupies one line.

## Limitations

### PreToolUse Only — Outcome Is Always Success

`PreToolUse` fires **before** the tool executes, so the real outcome is unknown.
Every emitted event carries `outcome: Success` unconditionally. These are
pre-execution metadata signals, not post-execution outcomes.

### No Session Lifecycle Events

`PreToolUse` has no session-open/close trigger; this integration emits no
`SessionStart`/`SessionEnd` events. Only subject-bearing tool events are produced.

### Session IDs Come From the Payload

The integration reads `session_id` directly from the `PreToolUse` payload (no
per-process UUID, no `CLAUDE_SESSION_ID`-style environment variables).

## Fail-Open Behavior

The command **always exits 0 with empty stdout** — scryrs never blocks tool
execution. On any internal error it appends a timestamped line to
**`.scryrs/hooks/claude-code-warnings.log`** (under the payload `cwd`) and still
exits 0.

| Scenario | Behavior |
|---|---|
| Malformed event JSON on stdin | Warning logged; exit 0 |
| Unknown harness routing | Warning logged; exit 0 |
| Trace store cannot be opened/written | Warning logged; exit 0 |
| `scryrs` binary not on `PATH` | Claude Code's missing-command handling lets the tool proceed |

Example warning log entry:

```
2026-06-24T12:00:00Z malformed JSON input
2026-06-24T12:00:05Z cannot open store: …
```

## Architecture Notes

- **Native, not JavaScript.** Translation and persistence are Rust; there is no
  `.mjs` file and no node dependency.
- **Not a proxy / not an MCP server.** The command is a pure observer of the
  `PreToolUse` event and is never a callable agent tool.
- **`hook` sits above `record`.** `scryrs hook` translates the foreign event,
  then reuses the same canonical `EventStore` that `scryrs record` writes to.

## Related Documentation

- [Trace Hook Contract](../../.devagent/docs/docs/trace-hook-contract.md) — full hook integration contract
- [CLI v0 Contract](../../.devagent/docs/docs/cli-v0-contract.md) — `scryrs record` output contract
