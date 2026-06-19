# CLI v0 Contract

## ADDED Requirements

### Requirement: Single Command Surface

The v0 CLI SHALL expose exactly one public command: `components`. No other command name SHALL be recognized, documented, or reachable through the public CLI dispatch.

#### Scenario: Help output shows only the v0 surface
- **WHEN** `scryrs --help` is invoked
- **THEN** the output SHALL list `components` as the only command
- **AND** the output SHALL NOT mention `trace`, `hotspots`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`, or any other command name

#### Scenario: Single command is invoked
- **WHEN** `scryrs components` is invoked
- **THEN** the command SHALL execute successfully with exit code 0

### Requirement: Machine-Readable Output Contract

The v0 CLI SHALL support `scryrs components --format json` producing a stable JSON object with a `schemaVersion` field and a `components` array.

#### Scenario: JSON output includes schema version
- **WHEN** `scryrs components --format json` is invoked
- **THEN** the output SHALL be valid JSON
- **AND** the output SHALL contain a `"schemaVersion"` field
- **AND** the output SHALL contain a `"components"` array

#### Scenario: JSON schema is stable within v0.x
- **WHEN** the JSON output contract is read
- **THEN** each `components` array element SHALL have `"id"`, `"title"`, and `"summary"` string fields
- **AND** the `schemaVersion` field SHALL be consistent with the current `SCHEMA_VERSION` constant

### Requirement: Fail-Fast for Unsupported Paths

The v0 CLI SHALL reject any command name other than `components` with a usage error on stderr and exit code 2. There SHALL be no soft-landing scaffold responses for stub command names.

#### Scenario: Stub command name produces usage error
- **WHEN** a previously-recognized stub command such as `trace` is invoked
- **THEN** the CLI SHALL write an error message containing the command name to stderr
- **AND** the CLI SHALL exit with code 2

#### Scenario: Truly unknown command produces usage error
- **WHEN** an unrecognized command such as `unknown` is invoked
- **THEN** the CLI SHALL write an error message containing the command name to stderr
- **AND** the CLI SHALL exit with code 2

#### Scenario: Unsupported flags on valid command fail fast
- **WHEN** `scryrs components` is invoked with flags other than `--format json`
- **THEN** the CLI SHALL exit with a non-zero code
- **AND** the CLI SHALL provide guidance on stderr

### Requirement: Global Help and Version Flags

The v0 CLI SHALL support `--help`/`-h` displaying usage information and `--version`/`-V` displaying the binary version.

#### Scenario: Help flag displays usage
- **WHEN** `scryrs --help` or `scryrs -h` is invoked
- **THEN** the CLI SHALL write usage information to stdout
- **AND** the CLI SHALL exit with code 0

#### Scenario: Version flag displays version
- **WHEN** `scryrs --version` or `scryrs -V` is invoked
- **THEN** the CLI SHALL write the version string to stdout
- **AND** the CLI SHALL exit with code 0

### Requirement: Exit Code Policy

The v0 CLI SHALL use the following exit codes with no exceptions:

| Exit Code | Meaning |
|-----------|--------|
| 0 | Success (component output, help text, version banner) |
| 1 | Write/internal CLI failure (I/O error writing to stdout or stderr) |
| 2 | Usage error, unknown command, or unsupported invocation |

#### Scenario: Success paths return exit 0
- **WHEN** `scryrs components`, `scryrs --help`, or `scryrs --version` completes successfully
- **THEN** exit code SHALL be 0

#### Scenario: Write failures return exit 1
- **WHEN** a write to stdout or stderr fails
- **THEN** exit code SHALL be 1

#### Scenario: Usage errors return exit 2
- **WHEN** an unknown command or unsupported invocation is detected
- **THEN** exit code SHALL be 2

### Requirement: Design Note Documents the Contract

A design note at `.devagent/docs/docs/cli-v0-contract.md` SHALL define the frozen v0 contract and SHALL be discoverable via the docs navigation at `.devagent/docs/docs/_nav.json`.

#### Scenario: Contract note exists and is navigable
- **WHEN** the docs navigation is rendered
- **THEN** an entry for the CLI v0 contract note SHALL be present
- **AND** the linked document SHALL define the binary name, single command, accepted inputs, output contract (text and JSON), stdout/stderr rules, exit codes, and fail-fast behavior

## REMOVED Requirements

### Requirement: No Stub Command Dispatch

The `is_known_stub()` function and its dispatch arm are removed. The ONLY recognized non-flag arguments are `components` (with optional `--format json`). All other argument sequences SHALL reach the unknown-command error path.
