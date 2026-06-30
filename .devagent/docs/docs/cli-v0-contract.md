# CLI Reference

scryrs CLI helps teams observe agent coding activity, detect knowledge hotspots, inspect them locally, and optionally centralize ingest for live monitoring. It solves the problem of invisible, repeated agent effort: when AI coding assistants spend session after session rediscovering the same files, searching the same terms, and failing the same lookups, scryrs surfaces those patterns so you can document, investigate, or fix the underlying issues.

scryrs supports three current workflow paths:

- **Central live-ingest flow (default):** `scryrs init`, `scryrs record`, and `scryrs dashboard` default to live mode. Configure a remote ingest URL and identity — preferably in a gitignored `.scryrs/.env` (also resolvable from `SCRYRS_REMOTE_*` env vars, CLI flags, or `scryrs.json` `remote`) — then `scryrs record` or `scryrs hook` submits events to a `scryrs server` running centrally. The server provides live hotspot rankings and an SSE signal stream for real-time monitoring across multiple agent instances.
- **Local observe → detect loop (`--mode local`):** Run `scryrs init --mode local`, then `scryrs hook` / `scryrs record --mode local` to capture trace events locally, `scryrs hotspots` to score them, and `scryrs dashboard --mode local` to browse ranked hotspots, sessions, and events — all from `.scryrs/` in your project root.
- **Promote → review artifact flow:** Run `scryrs graph`, `scryrs route`, and `scryrs propose` to produce machine-readable graph, route, and inbox proposal artifacts, then use `scryrs proposals list|accept|reject` to review those inbox proposals without mutating source-of-truth docs or graph/route artifacts.

For hotspot interpretation and scoring rationale, see [Hotspots](./hotspots.md). For harness integration rules and fail-open guarantees, see [Trace Hook Contract](./trace-hook-contract.md).

---

The current public CLI surface provides eleven implemented root commands: `hotspots`, `record`, `hook`, `init`, `doctor`, `graph`, `route`, `propose`, `proposals`, `dashboard`, and `server`. This document serves agent integrators and follow-up feature developers.

## Binary

**Name:** `scryrs`

## Commands

### `scryrs record --stdin | --file <PATH>`

Ingest JSONL trace events from stdin or a file. `--stdin` and `--file` are mutually exclusive; providing both or neither exits 2.

`record` supports two transport modes: **remote (default)** and local (explicit `--mode local`). The mode is selected before ingestion begins. Remote is the default — when it is selected but required config cannot be resolved, `record` fails fast (exit 2) with guidance rather than silently falling back to local.

#### Local mode (`--mode local`)

Local mode is now an explicit opt-in. It is unchanged in behavior from prior releases.

| Field | Value |
|-------|-------|
| Selection | `--mode local` (explicit) |
| Input | `--stdin` reads newline-delimited `TraceEvent` JSON from stdin; `--file <PATH>` reads from a JSONL file |
| Output | Single-line JSON summary on stdout; one JSON rejection diagnostic per rejected non-empty line on stderr |
| Exit 0 | All processed non-empty lines were accepted |
| Exit 1 | Ingestion completed but one or more non-empty lines were rejected; or I/O error writing output |
| Exit 2 | Fatal usage error (invalid mode, unreadable file, store failure) |

**Local stdout summary envelope:**

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

#### Remote mode (default)

Remote mode is the default. It resolves an ingest URL from the precedence chain below; when none resolves (and `--mode local` was not passed), `record` exits 2 with guidance rather than running locally.

**Config precedence (highest to lowest):**

1. CLI flags (where applicable; `init`/`dashboard` accept `--ingest-url`/`--server-url`, etc.)
2. Environment variables (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, `SCRYRS_REMOTE_TIMEOUT_MS`)
3. `.scryrs/.env` dotenv file (nearest ancestor)
4. `scryrs.json` `remote` section (nearest ancestor)
5. Git remote origin URL (`repository_id` fallback only)

**Required identity fields for remote mode:** `repository_id`, `workspace_id`, `agent_id`. Missing any of these when an ingest URL is configured exits 2 with a diagnostic. Missing the ingest URL under the default also exits 2 with remediation guidance (populate `.scryrs/.env`, or use `--mode local`).

**Remote semantics:**

- **No dual-write, no local fallback.** Remote mode skips `.scryrs/scryrs.db` entirely — events are submitted to the server and are NOT written locally.
- **No retry spool.** Failed submissions return diagnostics but are not queued for retry.
- **Default timeout:** 3000 ms, overridable via `SCRYRS_REMOTE_TIMEOUT_MS`.

**Remote stdout summary envelope:**

```json
{"command":"record","schemaVersion":"0.1.0","transport":"remote","accepted":5,"duplicate":2,"rejected":1,"failed":0}
```

