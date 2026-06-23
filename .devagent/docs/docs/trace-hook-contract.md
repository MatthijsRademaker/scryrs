# Trace Hook Contract

**Status:** Canonical — this is the single source of truth for harness integration with scryrs.

This document defines what harness integrators must capture, how to invoke `scryrs record`, what boundaries scryrs must never cross, and which integration path fits their harness. Harness authors and agent-platform builders should read this before writing any scryrs hook.

## Purpose and Boundaries

scryrs is a **trace-collection CLI** built with an observer-first philosophy. It ingests structured JSONL trace events from agent coding sessions and persists them for downstream hotspot analysis, graph building, and knowledge proposals. It is never a tool executor, proxy, MCP server, or agent-callable business tool.

**Default capture scope:** scryrs observes stable harness-native tools by default (file reads, edits, search, symbol inspection, document fetch). Bash command capture is **not part of default product behavior** — it is debug-gated behind `SCRYRS_DEBUG`. This keeps trace data focused on high-signal hotspot evidence rather than noisy shell commands.

| In scope | Out of scope |
|----------|--------------|
| Defining what harness hooks must capture and how to format it | Implementing harness hooks for any specific agent platform |
| Documenting the `scryrs record` ingestion contract | Changing the `TraceEvent` wire schema or CLI behavior |
| Describing the `scryrs.json` manifest shape | Creating a checked-in `scryrs.json` file |
| Defining integration tiers with explicit limitations | Building a hooks directory or reference implementation |
| Referencing the canonical `TraceEvent` schema in Rust | Redefining the schema from scratch |

## Non-Interference and Fail-Open Rules

The following rules are non-negotiable. Every scryrs hook, regardless of integration tier, must obey them.

### scryrs is trace-collection only

- **scryrs never rewrites** tool stdout, stderr, exit status, or semantics.
- **scryrs does not proxy** business-tool execution. Hooks invoke scryrs as a subprocess *after* the business tool completes; scryrs receives a copy of tool metadata but never sits in the tool execution path.
- **Hooks contain no business logic** beyond formatting event data and delegating to `scryrs record`. All intelligence — validation, scoring, analysis — lives inside scryrs crates.
- **scryrs is never registered** as an agent-callable business tool, MCP server, tool catalog entry, or any surface an agent can invoke directly. Agents do not call scryrs; hooks call scryrs.

### Fail-open guarantee

If a hook's invocation of `scryrs record` fails for any reason — process crash, pipe error, non-zero exit code, missing binary — the harness **must** proceed with the original tool execution normally. The original tool's stdout, stderr, and exit status are preserved unmodified. A scryrs failure is a tracing gap, not a tool-execution failure.

The design rule is: **scryrs can fail, tools cannot.**

## Rewrite-Tool Compatibility (Phase 1)

`CommandExecuted.payload.command` records the command string **observed by the hook at capture time**. scryrs never rewrites, normalizes, canonicalizes, or reconstructs original agent intent from the command string it receives. This is the Phase 1 compatibility policy — it resolves the original-versus-rewritten ambiguity by recording exactly what the hook observed, nothing more.

### What scryrs does NOT do

- scryrs does **not** invoke rewrite tools (e.g., RTK) from within any hook.
- scryrs does **not** strip rewrite prefixes (e.g., `rtk`) from observed command strings.
- scryrs does **not** split compound commands into multiple trace events.
- scryrs does **not** attempt to recover or preserve the pre-rewrite command text.
- scryrs does **not** normalize, canonicalize, or alter the command string in any way.

### Harness-specific semantics

Rewrite-tool co-installation behaves differently across harnesses, and Bash capture is only active when `SCRYRS_DEBUG` is set to a non-empty value. Integrators must understand these differences:

| Harness | Capture point | What the hook sees |
|---------|---------------|--------------------|
| **Pi** | `tool_result` (post-execution) | `event.input.command` from the `tool_result` event — reflects whatever command string the harness presents after execution completes. If an upstream rewrite extension mutated the `tool_call` input, and the harness propagates that mutation into `tool_result`, scryrs records the rewritten form. **Only active when `SCRYRS_DEBUG` is set.** |
| **Claude Code** | PreToolUse (pre-execution) | `tool_input.command` from the PreToolUse event — reflects whatever command string the harness presents at the time the scryrs hook runs in the PreToolUse pipeline. Co-installed rewrite hooks can change this value depending on hook order. **Only active when `SCRYRS_DEBUG` is set.** |

