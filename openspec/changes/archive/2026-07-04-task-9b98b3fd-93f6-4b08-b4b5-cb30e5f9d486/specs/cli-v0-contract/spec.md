## ADDED Requirements

### Requirement: Frozen binary name

The public binary name SHALL be `scryrs`.

#### Scenario: Binary name is scryrs
- **WHEN** the CLI is invoked via its installed binary
- **THEN** the binary name is `scryrs`

### Requirement: Single v0 placeholder command

The v0 CLI surface SHALL expose exactly one public command: `scryrs hotspots <PATH>`.

#### Scenario: Hotspots command with path argument
- **WHEN** `scryrs hotspots /some/repo` is invoked
- **THEN** a versioned JSON envelope is written to stdout
- **AND** the process exits with code 0

#### Scenario: Hotspots command without path argument
- **WHEN** `scryrs hotspots` is invoked without a PATH argument
- **THEN** a usage error is written to stderr
- **AND** the process exits with code 2

#### Scenario: Unknown command
- **WHEN** `scryrs <unknown-command>` is invoked where `<unknown-command>` is any string not matching `hotspots`, `--help`, `-h`, `--version`, or `-V`
- **THEN** an error message is written to stderr
- **AND** the process exits with code 2

#### Scenario: No second real command exists
- **WHEN** any command other than `hotspots` is invoked
- **THEN** the process does not produce a success result
- **AND** the process exits with a non-zero code

### Requirement: Global help and version flags

The CLI SHALL support `-h`/`--help` and `-V`/`--version` as the only global flags for v0.

#### Scenario: Help flag
- **WHEN** `scryrs --help` or `scryrs -h` is invoked
- **THEN** help text is written to stdout
- **AND** the help text lists only `scryrs hotspots <PATH>` as the available command
- **AND** the process exits with code 0

#### Scenario: Version flag
- **WHEN** `scryrs --version` or `scryrs -V` is invoked
- **THEN** the version string is written to stdout
- **AND** the process exits with code 0

### Requirement: Bare invocation behavior

Invoking `scryrs` with no arguments SHALL print help text to stdout and exit with code 0.

#### Scenario: Bare invocation prints help
- **WHEN** `scryrs` is invoked with no arguments and no flags
- **THEN** help text is written to stdout
- **AND** the process exits with code 0

### Requirement: Placeholder command output contract

The `scryrs hotspots <PATH>` command SHALL emit a versioned JSON object to stdout. No human-readable text SHALL be emitted to stdout for this command.

#### Scenario: JSON output envelope
- **WHEN** `scryrs hotspots <PATH>` is invoked with a valid local directory path
- **THEN** a JSON object is written to stdout
- **AND** the JSON object contains a `schemaVersion` field matching `scryrs-types::SCHEMA_VERSION`
- **AND** the JSON object contains a `command` field with value `"hotspots"`
- **AND** the JSON object contains a `status` field with value `"placeholder"`

#### Scenario: No human-readable fallback
- **WHEN** `scryrs hotspots <PATH>` is invoked
- **THEN** stdout contains only a valid JSON object
- **AND** no human-readable prose is written to stdout

### Requirement: Exit-code policy

The CLI SHALL use the following exit-code scheme:
- 0 for successful command execution, help display, and version display
- 2 for unknown commands, missing required arguments, invalid arguments, and all usage errors
- 1 for unexpected runtime failures (I/O errors, internal panics)

#### Scenario: Success exit code
- **WHEN** any valid v0 invocation produces its expected output
- **THEN** the process exits with code 0

#### Scenario: Usage error exit code
- **WHEN** any invalid v0 invocation produces a usage error
- **THEN** the process exits with code 2
- **AND** the error message is written to stderr

#### Scenario: Runtime failure exit code
- **WHEN** an unexpected runtime failure occurs (e.g., I/O error during output write)
- **THEN** the process exits with code 1

### Requirement: Stderr for errors and diagnostics

All error messages, usage errors, and human-facing diagnostics SHALL be written to stderr.

#### Scenario: Errors go to stderr
- **WHEN** any error condition occurs (unknown command, missing PATH, invalid argument)
- **THEN** the error message is written to stderr
- **AND** stdout contains no error output

### Requirement: Agent-facing contract intent

The CLI contract SHALL define when an agent should call `scryrs hotspots <PATH>`: when the agent needs scryrs' repository hotspot summary for a given local directory path.

#### Scenario: Agent invocation intent
- **GIVEN** an AI agent needs a repository hotspot summary for a local path
- **WHEN** the agent calls `scryrs hotspots <PATH>` with an explicit local directory path
- **THEN** the agent receives a parseable JSON envelope on stdout
- **AND** the agent can distinguish success (exit 0), usage errors (exit 2), and runtime failures (exit 1)

### Requirement: Out-of-scope commands fail fast

All commands not defined in the v0 contract SHALL fail with exit code 2 and an error message on stderr.

#### Scenario: Previously scaffolded commands fail
- **WHEN** `scryrs components` is invoked
- **THEN** the process exits with code 2
- **AND** an error message is written to stderr

#### Scenario: Future vision commands fail
- **WHEN** `scryrs trace`, `scryrs propose`, `scryrs graph`, `scryrs route`, `scryrs adapters`, `scryrs report`, or `scryrs suggest-docs` is invoked
- **THEN** the process exits with code 2
- **AND** an error message is written to stderr

### Requirement: Design note discoverability

A design note documenting the v0 CLI contract SHALL exist at `.devagent/docs/docs/cli-v0-contract.md` and SHALL be accessible via the docs navigation in `.devagent/docs/docs/_nav.json`.

#### Scenario: Design note exists
- **GIVEN** the repository at the v0 contract baseline
- **WHEN** a developer navigates to `.devagent/docs/docs/cli-v0-contract.md`
- **THEN** the file contains the frozen v0 CLI contract specification

#### Scenario: Design note is navigable
- **GIVEN** the docs navigation at `.devagent/docs/docs/_nav.json`
- **WHEN** a developer views the navigation structure
- **THEN** a "Technical" section exists
- **AND** the section contains an entry linking to the CLI v0 contract note

### Requirement: README reflects v0 surface

The repository README SHALL not advertise `components` or any second real command as part of the v0 public surface.

#### Scenario: README shows only v0 surface
- **GIVEN** the repository README.md
- **WHEN** a reader views the feature or usage section
- **THEN** no `scryrs components` example is present
- **AND** the primary example reflects `scryrs hotspots <PATH>` or `--help`/`--version`

### Requirement: Help text shows single command

The help text emitted by `scryrs --help` SHALL list only `scryrs hotspots <PATH>` as the available command.

#### Scenario: Help text is single-command
- **WHEN** `scryrs --help` is invoked
- **THEN** the help output includes `scryrs hotspots <PATH>`
- **AND** the help output does not include `scryrs components`, `scryrs trace`, `scryrs propose`, `scryrs graph`, `scryrs route`, `scryrs adapters`, `scryrs report`, or `scryrs suggest-docs`

### Requirement: Design note external-only scope

The design note at `.devagent/docs/docs/cli-v0-contract.md` SHALL describe only the external CLI contract. It SHALL NOT describe internal engine behavior, trace collection algorithms, hotspot scoring, or implementation details.

#### Scenario: Design note is contract-only
- **GIVEN** the design note at `.devagent/docs/docs/cli-v0-contract.md`
- **WHEN** a reader reviews the content
- **THEN** the content covers binary name, command surface, flags, exit codes, output contract, and agent-facing intent
- **AND** the content does not describe engine internals, algorithms, or implementation strategies