- `transport` is always `"remote"` when remote mode is active.
- `accepted`: count of events the server accepted (first-writer-wins).
- `duplicate`: count of idempotent (previously stored) events — non-fatal, exit 0 still applies when all non-duplicate events are accepted.
- `rejected`: count of events rejected (local validation failures).
- `failed`: count of server-rejected items (server responded with rejections).

**Remote exit codes:**

- 0: All events accepted (duplicates non-fatal).
- 1: One or more rejections or I/O error.
- 2: Missing remote identity, transport timeout, connection failure, non-2xx response, or malformed response.

For the server-side ingest API, see the [server command section](#scryrs-server---bind-addr---port-port---store-path) below.

### `scryrs hook <HARNESS> [--stdin | --file <PATH>]`

Translate a harness's native tool event into a canonical `TraceEvent` and persist it (locally or remotely). This is the harness-facing integration entry point.

**Fail-open contract:** `scryrs hook` **always exits 0 with empty stdout** and never blocks the harness. Any error — malformed input, unknown harness, translation failure, store error, remote submission failure — produces a timestamped warning line in `.scryrs/hooks/<harness>-warnings.log` (relative to the event's `cwd`) and exits 0. This is the inverse of `record`'s 1/2 exit policy.

**Supported harnesses:**

| Harness | Transport | Input | Details |
|---------|-----------|-------|---------|
| `claude-code` | Native command hook (no JavaScript) | stdin (`PreToolUse` event JSON) | Claude Code spawns `scryrs hook claude-code` as a subprocess and pipes the event on stdin |
| `pi` | Thin in-process extension delegating to CLI | `--file <PATH>` (raw event JSON) | Pi loads a transport shim module; the shim forwards events to `scryrs hook pi --file <tmp>` |

Transport asymmetry is intentional: Claude Code uses stdin because its `command` hooks spawn subprocesses with piped input; Pi uses `--file` because its `exec()` opens stdin as `/dev/null`.

**Warning log channel:** Errors are appended to `.scryrs/hooks/<harness>-warnings.log` (timestamped ISO 8601 lines). Warnings are never written to stdout or stderr — the agent-visible tool output is unchanged.

**Input modes (mutually exclusive, but mismatch is non-fatal — surfaces as a warning):**

| Mode | Flag | Description |
|------|------|-------------|
| stdin (default) | `--stdin` | Read the harness event from stdin |
| file | `--file <PATH>` | Read the harness event from PATH |

**Exit codes:**

| Code | Meaning |
|------|---------|
| 0 | Always — fail-open, never blocks the harness |

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

### `scryrs graph <PATH>`

Builds deterministic `KnowledgeGraphDocument` artifact from hotspot evidence and optional docs navigation metadata.

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` containing `.scryrs/hotspots.json`. Optional docs layer from `.devagent/docs/docs/_nav.json`. |
| Output | Single-line `KnowledgeGraphDocument` JSON on stdout; `.scryrs/graph.json` artifact file written to `<PATH>/.scryrs/` |
| Exit 0 | Graph written successfully |
| Exit 1 | Validation, serialization, stdout write, or artifact write failure |
| Exit 2 | PATH argument omitted, path cannot be resolved, hotspots artifact missing, or hotspots artifact malformed |

**Behavior notes:**

- Hotspot nodes are built from all five subject kinds.
- Docs hierarchy is optional. If docs directory or `_nav.json` is missing, empty, or malformed, the command warns on stderr and emits hotspot-only graph.
- Output is deterministic: top-level fields are `schemaVersion`, `metadata`, `nodes`, and `edges`.

### `scryrs route <PATH>`

Projects `.scryrs/graph.json` into deterministic `RouteManifestDocument` artifact for downstream runtime retrieval. Also accepts `scryrs route explain <PATH> --query <TEXT>` subcommand (see below).

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` containing `.scryrs/graph.json` |
| Output | Single-line `RouteManifestDocument` JSON on stdout; `.scryrs/routes.json` artifact file written to `<PATH>/.scryrs/` |
| Exit 0 | Route manifest written successfully |
| Exit 1 | Serialization, stdout write, or artifact write failure |
| Exit 2 | PATH argument omitted, path cannot be resolved, graph artifact missing, graph artifact malformed, or graph schema mismatch |

**Behavior notes:**

- Every graph node becomes exactly one route entry.
- `grouping` is present only when explicit `contains` edge from parent group node exists.
- Output is deterministic: routes sort by node `id` ascending and carry no wall-clock generation fields.

#### Route Hint Contract

The route manifest is accompanied by a deterministic `RouteHintDocument` projection (schema version `HINT_SCHEMA_VERSION = 1.0.0`, independent from `ROUTE_SCHEMA_VERSION`). Each `RouteEntry` produces exactly one `RouteHintItem`:

| Field | Type | Source | Description |
|-------|------|--------|-------------|
| `schemaVersion` | string | `HINT_SCHEMA_VERSION` | Always `"1.0.0"` |
| `hints` | array | `RouteEntry[]` input | One `RouteHintItem` per route entry, deterministic order |
| `hints[].routeId` | string | `RouteEntry.id` | Source route entry id |
| `hints[].target` | string | `RouteEntry.target` | Normalized load target |
| `hints[].label` | string | `RouteEntry.label` | Human-readable label |
| `hints[].rank` | number (u32) | Ordinal index (1-based) | Deterministic placeholder — NOT a frozen long-term ranking formula |
| `hints[].relevance` | number or absent | Always `None` | Deferred — use `scryrs route explain` for query-based filtering |
| `hints[].reason` | string | Template: `"Route '{label}' ({id}): {N} evidence link(s), subject kind {subjectKind}"` | Evidence count and identity explanation |
| `hints[].evidence` | array | `RouteEntry.evidenceLinks` (verbatim copy) | Provenance links for traceability |

**Example JSON:**

```json
{
  "schemaVersion": "1.0.0",
  "hints": [
    {
      "routeId": "file:src/main.rs",
      "target": "file:src/main.rs",
      "label": "src/main.rs",
      "rank": 1,
      "reason": "Route 'src/main.rs' (file:src/main.rs): 2 evidence link(s), subject kind file",
      "evidence": [
        {
          "sourceKind": "local_trace_row",
          "subject": "src/main.rs",
          "rowIds": [1, 2]
        }
      ]
    },
    {
      "routeId": "search:routing",
      "target": "search:routing",
      "label": "routing",
      "rank": 2,
      "reason": "Route 'routing' (search:routing): 3 evidence link(s), subject kind search",
      "evidence": [
        {
          "sourceKind": "local_trace_row",
          "subject": "routing",
          "rowIds": [3, 5, 7]
        }
      ]
    }
  ]
}
```

**Ranking policy:** `rank` is a deterministic ordinal placeholder derived from manifest entry sort order (by `id` ascending). `relevance` is deferred (`None`) and explicitly does NOT represent a frozen long-term ranking formula. Use `scryrs route explain <PATH> --query <TEXT>` for query-based filtering and ranking (see below).

### `scryrs route explain <PATH> --query <TEXT>`

Queries the route manifest artifact (`.scryrs/routes.json`) for entries matching a search term. Performs case-insensitive substring matching with tiered ordering — no model, no randomness, no graph inspection.

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` containing `.scryrs/routes.json`. Required `--query <TEXT>` argument. |
| Output | Single-line `RouteHintDocument` JSON on stdout. Each hint's `reason` appends `"; query match on <fields>"` suffix. Zero matches produces valid document with empty `hints` array. |
| Exit 0 | Query matched and results emitted (including zero-match results) |
| Exit 1 | Serialization or stdout write failure |
| Exit 2 | Missing PATH, missing `--query`, route artifact missing, malformed route artifact, or route schema version mismatch |

**Match fields** (case-insensitive substring match against each):

| Field | Example match for query "auth" |
|-------|-------------------------------|
| `label` | `"Authentication"` → match (substring) |
| `subject` | `"auth_handler"` → match (substring) |
| `id` | `"file:auth"` → match (substring) |
| `target` | `"file:auth"` → match (substring) |
| `kind` | `"doc_page"` → no match |
| `evidence_links[].subject` | `"auth_handler"` → match (substring) |

**Match tiers (descending priority):**

| Tier | Description |
|------|-------------|
| 3 (exact) | Field equals query string exactly (case-insensitive) |
| 2 (prefix) | Field starts with query string (case-insensitive) |
| 1 (substring) | Field contains query string (case-insensitive) |

**Tie-break:** Within each tier, entries follow manifest entry order (by `id` ascending).

**Example invocation:**

```bash
scryrs route explain . --query "authentication"
```

**Example output (successful match):**

```json
{"schemaVersion":"1.0.0","hints":[{"routeId":"file:auth","target":"file:auth","label":"auth","rank":1,"reason":"Route 'auth' (file:auth): 1 evidence link(s), subject kind file; query match on id, label, subject, target","evidence":[{"sourceKind":"local_trace_row","subject":"auth","rowIds":[1]}]}]}
```

**Example output (zero matches):**

```json
{"schemaVersion":"1.0.0","hints":[]}
```

**Artifact dependency:** Reads only `.scryrs/routes.json`. Does not inspect `.scryrs/graph.json`, proposals, or any other artifact directory. The explain command is a read-only operation — it never creates, modifies, or deletes filesystem artifacts.

### `scryrs propose <PATH>`

Generates validated review-only `ProposalDocument` inbox artifacts from hotspot and graph evidence.

| Field | Value |
|-------|-------|
| Input | Required local directory `<PATH>` containing `.scryrs/hotspots.json` and `.scryrs/graph.json` |
| Output | Plain-text count on stdout; individual proposal JSON files under `<PATH>/.scryrs/proposals/` |
| Exit 0 | Proposals generated and written successfully |
| Exit 1 | Generated proposal failed validation; proposals directory could not be created; serialization or file write failure |
| Exit 2 | PATH argument omitted, path cannot be resolved, hotspots artifact missing/malformed, or graph artifact missing/malformed |

**Behavior notes:**

- Writes are confined to `.scryrs/proposals/`.
- Each proposal is validated before persistence.
- Proposal generation does not mutate docs source, `.scryrs/graph.json`, or `.scryrs/routes.json`.
- Singular `propose` is generation-only; plural `proposals` is the review surface.

### `scryrs proposals list|accept|reject`

Reviews inbox proposal artifacts without mutating the inbox file itself.

| Subcommand | Input | Output | Exit 0 | Exit 1 | Exit 2 |
|-------|-------|-------|-------|-------|-------|
| `proposals list <PATH> [--state pending|accepted|rejected|all]` | Repository path containing `.scryrs/proposals/` and optional `.scryrs/accepted/` / `.scryrs/rejected/` | Deterministic JSON array of proposal rows sorted by `proposalId` ascending | Rows emitted successfully | Serialization failure writing stdout | Invalid filter, invalid proposal/review artifact, conflicting accepted+rejected state, or unreadable/malformed input artifact |
| `proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>` | Valid proposal inbox file plus explicit review metadata | No stdout; writes `.scryrs/accepted/{proposalId}.json` | Accepted artifact written, or idempotent byte-identical rerun | Filesystem write or serialization failure | Unknown proposal ID, invalid proposal document, invalid metadata, conflicting opposite-outcome artifact, or same-outcome overwrite with different bytes |
| `proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>` | Valid proposal inbox file plus explicit review metadata | No stdout; writes `.scryrs/rejected/{proposalId}.json` | Rejected artifact written, or idempotent byte-identical rerun | Filesystem write or serialization failure | Unknown proposal ID, invalid proposal document, invalid metadata, conflicting opposite-outcome artifact, or same-outcome overwrite with different bytes |

**Behavior notes:**

- `list` validates every encountered `ProposalDocument` and `ProposalReviewDecision` before emitting output.
- `accept` copies proposal `targetType`, `proposedContent`, and `evidence` into the accepted `ProposalReviewDecision`.
- `reject` copies only proposal `evidence`; rejected decisions omit `targetType` and `acceptedContent`.
- Review commands never mutate `.scryrs/proposals/{proposalId}.json`, `.devagent/docs/`, `.scryrs/graph.json`, or `.scryrs/routes.json`.
- `--help-json` represents `proposals` as a grouped command with nested `list`, `accept`, and `reject` subcommands. `surfaceVersion` is now `0.11.0`.

### `scryrs init --agent <NAME>`

Install the scryrs trace hook for a supported agent harness into the current working directory.

| Field | Value |
|-------|-------|
| Input | Required `--agent <NAME>` argument. Supported harnesses: `claude-code`, `pi`. |
| Output | Deterministic next-step instructions on stdout (plain text). Error diagnostics on stderr. |
| Exit 0 | Hook installed successfully |
| Exit 1 | I/O error (cannot create directory or write file) |
| Exit 2 | Usage error (unsupported harness, target file collision, self-install refusal, missing or empty `--agent`) |

**Installation targets:**

- `claude-code`: Create-or-merges `.claude/settings.json` (relative to CWD) with the native command hook `{"type":"command","command":"scryrs hook claude-code"}` under `PreToolUse`. No hook file is written.
- `pi`: Writes `.pi/extensions/pi-trace/index.ts` (the transport shim) relative to CWD.

**Runtime store scaffolding:** Before installing the hook, `init` eagerly scaffolds the `.scryrs/` runtime directory (relative to the resolved target base): a schema-initialized `.scryrs/scryrs.db` and a `.scryrs/.gitignore` that excludes runtime trace data from version control. This makes setup visible immediately and lets `scryrs hotspots` / `scryrs dashboard` succeed (returning an empty report) before any events are recorded; the hook still creates the store lazily as a fallback. Scaffolding is idempotent — an existing store is opened, never clobbered, and an existing `.gitignore` is preserved. It runs only after the harness name is validated, so an unsupported harness leaves the filesystem untouched.

**Collision behavior:** For `claude-code`, the installer merges into an existing `.claude/settings.json` — preserving unrelated keys and existing hooks, idempotent on re-run (the hook appears exactly once). For `pi`, if the target file already exists the installer exits 2 with remediation instructions rather than overwriting.

**Self-install guard:** The installer refuses to run inside the scryrs source repository (detected via dual-marker heuristic).

### `scryrs doctor [--json]`

Diagnose the current installation and readiness state for the working directory. The command uses the same local-vs-live resolution logic as `scryrs record`, never silently falls back from live to local, and reports findings with three severities: `ok`, `warn`, and `error`.

| Field | Value |
|-------|-------|
| Input | No required arguments. Optional `--json` flag for machine-readable output. |
| Output | Human-readable summary on stdout by default, or versioned JSON on stdout when `--json` is used. |
| Exit 0 | All findings are `ok` or `warn` |
| Exit 1 | Output write or serialization failure |
| Exit 2 | One or more structural `error` findings were reported |

**Minimum diagnostic categories:**

- binary version
- shipped command surface / feature availability
- resolved mode (`local` or `live`)
- local store status
- Claude Code hook status
- Pi hook status
- live server reachability when live mode is configured
- docs links

**Severity policy:**

- `ok` — the checked surface is usable as configured
- `warn` — advisory condition only (for example: no initialized local store yet, or an optional hook is absent)
- `error` — structural problem that makes the diagnosed setup non-operational (for example: malformed `scryrs.json`, unreadable local store, incomplete live identity, or unreachable configured live server)

**JSON output shape:**

```json
{
  "schemaVersion": "1.0.0",
  "command": "doctor",
  "version": "0.1.0",
  "mode": "local",
  "overallStatus": "warn",
  "commandSurface": {
    "commands": ["hotspots", "record", "hook", "init", "doctor", "graph", "route", "propose", "proposals", "dashboard", "server"],
    "features": ["core", "dashboard", "graph", "curator", "markdown", "runtime", "guardrails", "server"]
  },
  "findings": [
    {
      "category": "local_store",
      "status": "warn",
      "summary": "local store is not initialized at /repo/.scryrs/scryrs.db; run `scryrs init --agent <NAME>` or capture events first"
    }
  ],
  "docsLinks": [
    {
      "label": "CLI Reference",
      "path": ".devagent/docs/docs/cli-v0-contract.md"
    }
  ]
}
```

**Behavior notes:**

- Missing or empty local state is advisory in local mode and does not fail the command.
- Live mode with missing `repository_id`, `workspace_id`, or `agent_id` is a structural error and exits 2.
- Live mode probes `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` against the configured ingest server to verify reachability.
- The default human-readable output and `--json` mode cover the same diagnostic categories.

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
      "subjectKind": "file",
      "subject": "src/a.rs",
      "score": 42,
      "counts": {
        "eventType": { "FileOpened": 5, "EditMade": 3 },
        "outcome": { "success": 8 }
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
  "subjectKind": "file",
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

### `scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--server-url <URL> --repository-id <ID>] [--no-open] [--dev]`

Starts the embedded Vue.js dashboard HTTP server in one of two explicit modes:

- **Local mode (default):** reads `.scryrs/hotspots.json` and `.scryrs/scryrs.db` from the current working directory.
- **Live mode:** proxies live hotspot rankings and hotspot-signal SSE from `scryrs server` when both `--server-url` and `--repository-id` are provided.

| Field | Value |
|-------|-------|
| Input | No required arguments. Optional flags: `--port` (default `8080`), `-p <PORT>`, `--bind` (default `127.0.0.1`), `-b <ADDR>`, `--server-url <URL>`, `--repository-id <ID>`, `--no-open` (flag, no value), `--dev` (flag, no value) |
| Output | HTTP server with same-origin REST/SSE proxy endpoints. SPA served at `GET /` and `GET /assets/*`. Non-API, non-asset paths fall through to `index.html` for Vue Router push-state. |
| Exit 0 | Server shut down cleanly (SIGINT/SIGTERM) |
| Exit 1 | Port already in use or server startup I/O failure |
| Exit 2 | Usage error (invalid `--port`, invalid `--bind`, or partial live configuration) |

**Mode activation:** Omitting both live flags keeps local mode. Providing only one of `--server-url` or `--repository-id` exits 2 with a clear configuration error; the dashboard never guesses or silently falls back between local and live data sources.

**Startup behavior:** Prints `Dashboard available at <http://127.0.0.1:8080>` to stderr (adjusting for `--port` and `--bind`). Opens the default browser unless `--no-open` is set. In `--dev` mode, appends `(dev mode)` to the startup message and serves from the filesystem `crates/scryrs-dashboard/frontend/dist/` directory.

**REST/SSE API contract:**

| Endpoint | Mode | Response |
|----------|------|----------|
| `GET /api/meta` | local + live | `200 OK` with `{ "mode": "local"|"live", "repositoryPath": "<absolute path>", "repositoryId": string|null }`. |
| `GET /api/hotspots` | local | `200 OK` with `.scryrs/hotspots.json` content as JSON. `404 Not Found` if no hotspot report exists. |
| `GET /api/hotspots` | live | `200 OK` with the normalized live rankings payload from `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`, preserving upstream `cursor`. `502 Bad Gateway` when the configured server is unreachable or returns a non-success response. |
| `GET /api/signals?after=<id>` | live | `200 OK` with `text/event-stream`, proxied from `GET /v1/repositories/{repository_id}/signals?after=<id>`. The dashboard streams replayed and live events through without buffering the full upstream response. |
| `GET /api/sessions` | local | `200 OK` with JSON array of session objects (`sessionId`, `startedAt`, `endedAt`, `eventCount`, `source`), ordered by `startedAt DESC`, default limit 50. `404 Not Found` if no `.scryrs/scryrs.db`. `502 Bad Gateway` if store is corrupt. |
| `GET /api/sessions/:sessionId` | local | `200 OK` with `{ "session": { ... }, "events": [ ... ] }` — full session detail including all events. `404 Not Found` if session does not exist. `502 Bad Gateway` if store is corrupt. |
| `GET /api/events` | local | `200 OK` with `{ events: [...], nextCursor: string|null }`, cursor-based pagination via`?limit=N&cursor=<token>&session_id=<id>`. |
| `GET /api/sessions`, `GET /api/sessions/:sessionId`, `GET /api/events` | live | `404 Not Found` with an explanation that the route is unavailable in live mode. |

**SPA contract:** The SPA is a Vue 3 application built with Vite, Bun, Tailwind CSS v4, and shadcn-vue, then embedded in the binary via `rust-embed`. Local mode shows Hotspots, Sessions, Events, and About. Live mode shows Hotspots, Signals, and About, hides Sessions/Events from navigation, and renders readable unavailable views on direct navigation to local-only routes. The Signals view owns reconnect behavior in the browser: it starts at `/api/signals?after=0`, tracks the last seen SSE id in memory, reconnects with `?after=<last_seen_id>`, and ignores replay duplicates on resume.

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
| 0 | Hotspots, graph, route, and route-explain artifacts written successfully (route explain includes zero-match results); proposals written successfully; record local accepted all events; record remote accepted all events (duplicates non-fatal); init installed hook; doctor reported only `ok`/`warn` findings; dashboard/server shut down cleanly; hook always fail-open; help/version/surface display. |
| 1 | Hotspots/graph/route stdout or artifact write failure; record rejected one or more events or hit output I/O error; propose validation, directory creation, serialization, or file write failure; init I/O error; doctor output write or serialization failure; dashboard/server startup or runtime I/O failure. |
| 2 | Usage error. Also: hotspots missing/corrupt store; graph missing/malformed hotspots input; route missing/malformed/schema-mismatched graph input; route explain missing PATH, missing --query, or missing/malformed/schema-mismatched routes.json; propose missing/malformed hotspot or graph input; record local fatal I/O error; record remote identity/transport failure; init unsupported harness or collision; doctor structural error findings (malformed config, unreadable store, unusable configured live mode, unreachable live server); dashboard invalid flags, invalid bind address, or partial live-mode configuration; server invalid port/bind/store or feature not compiled. |

All error messages and human-facing diagnostics are written to stderr.

## Agent-facing contract

### Hotspots command

**When to call:** An agent should call `scryrs hotspots <PATH>` when the agent needs scryrs' repository hotspot summary for a given local directory path. The command opens `.scryrs/scryrs.db` at `<PATH>`, runs deterministic scoring over all persisted trace events, and emits a `HotspotsReport`.

**Input:** An explicit local directory path (required). The path must contain `.scryrs/scryrs.db`.

**Output:** A parseable JSON `HotspotsReport` on stdout and `.scryrs/hotspots.json` artifact file. The agent can distinguish outcomes by exit code:

- Exit 0: Report available (may have zero entries if store is empty).
- Exit 1: I/O or storage error (stdout write failure, artifact write failure). May retry.
- Exit 2: Missing PATH, store not found at `<PATH>/.scryrs/scryrs.db`, or corrupt store. Do not retry without fixing input.

### Graph command

**When to call:** An agent should call `scryrs graph <PATH>` when hotspot evidence already exists and the next step is to materialize deterministic graph structure for routing or proposal generation.

**Input:** Explicit local directory path containing `.scryrs/hotspots.json`. Docs navigation layer from `.devagent/docs/docs/_nav.json` is optional.

**Output:** Parseable JSON `KnowledgeGraphDocument` on stdout and `.scryrs/graph.json` artifact file.

**Exit codes:**

- Exit 0: Graph available.
- Exit 1: Validation, serialization, stdout write, or artifact write failure.
- Exit 2: Missing PATH, unresolved path, missing hotspots artifact, or malformed hotspots artifact.

### Route command

**When to call:** An agent should call `scryrs route <PATH>` when graph artifact exists and runtime-facing route projection is needed.

**Input:** Explicit local directory path containing `.scryrs/graph.json`.

**Output:** Parseable JSON `RouteManifestDocument` on stdout and `.scryrs/routes.json` artifact file.

**Exit codes:**

- Exit 0: Route manifest available.
- Exit 1: Serialization, stdout write, or artifact write failure.
- Exit 2: Missing PATH, unresolved path, missing graph artifact, malformed graph artifact, or graph schema mismatch.

### Route explain command

**When to call:** An agent should call `scryrs route explain <PATH> --query <TEXT>` when a route manifest artifact exists and the agent needs deterministic, query-filtered route recommendations without re-inspecting graph or hotspot evidence.

**Input:** Explicit local directory path containing `.scryrs/routes.json` and a required `--query <TEXT>` argument.

**Output:** Parseable JSON `RouteHintDocument` on stdout. Each matching hint's `reason` appends `"; query match on <fields>"`. Zero matches produces a valid document with empty `hints` array.

**Exit codes:**

- Exit 0: Query matched and results emitted (including zero-match results).
- Exit 1: Serialization or stdout write failure.
- Exit 2: Missing PATH, missing `--query`, missing `.scryrs/routes.json`, malformed route artifact, or route schema version mismatch.

**Match algorithm:** Case-insensitive substring match against `label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. Matches are tiered: exact (tier 3) > prefix (tier 2) > substring (tier 1). Within each tier, manifest entry sort order (by `id` ascending) is the final tie-break. Only entries that match at least one field appear in the output.

### Propose command

**When to call:** An agent should call `scryrs propose <PATH>` when hotspot and graph artifacts already exist and reviewable knowledge proposals are needed.

**Input:** Explicit local directory path containing `.scryrs/hotspots.json` and `.scryrs/graph.json`.

**Output:** Proposal count on stdout and validated `ProposalDocument` JSON files under `.scryrs/proposals/`.

**Exit codes:**

- Exit 0: Proposal inbox updated successfully.
- Exit 1: Validation, serialization, directory creation, or file write failure.
- Exit 2: Missing PATH, unresolved path, or missing/malformed hotspots or graph artifact.

**Naming note:** `propose` generates inbox artifacts. `proposals` reviews them.

### Proposals command

**When to call:** An agent should call `scryrs proposals list <PATH>` to inspect pending versus terminal review state, `scryrs proposals accept <PATH> <ID> ...` to publish an accepted `ProposalReviewDecision`, or `scryrs proposals reject <PATH> <ID> ...` to publish a rejected `ProposalReviewDecision`.

**Input:**

- `list`: explicit local directory path containing `.scryrs/proposals/` and optional `.scryrs/accepted/` / `.scryrs/rejected/`
- `accept` / `reject`: explicit local directory path, proposal ID, and mandatory `--reviewer`, `--rationale`, `--decided-at <RFC3339>` metadata

**Output:**

- `list`: JSON array of rows with `proposalId`, `title`, `targetType`, `createdAt`, and `state`
- `accept`: no stdout; writes `.scryrs/accepted/{proposalId}.json`
- `reject`: no stdout; writes `.scryrs/rejected/{proposalId}.json`

**Exit codes:**

- Exit 0: Listing succeeded or the requested review artifact exists/wrote successfully.
- Exit 1: Serialization or filesystem write failure.
- Exit 2: Invalid filter, invalid proposal/review artifact, unknown proposal ID, conflicting opposite-outcome artifact, or same-outcome rerun with different bytes.

### Doctor command

**When to call:** An agent or maintainer should call `scryrs doctor` when they need one installed-user diagnosis path for the current workspace: what binary is running, which commands/features are shipped, whether the workspace resolves to local or live mode, whether local state is usable, whether harness hooks are installed, whether the configured live server is reachable, and where the relevant operator docs live.

**Input:** No required arguments. Optional `--json` flag for machine-readable output.

**Output:**

- Default: human-readable summary on stdout.
- `--json`: machine-readable report with `schemaVersion`, `command`, `version`, `mode`, `overallStatus`, `commandSurface`, `findings`, and `docsLinks`.

**Exit codes:**

- Exit 0: all findings are `ok` or `warn`.
- Exit 1: serialization or stdout write failure.
- Exit 2: one or more structural `error` findings.

**Structural error examples:** malformed `scryrs.json`, unreadable or unsupported `.scryrs/scryrs.db`, incomplete live-mode identity, or unreachable configured live ingest server.

### Dashboard command

**When to call:** An agent should call `scryrs dashboard` when it needs a same-origin browser surface over either local `.scryrs/` artifacts or live hotspot state from `scryrs server`. The command starts the dashboard HTTP server and opens the SPA in the default browser.

**Input:** No required positional arguments. Optional flags: `--port <PORT>` (default `8080`), `-p <PORT>`, `--bind <ADDR>` (default `127.0.0.1`), `-b <ADDR>`, `--server-url <URL>`, `--repository-id <ID>`, `--no-open` (flag, no value), `--dev` (flag, no value).

**Output:** SPA plus same-origin API endpoints. `GET /api/meta` always reports the active mode; live mode additionally exposes `GET /api/signals?after=<id>` and routes `GET /api/hotspots` through the configured server.

**Exit codes:**

- Exit 0: Server shut down cleanly.
- Exit 1: Port already in use or server startup I/O failure.
- Exit 2: Invalid flags, invalid bind address, or partial live-mode configuration.

### Record command

**When to call:** An agent should call `scryrs record --stdin` to pipe JSONL `TraceEvent` data produced by hooks, or `scryrs record --file <PATH>` to ingest pre-recorded trace files.

**Input modes (mutually exclusive):**

- `--stdin`: Read newline-delimited `TraceEvent` JSON from stdin.
- `--file <PATH>`: Read JSONL from a file.

**Output:**

- Stdout: One JSON summary. Local mode: `{"command":"record","schemaVersion":"...","accepted":N,"rejected":M}`. Remote mode: `{"command":"record","schemaVersion":"...","transport":"remote","accepted":N,"duplicate":N,"rejected":N,"failed":N}`.
- Stderr: One JSON rejection diagnostic per rejected non-empty line (empty on 0 accept + 0 reject or when no rejections occur).

**Exit codes:**

- 0: All processed non-empty lines were accepted (local mode) or all events accepted/duplicate (remote mode).
- 1: One or more rejected lines (ingestion continued) or I/O error.
- 2: Usage error (both/neither mode specified, unknown flags), fatal I/O error (unreadable file, store failure), or remote mode identity/transport failures (missing remote identity, transport timeout, connection failure, non-2xx response, malformed response).

### Hook command

**When to call:** A harness hook should call `scryrs hook <HARNESS>` to translate a native tool event and persist it through the canonical store. This is the harness-facing integration entry point — agents never call `hook` directly.

**Input:**

- Positional `<HARNESS>` argument (required): `claude-code` or `pi`.
- Event data via stdin (default) or `--file <PATH>` (mutually exclusive, but mismatch is non-fatal — surfaces as a warning).

**Output:** Empty stdout. Always. No exceptions.

**Exit codes:**

- Exit 0: Always — fail-open, never blocks the harness. Errors append to `.scryrs/hooks/<harness>-warnings.log`.

**Warning log:** Timestamped error lines at `.scryrs/hooks/<harness>-warnings.log` under the event's `cwd`. Warnings cover: malformed input, unknown harness, translation failure, store errors, remote submission failures.

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

### Server command

**When to call:** An agent or CI pipeline should call `scryrs server` to start a long-lived central trace ingest server. Multiple agent containers can then POST trace event batches to `POST /v1/trace-events/batch` without writing to the same SQLite file directly.

**Input:** No required positional arguments. Optional flags: `--port <PORT>` (default `8081`), `-p <PORT>`, `--bind <ADDR>` (default `127.0.0.1`), `-b <ADDR>`, `--store <PATH>` (default `.scryrs/server.db`).

**Output:** HTTP server with a single POST endpoint and two GET endpoints. Startup message written to stderr (e.g., `scryrs server listening on http://127.0.0.1:8081, store .scryrs/server.db`).

**Exit codes:**

- Exit 0: Server shut down cleanly (SIGINT/SIGTERM).
- Exit 1: Port already in use, bind failure, or server I/O error.
- Exit 2: Invalid flags, port zero, empty store path, or feature not compiled.

**Feature gate:** The server command is available when the `server` Cargo feature is enabled (included in `default` and `full` feature sets). When the feature is absent, invoking `scryrs server` exits 2 with "server feature not enabled."

## Out of scope for v0

Any command other than the eleven implemented root commands (`hotspots`, `record`, `hook`, `init`, `doctor`, `graph`, `route`, `propose`, `proposals`, `dashboard`, `server`) is unrecognized and exits 2 with a usage error.

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
```

## Related Pages

- [Hotspots](./hotspots.md) — domain concept and interpretation guide for hotspot outputs
- [Graph](./graph.md) — graph artifact semantics and evidence boundaries
- [Route Manifests](./route-manifests.md) — route artifact semantics and identity preservation
- [Proposals](./proposals.md) — review-first proposal flow and optional model-assisted curation boundary
- [Live Hotspots](./live-hotspots.md) — domain narrative, mode comparison, and end-to-end live workflow
- [Trace Hook Contract](./trace-hook-contract.md) — how harness hooks capture TraceEvent records for hotspot analysis
- [Architecture](./architecture.mdx) — crate topology including scoring, graph, route, and proposal composition