### Limitations

**Bash command capture is debug-gated.** No `CommandExecuted` events are emitted by default. Set `SCRYRS_DEBUG` to any non-empty value to re-enable Bash tracing for diagnostic sessions. When enabled, `CommandExecuted.payload.command` records the command string observed by the hook at capture time — scryrs never rewrites, normalizes, canonicalizes, or reconstructs original agent intent.

- **Hotspot subjects remain fragmented** between rewritten and non-rewritten commands (e.g., `ls -la` and `rtk ls -la` are distinct subjects). Command canonicalization remains a known limitation not scheduled for any current roadmap phase.
- **Pi mutation propagation** from `tool_call` input mutations through to `tool_result` is an empirical assumption. If not yet verified, this behavior is presented as a limitation rather than a guarantee.
- **Claude Code updated-input forwarding** between PreToolUse hooks is platform-dependent. The observed command may differ if hook-order changes between environments.
- The `CommandExecutedPayload` schema contains a single `command` field. Preserving both original and effective commands within a single trace event is not supported in Phase 1.

## TraceEvent Schema

The canonical `TraceEvent` schema is defined in [`crates/scryrs-types/src/lib.rs`](https://github.com/scryrs-project/scryrs/blob/main/crates/scryrs-types/src/lib.rs). Do not redefine it. The Rust types are the executable source of truth for the wire contract.

### Envelope Fields

Every `TraceEvent` on the wire must carry these fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `schema_version` | string | **yes** | Current `SCHEMA_VERSION` from scryrs-types (`"0.1.0"`) |
| `timestamp` | string | **yes** | RFC 3339 timestamp (e.g. `"2026-06-20T12:00:00Z"`) |
| `session_id` | string | **yes** | Unique session identifier, stable from SessionStart through SessionEnd |
| `event_type` | string | **yes** | One of the nine `TraceEventType` variants (see Event Families below) |
| `tool_name` | string | optional | The harness tool name for subject-bearing events; omitted for lifecycle events |
| `payload` | object | **yes** | Self-describing JSON object with a `type` tag identifying the payload family |
| `outcome` | object | **yes** | `{"result": "Success"}` or `{"result": "Failure", "reason": "..."}` |

### Event Families

Nine event families exist. Each maps to one `TraceEventType` variant and its corresponding payload shape. The `type` field in `payload` mirrors `event_type` so consumers can identify the concrete shape from JSON alone.

| event_type | Payload type | Subject-bearing? | Key payload fields |
|------------|-------------|-------------------|--------------------|
| `SessionStart` | `SessionStart` | No (lifecycle) | none (unit struct) |
| `SessionEnd` | `SessionEnd` | No (lifecycle) | none (unit struct) |
| `FileOpened` | `FileOpened` | Yes | `path`: string |
| `SearchRun` | `SearchRun` | Yes | `query`: string |
| `SymbolInspected` | `SymbolInspected` | Yes | `name`: string |
| `CommandExecuted` | `CommandExecuted` | Yes | `command`: string |
| `DocRetrieved` | `DocRetrieved` | Yes | `doc_ref`: string |
| `EditMade` | `EditMade` | Yes | `target`: string |
| `FailedLookup` | `FailedLookup` | Yes | `subject`: string |

**Mapping guidance for harness authors:**

- **FileOpened** — emit when an agent reads or opens a file. `payload.path` is the file path.
- **SearchRun** — emit when an agent executes a code search. `payload.query` is the search query.
- **SymbolInspected** — emit when an agent inspects a symbol (definition, references, type info). `payload.name` is the symbol name.
- **CommandExecuted** — emit when an agent runs a shell command. `payload.command` is the command string.
- **DocRetrieved** — emit when an agent retrieves documentation. `payload.doc_ref` is the document reference or path.
- **EditMade** — emit when an agent edits a file. `payload.target` is the file path.
- **FailedLookup** — emit when an agent fails to find a symbol, file, or concept. `payload.subject` is what the agent was looking for. This event should carry `outcome: Failure`.

Every subject-bearing event should carry `tool_name` set to the harness tool name (`"read"`, `"search"`, `"bash"`, `"edit"`, etc.) so downstream hotspot analysis can attribute activity to the correct tool.

## Session Demarcation

Session boundaries are **first-class lifecycle events**, not implicit heuristics.

Every trace session:

1. **Starts** with a `SessionStart` event carrying a new unique `session_id`.
2. **Contains** zero or more subject-bearing events, all carrying the same `session_id`.
3. **Ends** with a `SessionEnd` event carrying the same `session_id`.

`SessionStart` and `SessionEnd` are lifecycle events. They omit `tool_name` since they have no hotspot subject.

No implicit boundaries exist. A session is explicitly opened and closed. Consumers downstream detect session completion from the `SessionEnd` event in the stream.

### Example session outline

```jsonl
{"schema_version":"0.1.0","timestamp":"2026-06-20T12:00:00Z","session_id":"sess-abc123","event_type":"SessionStart","tool_name":null,"payload":{"type":"SessionStart"},"outcome":{"result":"Success"}}
{"schema_version":"0.1.0","timestamp":"2026-06-20T12:00:05Z","session_id":"sess-abc123","event_type":"FileOpened","tool_name":"read","payload":{"type":"FileOpened","path":"src/main.rs"},"outcome":{"result":"Success"}}
{"schema_version":"0.1.0","timestamp":"2026-06-20T12:00:10Z","session_id":"sess-abc123","event_type":"SearchRun","tool_name":"search","payload":{"type":"SearchRun","query":"error handling"},"outcome":{"result":"Success"}}
{"schema_version":"0.1.0","timestamp":"2026-06-20T12:00:15Z","session_id":"sess-abc123","event_type":"SessionEnd","tool_name":null,"payload":{"type":"SessionEnd"},"outcome":{"result":"Success"}}
```

## scryrs record Invocation Contract

`scryrs record` is the **only ingestion endpoint**. There are exactly two invocation modes, and they are mutually exclusive.

### Supported modes

| Mode | Syntax | Description |
|------|--------|-------------|
| stdin pipe | `scryrs record --stdin` | Read newline-delimited `TraceEvent` JSON from stdin |
| file read | `scryrs record --file <PATH>` | Read JSONL from a file |

**No other ingestion paths exist.** scryrs has no socket, HTTP endpoint, IPC mechanism, pipe wrapper, or alternate command for trace ingestion.

### Output contract

The full output shape, exit codes, and rejection diagnostics are defined in the [CLI v0 Contract](./cli-v0-contract.md). Key points for harness authors:

- **Stdout:** A single-line JSON summary `{"command":"record","schemaVersion":"0.1.0","accepted":N,"rejected":M}`.
- **Stderr:** One JSON rejection diagnostic per rejected non-empty line.
- **Exit 0:** All lines accepted.
- **Exit 1:** One or more lines rejected (ingestion continues).
- **Exit 2:** Fatal usage error (both/neither mode, unreadable file, store failure).

### Harness invocation example (stdin pipe)

```bash
# Hook collects events into a JSONL string and pipes to scryrs
echo "$events_jsonl" | scryrs record --stdin
```

### Harness invocation example (file mode)

```bash
# Hook writes events to a temp file, then invokes scryrs
scryrs record --file /tmp/scryrs-session.jsonl
```

### Fail-open invocation pattern

Hook authors must ensure that `scryrs record` failure never blocks the harness tool. Recommended pattern:

```text
1. Tool execution completes
2. Hook formats TraceEvent(s) from tool metadata
3. Hook spawns `scryrs record --stdin` as subprocess with timeout
4. If scryrs subprocess times out, crashes, or exits non-zero: log the tracing gap, continue
5. Original tool stdout/stderr/exit status returned to agent unchanged
```

## scryrs.json Manifest Shape

`scryrs.json` is a **hook-interface and record-invocation manifest** placed at the repository root. It describes which hooks are configured, which event families they capture, and how `scryrs record` is invoked.

### What scryrs.json is

- A declarative description of the hook interface for a repository.
- A record of which event families the hook captures.
- A reference for `scryrs record` invocation parameters.

### What scryrs.json is NOT

- **Not a tool catalog.** It does not describe callable agent tools.
- **Not an MCP descriptor.** It does not advertise server capabilities or tool schemas.
- **Not a business-tool surface.** Agents do not read or invoke scryrs through this manifest.
- **Not a registry of agent-accessible functions.** If you want a tool catalog, use your harness's native tool registration system — not scryrs.json.

### Intended location

`scryrs.json` lives at the repository root, alongside `Cargo.toml`, `package.json`, or equivalent project root marker. A checked-in `scryrs.json` exists in the scryrs repository root defining the Phase 1 hook-interface and record-invocation contract.

### Current shape (v0.1)

The manifest schema is **v0.1**, stable for Phase 1. The checked-in `scryrs.json` at the repository root is the source of truth for the hook-interface and record-invocation contract.

**Example minimal skeleton:**

```json
{
  "manifest_version": "0.1.0",
  "hooks": {
    "tool_events": {
      "capture": [
        "FileOpened",
        "SearchRun",
        "SymbolInspected",
        "CommandExecuted",
        "DocRetrieved",
        "EditMade",
        "FailedLookup"
      ],
      "record": {
        "command": "scryrs",
        "args": ["record", "--stdin"]
      }
    },
    "session_lifecycle": {
      "capture": [
        "SessionStart",
        "SessionEnd"
      ],
      "record": {
        "command": "scryrs",
        "args": ["record", "--stdin"]
      }
    }
  }
}
```

**Field notes:**

- `manifest_version` — version of the manifest schema itself (independent of `SCHEMA_VERSION`).
- `hooks.tool_events` — configuration for subject-bearing tool-event capture.
- `hooks.session_lifecycle` — configuration for session lifecycle event capture.
- `capture` — array of `event_type` strings the hook emits.
- `record.command` / `record.args` — how `scryrs record` is invoked by hooks.

### Anti-pattern warning

**Do not** interpret `scryrs.json` as describing callable tools. It describes a hook interface — a one-way data flow from harness tools into scryrs' ingestion path. There is no output path from scryrs back to agent tools through this manifest.

## Integration-Tier Matrix

Three integration tiers exist, offering different levels of coverage and requiring different levels of harness support. Choose the highest tier your harness supports.

| Tier | Mechanism | Session demarcation | Event coverage | Limitations |
|------|-----------|---------------------|----------------|-------------|
| **Full hook** | Harness-native subprocess hook system (e.g. Pi `.pi/hooks/`, Claude Code hook system) | Automatic | Full — all tool events intercepted | Requires harness with hook/subprocess extension support |
| **Plugin** | Harness-specific plugin/extension API | Depends on plugin API | Partial — coverage limited by plugin capabilities | Requires plugin auth, development, and maintenance per harness |
| **Rules-file fallback** | Manual event-rule authoring by user | Not automatic | Inherently partial | Requires manual rule authoring; cannot intercept tool events without harness cooperation; no automatic session demarcation |

### Tier 1: Full Hook

**What it is:** A harness-native subprocess hook that runs after every tool execution. The hook formats the tool's metadata into `TraceEvent` JSON and invokes `scryrs record --stdin`.

**Guarantees:**

- Automatic session demarcation (SessionStart/SessionEnd).
- Full event coverage across all nine event families.
- Fail-open by construction: hook runs as a subprocess *after* the tool, not as a proxy.

**Implemented harness coverage:**

- **Pi** — implemented at `hooks/pi/index.ts`. Pi's `.pi/extensions/` directory provides native hook support. Captures `SessionStart` and five default tool-result events (read, ast_grep_search, lsp_navigation, edit, write). Bash is debug-gated via `SCRYRS_DEBUG`.
- **Claude Code** — implemented at `hooks/claude-code/scryrs-hook.mjs`. Claude Code's PreToolUse hook system provides tool-execution interception. Captures eight default PreToolUse events (Read, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, WebFetch). Bash is debug-gated via `SCRYRS_DEBUG`. PreToolUse-only; no lifecycle events.

### Tier 2: Plugin

**What it is:** A harness-specific plugin or extension that registers hooks through the harness's plugin API rather than through a generic subprocess hook system.

**Limitations:**

- Requires plugin authentication and development per harness.
- Event coverage depends on the specific plugin API's capabilities — some plugin APIs may not expose all tool events.
- Each harness requires its own plugin implementation and maintenance.

**Harness coverage:** TBD — dependent on harness plugin API availability. Pi and Claude Code are targeted at the full-hook tier first.

### Tier 3: Rules-File Fallback

**What it is:** Manual event-rule authoring by the user. The user writes or configures rules (e.g., prompt instructions) that cause the agent to emit trace events at specific points. No automatic interception of tool events is possible.

**Explicit limitations:**

- **Cannot guarantee automatic session demarcation.** Session boundaries depend on the user manually inserting SessionStart/SessionEnd events.
- **Requires manual rule authoring** by the user — no automatic event generation.
- **Event coverage is inherently partial.** Only events the user's rules explicitly request are captured.
- **Cannot intercept tool events without harness cooperation.** The rules-file approach relies on the agent *voluntarily* emitting events, not on the harness intercepting tool execution.

This tier exists as a lowest-common-denominator fallback for harnesses with no hook or plugin support. It is not a replacement for a proper hook.

## Install and Setup

Hook installation is automated via `scryrs init --agent <name>`. Run the installer from the target project directory:

```bash
scryrs init --agent claude-code  # install Claude Code hook
scryrs init --agent pi           # install Pi hook
```

The installer writes hook files to project-local directories, refuses to overwrite existing files, and provides deterministic next-step instructions on success.

### Manual setup (alternative)

1. **Install hook files:** Copy the reference hook source into the harness-specific location (see Reference Hooks below).
2. **Ensure scryrs is on `$PATH`** — the hook subprocess must be able to invoke `scryrs record`.
3. **Configure harness hook** — create or edit the harness's hook configuration to invoke scryrs after tool execution.
4. **Create `scryrs.json`** at the repository root (optional, recommended).
5. **Verify fail-open behavior** — confirm that scryrs failures do not block tool execution.

## Reference Hooks

Reference hook implementations for Pi and Claude Code live under `hooks/`.

- **Pi hook** — exists at `hooks/pi/`. A Pi extension (`index.ts`) that observes tool_result events for read, bash, ast_grep_search, lsp_navigation, edit, and write tools and forwards canonical TraceEvent JSONL to `scryrs record --stdin`. Session demarcation is automatic via session_start event. See `hooks/pi/README.md` for details.
- **Claude Code hook** — exists at [`hooks/claude-code/`](https://github.com/scryrs-project/scryrs/blob/main/hooks/claude-code/scryrs-hook.mjs). A PreToolUse JavaScript hook that intercepts Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, and WebFetch events and forwards canonical TraceEvent JSONL to `scryrs record --stdin`. See `hooks/claude-code/README.md` for installation and usage instructions.

### Claude Code Hook Limitations

The Claude Code hook is a **PreToolUse-only** hook. This creates specific limitations that integrators must understand:

- **Unconditional Success outcome:** PreToolUse hooks fire *before* tool execution. The real outcome (success or failure) cannot be determined. Every emitted event carries `outcome: Success` unconditionally. These are pre-execution metadata signals, not post-execution outcomes.
- **No session lifecycle events:** PreToolUse hooks have no session-open or session-close trigger. No `SessionStart` or `SessionEnd` lifecycle events are emitted. Only subject-bearing tool events are produced.
- **Per-process session IDs:** The hook generates a UUID v4 session ID once per hook process lifetime. If Claude Code exposes a session-scoped identifier via environment variables, the hook prefers that; otherwise it generates its own.

### Claude Code Hook Fail-Open Warning Channel

The Claude Code hook writes fail-open warnings to a dedicated log file outside agent context:

- **Log file:** `.scryrs/hooks/claude-code-warnings.log` (resolved to an absolute path from the hook process working directory).
- **Format:** ISO-8601 timestamp followed by a human-readable reason.
- **Warnings are never written to stdout or stderr** — the agent-visible tool output is unchanged.
- **Scenarios that produce warnings:** `scryrs` binary not found on PATH, `scryrs record` exits non-zero, subprocess spawn error, stdin write failure.

No reference hooks for other harnesses are planned at this time. Harness authors targeting other platforms should follow the integration-tier matrix above and use the Pi/Claude Code hooks as reference patterns.

## Related Pages

- [CLI v0 Contract](./cli-v0-contract.md) — deterministic output and exit-code contract for `scryrs record` and `scryrs hotspots`.
- [Product Roadmap](./roadmap.mdx) — delivery sequence including Phase 1 proxy capture and reference hook work.
- [Architecture](./architecture.mdx) — crate topology and runtime flow.
