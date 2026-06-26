# live-hotspot-server-contract Specification

## Purpose

Defines the versioned server ingest and identity contract for scryrs Phase 4 (Live Hotspot Server and Signals). Specifies the `ServerIngestEnvelope` batch wrapper around existing `TraceEvent` payloads, three HTTP endpoint shapes (`POST /v1/trace-events/batch` for ingest, `GET /v1/repositories/{repository_id}/hotspots` for live query, `GET /v1/repositories/{repository_id}/signals` for SSE signal stream), deduplication on composite key `(repository_id, workspace_id, agent_id, producer_event_id)` with first-writer-wins semantics, clock-domain separation (`client_timestamp` vs server `received_at`), and source-of-truth rules separating local-only mode from remote-live mode. All types are additive to `crates/scryrs-types`; the inner `TraceEvent` schema and existing local-only pipeline are unchanged.
## Requirements
### Requirement: ServerIngestEnvelope wraps TraceEvent with stable identity fields

The system SHALL define a `ServerIngestEnvelope` struct in `crates/scryrs-types` as a versioned JSON batch wrapper. The envelope SHALL carry submission-context identity fields at the top level and an array of per-event items, each pairing identity and timing metadata with an inner `TraceEvent`. The inner `TraceEvent` schema SHALL NOT be modified.

#### Scenario: Envelope carries all required identity fields

- **GIVEN** an agent submits a batch of trace events to the server ingest API
- **WHEN** the `ServerIngestEnvelope` is serialized
- **THEN** the JSON includes `envelope_version` (a semantic version string starting at `"1.0.0"`)
- **AND** the JSON includes `repository_id` (a stable repository identity string)
- **AND** the JSON includes `workspace_id` (a logical hook-installation scope identifier string)
- **AND** the JSON includes `agent_id` (a string identifying the harness or agent type, e.g. `"pi"`, `"claude-code"`)
- **AND** the JSON includes `events` as an array of `EnvelopeEvent` items
- **AND** no inner `TraceEvent` fields are added, removed, or renamed

#### Scenario: EnvelopeEvent pairs identity and timing with inner TraceEvent

