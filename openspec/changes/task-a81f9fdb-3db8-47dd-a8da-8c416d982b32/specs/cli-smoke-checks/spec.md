## ADDED Requirements

### Requirement: run() entrypoint is tested by smoke tests

The public `run()` function (which collects environment arguments and delegates to `run_with_writers`) SHALL be exercised by smoke tests that verify exit code propagation and basic I/O behavior for each major invocation pattern.

#### Scenario: --help smoke test

- **WHEN** the test suite calls `run(["--help"])` via the public entrypoint
- **THEN** exit code is 0
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: --version smoke test

- **WHEN** the test suite calls `run(["--version"])` via the public entrypoint
- **THEN** exit code is 0
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: hotspots /tmp smoke test

- **WHEN** the test suite calls `run(["hotspots", "/tmp"])` via the public entrypoint
- **THEN** exit code is 0
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: Bare invocation smoke test

- **WHEN** the test suite calls `run([])` (empty arguments) via the public entrypoint
- **THEN** exit code is 0
- **AND** stdout is non-empty
- **AND** no panic or abort occurs

#### Scenario: Unknown command smoke test

- **WHEN** the test suite calls `run(["unknown"])` via the public entrypoint
- **THEN** exit code is 2
- **AND** stderr is non-empty (error message)
- **AND** no panic or abort occurs

#### Scenario: hotspots without PATH smoke test

- **WHEN** the test suite calls `run(["hotspots"])` via the public entrypoint (missing required PATH)
- **THEN** exit code is 2
- **AND** stderr is non-empty (usage error)
- **AND** no panic or abort occurs

### Requirement: Smoke tests do not duplicate snapshot assertions

Smoke tests SHALL verify exit code and I/O presence only — they SHALL NOT duplicate the exact-output assertions covered by snapshot tests.

#### Scenario: Smoke tests use lightweight output checks

- **WHEN** a smoke test receives output from the `run()` entrypoint
- **THEN** it SHALL assert exit code (numeric) and output presence (non-empty or empty), but SHALL NOT assert specific content (that is covered by `run_with_writers` snapshot tests)
