# hotspot-report Specification

## Purpose

Defines the deterministic hotspot scoring contract, output schema, and ranking rules over SQLite trace evidence. Every `scryrs hotspots <PATH>` invocation produces a versioned `HotspotsReport` JSON envelope with ranked `HotspotEntry` results, computed exclusively from persisted `trace_events` rows using a documented integer weight table and a six-key tie-break chain.
## Requirements
### Requirement: Hotspot report envelope is versioned and self-describing

The system SHALL emit a `HotspotsReport` JSON envelope to stdout for every successful `scryrs hotspots <PATH>` invocation. The envelope SHALL include a `schemaVersion` field set to `HOTSPOT_SCHEMA_VERSION` (`"1.0.0"`), independent of `SCHEMA_VERSION` (`"0.1.0"`) which governs trace event wire format.

#### Scenario: Envelope carries all required top-level fields

- **GIVEN** a valid `.scryrs/scryrs.db` with subject-bearing events
- **WHEN** `scryrs hotspots <PATH>` completes successfully
- **THEN** the JSON output contains `schemaVersion` set to `"1.0.0"`
- **AND** the output contains `command` set to `"hotspots"`
- **AND** the output contains `repositoryPath` as the resolved absolute path of `<PATH>`
- **AND** the output contains `storePath` as the resolved absolute path to `.scryrs/scryrs.db`
- **AND** the output contains `runMetadata` with `storeSchemaVersion`, `analyzedEventCount`, `analyzedSubjectCount`, `firstEventId`, and `lastEventId`
- **AND** the output contains `generatedAt` as an ISO 8601 timestamp
- **AND** the output contains `entries` as a JSON array of hotspot entries

#### Scenario: Schema version is independent of trace event version

- **GIVEN** `SCHEMA_VERSION` is `"0.1.0"`
- **WHEN** `HOTSPOT_SCHEMA_VERSION` is defined
- **THEN** `HOTSPOT_SCHEMA_VERSION` is `"1.0.0"`
- **AND** the `schemaVersion` field in the hotspot report uses `HOTSPOT_SCHEMA_VERSION`, not `SCHEMA_VERSION`

#### Scenario: runMetadata is deterministic and reproducible

- **GIVEN** the same `.scryrs/scryrs.db` with unchanged data
- **WHEN** `scryrs hotspots` runs twice
- **THEN** `runMetadata` fields are identical across both runs
- **AND** `storeSchemaVersion` matches the `schema_meta.datastore_schema_version` value in the database
- **AND** `analyzedEventCount` equals the number of subject-bearing events (excluding lifecycle events)
- **AND** `analyzedSubjectCount` equals the number of unique `(subject_kind, subject)` groups
- **AND** `firstEventId` is the minimum SQLite `id` among subject-bearing events
- **AND** `lastEventId` is the maximum SQLite `id` among subject-bearing events

#### Scenario: generatedAt is a wall-clock timestamp

- **GIVEN** a successful hotspot analysis
- **WHEN** `generatedAt` is inspected
- **THEN** it is a valid ISO 8601 timestamp
- **AND** it may differ between two runs against the same database (wall-clock dependency)

### Requirement: Each hotspot entry carries full evidence

Each entry in the `entries` array SHALL include `rank`, `subjectKind`, `subject`, `score`, `counts`, `sessionCount`, `firstSeen`, `lastSeen`, and `evidence` fields. All fields SHALL be derived exclusively from persisted SQLite columns.

#### Scenario: Entry carries rank and identity

- **GIVEN** a ranked hotspot entry for subject `"src/main.rs"` with kind `"file"`
- **WHEN** the consumer inspects the entry
- **THEN** `rank` is a 1-based integer reflecting position in the ordered results
- **AND** `subjectKind` is `"file"`
- **AND** `subject` is `"src/main.rs"`

#### Scenario: Entry carries computed score

- **GIVEN** three `FileOpened` events and one `EditMade` event for `"src/main.rs"`, all with `Outcome::Success`
- **WHEN** the entry is scored
- **THEN** `score` equals `(3 * 1) + (1 * 3) = 6`

#### Scenario: Entry carries per-event-type counts

- **GIVEN** a subject with events of types `FileOpened`, `EditMade`, and `SearchRun`
- **WHEN** the consumer inspects `counts.eventType`
- **THEN** it contains keys for each event type with at least one occurrence for that subject
- **AND** each value is the count of events of that type for that subject
- **AND** event types with zero occurrences for that subject are absent from the map

#### Scenario: Entry carries per-outcome counts

- **GIVEN** a subject with some `Outcome::Success` events and some `Outcome::Failure` events
- **WHEN** the consumer inspects `counts.outcome`
- **THEN** it contains `"success"` and/or `"failure"` keys with their respective counts

