## MODIFIED Requirements

### Requirement: Accepted events are persisted through a versioned local SQLite datastore

The system SHALL preserve the existing local SQLite persistence path when remote ingest mode is not active. The canonical local store SHALL remain `.scryrs/scryrs.db` relative to the current working directory, and accepted events from one local-mode `scryrs record` invocation SHALL still be inserted through one explicit SQLite transaction or equivalent batch boundary. When remote ingest mode is active, `scryrs record` SHALL skip the EventStore open/create/write path entirely and SHALL use one remote batch submission as the accepted-event batch boundary instead of opening `.scryrs/scryrs.db`.

#### Scenario: Local mode keeps current SQLite behavior

- **WHEN** `scryrs record` runs with no resolved remote ingest URL
- **THEN** the system opens or creates `.scryrs/scryrs.db`
- **AND** accepted events are persisted through the existing local SQLite path
- **AND** no remote network submission is attempted

#### Scenario: Remote mode does not open or create the local datastore

- **GIVEN** explicit remote ingest configuration is resolved
- **WHEN** `scryrs record` accepts one or more events
- **THEN** the system does not open, create, or write `.scryrs/scryrs.db`
- **AND** the accepted-event batch boundary is one remote submission

### Requirement: Record output and exit codes are deterministic

The `record` command SHALL preserve the existing local-mode stdout summary shape and `0/1/2` exit-code semantics when remote ingest mode is not active. In remote mode, stdout SHALL emit exactly one JSON summary object with fields `command`, `schemaVersion`, `transport`, `accepted`, `duplicate`, `rejected`, and `failed`, where `transport` is `"remote"`. The remote `rejected` count SHALL include both locally rejected non-empty lines and server-rejected submitted items. Local validation rejections MUST still be emitted deterministically to stderr as one JSON object per rejected non-empty line containing `line`, `field` when available, and `reason`. Remote per-item results with status `idempotent` SHALL increment `duplicate` and SHALL NOT count as failures. Remote per-item rejections SHALL increment `rejected` and SHALL cause exit code `1` after the final summary if `failed` is `0`. Fatal remote submission failures, including timeout, connection failure, non-2xx response, unsupported envelope response, or malformed response body, SHALL write deterministic stderr diagnostics, SHALL NOT emit a success summary, and SHALL exit with code `2`.

#### Scenario: Local mode summary remains unchanged

- **WHEN** `scryrs record` runs with no resolved remote ingest URL
- **THEN** stdout contains the current local summary object with `command`, `schemaVersion`, `accepted`, and `rejected`
- **AND** remote-only fields are not added in local mode

#### Scenario: Remote batch with accepted and duplicate items succeeds

- **GIVEN** remote mode is active
- **AND** the server acknowledges submitted items as a mix of `accepted` and `idempotent`
- **WHEN** the command finishes
- **THEN** stdout contains one remote summary with deterministic `accepted`, `duplicate`, `rejected`, and `failed` counts
- **AND** `failed` is `0`
- **AND** the command exits with code `0`

#### Scenario: Remote batch with rejected items exits one

- **GIVEN** remote mode is active
- **AND** at least one non-empty line is rejected locally or by the server
- **WHEN** the command finishes without a transport failure
- **THEN** stdout contains one remote summary with `rejected` greater than `0`
- **AND** stderr contains deterministic rejection diagnostics for rejected lines or items
- **AND** the command exits with code `1`

#### Scenario: Fatal remote submission failure exits two without fake success

- **GIVEN** remote mode is active
- **WHEN** the server times out, cannot be reached, returns a non-2xx response, or returns a malformed body
- **THEN** stderr contains a deterministic fatal diagnostic for the remote failure
- **AND** stdout does not contain a success summary for that failed submission
- **AND** the command exits with code `2`

### Requirement: Discovery surfaces describe the record endpoint accurately

The CLI discovery surfaces SHALL document both the unchanged local default and the explicit remote mode. `scryrs --help`, `scryrs --help-json`, `README.md`, and the CLI contract note MUST describe the remote configuration sources and precedence, the default `3000` ms timeout and its override, the remote summary counts (`accepted`, `duplicate`, `rejected`, `failed`), the loud transport-failure behavior, and that remote mode skips `.scryrs/scryrs.db` rather than dual-writing to it.

#### Scenario: Help surfaces describe explicit remote mode

- **WHEN** a reader checks `scryrs --help` or `scryrs --help-json`
- **THEN** they can see that local mode is the default when remote ingest is not configured
- **AND** they can see which configuration sources activate remote mode
- **AND** they can see the remote timeout and summary-count contract

