# cli-discovery-ux Specification

## Purpose
TBD - created by archiving change task-7b4029eb-1fdc-4a55-ba26-13cf68495cd1. Update Purpose after archive.
## Requirements
### Requirement: Help text provides standalone discovery

The `scryrs --help` output SHALL serve as a complete discovery surface for the v0 CLI. The output SHALL include sections for purpose, usage, arguments, output contract, examples, options, and exit codes.

#### Scenario: Help includes purpose section

- **WHEN** `scryrs --help` or `scryrs -h` is invoked
- **THEN** the output includes text describing what scryrs does and when to use it

#### Scenario: Help includes usage section

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes a section indicating the command syntax `scryrs hotspots <PATH>`

#### Scenario: Help includes arguments section

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes a description of the `<PATH>` argument and that it is required

#### Scenario: Help includes output description

- **WHEN** `scryrs --help` is invoked
- **THEN** the output describes the JSON envelope format produced by `scryrs hotspots`

#### Scenario: Help includes examples

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes at least one example invocation of `scryrs hotspots <PATH>`

#### Scenario: Help includes options section

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes available flags (`-h`, `--help`, `-V`, `--version`)

#### Scenario: Help includes exit codes

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the exit-code policy (0/1/2) and their meanings

#### Scenario: No implementation-facing language in help

- **WHEN** `scryrs --help` is invoked
- **THEN** the output does not contain the phrase "v0 placeholder contract" or "placeholder" in descriptive text (the output contract description may reference placeholder status factually)

### Requirement: Error messages follow consistent format

All usage error messages SHALL follow a consistent format that states the error in context, shows usage, and routes toward `--help`.

#### Scenario: Unknown command error format

- **WHEN** `scryrs <unknown-command>` is invoked
- **THEN** the stderr output contains the unknown command name
- **AND** the stderr output contains `See \`scryrs --help\`` or equivalent escalation hint

#### Scenario: Missing PATH error format

- **WHEN** `scryrs hotspots` is invoked without a PATH argument
- **THEN** the stderr output contains the command name and the missing argument description
- **AND** the stderr output contains the usage line `Usage: scryrs hotspots <PATH>` or equivalent
- **AND** the stderr output contains a reference to `--help`

#### Scenario: Extra arguments error format

- **WHEN** `scryrs hotspots /path extra` is invoked with more than one positional argument
- **THEN** the stderr output indicates that extra arguments are not accepted
- **AND** the stderr output contains the usage line `Usage: scryrs hotspots <PATH>` or equivalent
- **AND** the stderr output contains a reference to `--help`

### Requirement: Help output matches across bare and flag invocation

The help text produced by bare `scryrs` SHALL be identical to the help text produced by `scryrs --help`.

#### Scenario: Bare and flag help produce same output

- **WHEN** `scryrs` (no arguments) and `scryrs --help` are both invoked
- **THEN** both produce identical help output

### Requirement: Tests validate structural properties

Tests for help text and error messages SHALL use structural assertions (section header presence, keyword presence) rather than exact string matching, to reduce brittleness from copy changes.

#### Scenario: Help test uses structural assertions

- **GIVEN** the test for `scryrs --help` output
- **WHEN** the test validates help content
- **THEN** it asserts the presence of expected section markers (e.g., "USAGE", "EXAMPLES", "OPTIONS") rather than exact help text

#### Scenario: Error test uses structural assertions

- **GIVEN** the test for usage error output
- **WHEN** the test validates error content
- **THEN** it asserts the presence of the command target, the problem description, the usage line, and the escalation reference rather than exact error text

### Requirement: Version flag and bare invocation remain unchanged

The `--version`/`-V` flag SHALL continue to print `scryrs <VERSION>` to stdout and exit 0. Bare invocation SHALL continue to print help to stdout and exit 0.

#### Scenario: Version flag unchanged

- **WHEN** `scryrs --version` or `scryrs -V` is invoked
- **THEN** the version string `scryrs <VERSION>` is written to stdout
- **AND** the process exits with code 0

#### Scenario: Bare invocation unchanged

- **WHEN** `scryrs` is invoked with no arguments
- **THEN** help text is written to stdout
- **AND** the process exits with code 0

### Requirement: Help text documents workspace live bootstrap commands

The `scryrs --help` output SHALL document the workspace-local live bootstrap workflow, including the `scryrs up` command and the live init inputs needed to scaffold a managed external-network live server.

#### Scenario: Help output lists `scryrs up`

- **WHEN** `scryrs --help` is invoked
- **THEN** the command list includes `scryrs up`
- **AND** the description states that it starts the workspace-managed live-server Compose stack

#### Scenario: Help output includes live bootstrap example

- **WHEN** `scryrs --help` is invoked
- **THEN** the examples or usage guidance show a live bootstrap flow that includes `scryrs init --agent <NAME>` with external-network configuration
- **AND** that flow shows `scryrs up` as the follow-up step to start the managed server

