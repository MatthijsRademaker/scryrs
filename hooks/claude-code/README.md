# scryrs Claude Code Reference Hook

This directory contains the scryrs reference hook for Claude Code — a thin JavaScript transport module that intercepts Claude Code PreToolUse events, maps them to canonical `TraceEvent` JSON objects, and forwards them to `scryrs record --stdin`.

## How It Works

The hook (`scryrs-hook.mjs`) is a Claude Code PreToolUse hook module. It:

1. **Intercepts** eight Claude Code tools by default: Read, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, WebFetch. **Bash is not captured by default** — it is re-enabled only when `SCRYRS_DEBUG` is set to a non-empty value.
2. **Maps** each tool invocation to a scryrs `TraceEvent` with the canonical schema.
3. **Forwards** the event as newline-delimited JSON to `scryrs record --stdin`.
4. **Stays invisible** — the hook never alters the original tool's stdout, stderr, or exit status. It is a pure observer.

## Prerequisites

- **`scryrs` must be on your `PATH`.** The hook spawns `scryrs record --stdin` as a child process. If `scryrs` is not found, the hook logs a warning and continues (see [Fail-Open Behavior](#fail-open-behavior)).
- **Claude Code must have hooks enabled.** Hook support is built into Claude Code. You configure it via `.claude/settings.json` (see [Installation](#installation)).

## Installation

### 1. Copy the hook file

Copy `scryrs-hook.mjs` from this directory into your project (or reference it by path in your Claude Code hook configuration):

```bash
cp hooks/claude-code/scryrs-hook.mjs /path/to/your/project/.claude/hooks/scryrs-hook.mjs
```

### 2. Configure Claude Code to use the hook

Add the hook to your `.claude/settings.json` (or `.claude/settings.local.json`):

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "",
        "hook": "node .claude/hooks/scryrs-hook.mjs"
      }
    ]
  }
}
```

> **Important:** The `.claude/` directory is your consumer-side configuration. It is **never committed** to the scryrs repository. You must create and maintain it in your own project.

### 3. Verify the hook is working

Run the automated cross-harness verification entrypoint:

```bash
scripts/verify-trace-capture --claude-only
```

This builds the real `scryrs` binary and exercises the hook against it in a
Docker-backed environment (no host Node.js required). It verifies:

- JSON shaping for all eight default whitelisted tools (Bash excluded)
- Default mode: Bash is not captured
- Debug mode (`SCRYRS_DEBUG=1`): Bash is captured as `CommandExecuted`
- Event persistence to `.scryrs/scryrs.db` with canonical `TraceEvent` envelope shape
- Non-interference: hook produces zero stdout/stderr
- Fail-open: hook returns `{continue: true}` when scryrs is missing
- Pass-through: unlisted tools produce no events

For rapid development feedback, run the fast-path test instead:

```bash
scripts/hook-test
```

This uses a fake shell-script scryrs and validates JSON shaping and fail-open
logic without building the Rust binary.

## Rewrite-tool compatibility

### Bash capture is debug-gated

Bash is **not captured by default**. Set `SCRYRS_DEBUG` to any non-empty value to re-enable Bash tracing for diagnostic sessions.

When Bash capture is enabled, the Claude Code hook captures Bash commands from `tool_input.command` during the **PreToolUse** event — before the tool executes. Whatever command string the harness presents at the time the scryrs hook runs in the PreToolUse pipeline is recorded as `CommandExecuted.payload.command` exactly as observed.

### Hook-order dependence

Co-installed rewrite hooks (e.g., RTK) **can change** what scryrs observes, depending on hook execution order and platform forwarding behavior:

- If scryrs runs **before** a rewrite hook, it records the original agent-entered command.
- If scryrs runs **after** a rewrite hook, it records the rewritten command.
- If the platform forwards `updatedInput` between hooks, the observed command depends on whether scryrs is positioned before or after the rewrite hook in the pipeline.

Hook order is determined by the consumer's `.claude/settings.json` hook configuration. scryrs does not control or enforce hook ordering.

### What is NOT guaranteed (when Bash capture is enabled)

- `CommandExecuted.payload.command` **is not guaranteed** to preserve the original agent-entered command. It records whichever command string PreToolUse presents when scryrs runs.
- The single-string `CommandExecutedPayload` schema **cannot** preserve both original and rewritten commands in one event.
- Whether the platform forwards `updatedInput` between hooks is platform-dependent and may vary between Claude Code versions.

## Tool-to-Event Mapping

Eight Claude Code tools are intercepted by default. Bash interception is debug-gated and only active when `SCRYRS_DEBUG` is set to a non-empty value.

### Default intercepted tools

| Claude Code Tool | TraceEvent `event_type` | Payload Field | Description |
|---|---|---|---|
| `Read` | `FileOpened` | `path` | File path from tool input |
| `Grep` | `SearchRun` | `query` | Search pattern from tool input |
| `Glob` | `SearchRun` | `query` | Glob pattern from tool input |
| `Edit` | `EditMade` | `target` | File path from tool input |
| `Write` | `EditMade` | `target` | File path from tool input |
| `NotebookEdit` | `EditMade` | `target` | File path from tool input |
| `WebSearch` | `SearchRun` | `query` | Search term from tool input |
| `WebFetch` | `DocRetrieved` | `doc_ref` | URL from tool input |

### Debug-only tool

| Claude Code Tool | Capture mode | TraceEvent `event_type` | Payload Field | Description |
|---|---|---|---|---|
| `Bash` | debug-only (`SCRYRS_DEBUG`) | `CommandExecuted` | `command` | Command string from tool input |

Each event carries `tool_name` set to the original Claude Code tool name (e.g., `"bash"`, `"web_search"`).

## Limitations

### PreToolUse Only — Outcome Is Always Success

This hook uses Claude Code's **PreToolUse** event, which fires **before** the tool executes. The hook cannot determine the real outcome (success or failure) of the tool. Every emitted event carries `outcome: Success` unconditionally.

These events are **pre-execution metadata signals**, not post-execution outcomes. If you need outcome-accurate trace data, a PostToolUse hook (future work) would be required.

### No Session Lifecycle Events

PreToolUse hooks have no session-open or session-close trigger. This hook does **not** emit `SessionStart` or `SessionEnd` lifecycle events. Only subject-bearing tool events are produced.

### Session IDs Are Per-Process

The hook generates a UUID v4 session ID once per hook process lifetime. If Claude Code provides a session-scoped identifier via environment variables (`CLAUDE_SESSION_ID`, `CC_SESSION_ID`, or `CLAUDE_CODE_SESSION_ID`), the hook prefers that; otherwise it generates its own UUID v4.

All events within a single hook process share the same `session_id`. Across restarts of Claude Code, a new session ID is generated.

## Fail-Open Behavior

The hook **fails open**: scryrs failures never block tool execution.

| Scenario | Behavior |
|---|---|
| `scryrs` binary not on `PATH` | Warning logged; tool executes normally |
| `scryrs record` exits non-zero | Warning logged; tool executes normally |
| `scryrs record` times out (5s) | Warning logged; scryrs killed; tool executes normally |
| `scryrs record` crashes (signal) | Warning logged; tool executes normally |
| Subprocess stdin write fails | Warning logged; tool executes normally |

Warnings are written to **`.scryrs/hooks/claude-code-warnings.log`** (relative to your project root) with ISO-8601 timestamps. Warnings are **never** written to stdout or stderr — the agent-visible tool output is unchanged.

Example warning log entry:

```
2026-06-20T12:00:00.000Z scryrs binary not found on PATH
2026-06-20T12:00:05.000Z scryrs record exited with code 1
```

## Consumer-Side Configuration

The `.claude/` directory (containing `settings.json`, hook configuration, etc.) is **consumer-side configuration** — it belongs to the user's project or home directory, not to the scryrs repository.

**No `.claude/` configuration is committed in this repository.** You must create and maintain your own `.claude/` hook configuration in your project.

## Architecture Notes

- **Not a proxy.** The hook is a pure observer. It receives a copy of the PreToolUse event but never sits in the tool execution path.
- **Not an MCP server.** scryrs is never registered as a callable tool. Hooks call scryrs; agents do not.
- **No business logic.** The hook contains no validation, scoring, analysis, or intelligence. All logic beyond formatting and subprocess delegation lives inside scryrs crates.
- **stdin pipe only.** The hook uses `scryrs record --stdin`. No file mode or alternate ingestion path.
- **Multi-line safety.** Embedded newlines in payload values (e.g., multi-line Bash commands) are collapsed to a visible marker (` ⏎ `) to maintain JSONL line integrity.

## Related Documentation

- [Trace Hook Contract](.devagent/docs/docs/trace-hook-contract.md) — full hook integration contract
- [Product Roadmap](.devagent/docs/docs/roadmap.mdx) — Phase 1 delivery sequence
- [CLI v0 Contract](.devagent/docs/docs/cli-v0-contract.md) — `scryrs record` output contract
