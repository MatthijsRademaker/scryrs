# scryrs Claude Code Reference Hook

Claude Code integration is the **native `scryrs hook claude-code` subcommand** —
not a JavaScript file. Claude Code spawns it as a `command` hook on **both
`PostToolUse` and `PostToolUseFailure`** and pipes the event JSON on stdin. There
is no `.mjs`, no node runtime, and no hook source file in this directory.
Tool→event translation lives once, in the Rust `scryrs-adapter-harness` crate.

## How It Works

On every `PostToolUse` and `PostToolUseFailure`, Claude Code runs
`scryrs hook claude-code` and pipes the event JSON to it on stdin. The command:

1. **Reads** `session_id`, `cwd`, `hook_event_name`, `tool_name`, and
   `tool_input` from the payload.
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
    "PostToolUse": [
      {
        "matcher": "",
        "hooks": [
          { "type": "command", "command": "scryrs hook claude-code" }
        ]
      }
    ],
    "PostToolUseFailure": [
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

This builds the real `scryrs` binary and drives `scryrs hook claude-code` with
`PostToolUse` and `PostToolUseFailure` payloads on stdin in a Docker-backed
environment (no host Node hook).
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

### Two Events Are Required, Not One

`PostToolUse` fires **only after a tool call succeeds**. Failures arrive on the
separate **`PostToolUseFailure`** event. Registering on `PostToolUse` alone would
make every recorded outcome `Success` — the exact defect that made the previous
`PreToolUse` integration unusable as an outcome source.

So `scryrs init --agent claude-code` registers the command on both events, and
the adapter derives the outcome from `hook_event_name` rather than inferring it
from response contents. The two events are mutually exclusive per tool call, so
registering both does not double-count.

A failed `Read` is recorded as `FailedLookup` (the agent addressed a path that
does not exist), not as `FileOpened` with a failure outcome — matching the Pi
adapter. This makes `FailedLookup` reachable on Claude Code for the first time.

### Migrating From PreToolUse

`scryrs init --agent claude-code` removes a `scryrs hook claude-code`
registration under `PreToolUse` that a previous release wrote, because running
both would double-count every event. Foreign `PreToolUse` entries belonging to
other tools are left untouched.

`scryrs doctor` reports a leftover `PreToolUse` registration, and a registration
present on only one of the two post-tool events, as actionable warnings.

### Non-Interference Requires Restraint on PostToolUse

`PostToolUse` supports `decision: "block"` and `updatedToolOutput`, which would
replace the tool's result before Claude sees it. `PreToolUse` gave scryrs no such
power. **The hook emits neither field**: it writes nothing to stdout, never
modifies the agent-visible tool result, and always exits 0. scryrs is an
observer, and the hook event it uses does not change that.

Diagnostics therefore go to `.scryrs/hooks/claude-code-warnings.log`, not to
stderr — Claude Code can surface hook stderr, and a trace hook must not put its
own noise into the model's context.

### No Session Lifecycle Events

This integration emits no `SessionStart`/`SessionEnd` events; only
subject-bearing tool events are produced. Claude Code does expose `SessionStart`
and `SessionEnd` hook events, so this gap is closable — it is simply not wired
yet.

### Session IDs Come From the Payload

The integration reads `session_id` directly from the hook payload (no
per-process UUID, no `CLAUDE_SESSION_ID`-style environment variables).

### Path Subjects Are Normalized

Path subjects are normalized to canonical repository-relative POSIX form against
the payload `cwd`, so a file addressed absolutely and relatively produces one
hotspot subject. Paths outside the repository are recorded verbatim and grouped
separately under the `external_file` subject kind.

### A Missing Key Input Field Drops the Event

If a supported tool's key input field is absent, the event is **dropped** and the
contract violation is recorded in `.scryrs/hooks/claude-code-warnings.log`. No
placeholder subject is ever persisted — an `"unknown"` or empty subject is a
fabrication that looks like data.

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
  post-tool events and is never a callable agent tool.
- **`hook` sits above `record`.** `scryrs hook` translates the foreign event,
  then reuses the same canonical `EventStore` that `scryrs record` writes to.

## Related Documentation

- [Trace Hook Contract](../../.devagent/docs/docs/trace-hook-contract.md) — full hook integration contract
- [CLI v0 Contract](../../.devagent/docs/docs/cli-v0-contract.md) — `scryrs record` output contract