#### Scenario: Docs describe no dual-write or local fallback in remote mode

- **WHEN** a reader reviews the README or CLI contract note for remote ingest behavior
- **THEN** the docs state that remote mode skips local SQLite writes
- **AND** the docs do not describe a retry spool or silent local fallback on remote failure

### Requirement: Record remains ingestion-only

The `record` endpoint SHALL remain limited to validation, transport, local persistence, summary output, and diagnostics. This change MUST NOT add offline retry spooling, background resend, dual-write local-plus-remote persistence, hotspot scoring, promotion logic, graph building, routing, LLM calls, or harness-side HTTP logic.

#### Scenario: Remote failure does not enqueue retry or fall back locally

- **WHEN** remote submission fails
- **THEN** the command reports the failure loudly
- **AND** it does not queue the batch for later resend
- **AND** it does not silently write the same events into `.scryrs/scryrs.db`

## ADDED Requirements

### Requirement: Remote ingest mode is explicit and configuration-driven

The `record` command SHALL resolve transport mode before reading input or opening the local store. Remote mode SHALL activate only when a non-empty ingest URL is supplied through explicit configuration. Configuration values SHALL be resolved from the nearest ancestor `scryrs.json` `remote` section, overridden by environment variables `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, and `SCRYRS_REMOTE_TIMEOUT_MS`. If no ingest URL resolves, local mode SHALL remain active regardless of any other remote fields. In remote mode, `workspace_id` and `agent_id` are required explicit values; `repository_id` SHALL resolve from explicit configuration or the normalized Git remote-origin contract defined by `live-hotspot-server-contract`; any unresolved required remote identity SHALL fail before any network call with exit code `2`. `timeout_ms` SHALL default to `3000` when not configured.

#### Scenario: No ingest URL keeps local mode active

- **WHEN** `scryrs record` runs without a configured remote ingest URL
- **THEN** local mode remains active
- **AND** other remote config fields do not trigger network behavior by themselves

#### Scenario: Environment overrides manifest remote defaults

- **GIVEN** `scryrs.json` provides a `remote` section
- **AND** one or more `SCRYRS_REMOTE_*` variables are also set
- **WHEN** `scryrs record` resolves remote configuration
- **THEN** the environment values override the manifest defaults for the same fields

#### Scenario: Invoking from a subdirectory finds the nearest ancestor manifest

- **GIVEN** a repository contains `scryrs.json` in an ancestor directory of the current working directory
- **WHEN** `scryrs record` runs from that subdirectory
- **THEN** the command discovers the nearest ancestor `scryrs.json`
- **AND** uses its `remote` section as the file-based configuration source

#### Scenario: Missing remote identity fails before submission

- **GIVEN** a remote ingest URL is configured
- **AND** one or more required remote identity fields cannot be resolved
- **WHEN** `scryrs record` starts
- **THEN** the command reports a deterministic configuration error to stderr
- **AND** exits with code `2`
- **AND** does not attempt a network call

### Requirement: Remote mode validates locally and submits accepted events as one server batch

When remote mode is active, `scryrs record` SHALL continue to parse newline-delimited `TraceEvent` JSON locally using the existing validation semantics, skip blank lines, emit deterministic rejection diagnostics for malformed or schema-invalid lines, and submit only the accepted events to `POST /v1/trace-events/batch` in one `ServerIngestEnvelope` with `envelope_version` `"1.0.0"`. Each submitted `EnvelopeEvent.producer_event_id` SHALL be derived deterministically as the SHA-256 hex digest of the canonical serialized accepted `TraceEvent`, followed by `:`, followed by the 1-based physical line number. Remote mode SHALL not mutate the inner `TraceEvent` schema.

#### Scenario: Mixed valid and invalid lines submit only accepted events

- **GIVEN** remote mode is active
- **AND** the input contains accepted events and rejected non-empty lines
- **WHEN** `scryrs record` processes the input
- **THEN** only the accepted events are included in the remote `ServerIngestEnvelope`
- **AND** rejected lines are reported locally with deterministic diagnostics

#### Scenario: Replaying identical JSONL yields the same producer event IDs

- **GIVEN** the same valid JSONL input is submitted twice in remote mode
- **WHEN** the command derives `producer_event_id` for accepted events
- **THEN** each accepted event receives the same deterministic `producer_event_id` on both runs
- **AND** duplicate replay remains idempotent at the server contract boundary
