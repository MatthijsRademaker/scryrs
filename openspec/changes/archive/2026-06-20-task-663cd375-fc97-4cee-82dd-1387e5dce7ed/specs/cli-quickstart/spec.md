## ADDED Requirements

### Requirement: Quickstart section exists in README

The repository README SHALL include a Quickstart section that enables a first-time user to build, run, and understand the scryrs CLI without reading source code or developer-internal documentation.

#### Scenario: Quickstart is present in README

- **GIVEN** the repository is freshly cloned
- **WHEN** a user opens `README.md`
- **THEN** the document contains a "Quickstart" section
- **AND** the Quickstart section appears before the "Current status" section

### Requirement: Quickstart covers all build and run steps

The Quickstart section SHALL include all steps needed to go from a freshly cloned repository to successfully running the CLI.

#### Scenario: Quickstart covers prerequisites

- **WHEN** a user reads the Quickstart section
- **THEN** it lists the required prerequisites (Rust toolchain version 1.85+ or Docker)

#### Scenario: Quickstart covers build step

- **WHEN** a user reads the Quickstart section
- **THEN** it includes a copy-paste runnable command to build the CLI from source
- **AND** the build command references the `scryrs-cli` crate

#### Scenario: Quickstart covers CLI surface exploration

- **WHEN** a user reads the Quickstart section
- **THEN** it includes a command to run `scryrs --help` with expected output
- **AND** it includes a command to run `scryrs --version` with expected output
- **AND** it includes a command to run `scryrs --help-json` (or instruction to try it)

#### Scenario: Quickstart covers the placeholder command

- **WHEN** a user reads the Quickstart section
- **THEN** it includes a command to run `scryrs hotspots <PATH>` with a known path (e.g., `.`)
- **AND** it shows the expected JSON placeholder output
- **AND** it explains what the output means

#### Scenario: Quickstart shows error path behavior

- **WHEN** a user reads the Quickstart section
- **THEN** it shows at least one error path (e.g., missing PATH argument) with expected error output
- **AND** it explains the exit code convention (0/1/2)

### Requirement: Current limitations documented

The Quickstart section SHALL include a subsection that honestly documents the current limitations of the v0 CLI.

#### Scenario: Limitations are documented

- **WHEN** a user reads the Quickstart section
- **THEN** it contains a subsection documenting current limitations
- **AND** the limitations state that only one command exists (`hotspots`)
- **AND** the limitations state that the command output is a placeholder envelope, not real data
- **AND** the limitations state that no engine behavior is implemented
- **AND** the limitations do not speculate about future commands or features
