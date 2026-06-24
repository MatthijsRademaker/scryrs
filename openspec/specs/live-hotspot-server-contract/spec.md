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

The system SHALL define a `GET /v1/repositories/{repository_id}/hotspots` endpoint that returns live hotspot state from the server. The response SHALL use a `LiveHotspotsResponse` envelope that is separate from the local-only `HotspotsReport` — it SHALL NOT carry `repositoryPath` or `storePath` fields that are meaningful only in local filesystem contexts.

#### Scenario: Live query returns current hotspot rankings

- **GIVEN** the server has ingested events for `repository_id = "repo-a"`
- **WHEN** `GET /v1/repositories/repo-a/hotspots` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `LiveHotspotsResponse`
- **AND** `LiveHotspotsResponse.schemaVersion` is the live hotspot schema version
- **AND** `LiveHotspotsResponse.repositoryId` is `"repo-a"`
- **AND** `LiveHotspotsResponse.entries` contains ranked `HotspotEntry` items computed from server state
- **AND** `LiveHotspotsResponse.generatedAt` is the server time when the response was computed
- **AND** `LiveHotspotsResponse.cursor` is an opaque cursor for pagination or stream resumption

#### Scenario: Live query response does not contain local filesystem fields

- **GIVEN** the local `HotspotsReport` schema includes `repositoryPath` and `storePath` as absolute filesystem paths
- **WHEN** `LiveHotspotsResponse` is serialized
- **THEN** the response does NOT contain `repositoryPath`
- **AND** the response does NOT contain `storePath`

#### Scenario: Live query uses same scoring as local hotspots

- **GIVEN** the same set of subject-bearing events ingested through the server
- **WHEN** `LiveHotspotsResponse` is compared to a local `HotspotsReport`
- **THEN** hotspot scores, rankings, and `HotspotEntry` fields are computed using the same deterministic weight table and tie-break chain
- **AND** only the envelope fields differ (no local-path metadata in live mode)

#### Scenario: Unknown repository returns empty entries

- **GIVEN** no events have been ingested for `repository_id = "unknown-repo"`
- **WHEN** `GET /v1/repositories/unknown-repo/hotspots` is called
- **THEN** the response status is `200 OK`
- **AND** `LiveHotspotsResponse.entries` is an empty array
- **AND** all other envelope fields are present

### Requirement: SSE signal stream delivers hotspot delta events

The system SHALL define a `GET /v1/repositories/{repository_id}/signals` endpoint for one-way streaming of hotspot delta events. The transport SHALL be Server-Sent Events (SSE) with media type `text/event-stream`. The exact signal payload schema is deferred to implementation tasks; this contract specifies only the transport and event framing.

#### Scenario: SSE endpoint returns text/event-stream

- **GIVEN** a client connects to `GET /v1/repositories/repo-a/signals`
- **WHEN** the server responds
- **THEN** the `Content-Type` header is `text/event-stream`
- **AND** the connection is a long-lived HTTP response

#### Scenario: SSE events carry id and data fields

- **GIVEN** a connected SSE client
- **WHEN** a hotspot delta event occurs
- **THEN** the SSE message includes an `id:` field carrying the event cursor or sequence number
- **AND** the SSE message includes a `data:` field carrying a JSON payload
- **AND** the message is terminated by a blank line per SSE protocol

#### Scenario: Signal payload schema is not defined in this contract

- **WHEN** this contract is consulted for signal event semantics
- **THEN** the document states that the signal payload schema (what `data:` contains) is deferred to a follow-up implementation task
- **AND** no specific `HotspotSignal` type or field definitions are included in this contract

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

This change SHALL define the contract, add types in `scryrs-types`, write one new OpenSpec spec, add serialization tests, and update the trace-hook-contract documentation. It SHALL NOT add server runtime code, CLI transport behavior, dashboard streaming, or any mutation of existing crate behavior.

#### Scenario: No server implementation

- **WHEN** this change is implemented
- **THEN** no HTTP server, request handler, or networking code is added to any crate
- **AND** no `scryrs-server` or equivalent crate is created

#### Scenario: No CLI transport changes

- **WHEN** this change is implemented
- **THEN** `scryrs record` does not gain a remote transport flag or mode
- **AND** `scryrs-cli/src/record.rs` is unchanged
- **AND** `scryrs-core/src/store.rs` is unchanged

#### Scenario: No dashboard changes

- **WHEN** this change is implemented
- **THEN** `scryrs-dashboard` is unchanged
- **AND** no live streaming or mutation UI behavior is added

#### Scenario: Changes are limited to contract artifacts

- **WHEN** this change is implemented
- **THEN** the only modified crate is `scryrs-types` (new structs, new tests)
- **AND** the only new documentation is the OpenSpec spec and trace-hook-contract Remote Mode appendix
- **AND** no hook source files are modified