#### Scenario: Entry carries session count

- **GIVEN** a subject with events from sessions `"s1"`, `"s1"`, and `"s2"`
- **WHEN** the consumer inspects `sessionCount`
- **THEN** `sessionCount` is `2` (unique session IDs)

#### Scenario: Entry carries time span

- **GIVEN** a subject with events at timestamps `"2026-06-21T09:00:00Z"`, `"2026-06-21T10:00:00Z"`, and `"2026-06-21T12:00:00Z"`
- **WHEN** the consumer inspects `firstSeen` and `lastSeen`
- **THEN** `firstSeen` is `"2026-06-21T09:00:00Z"`
- **AND** `lastSeen` is `"2026-06-21T12:00:00Z"`

#### Scenario: Entry carries evidence references

- **GIVEN** a subject with three contributing events having SQLite row `id` values `5`, `12`, `23`
- **WHEN** the consumer inspects `evidence.rowIds`
- **THEN** `rowIds` is `[5, 12, 23]` ordered by `timestamp ASC, id ASC`
- **AND** each row ID can be joined back to `trace_events.id` in the SQLite store for full event details

### Requirement: Scoring formula is deterministic and based on integer weights

The system SHALL compute a hotspot score for each `(subject_kind, subject)` group using a documented integer weight table applied per event row. The formula SHALL use only persisted SQLite columns and SHALL NOT involve LLM inference, randomization, or wall-clock timing.

#### Scenario: Base weights are applied per event type

- **GIVEN** the weight table:
  - `FileOpened` weight 1
  - `SearchRun` weight 2
  - `SymbolInspected` weight 2
  - `CommandExecuted` weight 1
  - `DocRetrieved` weight 2
  - `EditMade` weight 3
  - `FailedLookup` weight 4
- **WHEN** a subject has one `FileOpened` event and one `SearchRun` event, both `Outcome::Success`
- **THEN** the score is `1 + 2 = 3`

#### Scenario: Failure bonus is additive to base weight

- **GIVEN** the failure bonus is `+2` for each event with `Outcome::Failure`
- **WHEN** a subject has one `EditMade` event with `Outcome::Failure`
- **THEN** the score is `3 + 2 = 5`

#### Scenario: Failure bonus applies to all event types with failure outcome

- **GIVEN** a subject has one `FailedLookup` event (which always carries `Outcome::Failure`)
- **WHEN** the score is computed
- **THEN** the score is `4 + 2 = 6`

#### Scenario: Failure bonus applies to non-FailedLookup failure events

- **GIVEN** a subject has one `CommandExecuted` event with `Outcome::Failure`
- **WHEN** the score is computed
- **THEN** the score is `1 + 2 = 3`

#### Scenario: Lifecycle events are excluded from scoring

- **GIVEN** `trace_events` contains `SessionStart` and `SessionEnd` events
- **WHEN** scores are computed
- **THEN** lifecycle events do not contribute to any subject's score, counts, sessions, or evidence

### Requirement: Ranking is deterministic with explicit tie-break

The system SHALL sort `HotspotEntry` results deterministically using a six-key tie-break chain. Given the same `trace_events` rows, repeated analysis SHALL produce identical ordering. The final `firstEventId` tie-break SHALL use the SQLite row id of the chronologically first contributing event for that subject, using the same `timestamp ASC, id ASC` evidence order exposed in `evidence.rowIds`.

#### Scenario: Final tie-break honors chronological evidence order when row ids are non-monotonic

- **GIVEN** subject A and subject B are identical on `score`, `sessionCount`, `lastSeen`, `subjectKind`, and `subject`
- **AND** subject A's contributing events appear in evidence order as row ids `[5, 3]` because row `5` has the earlier timestamp
- **AND** subject B's contributing events appear in evidence order as row ids `[4, 6]`
- **WHEN** entries are sorted
- **THEN** subject B appears before subject A because `4 < 5`
- **AND** the comparison uses the first row id in evidence order, not the minimum row id in each subject's evidence set

### Requirement: Subjects are grouped by subject_kind and subject

The system SHALL group events by the composite key `(subject_kind, subject)` for scoring and ranking. Two events with the same subject string but different `subject_kind` values SHALL be scored as independent entries.

#### Scenario: Same subject in different kinds produces separate entries

- **GIVEN** a `FileOpened` event with `subject_kind = "file"` and `subject = "routing"`
- **AND** a `SearchRun` event with `subject_kind = "search"` and `subject = "routing"`
- **WHEN** hotspots are scored
- **THEN** two separate entries are produced: one with `subjectKind = "file"` and one with `subjectKind = "search"`

#### Scenario: Same subject kind and subject are grouped together

