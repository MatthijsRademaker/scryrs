# hotspot-verification Specification

## Purpose

Defines verification requirements that prove the public `scryrs record → .scryrs/scryrs.db → scryrs hotspots → .scryrs/hotspots.json` pipeline works end-to-end through the canonical CWD-based path, and that hotspot output contract drift is caught by automated snapshot assertions.

## ADDED Requirements

### Requirement: End-to-end pipeline is proven through the public CLI

The system SHALL include an integration test that exercises the complete public pipeline: pipe multi-event-family JSONL through `scryrs record --stdin`, verify SQLite rows in the canonical `.scryrs/scryrs.db`, run `scryrs hotspots <PATH>`, and assert the artifact file is written and matches stdout.

#### Scenario: E2E test uses the canonical store path not store override

- **GIVEN** a temporary repository directory
- **WHEN** the E2E test changes CWD to the temp repo and pipes JSONL through `scryrs record --stdin`
- **THEN** `.scryrs/scryrs.db` is created at the canonical path relative to that CWD
- **AND** the test does NOT call `store_override::set()`
- **AND** the test does NOT call `populate_store()` or open `EventStore` directly

#### Scenario: E2E test verifies SQLite rows after record

- **GIVEN** fixture JSONL has been piped through `scryrs record --stdin` successfully
- **WHEN** the test opens `.scryrs/scryrs.db` via `rusqlite::Connection`
- **THEN** `SELECT COUNT(*) FROM trace_events` matches the expected number of accepted events
- **AND** at least one row exists for each subject-bearing event family in the fixture

#### Scenario: E2E test verifies hotspot output and artifact

- **GIVEN** the canonical `.scryrs/scryrs.db` is populated with fixture events
- **WHEN** the test runs `scryrs hotspots <PATH>` where `<PATH>` is the temp repo directory
- **THEN** exit code is 0
- **AND** `.scryrs/hotspots.json` exists at the temp repo root
- **AND** the artifact file content matches stdout content after removing only stdout's trailing newline
- **AND** the top-ranked entry reflects the subject with the highest computed score

#### Scenario: E2E test covers empty store

- **GIVEN** a temp repo with an initialized-but-empty `.scryrs/scryrs.db`
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** exit code is 0
- **AND** the `entries` array is empty
- **AND** `.scryrs/hotspots.json` is written with an empty `entries` array

#### Scenario: E2E test covers missing store

- **GIVEN** a temp repo without `.scryrs/scryrs.db` or `.scryrs/` directory
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** exit code is 2
- **AND** stderr contains "datastore not found"
- **AND** no JSON is written to stdout

### Requirement: Multi-event-family fixture covers all subject-bearing event types

The E2E fixture and extended inline-assertion test SHALL include at least one event from each of the 7 subject-bearing event families: `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, and `FailedLookup`.

#### Scenario: Fixture includes FileOpened events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: FileOpened` with a valid `FileOpenedPayload`

#### Scenario: Fixture includes SearchRun events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: SearchRun` with a valid `SearchRunPayload`

#### Scenario: Fixture includes SymbolInspected events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: SymbolInspected` with a valid `SymbolInspectedPayload`

#### Scenario: Fixture includes CommandExecuted events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: CommandExecuted` with a valid `CommandExecutedPayload`

#### Scenario: Fixture includes DocRetrieved events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: DocRetrieved` with a valid `DocRetrievedPayload`

#### Scenario: Fixture includes EditMade events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: EditMade` with a valid `EditMadePayload`

