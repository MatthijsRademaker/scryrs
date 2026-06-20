# trace-event-schema Specification

## Purpose
TBD - created by archiving change task-c1d32950-524f-4c82-8d1e-c98db9075f55. Update Purpose after archive.
## Requirements
### Requirement: Shared harness-agnostic trace event envelope

The system SHALL define a shared trace event contract in `scryrs-types` as a versioned JSON-serializable envelope used by trace producers and consumers across the workspace.

#### Scenario: Hook event can be ingested without harness-specific parsing

- **GIVEN** a harness hook records agent activity
- **WHEN** it serializes a trace event
- **THEN** the event includes `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`
- **AND** downstream consumers can deserialize it without harness-specific fields or parsing rules

### Requirement: Trace event type coverage

The trace event type enum SHALL cover session lifecycle and every vision-listed activity family.

#### Scenario: Supported event types are defined

- **WHEN** a developer inspects the trace event type enum
- **THEN** it includes variants for `SessionStart`, `SessionEnd`, `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, and `FailedLookup`

### Requirement: Session lifecycle events demarcate trace boundaries

Session boundaries SHALL be represented as first-class events in the shared schema.

#### Scenario: Session start is recorded

- **WHEN** a trace session begins
- **THEN** a `SessionStart` event can be serialized with the shared envelope
- **AND** the lifecycle event may omit `tool_name`

#### Scenario: Session end is recorded

- **WHEN** a trace session completes
- **THEN** a `SessionEnd` event can be serialized with the shared envelope
- **AND** downstream aggregation can detect that the session finished

### Requirement: Tool-specific payloads are typed and minimal

Each activity family SHALL use a dedicated payload shape with required fields sufficient for hotspot subject extraction and optional fields only for nonessential details.

#### Scenario: Activity payload families exist

- **WHEN** a developer inspects the trace payload definitions
- **THEN** dedicated payload types exist for file opened, search run, symbol inspected, command executed, doc retrieved, edit made, failed lookup, and session lifecycle events

#### Scenario: Payloads avoid content bodies

- **WHEN** payload fields are defined for commands, docs, or edits
- **THEN** they use identifiers, paths, queries, names, or references
- **AND** they do not require stdout/stderr, document contents, or edit diffs

### Requirement: Outcome is explicit on every event

Every trace event SHALL carry an explicit success or failure outcome.

#### Scenario: Successful event

- **WHEN** an activity completes successfully
- **THEN** the event serializes with outcome `Success`

#### Scenario: Failed event

- **WHEN** an activity fails
- **THEN** the event serializes with outcome `Failure`
- **AND** the failure outcome may carry a generic reason/message without harness-specific metadata

### Requirement: JSON wire format is stable and versioned

The trace contract SHALL round-trip through JSON using a stable self-describing wire format.

#### Scenario: Event round-trip succeeds

- **GIVEN** an example event for any supported event type
- **WHEN** it is serialized to JSON and deserialized back
- **THEN** the reconstructed event equals the original value

#### Scenario: Payload dispatch is self-describing

- **WHEN** an event is serialized
- **THEN** the payload encoding includes explicit type information alongside payload data
- **AND** consumers can identify the concrete payload family from the JSON alone

#### Scenario: Schema version is carried on each event

- **WHEN** an event is serialized
- **THEN** it includes a schema version field
- **AND** the version value remains compatible with the existing `SCHEMA_VERSION` constant

### Requirement: Core hotspot scoring remains deterministic

The shared schema SHALL preserve deterministic hotspot scoring behavior in `scryrs-core`.

#### Scenario: Subject extraction is available for subject-bearing events

- **WHEN** `scryrs-core` scores trace events
- **THEN** it can obtain a hotspot subject from file, search, symbol, command, doc, edit, and failed lookup events without depending on harness-specific payload fields

#### Scenario: Lifecycle events do not create hotspots

- **WHEN** `scryrs-core` receives session lifecycle events
- **THEN** those events do not contribute subjects to hotspot scoring
- **AND** repeated-subject ranking remains deterministic for the remaining events

### Requirement: Scope is limited to the shared schema foundation

This change SHALL not expand beyond the trace schema foundation.

#### Scenario: No record endpoint or CLI expansion is introduced

- **WHEN** the change is implemented
- **THEN** it does not add `scryrs record`, trace storage, aggregation behavior, harness hook integrations, or new public CLI commands

