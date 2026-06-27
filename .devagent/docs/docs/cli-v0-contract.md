# CLI v0 Contract

The v0 CLI surface for `scryrs` provides five implemented commands: `hotspots`, `record`, `init`, `dashboard`, and `server`. This contract serves agent integrators and follow-up feature developers.

## Binary

**Name:** `scryrs`

## Commands

### `scryrs record --stdin | --file <PATH>`

Ingest JSONL trace events from stdin or a file. `--stdin` and `--file` are mutually exclusive; providing both or neither exits 2.

| Field | Value |
|-------|-------|
| Input | `--stdin` reads newline-delimited `TraceEvent` JSON from stdin; `--file <PATH>` reads from a JSONL file |
| Output | Single-line JSON summary on stdout; one JSON rejection diagnostic per rejected non-empty line on stderr |
| Exit 0 | All processed non-empty lines were accepted |
| Exit 1 | Ingestion completed but one or more non-empty lines were rejected; or I/O error writing output |
| Exit 2 | Fatal usage error (invalid mode, unreadable file, store failure) |

**Stdout summary envelope:**

```json
{"command":"record","schemaVersion":"0.1.0","accepted":5,"rejected":2}
```

- `command` is always `"record"`.
- `schemaVersion` matches `scryrs-types::SCHEMA_VERSION`.
- `accepted` and `rejected` are counts of processed non-empty lines.

**Stderr rejection diagnostics (one per rejected non-empty line):**

```json
{"line":3,"field":null,"reason":"expected value at line 1 column 1"}
```

- `line` is the 1‑based physical line number.
- `field` is `null` when the deserializer cannot determine a failing field, or a quoted field/path string.
- `reason` is a human-readable string describing the rejection.

**Ingestion behavior:** Blank or whitespace-only lines are skipped without incrementing accepted or rejected counts. Malformed JSON and schema-invalid `TraceEvent` lines are rejected with diagnostics, and ingestion continues with later lines.

**Persistence:** Accepted events are persisted to `.scryrs/scryrs.db` (the canonical SQLite trace datastore) in the current working directory. This store is append-only and ingestion-only; no query, delete, or analysis APIs are provided. `.scryrs/events.jsonl` is the ingestion input format and is NOT used as the canonical persistence store.

### `scryrs hotspots <PATH>`