#### Scenario: Fixture includes FailedLookup events

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` has `event_type: FailedLookup` with a valid `FailedLookupPayload` and `outcome: Failure`

#### Scenario: Fixture includes a non-FailedLookup failure event

- **GIVEN** the multi-event fixture
- **WHEN** the fixture is inspected
- **THEN** at least one `TraceEvent` with a subject-bearing type other than `FailedLookup` has `outcome: Failure` with a `reason` string
- **AND** the failure bonus of `+2` is verified in the resulting score for that subject

### Requirement: Hotspot output snapshots catch contract drift

The system SHALL include `insta` snapshot assertions for hotspot stdout and artifact JSON. Snapshots SHALL normalize volatile fields (`generatedAt`, `repositoryPath`, `storePath`) to deterministic placeholders before comparison so that only intentional contract changes cause snapshot failures.

#### Scenario: Stdout snapshot is asserted with normalized volatile fields

- **GIVEN** a successful `scryrs hotspots <PATH>` run producing stdout JSON
- **WHEN** the E2E test asserts the snapshot
- **THEN** the JSON is parsed and `generatedAt` is replaced with `"<GENERATED_AT>"`
- **AND** `repositoryPath` is replaced with `"<REPO>"`
- **AND** `storePath` is replaced with `"<STORE>"`
- **AND** the normalized JSON is passed to `insta::assert_snapshot!`

#### Scenario: Artifact JSON snapshot is asserted with normalized volatile fields

- **GIVEN** a successful `scryrs hotspots <PATH>` run producing `.scryrs/hotspots.json`
- **WHEN** the E2E test asserts the artifact snapshot
- **THEN** the artifact JSON is parsed and volatile fields are normalized identically to the stdout snapshot
- **AND** the normalized JSON is passed to `insta::assert_json_snapshot!`

#### Scenario: Intentional contract change breaks snapshot

- **GIVEN** snapshot files have been accepted via `cargo insta review`
- **WHEN** a production code change modifies the hotspot output schema (e.g., adding or removing a field, changing score computation)
- **THEN** the snapshot test fails
- **AND** the failure requires deliberate review and snapshot update via `cargo insta review`

### Requirement: Inline assertions verify scoring contract with full fixture

The existing `hotspot_integration_tests` module in `crates/scryrs-cli/src/lib.rs` SHALL include a test that populates the store with the full multi-event-family fixture (via `populate_store`, bypassing `record`) and asserts expected scores, ranking, and evidence fields using inline `assert_eq!` assertions.

#### Scenario: Inline test verifies scores for all event types

- **GIVEN** a store populated with at least one `FileOpened` event (weight 1), one `SearchRun` event (weight 2), and one `FailedLookup` event (weight 4 plus failure bonus 2 equals 6)
- **WHEN** `scryrs hotspots <PATH>` is invoked and output parsed
- **THEN** the entry for the `FailedLookup` subject has score 6
- **AND** the entry for the `SearchRun` subject has the correct score based on its event count
- **AND** the entry for the `FileOpened` subject has the correct score
- **AND** scores reflect the documented weight table

#### Scenario: Inline test verifies counts and evidence

- **GIVEN** a store populated with events of multiple types for the same subject
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** `counts.eventType` contains the correct per-type counts for that subject
- **AND** `counts.outcome` separates success and failure counts correctly
- **AND** `evidence.rowIds` contains the correct SQLite row IDs
- **AND** `sessionCount` reflects unique session IDs for that subject

### Requirement: Existing error-path tests remain unchanged

Tests covering missing store (exit 2), unsupported store (exit 2), corrupt store (exit 1), artifact write failure (exit 1), and empty or lifecycle-only store (exit 0 with empty entries) SHALL continue to pass without modification.

#### Scenario: Missing store test still passes

- **GIVEN** the existing `missing_store_exits_2_with_error` test in `hotspot_integration_tests`
- **WHEN** the test suite runs
- **THEN** the test passes with exit code 2 and expected stderr message

#### Scenario: Corrupt store test still passes

- **GIVEN** the existing `corrupt_store_exits_1_with_error` test in `hotspot_integration_tests`
- **WHEN** the test suite runs
- **THEN** the test passes with exit code 1 and expected stderr message

#### Scenario: Artifact write failure test still passes

- **GIVEN** the existing `artifact_write_failure_exits_1_with_stderr_populated` test
- **WHEN** the test suite runs
- **THEN** the test passes with exit code 1 and stderr containing "cannot write artifact file"

### Requirement: No production code changes

This change SHALL NOT modify any production code path. All changes SHALL be additive test code in `crates/scryrs-cli/tests/` and test modules within `crates/scryrs-cli/src/lib.rs`.

#### Scenario: Production code is untouched

- **GIVEN** the diff of this change
- **WHEN** a reviewer inspects changed files
- **THEN** no changes exist outside test files and test modules
- **AND** `#[cfg(test)]` boundaries are not moved or removed
- **AND** no new `#[cfg(not(test))]` or feature-gated production code is added

#### Scenario: Graph proposal adapter runtime and LLM code paths are untouched

- **GIVEN** the diff of this change
- **WHEN** a reviewer inspects changed files
- **THEN** no changes exist in `crates/scryrs-graph/`, `crates/scryrs-adapter-*/`, `crates/scryrs-runtime/`, or `crates/scryrs-llm/`
- **AND** no new dependencies on graph, proposal, adapter, runtime, or LLM crates are added