- **GIVEN** two `FileOpened` events with `subject_kind = "file"` and `subject = "src/main.rs"`
- **AND** one `EditMade` event with `subject_kind = "file"` and `subject = "src/main.rs"`
- **WHEN** hotspots are scored
- **THEN** a single entry is produced with `subjectKind = "file"`, `subject = "src/main.rs"`
- **AND** `counts.eventType` includes both `"FileOpened": 2` and `"EditMade": 1`

### Requirement: No-data success output is explicit

The system SHALL distinguish between a valid store with zero rankable subjects and store-level errors. A valid store with zero subject-bearing events (e.g., only lifecycle events, or empty `trace_events`) SHALL produce exit code 0 with the standard envelope and `entries: []`.

#### Scenario: Empty store produces success with empty entries

- **GIVEN** a valid `.scryrs/scryrs.db` with `trace_events` containing zero rows
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 0
- **AND** the JSON output contains `"entries": []`
- **AND** all other envelope fields (`schemaVersion`, `command`, `repositoryPath`, `storePath`, `runMetadata`, `generatedAt`) are present
- **AND** `runMetadata.analyzedEventCount` is 0
- **AND** `runMetadata.analyzedSubjectCount` is 0

#### Scenario: Store with only lifecycle events produces success with empty entries

- **GIVEN** a valid `.scryrs/scryrs.db` with `trace_events` containing only `SessionStart` and `SessionEnd` events
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 0
- **AND** `entries` is an empty array
- **AND** `runMetadata.analyzedEventCount` is 0 (lifecycle events are not subject-bearing)

### Requirement: Store errors produce distinct exit codes and error messages

The system SHALL handle store-level errors with explicit exit codes and descriptive error messages on stderr. Missing, unsupported, and corrupt stores SHALL produce distinct exit codes.

#### Scenario: Missing store exits 2 with error on stderr

- **GIVEN** no `.scryrs/scryrs.db` exists at `<PATH>/.scryrs/scryrs.db`
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 2
- **AND** an error message is written to stderr indicating the datastore was not found
- **AND** no JSON is written to stdout

#### Scenario: Unsupported store exits 2 with error on stderr

- **GIVEN** `.scryrs/scryrs.db` exists with a schema version other than the expected datastore version
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 2
- **AND** an error message describing the version mismatch is written to stderr

#### Scenario: Storage error exits 1 with error on stderr

- **GIVEN** `.scryrs/scryrs.db` is a corrupt or non-SQLite file
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 1
- **AND** an error message describing the storage error is written to stderr

### Requirement: Artifact file is written to .scryrs/hotspots.json

On successful analysis, the system SHALL write the same `HotspotsReport` JSON to `.scryrs/hotspots.json` at the repository root, in addition to stdout. If the report cannot be written to the artifact path, the command SHALL fail instead of reporting success.

#### Scenario: Artifact write failure fails the command

- **GIVEN** a valid `.scryrs/scryrs.db` whose hotspot analysis would otherwise succeed
- **AND** `<PATH>/.scryrs/hotspots.json` cannot be created or overwritten because of a filesystem I/O error
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 1
- **AND** an error message is written to stderr describing the artifact write failure
- **AND** the command does not report success

### Requirement: CLI --help-json surface describes the new output contract

The `scryrs --help-json` output SHALL describe the hotspot output fields matching the `HotspotsReport` schema, replacing the previous placeholder fields (`schemaVersion`, `command`, `status`).

#### Scenario: --help-json hotspot output fields match the report schema

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the hotspots command output section lists fields: `schemaVersion`, `command`, `repositoryPath`, `storePath`, `runMetadata`, `generatedAt`, `entries`
- **AND** the previous `status` field is removed

#### Scenario: --help-json exit codes are accurate

- **WHEN** `scryrs --help-json` is invoked
- **THEN** exit code 0 describes success with data or empty entries
- **AND** exit code 1 describes I/O or storage errors
- **AND** exit code 2 describes missing store, unsupported store, or usage errors

### Requirement: Hotspot command surfaces no longer describe placeholder output

User-visible hotspot command surfaces SHALL describe real SQLite-derived hotspot analysis and SHALL NOT describe the command or its output as a placeholder.

#### Scenario: CLI help text describes real hotspot reporting

- **WHEN** `scryrs --help` is invoked
- **THEN** the hotspots command summary describes emitting a versioned hotspot report
- **AND** the hotspot help text does not describe the command or stdout output as a placeholder

#### Scenario: README hotspot examples describe recorded-evidence analysis

- **GIVEN** the repository README sections that document `scryrs hotspots`
- **WHEN** a reader inspects those examples
- **THEN** they describe analysis of recorded SQLite trace data and versioned hotspot report output
- **AND** they do not describe placeholder-only behavior

