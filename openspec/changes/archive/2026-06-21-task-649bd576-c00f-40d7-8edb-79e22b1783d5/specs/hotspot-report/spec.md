# hotspot-report Specification

## MODIFIED Requirements

### Requirement: Ranking is deterministic with explicit tie-break

The system SHALL sort `HotspotEntry` results deterministically using a six-key tie-break chain. Given the same `trace_events` rows, repeated analysis SHALL produce identical ordering. The final `firstEventId` tie-break SHALL use the SQLite row id of the chronologically first contributing event for that subject, using the same `timestamp ASC, id ASC` evidence order exposed in `evidence.rowIds`.

#### Scenario: Final tie-break honors chronological evidence order when row ids are non-monotonic

- **GIVEN** subject A and subject B are identical on `score`, `sessionCount`, `lastSeen`, `subjectKind`, and `subject`
- **AND** subject A's contributing events appear in evidence order as row ids `[5, 3]` because row `5` has the earlier timestamp
- **AND** subject B's contributing events appear in evidence order as row ids `[4, 6]`
- **WHEN** entries are sorted
- **THEN** subject B appears before subject A because `4 < 5`
- **AND** the comparison uses the first row id in evidence order, not the minimum row id in each subject's evidence set

### Requirement: Artifact file is written to .scryrs/hotspots.json

On successful analysis, the system SHALL write the same `HotspotsReport` JSON to `.scryrs/hotspots.json` at the repository root, in addition to stdout. If the report cannot be written to the artifact path, the command SHALL fail instead of reporting success.

#### Scenario: Artifact write failure fails the command

- **GIVEN** a valid `.scryrs/scryrs.db` whose hotspot analysis would otherwise succeed
- **AND** `<PATH>/.scryrs/hotspots.json` cannot be created or overwritten because of a filesystem I/O error
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** the system exits 1
- **AND** an error message is written to stderr describing the artifact write failure
- **AND** the command does not report success

## ADDED Requirements

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
