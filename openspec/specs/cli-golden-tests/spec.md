# cli-golden-tests Specification

## Purpose
TBD - created by archiving change task-a81f9fdb-3db8-47dd-a8da-8c416d982b32. Update Purpose after archive.
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

