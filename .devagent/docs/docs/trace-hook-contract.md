# Trace Hook Contract

**Status:** Canonical — this is the single source of truth for harness integration with scryrs.

This document defines what harness integrators must capture, how to invoke `scryrs record`, what boundaries scryrs must never cross, and which integration path fits their harness. Harness authors and agent-platform builders should read this before writing any scryrs hook.

## Purpose and Boundaries

scryrs is a **trace-collection CLI**. It ingests structured JSONL trace events from agent coding sessions and persists them for downstream hotspot analysis, graph building, and knowledge proposals. It is never a tool executor, proxy, MCP server, or agent-callable business tool.

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

`scryrs.json` lives at the repository root, alongside `Cargo.toml`, `package.json`, or equivalent project root marker.

### Provisional shape (v0.1)

The manifest schema is **provisional v0.1**. Field names, file location, and schema may change before Phase 1 stabilization. No checked-in `scryrs.json` file is created by this document.

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

**Planned harness coverage:**

- **Pi** — planned. Pi's `.pi/hooks/` directory provides native subprocess hook support.
- **Claude Code** — planned. Claude Code's hook system provides tool-execution interception.

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

Hook installation is currently a **manual process** pending the `scryrs init --agent` installer (planned for Phase 1 of the [Product Roadmap](./roadmap.md)).

### Manual setup steps (current state)

1. **Ensure scryrs is on `$PATH`** — the hook subprocess must be able to invoke `scryrs record`.
2. **Configure harness hook** — create or edit the harness's hook configuration to invoke scryrs after tool execution:
   - **Full hook tier:** Configure the harness's native hook system (e.g., Pi `.pi/hooks/` scripts, Claude Code hook configuration).
   - **Plugin tier:** Install and configure the harness-specific scryrs plugin.
   - **Rules-file fallback:** Add agent instruction rules describing trace-event emission.
3. **Create `scryrs.json`** at the repository root (optional, recommended). Describes which event families the hook captures and how `scryrs record` is invoked.
4. **Verify fail-open behavior** — confirm that scryrs failures do not block tool execution.

Once `scryrs init --agent <name>` is implemented, these steps will be automated to a single command.

## Reference Hooks

Reference hook implementations for Pi and Claude Code are **forthcoming Phase 1 deliverables**. They do not exist in the repository yet.

- **Pi hook** — will live under `hooks/pi/` and leverage Pi's `.pi/hooks/` subprocess hook system. Marked as forthcoming in the [Product Roadmap](./roadmap.md) Phase 1.
- **Claude Code hook** — will live under `hooks/claude-code/` and leverage Claude Code's hook/interception system. Marked as forthcoming in the [Product Roadmap](./roadmap.md) Phase 1.

No reference hooks for other harnesses are planned at this time. Harness authors targeting other platforms should follow the integration-tier matrix above and use the Pi/Claude Code hooks as reference patterns once available.

## Related Pages

- [CLI v0 Contract](./cli-v0-contract.md) — deterministic output and exit-code contract for `scryrs record` and `scryrs hotspots`.
- [Product Roadmap](./roadmap.md) — delivery sequence including Phase 1 proxy capture and reference hook work.
- [Architecture](./architecture.md) — crate topology and runtime flow.
