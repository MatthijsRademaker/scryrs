# cli-golden-tests Specification

## Purpose
TBD - created by archiving change task-a81f9fdb-3db8-47dd-a8da-8c416d982b32. Update Purpose after archive.

## RECONCILIATION — Phase 2 Hotspot Materialization (2026-06-21)

The Phase 2 hotspot materialization has been fully implemented and delivered. The following requirement is **superseded** by the Phase 2 verification specs:

- **Superseded**: Requirement "hotspots placeholder output is verified by inline snapshot" and its scenario "hotspots /tmp produces exact inline snapshot output" — these asserted that `scryrs hotspots <PATH>` emits a placeholder JSON envelope `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}` and that this output is verified by an insta inline snapshot. The live implementation now emits a real `HotspotsReport` (schemaVersion 1.0.0) with entries, scoring, and a `.scryrs/hotspots.json` artifact file, verified by the end-to-end test at `crates/scryrs-cli/tests/hotspot_e2e.rs` and integration tests at `crates/scryrs-cli/tests/`.

**Canonical hotspot test coverage** (supersedes the above):
- [`crates/scryrs-cli/tests/hotspot_e2e.rs`](../../crates/scryrs-cli/tests/hotspot_e2e.rs) — full record → SQLite → hotspots → artifact pipeline, multievent-family fixtures, empty-store, missing-store.
- `crates/scryrs-cli/tests/` — hotspot integration tests with real SQLite fixtures.
- [`openspec/specs/hotspot-verification/spec.md`](../hotspot-verification/spec.md) — canonical E2E verification requirements.

**Closure change traceability**: `openspec/changes/archive/2026-06-21-task-56573ced-fdeb-49b2-aea6-41b30f19d2bf/specs/phase-2-closure/spec.md` documents the full evidence matrix mapping code/test artifacts to Phase 2 deliverables.

All other requirements in this spec (help, help-json, version, snapshot update workflow) remain valid and are not affected by this reconciliation.
## Requirements
### Requirement: Help text output is verified by exact-match snapshot

The CLI `--help` output SHALL be verified by an `insta` file snapshot that matches the full help text byte-for-byte.

#### Scenario: --help produces exact snapshot output

- **WHEN** the test suite calls `run_with_writers(["--help"], out, err)`
- **THEN** the captured stdout matches the committed `insta` file snapshot
- **AND** stderr is empty
- **AND** exit code is 0

#### Scenario: -h produces identical output to --help

- **WHEN** the test suite calls `run_with_writers(["-h"], out, err)`
- **THEN** the captured stdout is byte-for-byte identical to `--help` output
- **AND** stderr is empty
- **AND** exit code is 0

#### Scenario: Bare invocation produces identical output to --help

- **WHEN** the test suite calls `run_with_writers([], out, err)` (no arguments)
- **THEN** the captured stdout is byte-for-byte identical to `--help` output
- **AND** stderr is empty
- **AND** exit code is 0

### Requirement: --help-json output is verified by exact-match snapshot

The CLI `--help-json` output SHALL be verified by an `insta` file snapshot that matches the complete JSON surface document byte-for-byte.

#### Scenario: --help-json produces exact snapshot output

- **WHEN** the test suite calls `run_with_writers(["--help-json"], out, err)`
- **THEN** the captured stdout matches the committed `insta` file snapshot
- **AND** stderr is empty
- **AND** exit code is 0

#### Scenario: -hj produces identical output to --help-json

- **WHEN** the test suite calls `run_with_writers(["-hj"], out, err)`
- **THEN** the captured stdout is byte-for-byte identical to `--help-json` output
- **AND** stderr is empty
- **AND** exit code is 0

#### Scenario: --help-json is idempotent

- **WHEN** the test suite calls `run_with_writers(["--help-json"], out, err)` twice in succession
- **THEN** both invocations produce byte-for-byte identical output

### Requirement: hotspots placeholder output is verified by inline snapshot

The `scryrs hotspots <PATH>` placeholder JSON envelope SHALL be verified by an `insta` inline snapshot.

#### Scenario: hotspots /tmp produces exact inline snapshot output

- **WHEN** the test suite calls `run_with_writers(["hotspots", "/tmp"], out, err)`
- **THEN** the captured stdout matches the committed `insta` inline snapshot
- **AND** stderr is empty
- **AND** exit code is 0

### Requirement: Snapshot update workflow is documented

The process for updating snapshots after intentional contract changes SHALL be documented in the CLI contract design note.

#### Scenario: Snapshot update command is documented

- **WHEN** a developer reads the local check documentation in `.devagent/docs/docs/cli-v0-contract.md`
- **THEN** they SHALL find instructions for running tests, viewing snapshot diffs, and accepting snapshot updates via `cargo insta review` or `cargo insta test --accept`