- **GIVEN** an `EnvelopeEvent` item within the `events` array
- **WHEN** the item is serialized
- **THEN** it includes `producer_event_id` (a string unique within the producer scope)
- **AND** it includes `client_timestamp` (an RFC 3339 timestamp from the producer's wall clock at submission)
- **AND** it includes `event` (the inner `TraceEvent` unchanged)

#### Scenario: Envelope version is independent of inner schema version

- **GIVEN** `SCHEMA_VERSION` is `"0.1.0"` (governing inner `TraceEvent` wire format)
- **WHEN** a `ServerIngestEnvelope` is serialized
- **THEN** `envelope_version` is `"1.0.0"`
- **AND** `envelope_version` is independent of and may differ from the inner event's `schema_version` field

#### Scenario: Serialization round-trips through JSON

- **GIVEN** an example `ServerIngestEnvelope` with one valid `EnvelopeEvent`
- **WHEN** it is serialized to JSON and deserialized back
- **THEN** the reconstructed envelope equals the original
- **AND** the inner `TraceEvent` is unchanged through the round-trip

### Requirement: Repository identity is stable and container-independent

The `repository_id` field SHALL be a stable, container-independent repository identity. It SHALL NOT use absolute filesystem paths, which vary across container instances and cloned workspaces.

#### Scenario: Repository ID is derived from Git remote origin URL

- **GIVEN** a repository with a Git remote origin URL
- **WHEN** `repository_id` is derived
- **THEN** the derivation normalizes the URL: lowercased, trailing-slash-stripped, protocol-agnostic
- **AND** two clones of the same repository on different machines produce the same `repository_id`

#### Scenario: Repository without Git remote requires explicit configuration

- **GIVEN** a repository with no Git remote origin
- **WHEN** remote mode is activated
- **THEN** the producer MUST supply an explicit `repository_id` via `scryrs.json` configuration or an environment variable
- **AND** omitting `repository_id` SHALL produce a validation error before submission

#### Scenario: repository_id is not an absolute filesystem path

- **GIVEN** the existing `HotspotsReport.repositoryPath` carries an absolute filesystem path
- **WHEN** `repository_id` is defined for the remote contract
- **THEN** it is a stable identifier derived from repository metadata
- **AND** it does not leak container-local path information
- **AND** it does not change when the same repository is checked out in a different directory

### Requirement: Deduplication prevents double-counting of producer events

The server SHALL deduplicate events using a composite key of `(repository_id, workspace_id, agent_id, producer_event_id)`. The first accepted submission for a given key SHALL create the event record; subsequent submissions with the same key SHALL be acknowledged as idempotent and SHALL NOT increment hotspot scores, event counts, or create duplicate rows.

#### Scenario: First submission is accepted

- **GIVEN** no prior event exists for key `(repo-a, ws-1, pi, evt-001)`
- **WHEN** a batch containing that event is submitted
- **THEN** the server creates an event record
- **AND** the response `EventAck.status` is `"accepted"`
- **AND** the response `EventAck.received_at` records the server receipt time

#### Scenario: Duplicate submission is idempotent

- **GIVEN** an event for key `(repo-a, ws-1, pi, evt-001)` was already accepted
- **WHEN** a batch containing the same key is submitted again
- **THEN** the server does not create a new event record
- **AND** the response `EventAck.status` is `"idempotent"`
- **AND** the response `EventAck.received_at` returns the original event's receipt time
- **AND** hotspot scores do not double-count the duplicate

#### Scenario: Different agent_id with same producer_event_id are separate events

- **GIVEN** an event exists for key `(repo-a, ws-1, pi, evt-001)`
- **WHEN** an event with key `(repo-a, ws-1, claude-code, evt-001)` is submitted
- **THEN** the two events are treated as distinct
- **AND** each is scored independently

#### Scenario: Producer_event_id uniqueness is scoped, not global

- **GIVEN** two different agents in different repositories each use `producer_event_id = "evt-001"`
- **WHEN** both events are submitted
- **THEN** they are treated as distinct events (different `repository_id` or `agent_id`)
- **AND** no collision occurs despite sharing the same `producer_event_id`

### Requirement: Clock domains separate producer timing from server ordering

Each `EnvelopeEvent` SHALL carry a `client_timestamp` representing the producer's wall clock at submission time. The inner `TraceEvent.timestamp` SHALL NOT be reinterpreted. The server SHALL independently stamp `received_at` on each accepted event using its own clock. `received_at` SHALL be the authoritative field for server-side ordering and audit; `client_timestamp` SHALL be preserved for client-side correlation but SHALL NOT be used for server-side ordering decisions.

#### Scenario: client_timestamp is preserved alongside TraceEvent.timestamp

- **GIVEN** a producer records a `TraceEvent` with `timestamp = "2026-06-24T10:00:00Z"`
- **AND** the producer submits the event in a batch at wall clock `"2026-06-24T10:00:05Z"`
- **WHEN** the `EnvelopeEvent` is serialized
- **THEN** `client_timestamp` is `"2026-06-24T10:00:05Z"`
- **AND** the inner `event.timestamp` remains `"2026-06-24T10:00:00Z"`

#### Scenario: received_at is server-stamped independently

- **GIVEN** a batch is received by the server at server time `"2026-06-24T10:00:07Z"`
- **WHEN** the server processes the batch
- **THEN** each accepted `EventAck.received_at` is `"2026-06-24T10:00:07Z"`
- **AND** this time reflects the server's clock, not the producer's `client_timestamp`

#### Scenario: received_at is authoritative for ordering

- **GIVEN** events with different `client_timestamp` values that diverge from server receipt order
- **WHEN** downstream consumers order events or compute audit trails
- **THEN** `received_at` is the authoritative ordering field
- **AND** `client_timestamp` is not used for ordering

#### Scenario: client_timestamp far outside skew window is flagged but accepted

- **GIVEN** a producer with a system clock set 24 hours in the future
- **WHEN** the event is submitted with a far-future `client_timestamp`
- **THEN** the event SHALL be accepted normally
- **AND** the server SHOULD flag the timestamp anomaly in server logs
- **AND** downstream consumers MUST NOT rely on `client_timestamp` as an ordering guarantee

### Requirement: Batch ingest endpoint accepts and acknowledges events

The system SHALL define a `POST /v1/trace-events/batch` endpoint for batch event ingestion. The endpoint SHALL accept a JSON `ServerIngestEnvelope` body and return a JSON `BatchIngestResponse` with per-event acknowledgment status.

#### Scenario: Successful batch ingest returns acknowledgment

- **GIVEN** a valid `ServerIngestEnvelope` with three events
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `BatchIngestResponse`
- **AND** `BatchIngestResponse.received_count` equals `3`
- **AND** `BatchIngestResponse.events` contains three `EventAck` entries
- **AND** `BatchIngestResponse.received_at` is the server receipt timestamp

#### Scenario: Envelope version mismatch returns error

- **GIVEN** a `ServerIngestEnvelope` with `envelope_version` the server does not support
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the response body describes the unsupported version

#### Scenario: Invalid inner TraceEvent within envelope is rejected per-event

- **GIVEN** a `ServerIngestEnvelope` containing one valid `TraceEvent` and one `TraceEvent` failing validation
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the valid event is accepted with status `"accepted"`
- **AND** the invalid event is rejected with an error reason in its `EventAck`
- **AND** the response `received_count` reflects only accepted events (excluding idempotent)
- **AND** the response reflects the rejection in a `rejected` array or equivalent per-event error field

#### Scenario: Empty events array is accepted

- **GIVEN** a valid `ServerIngestEnvelope` with `events: []`
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response status is `200 OK`
- **AND** `BatchIngestResponse.received_count` is `0`

### Requirement: Live hotspot query returns server-authoritative state

The system SHALL define a read-only `GET /v1/repositories/{repository_id}/hotspots` endpoint that returns live hotspot state from server-owned state rather than artifact files. The response SHALL use a JSON `LiveHotspotsResponse` envelope and SHALL support `window` plus optional `session_id` query parameters. For this task, `window` SHALL accept only `"cumulative"`; any other value SHALL return `400 Bad Request` with a descriptive error. When `session_id` is omitted, the server SHALL materialize cumulative rankings from repository live state. When `session_id` is provided, the server SHALL recompute rankings from matching `server_trace_events` rows using the existing deterministic hotspot scoring path and SHALL NOT derive session-scoped results by filtering aggregate accumulator rows. The response SHALL return all ranked entries for the requested scope and SHALL keep `cursor` as an opaque response field for future use.

#### Scenario: Unfiltered cumulative query returns current hotspot rankings

- **GIVEN** the server has ingested subject-bearing events for `repository_id = "repo-a"`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `LiveHotspotsResponse`
- **AND** `LiveHotspotsResponse.schemaVersion` is the live hotspot schema version
- **AND** `LiveHotspotsResponse.repositoryId` is `"repo-a"`
- **AND** `LiveHotspotsResponse.generatedAt` is the server time when the response was computed
- **AND** `LiveHotspotsResponse.entries` contains ranked `HotspotEntry` items with evidence row IDs from server state

#### Scenario: Unsupported window is rejected clearly

- **GIVEN** the live hotspot API supports only the cumulative window in this foundation
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=recent` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the response body describes that only `window=cumulative` is supported
- **AND** the server does not guess or default the request to another window

#### Scenario: Session-scoped query recomputes rankings from matching events only

- **GIVEN** the server has accepted events for repository `repo-a` across sessions `s1` and `s2`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative&session_id=s1` is called
- **THEN** the response status is `200 OK`
- **AND** the ranked entries reflect only events from session `s1`
- **AND** the ranking uses the existing deterministic hotspot scoring semantics
- **AND** the response is not produced by filtering repository-level accumulator totals by session membership

#### Scenario: Unfiltered live query uses the same deterministic scoring semantics as local hotspots

- **GIVEN** the same accepted server event set that would produce a local hotspot report for `repo-a`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative` is compared to that deterministic scoring result
- **THEN** scores, rankings, counts, session counts, first seen, last seen, and evidence ordering match for the overlapping cumulative semantics
- **AND** only the live response envelope differs from the local artifact envelope

#### Scenario: Unknown repository returns an empty live envelope

- **GIVEN** no events have been ingested for `repository_id = "unknown-repo"`
- **WHEN** `GET /v1/repositories/unknown-repo/hotspots?window=cumulative` is called
- **THEN** the response status is `200 OK`
- **AND** `LiveHotspotsResponse.entries` is an empty array
- **AND** all other envelope fields are present

#### Scenario: Live query response does not contain local filesystem fields

- **GIVEN** the local `HotspotsReport` schema includes `repositoryPath` and `storePath`
- **WHEN** `LiveHotspotsResponse` is serialized for the live query API
- **THEN** the response does NOT contain `repositoryPath`
- **AND** the response does NOT contain `storePath`

### Requirement: SSE signal stream delivers hotspot delta events

The system SHALL define a read-only `GET /v1/repositories/{repository_id}/signals` endpoint for one-way hotspot signal streaming. The transport SHALL be Server-Sent Events (SSE) with media type `text/event-stream`. Each SSE message SHALL use the persisted `hotspot_signals.id` as the SSE `id` and SHALL serialize a `HotspotSignal` JSON payload in the `data` field. The endpoint SHALL support an optional `after` query parameter interpreted as a signal-row cursor. On connection, the server SHALL replay persisted signals for the repository with `id > after` ordered by `id ASC`, then tail newly committed repository signals in the same deterministic order without requiring artifact polling. `after=0` SHALL replay all persisted signals for the repository.

#### Scenario: SSE endpoint returns text/event-stream with HotspotSignal payloads

- **GIVEN** a client connects to `GET /v1/repositories/repo-a/signals`
- **WHEN** the server responds
- **THEN** the `Content-Type` header is `text/event-stream`
- **AND** the connection is a long-lived HTTP response
- **AND** each delivered SSE message includes an `id:` field carrying the signal row id
- **AND** each delivered SSE message includes a `data:` field carrying serialized `HotspotSignal` JSON

#### Scenario: Cursor replay returns only signals after the supplied id

- **GIVEN** repository `repo-a` has persisted signals with ids `5`, `6`, and `7`
- **WHEN** a client connects to `GET /v1/repositories/repo-a/signals?after=5`
- **THEN** the replay stream starts with signals `6` and `7`
- **AND** signal `5` is not replayed again
- **AND** the replay order is `id ASC`

#### Scenario: after=0 replays the full persisted signal history for a repository

- **GIVEN** repository `repo-a` has one or more persisted hotspot signals
- **WHEN** a client connects to `GET /v1/repositories/repo-a/signals?after=0`
- **THEN** the server replays all persisted signals for `repo-a`
- **AND** the replay order is `hotspot_signals.id ASC`

#### Scenario: Newly committed repository signals are streamed in deterministic order

- **GIVEN** a client is already connected to `GET /v1/repositories/repo-a/signals`
- **WHEN** new `HotspotSignal` records are committed for `repo-a`
- **THEN** the client receives those new signals without polling artifact files
- **AND** delivery order follows persisted `hotspot_signals.id ASC`

### Requirement: Remote mode is exclusive and explicitly configured

When remote ingest is configured, the CLI SHALL use server state as the authoritative source for hotspot queries and SHALL skip local `.scryrs/scryrs.db` storage entirely. Remote mode SHALL be activated only by explicit configuration; it SHALL NOT be activated by implicit detection or environmental heuristics. The `.scryrs/hotspots.json` artifact file SHALL remain available as an export/cache artifact for local workflows but SHALL NOT be the source of truth for live hotspot queries when remote mode is active.

#### Scenario: Remote mode skips local store

- **GIVEN** remote ingest is explicitly configured
- **WHEN** `scryrs record` receives trace events
- **THEN** events are submitted to the remote server
- **AND** events are NOT written to `.scryrs/scryrs.db`
- **AND** no local SQLite store is opened or created

#### Scenario: Remote mode activated by explicit configuration only

- **GIVEN** a `scryrs.json` with a `remote` section specifying `ingest_url`
- **WHEN** `scryrs record` is invoked
- **THEN** remote mode is active
- **AND** the remote transport is used for ingestion

#### Scenario: Local-only is the default

- **GIVEN** no remote configuration is present
- **WHEN** `scryrs record` is invoked
- **THEN** local-only mode is active
- **AND** events are persisted to `.scryrs/scryrs.db`
- **AND** no HTTP calls are made

#### Scenario: No silent mixing of local and remote stores

- **GIVEN** remote mode is configured
- **AND** a local `.scryrs/scryrs.db` exists from prior local-only sessions
- **WHEN** `scryrs hotspots` or equivalent query is invoked
- **THEN** hotspot state is read from the server
- **AND** the local `.scryrs/scryrs.db` is not consulted or merged

#### Scenario: Artifact export remains available in remote mode

- **GIVEN** remote mode is active
- **WHEN** a user or process explicitly requests a hotspot export
- **THEN** `.scryrs/hotspots.json` can be written as a cache/export of server state
- **AND** the artifact file is not the live source of truth

### Requirement: New types are additive and do not modify existing schemas

All new types defined by this contract SHALL be additive to `crates/scryrs-types` and SHALL NOT modify the existing `TraceEvent`, `HotspotsReport`, `HotspotEntry`, or any other existing struct. Existing specs (`trace-event-schema`, `trace-hook-contract`, `scryrs-record-endpoint`, `hotspot-report`) SHALL NOT be modified by this change.

#### Scenario: TraceEvent is unchanged

- **WHEN** this change is implemented
- **THEN** `TraceEvent` in `scryrs-types/src/lib.rs` retains all existing fields unchanged
- **AND** `TraceEvent::validate()` accepts the same events as before
- **AND** existing serde round-trip tests continue to pass

#### Scenario: HotspotsReport is unchanged

- **WHEN** this change is implemented
- **THEN** `HotspotsReport` in `scryrs-types/src/lib.rs` retains `repositoryPath`, `storePath`, and all other fields unchanged
- **AND** `.scryrs/hotspots.json` artifact semantics are preserved for local-only mode

#### Scenario: No existing spec is modified

- **WHEN** this change is implemented
- **THEN** `openspec/specs/trace-event-schema/spec.md` is unchanged
- **AND** `openspec/specs/trace-hook-contract/spec.md` is unchanged
- **AND** `openspec/specs/scryrs-record-endpoint/spec.md` is unchanged
- **AND** `openspec/specs/hotspot-report/spec.md` is unchanged

### Requirement: Scope is limited to contract definition and types

This change SHALL implement only the read-only live hotspot query and signal streaming behavior on top of the existing ingest and accumulator foundations. It SHALL preserve `.scryrs/hotspots.json` as an explicit export/cache path and SHALL NOT use that artifact as the live query source of truth. It SHALL NOT add websocket transport, graph, proposal, route, or runtime retrieval APIs, dashboard mutation behavior, or local/remote merge behavior.

#### Scenario: Artifact export remains available but separate from live query state

- **GIVEN** a user explicitly runs or requests hotspot export
- **WHEN** `.scryrs/hotspots.json` is produced from local or server-owned state
- **THEN** the artifact remains available as a portable report file
- **AND** `GET /v1/repositories/{repository_id}/hotspots` does not read that artifact as its source of truth

#### Scenario: Existing ingest behavior remains the only write surface

- **WHEN** this change is implemented
- **THEN** `POST /v1/trace-events/batch` remains the only server write API in scope
- **AND** any internal signal notification added for SSE does not change the public ingest contract

#### Scenario: No non-hotspot retrieval or mutation APIs are added

- **WHEN** this change is implemented
- **THEN** no graph, proposal, route, or runtime retrieval API is added
- **AND** no websocket transport is added
- **AND** no dashboard mutation behavior is added

