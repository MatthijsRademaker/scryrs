## ADDED Requirements

### Requirement: CLI provides machine-readable surface via --help-json

The system SHALL provide a `--help-json` flag that emits a versioned JSON document to stdout describing the complete CLI surface.

#### Scenario: Agent discovers CLI surface via --help-json

- **WHEN** the agent runs `scryrs --help-json`
- **THEN** the system SHALL emit a single JSON object to stdout
- **THEN** the system SHALL exit with code 0
- **THEN** the JSON object SHALL contain a `surfaceVersion` field with the surface document format version
- **THEN** the JSON object SHALL contain a `binary` field with the value `"scryrs"`
- **THEN** the JSON object SHALL contain a `commands` array describing all available commands
- **THEN** the JSON object SHALL contain a `globalFlags` array describing all global flags
- **THEN** the JSON object SHALL contain a `rootBehavior` object describing bare invocation behavior
- **THEN** the JSON object SHALL contain an `exitCodes` object mapping exit codes to their meanings
- **THEN** stderr SHALL be empty

#### Scenario: Short form --help-json flag works

- **WHEN** the agent runs `scryrs -hj`
- **THEN** the system SHALL emit the same surface document as `--help-json` to stdout
- **THEN** the system SHALL exit with code 0

### Requirement: Command entries include argument and output contract metadata

Each command in the `commands` array SHALL include enough metadata for an agent to construct a valid invocation and parse the result.

#### Scenario: Each command entry contains name, description, arguments, and output

- **WHEN** the surface document contains a command entry
- **THEN** the entry SHALL include a `name` field (string)
- **THEN** the entry SHALL include a `description` field (string)
- **THEN** the entry SHALL include an `arguments` array describing each positional argument
- **THEN** the entry SHALL include an `output` object with `mimeType` and `fields` array
- **THEN** each entry in `fields` SHALL include `name`, `type`, `description`, and `optional` (boolean)

#### Scenario: Arguments include name, type, required flag, and description

- **WHEN** a command has positional arguments
- **THEN** each argument entry SHALL include `name` (string)
- **THEN** each argument entry SHALL include `type` (string, e.g., `"string"`, `"path"`, `"number"`)
- **THEN** each argument entry SHALL include `required` (boolean)
- **THEN** each argument entry SHALL include `description` (string)

#### Scenario: hotspots command surface is documented

- **WHEN** the surface document contains the `hotspots` command entry
- **THEN** `name` SHALL be `"hotspots"`
- **THEN** `arguments` SHALL contain exactly one entry with `name: "PATH"`, `type: "string"`, `required: true`
- **THEN** `output.mimeType` SHALL be `"application/json"`
- **THEN** `output.fields` SHALL contain entries for `schemaVersion`, `command`, and `status`
- **THEN** the `status` field entry SHALL have `optional: false`

### Requirement: Global flags include name, short, long, and description

Each entry in the `globalFlags` array SHALL describe a single flag available at the top level.

#### Scenario: Each flag entry contains all metadata

- **WHEN** the surface document contains a global flag entry
- **THEN** the entry SHALL include `name` (string)
- **THEN** the entry SHALL include `short` (string, the short form with leading dash, or null)
- **THEN** the entry SHALL include `long` (string, the long form with leading dashes)
- **THEN** the entry SHALL include `description` (string)
- **THEN** the entry SHALL include `action` (string, one of `"help"`, `"version"`, `"helpJson"`)

#### Scenario: All expected global flags are present

- **WHEN** the surface document's `globalFlags` array is examined
- **THEN** it SHALL contain exactly three entries
- **THEN** one entry SHALL have `name: "help"`, `short: "-h"`, `long: "--help"`, `action: "help"`
- **THEN** one entry SHALL have `name: "version"`, `short: "-V"`, `long: "--version"`, `action: "version"`
- **THEN** one entry SHALL have `name: "help-json"`, `short: "-hj"`, `long: "--help-json"`, `action: "helpJson"`

### Requirement: Root behavior documents bare invocation

The `rootBehavior` object SHALL document what happens when `scryrs` is invoked with no arguments.

#### Scenario: Root behavior entry describes bare invocation

- **WHEN** the surface document's `rootBehavior` object is examined
- **THEN** it SHALL contain an `action` field with value `"help"`
- **THEN** it SHALL contain an `exitCode` field with value `0`

### Requirement: Exit codes are documented

The `exitCodes` object SHALL map numeric exit codes to their meaning.

#### Scenario: All expected exit codes are present

- **WHEN** the surface document's `exitCodes` object is examined
- **THEN** it SHALL contain keys `"0"`, `"1"`, and `"2"`
- **THEN** the value for `"0"` SHALL be `"Success"`
- **THEN** the value for `"1"` SHALL be `"I/O error"`
- **THEN** the value for `"2"` SHALL be `"Usage error"`

### Requirement: --help-json preserves existing contract

The `--help-json` flag SHALL NOT alter or interfere with any existing CLI behavior.

#### Scenario: --help-json is discovered and invoked without side effects

- **WHEN** the agent runs `scryrs --help-json`
- **THEN** no files are created, no network requests are made, no persistent state changes occur
- **THEN** a subsequent `scryrs hotspots /tmp` SHALL produce the same output as before

#### Scenario: Existing flags continue to work unchanged

- **WHEN** the agent runs `scryrs --help`
- **THEN** the output SHALL be identical to the pre-change help text
- **WHEN** the agent runs `scryrs --version`
- **THEN** the output SHALL be identical to the pre-change version string
- **WHEN** the agent runs `scryrs hotspots /tmp`
- **THEN** the JSON output SHALL be identical to the pre-change envelope
- **WHEN** the agent runs `scryrs hotspots` (no PATH)
- **THEN** the system SHALL exit 2 with usage error on stderr

### Requirement: --help-json surface describes the exact current state

The surface document SHALL accurately reflect the CLI's actual behavior, not a speculative future state.

#### Scenario: Surface document matches live CLI behavior

- **WHEN** the surface document describes an exit code for a given code
- **THEN** invoking the CLI in a way that produces that exit code SHALL produce the documented meaning
- **WHEN** the surface document lists a command
- **THEN** invoking that command with valid arguments SHALL execute successfully
