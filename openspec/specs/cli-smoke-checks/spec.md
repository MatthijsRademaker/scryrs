# cli-smoke-checks Specification

## Purpose
TBD - created by archiving change task-a81f9fdb-3db8-47dd-a8da-8c416d982b32. Update Purpose after archive.
## Requirements
### Requirement: run() entrypoint does not panic

The public `run()` entrypoint (which collects environment arguments and delegates to `run_with_writers`) SHALL be exercised by a single smoke test that verifies all major invocation patterns complete without panic or abort.

#### Scenario: public run() entrypoint no-panic smoke test

- **WHEN** the test suite calls `run()` via the public entrypoint for all major invocation patterns (`--help`, `--version`, `--help-json`, `hotspots /tmp`, bare invocation, unknown command, hotspots without PATH)
- **THEN** all invocations return the expected exit code
- **AND** no panic or abort occurs

### Requirement: run_with_writers is exercised by I/O smoke tests

The `run_with_writers` function (which contains all CLI dispatch logic) SHALL be exercised by smoke tests that verify exit code propagation and basic I/O presence for each major invocation pattern. Smoke tests SHALL use captured `Vec<u8>` writers to enable I/O verification without depending on the terminal stdout/stderr handles.

#### Scenario: --help smoke test

- **WHEN** the test suite calls `run_with_writers(["--help"], out, err)` with captured writers
- **THEN** exit code is 0
- **AND** stderr is empty
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: --version smoke test

- **WHEN** the test suite calls `run_with_writers(["--version"], out, err)` with captured writers
- **THEN** exit code is 0
- **AND** stderr is empty
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: hotspots /tmp smoke test

- **WHEN** the test suite calls `run_with_writers(["hotspots", "/tmp"], out, err)` with captured writers
- **THEN** exit code is 0
- **AND** stderr is empty
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: Bare invocation smoke test

- **WHEN** the test suite calls `run_with_writers([], out, err)` (empty arguments) with captured writers
- **THEN** exit code is 0
- **AND** stderr is empty
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: Unknown command smoke test

- **WHEN** the test suite calls `run_with_writers(["unknown"], out, err)` with captured writers
- **THEN** exit code is 2
- **AND** stdout is empty
- **AND** stderr is non-empty (error message)
- **AND** no panic or abort occurs

#### Scenario: hotspots without PATH smoke test

- **WHEN** the test suite calls `run_with_writers(["hotspots"], out, err)` (missing required PATH) with captured writers
- **THEN** exit code is 2
- **AND** stdout is empty
- **AND** stderr is non-empty (usage error)
- **AND** no panic or abort occurs

### Requirement: Smoke tests do not duplicate snapshot assertions

Smoke tests SHALL verify exit code and I/O presence only — they SHALL NOT duplicate the exact-output assertions covered by snapshot tests.

#### Scenario: Smoke tests use lightweight output checks

- **WHEN** a smoke test receives output from `run_with_writers` with captured buffers
- **THEN** it SHALL assert exit code (numeric) and output presence (non-empty or empty), but SHALL NOT assert specific content (that is covered by snapshot tests)