Analyzes persisted trace events in `.scryrs/scryrs.db` and emits a deterministic `HotspotsReport` to stdout (JSON) and `.scryrs/hotspots.json` (artifact file).

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` containing `.scryrs/scryrs.db` |
| Output | `HotspotsReport` JSON on stdout; `.scryrs/hotspots.json` artifact file written to `<PATH>/.scryrs/` |
| Exit 0 | Report written successfully (may have zero entries for empty stores) |
| Exit 1 | I/O or storage error (stdout write failure, artifact write failure) |
| Exit 2 | PATH argument omitted, directory does not contain `.scryrs/scryrs.db`, or corrupt/unreadable store (usage/fatal error on stderr) |

**JSON envelope (HotspotsReport):**

```json
{
  "command": "hotspots",
  "entries": [
    {
      "counts": {
        "eventType": {
          "EditMade": 1,
          "FileOpened": 5
        },
        "outcome": {
          "failure": 1,
          "success": 5
        }
      },
      "evidence": {
        "rowIds": [1, 2, 3, 7, 11, 15]
      },
      "firstSeen": "2026-06-20T12:00:05Z",
      "lastSeen": "2026-06-20T12:04:55Z",
      "rank": 1,
      "score": 10,
      "sessionCount": 3,
      "subject": "src/main.rs",
      "subjectKind": "file"
    }
  ],
  "generatedAt": "2026-06-20T12:05:01Z",
  "repositoryPath": "/absolute/path/to/repo",
  "runMetadata": {
    "analyzedEventCount": 42,
    "analyzedSubjectCount": 12,
    "firstEventId": 1,
    "lastEventId": 42,
    "storeSchemaVersion": 1
  },
  "schemaVersion": "1.0.0",
  "storePath": "/absolute/path/to/repo/.scryrs/scryrs.db"
}
```

- `schemaVersion` is `"1.0.0"` — independent of the `record` envelope `schemaVersion`.
- `command` is always `"hotspots"`.
- `repositoryPath` and `storePath` are absolute paths.
- `runMetadata` carries `storeSchemaVersion` (SQLite user version, integer), `analyzedEventCount` (total subject-bearing events analyzed, integer), `analyzedSubjectCount` (distinct subjects, integer), `firstEventId` (earliest event row ID, integer), and `lastEventId` (latest event row ID, integer). All are `0` when the store is empty.
- `entries` is sorted by score descending with a deterministic six-key tie-break: `score DESC, sessionCount DESC, lastSeen DESC, subjectKind ASC, subject ASC, firstEventId ASC`.
- Each entry's `counts` contains two sub-objects: `eventType` maps each trace event type name to its per-subject occurrence count, and `outcome` maps `success`/`failure` to per-subject outcome counts. Only non-zero counts are included.
- Each entry's `evidence.rowIds` is an ordered list of SQLite row IDs identifying the trace events that contributed to this hotspot entry.
- `subjectKind` uses short category tags: `file`, `search`, `symbol`, `command`, or `document`.

**Scoring dimensions:**

| Event type | Weight |
|------------|--------|
| `FileOpened` | 1 |
| `SearchRun` | 2 |
| `SymbolInspected` | 2 |
| `CommandExecuted` | 1 |
| `DocRetrieved` | 2 |
| `EditMade` | 3 |
| `FailedLookup` | 4 (+2 failure bonus) |

Per-subject score = sum of event weights multiplied by per-type counts. `FailedLookup` events add a fixed `FAILURE_BONUS` of 2 per occurrence in addition to the weight.

### `scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]`

Starts a long-lived HTTP server for central trace event ingest and live hotspot query/streaming. Accepts versioned trace-event batches at `POST /v1/trace-events/batch` with deterministic validation and first-writer-wins idempotency. Also serves read-only live hotspot rankings and a Server-Sent Events signal stream.

| Field | Value |
|-------|-------|
| Input | No required arguments. Optional flags: `--bind` (default `127.0.0.1`), `--port` (default `8081`), `--store` (default `.scryrs/server.db`) |
| Output | HTTP server with three REST endpoints (see table below). Startup message written to stderr. |
| Exit 0 | Server shut down cleanly (SIGINT/SIGTERM) |
| Exit 1 | Server I/O failure (port already in use, bind failure) |
| Exit 2 | Usage error (invalid `--port`, `--bind`, or `--store`) or feature not compiled |

**REST API contract:**

| Endpoint | Method | Request Body / Params | Response |
|----------|--------|-----------------------|----------|
| `/v1/trace-events/batch` | POST | `ServerIngestEnvelope` (JSON) | `200 OK` with `BatchIngestResponse` (JSON) containing `accepted_count`, `duplicate_count`, `rejected_count`, `received_count`, per-item `events` array with status and diagnostics, and `received_at` timestamp. `400 Bad Request` for malformed envelope, unsupported version, or missing identity fields. |
| `/v1/repositories/{repository_id}/hotspots` | GET | `?window=cumulative` (default, only supported value), optional `?session_id=<id>` | `200 OK` with `LiveHotspotsResponse` (JSON) containing `schemaVersion`, `repositoryId`, `cursor`, `generatedAt`, and ranked `entries`. `400 Bad Request` for unsupported `window` values. |
| `/v1/repositories/{repository_id}/signals` | GET | Optional `?after=<signal_id>` (cursor for replay/resume) | `200 OK` with `text/event-stream` (SSE). Each event carries `id: <signal_id>` and `data: <HotspotSignal JSON>`. The stream replays persisted signals with `id > after`, then continues with live signals. Includes a 15-second `keep-alive` heartbeat. |

**ServerIngestEnvelope structure:**

```json
{
  "envelope_version": "1.0.0",
  "repository_id": "<repo-id>",
  "workspace_id": "<workspace-id>",
  "agent_id": "<agent-id>",
  "events": [
    {
      "producer_event_id": "<unique-event-id>",
      "client_timestamp": "<RFC 3339>",
      "event": { "...": "..." }
    }
  ]
}
```

**BatchIngestResponse structure:**

```json
{
  "accepted_count": 3,
  "duplicate_count": 1,
  "rejected_count": 2,
  "received_count": 3,
  "events": [
    {
      "index": 0,
      "producer_event_id": "evt-001",
      "status": "accepted",
      "server_event_id": "srv-42",
      "error_reason": null,
      "received_at": "2026-06-24T10:00:07Z"
    }
  ],
  "received_at": "2026-06-24T10:00:07Z"
}
```

- `accepted_count`: Number of new events accepted (first-writer-wins).
- `duplicate_count`: Number of idempotent (previously stored) events.
- `rejected_count`: Number of invalid events (malformed timestamp, schema-invalid TraceEvent, serialization error).
- `received_count`: `accepted_count` (accepted events only; duplicates are counted in `duplicate_count`).
- Per-item `status` is one of `"accepted"`, `"idempotent"`, or `"rejected"`.
- `server_event_id` is present only for accepted events, formatted as `"srv-{rowid}"`.
- `error_reason` is present only for rejected events.

**Idempotency:** Events are deduplicated by composite key `(repository_id, workspace_id, agent_id, producer_event_id)`. Re-submitting an identical event returns status `"idempotent"` with the original `received_at` timestamp.

**LiveHotspotsResponse structure:**

```json
{
  "schemaVersion": "1.0.0",
  "repositoryId": "repo-a",
  "cursor": "",
  "generatedAt": "2026-06-26T10:00:00Z",
  "entries": [
    {
      "rank": 1,
      "subjectKind": "File",
      "subject": "src/a.rs",
      "score": 42,
      "counts": {
        "eventType": { "FileOpened": 5, "EditMade": 3 },
        "outcome": { "Success": 8 }
      },
      "sessionCount": 3,
      "firstSeen": "2026-06-24T10:00:00Z",
      "lastSeen": "2026-06-26T09:00:00Z",
      "evidence": {
        "rowIds": [1, 2, 3, 5, 7, 8, 10, 12]
      }
    }
  ]
}
```

- `schemaVersion`: Independent of trace event version; currently `"1.0.0"`.
- `entries`: Ranked by six-key tie-break (score DESC, sessionCount DESC, lastSeen DESC, subjectKind ASC, subject ASC, firstEventId ASC). Empty array for repositories with no data.
- `sessionCount`: Number of distinct sessions that touched the subject (cumulative query) or `1` (session-scoped query).
- `evidence.rowIds`: Ordered list of `server_trace_events` row IDs contributing to the accumulator.
- Session-scoped queries (`?session_id=<id>`) recompute rankings from raw events via `score_hotspots` rather than reading accumulators; this ensures session-level correctness at the cost of a full event scan.

**HotspotSignal (SSE data field) structure:**

```json
{
  "repositoryId": "repo-a",
  "subjectKind": "File",
  "subject": "src/a.rs",
  "score": 42,
  "delta": 1,
  "window": "cumulative",
  "threshold": 10,
  "evidenceRowIds": [1, 2, 3],
  "createdAt": "2026-06-26T10:00:00Z"
}
```

- `id` (SSE event id): Monotonically increasing signal row ID, usable as `?after=` cursor value.
- `delta`: Score change since the previous signal for the same subject (minimum 1).
- `threshold`: The signal threshold in effect at creation time.
- The SSE stream is infinite (replay phase then live broadcast). Clients should disconnect when no longer needed; reconnection with `?after=<last_seen_id>` resumes without gaps.

### `scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]`

Starts a local HTTP server and serves an embedded Vue.js SPA dashboard for visual browsing of `.scryrs/` hotspot, session, and event data. Reads `.scryrs/hotspots.json` and `.scryrs/scryrs.db` from the current working directory.

| Field | Value |
|-------|-------|
| Input | No required arguments. Optional flags: `--port` (default `8080`), `--bind` (default `127.0.0.1`), `--no-open` (suppress browser open), `--dev` (serve from filesystem instead of embedded assets) |
| Output | HTTP server with REST API at `GET /api/hotspots`, `GET /api/sessions`, `GET /api/events`. SPA served at `GET /` and `GET /assets/*`. Non-API, non-asset paths fall through to `index.html` for Vue Router push-state. |
| Exit 0 | Server shut down cleanly (SIGINT/SIGTERM) |
| Exit 1 | Port already in use or server startup I/O failure |
| Exit 2 | Usage error (invalid `--port` or `--bind`) |

**Startup behavior:** Prints "Dashboard available at <http://127.0.0.1:8080>" to stderr (adjusting for `--port` and `--bind` flags). Opens the default browser unless `--no-open` is set. In `--dev` mode, appends "(dev mode)" to the startup message and serves from the filesystem `frontend/dist/` directory.

**REST API contract:**

| Endpoint | Method | Response |
|----------|--------|----------|
| `/api/hotspots` | GET | `200 OK` with `.scryrs/hotspots.json` content as JSON. `404 Not Found` if no hotspot report exists. |
| `/api/sessions` | GET | `200 OK` with JSON array of session objects (`sessionId`, `startedAt`, `endedAt`, `eventCount`, `source`), ordered by `startedAt DESC`, default limit 50. `404 Not Found` if no `.scryrs/scryrs.db`. `502 Bad Gateway` if store is corrupt. |
| `/api/events` | GET | `200 OK` with JSON object `{ events: [...], nextCursor: string|null }`, cursor-based pagination via`?limit=N&cursor=<token>`. Each event has`eventId`,`eventType`,`timestamp`,`subjectKind`,`subject`,`payload`.`404 Not Found` if no store. `502 Bad Gateway` if corrupt. |

**SPA contract:** The SPA is a Vue 3 application built with Vite, Bun, Tailwind CSS v4, and shadcn-vue, then embedded in the binary via `rust-embed`. Views: `/` (hotspot table, landing page), `/subjects/:subjectKind/:subject` (subject detail), `/sessions` (session list), `/sessions/:sessionId` (session detail), `/events` (event distribution visualization), `/about` (version info). Unknown routes display a 404 page with a link back to the landing page.

## Global flags

| Flag | Behavior | Exit code |
|------|----------|-----------|
| `-h`, `--help` | Help text to stdout | 0 |
| `-V`, `--version` | Version string to stdout | 0 |
| `-hj`, `--help-json` | Machine-readable CLI surface document to stdout | 0 |

Bare `scryrs` (no arguments) prints help to stdout and exits 0.

### `--help-json` surface document

The `--help-json` flag emits a versioned JSON document describing the complete CLI surface — commands, arguments, flags, output contracts, and exit codes — in a deterministic, machine-readable format.

**When to use:** An agent should call `scryrs --help-json` to discover the CLI surface before invoking commands. The document is idempotent: calling it repeatedly returns identical output.

**Output:** A single JSON object written to stdout. Stderr is empty. Exit code is 0.

**Top-level fields:**

| Field | Type | Description |
|-------|------|-------------|
| `surfaceVersion` | string | Semver version of the surface document format (independent of output envelope `schemaVersion`) |
| `binary` | string | Binary name (`"scryrs"`) |
| `commands` | array | Command entries, each with `name`, `description`, `arguments`, and `output` metadata |
| `globalFlags` | array | Flag entries, each with `name`, `short`, `long`, `description`, and `action` |
| `rootBehavior` | object | Describes bare-invocation behavior (`action`, `exitCode`) |
| `exitCodes` | object | Maps numeric exit codes (`"0"`, `"1"`, `"2"`) to their meanings |

**Command entry structure:** Each entry in `commands` includes:

- `name` (string): Command name
- `description` (string): Human-readable description
- `arguments` (array): Positional arguments, each with `name`, `type`, `required` (boolean), and `description`
- `output` (object): Output contract with `mimeType` and `fields` array (each field: `name`, `type`, `description`, `optional`)

**Versioning policy:** The `surfaceVersion` field follows semver:

- **Major bump:** Breaking changes to the surface document format (field renames, removals, structural changes)
- **Minor bump:** Additive changes (new commands, new flags, new fields)
- **Patch bump:** Clarifications (description text changes, documentation fixes)

Agents should check `surfaceVersion` before parsing to detect format changes. The surface version is independent of the output envelope's `schemaVersion`.

## Exit-code policy

| Code | Meaning |
|------|---------|
| 0 | Hotspots: report written successfully (may have zero entries). Record: all processed non-empty lines were accepted. Dashboard: server shut down cleanly. Server: server shut down cleanly. Help/version/surface display. |
| 1 | Hotspots: I/O error writing stdout or artifact file. Record: one or more events rejected, or I/O error writing output. Dashboard: port in use or server startup I/O failure. Server: port in use, bind failure, or server I/O error. |
| 2 | Hotspots: missing PATH, store not found, corrupt store. Dashboard: invalid flags or bind address. Server: invalid port/bind/store, or feature not compiled. Unknown commands, missing required arguments, invalid arguments, unsupported paths (usage errors). Record: fatal I/O error (unreadable file or store failure). |

All error messages and human-facing diagnostics are written to stderr.

## Agent-facing contract

### Hotspots command

**When to call:** An agent should call `scryrs hotspots <PATH>` when the agent needs scryrs' repository hotspot summary for a given local directory path. The command opens `.scryrs/scryrs.db` at `<PATH>`, runs deterministic scoring over all persisted trace events, and emits a `HotspotsReport`.

**Input:** An explicit local directory path (required). The path must contain `.scryrs/scryrs.db`.

**Output:** A parseable JSON `HotspotsReport` on stdout and `.scryrs/hotspots.json` artifact file. The agent can distinguish outcomes by exit code:

- Exit 0: Report available (may have zero entries if store is empty).
- Exit 1: I/O or storage error (stdout write failure, artifact write failure). May retry.
- Exit 2: Missing PATH, store not found at `<PATH>/.scryrs/scryrs.db`, or corrupt store. Do not retry without fixing input.

### Dashboard command

**When to call:** An agent should call `scryrs dashboard` when it needs to visually browse hotspot, session, and event data from a local `.scryrs/` store. The command starts a local HTTP server and opens the dashboard SPA in the default browser.

**Input:** No required positional arguments. Optional flags: `--port <PORT>` (default `8080`), `-p <PORT>`, `--bind <ADDR>` (default `127.0.0.1`), `-b <ADDR>`, `--no-open` (flag, no value), `--dev` (flag, no value).

**Output:** SPA and REST API served over HTTP. Server lifecycle messages written to stderr.

**Exit codes:**

- Exit 0: Server shut down cleanly.
- Exit 1: Port already in use or server startup I/O failure.
- Exit 2: Invalid flags or bind address.

### Record command

**When to call:** An agent should call `scryrs record --stdin` to pipe JSONL `TraceEvent` data produced by hooks, or `scryrs record --file <PATH>` to ingest pre-recorded trace files.

**Input modes (mutually exclusive):**

- `--stdin`: Read newline-delimited `TraceEvent` JSON from stdin.
- `--file <PATH>`: Read JSONL from a file.

**Output:**

- Stdout: One JSON summary `{"command":"record","schemaVersion":"...","accepted":N,"rejected":M}`.
- Stderr: One JSON rejection diagnostic per rejected non-empty line (empty on 0 accept + 0 reject or when no rejections occur).

**Exit codes:**

- 0: All processed non-empty lines were accepted.
- 1: One or more rejected lines (ingestion continued).
- 2: Usage error (both/neither mode specified, unknown flags) or fatal I/O error (unreadable file, unwritable store).

**Fail-fast paths:** The following always exit 2 and write an error to stderr:

- Any command other than `hotspots`, `dashboard`, `record`, `init`, or `server` (including `components`, `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`). `scryrs dashboard` and `scryrs server` are valid implemented commands.
- `scryrs hotspots` without a PATH argument
- `scryrs record` with neither or both input modes (mutually exclusive)
- `scryrs record --file` with an unreadable path
- `scryrs hotspots <FLAG>` / `scryrs record <FLAG>` — flags after a command fall through to the positional argument parser and are rejected as invalid arguments (no per-command introspection in v0)

### Init command

**When to call:** An agent should call `scryrs init --agent <NAME>` to install the scryrs trace hook for a supported agent harness into the current working directory.

**Input:** A required `--agent <NAME>` argument. Supported harness names: `claude-code`, `pi`.

**Output:**

- Stdout: Deterministic next-step instructions (plain text).
- Stderr: Error diagnostics on failure.

**Exit codes:**

- 0: Hook installed successfully.
- 1: I/O error (cannot create directory or write file).
- 2: Usage error (unsupported harness, target file collision, self-install refusal, missing `--agent`).

**Installation targets:**

- `claude-code`: Create-or-merges `.claude/settings.json` (relative to CWD) with the native command hook `{"type":"command","command":"scryrs hook claude-code"}` under `PreToolUse`. No hook file is written.
- `pi`: Writes `.pi/extensions/pi-trace/index.ts` (the transport shim) relative to CWD.

**Runtime store scaffolding:** Before installing the hook, `init` eagerly scaffolds the `.scryrs/` runtime directory (relative to the resolved target base): a schema-initialized `.scryrs/scryrs.db` and a `.scryrs/.gitignore` that excludes runtime trace data from version control. This makes setup visible immediately and lets `scryrs hotspots` / `scryrs dashboard` succeed (returning an empty report) before any events are recorded; the hook still creates the store lazily as a fallback. Scaffolding is idempotent — an existing store is opened, never clobbered, and an existing `.gitignore` is preserved. It runs only after the harness name is validated, so an unsupported harness leaves the filesystem untouched.

**Collision behavior:** For `claude-code`, the installer merges into an existing `.claude/settings.json` — preserving unrelated keys and existing hooks, idempotent on re-run (the hook appears exactly once). For `pi`, if the target file already exists the installer exits 2 with remediation instructions rather than overwriting.

**Self-install guard:** The installer refuses to run inside the scryrs source repository (detected via dual-marker heuristic).

### Server command

**When to call:** An agent or CI pipeline should call `scryrs server` to start a long-lived central trace ingest server. Multiple agent containers can then POST trace event batches to `POST /v1/trace-events/batch` without writing to the same SQLite file directly.

**Input:** No required positional arguments. Optional flags: `--port <PORT>` (default `8081`), `-p <PORT>`, `--bind <ADDR>` (default `127.0.0.1`), `-b <ADDR>`, `--store <PATH>` (default `.scryrs/server.db`).

**Output:** HTTP server with a single POST endpoint. Startup message written to stderr (e.g., `scryrs server listening on http://127.0.0.1:8081, store .scryrs/server.db`).

**Exit codes:**

- Exit 0: Server shut down cleanly (SIGINT/SIGTERM).
- Exit 1: Port already in use, bind failure, or server I/O error.
- Exit 2: Invalid flags, port zero, empty store path, or feature not compiled.

**Feature gate:** The server command is available when the `server` Cargo feature is enabled (included in `default` and `full` feature sets). When the feature is absent, invoking `scryrs server` exits 2 with "server feature not enabled."

## Out of scope for v0

The following commands are **not** defined in the v0 contract and will exit 2 with a usage error if invoked:

- `components`
- `trace`
- `propose`
- `graph`
- `route`
- `adapters`
- `report`
- `suggest-docs`

These names align with the vision document's future command vocabulary but are not part of the v0 surface.

## Local Development Testing

### Running tests

```bash
cargo test -p scryrs-cli
```

All tests for the `scryrs-cli` crate run through Cargo's built-in test runner. The crate includes:

- **Snapshot tests** (via `insta`) for `--help`, `--help-json`, and `hotspots` output — these verify exact output byte-for-byte against committed `.snap` files. Hotspots integration tests use real SQLite fixtures and full pipeline assertions.
- **Identity tests** — verify `-h` produces identical output to `--help`, `-hj` to `--help-json`, and bare invocation to `--help`.
- **Error-path tests** — verify exit codes and error messages for unknown commands, missing arguments, and extra arguments.
- **Smoke tests** — exercise the public `run()` entrypoint to verify arg-collection wiring from the environment args iterator to the writer-based logic.

### Viewing snapshot diffs

When a snapshot test fails, Cargo prints a diff showing what changed between the expected (`.snap`) and actual output. The diff is human-readable and pinpoints every divergence — whitespace, wording, ordering, field presence.

### Updating snapshots

After an intentional change to the CLI contract (help text, `--help-json` surface document, or `hotspots` JSON envelope), update the committed snapshots. For hotspots, this also means updating the integration test snapshots (`hotspot_integration_tests`) and E2E test:

```bash
# Batch-accept all new or changed snapshots:
cargo insta test --accept -p scryrs-cli

# Or review interactively (requires cargo-insta):
cargo insta review
```

### Installing cargo-insta

`cargo-insta` is optional — tests run and diff output works without it. It is only needed for the snapshot review/accept workflow:

```bash
cargo install cargo-insta

## Related Pages

- [Hotspots](./hotspots.md) — domain concept and interpretation guide (this page covers the CLI output contract and exit codes)
- [Trace Hook Contract](./trace-hook-contract.md) — how harness hooks capture TraceEvent records for hotspot analysis
- [Architecture](./architecture.mdx) — crate topology including scryrs-core scoring and the HotspotsReport data flow
```
