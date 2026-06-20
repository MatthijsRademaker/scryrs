# cli-examples Specification

## Purpose

Specifies requirements for CLI surface command examples in the Quickstart documentation, ensuring they are copy-paste runnable, comprehensive, output-verified, and non-speculative.

## Requirements

### Requirement: Examples are copy-paste runnable

The Quickstart section SHALL include shell command examples that a user can copy and paste directly into a terminal without modification.

#### Scenario: Examples are valid shell commands

- **WHEN** a user copies an example command from the Quickstart section
- **AND** pastes it into a terminal
- **THEN** the command executes without syntax errors
- **AND** the command does not require interactive input

### Requirement: Examples cover all CLI surface commands

The Quickstart section SHALL include examples demonstrating every CLI surface entrypoint.

#### Scenario: All surface commands have examples

- **WHEN** a user reads the Quickstart section
- **THEN** it includes an example for `--help` or bare invocation
- **AND** it includes an example for `--version`
- **AND** it includes an example for `--help-json`
- **AND** it includes an example for `hotspots <PATH>` with a valid path
- **AND** it includes an example showing an error path (e.g., missing PATH)

### Requirement: Examples show expected output

Each example in the Quickstart section SHALL show the expected output after the command, using representative output from the actual CLI.

#### Scenario: Expected output shown for each example

- **WHEN** a user reads an example in the Quickstart section
- **THEN** the expected output is shown with the command (either inline using a shell prompt prefix or in a separate code block)
- **AND** the output matches the shape and content of the actual CLI output (verified against snapshot tests)

### Requirement: Examples align with tested CLI behavior

Every command shown in an example SHALL have a corresponding snapshot test in the `scryrs-cli` test suite that verifies its exact output.

#### Scenario: Examples are covered by tests

- **GIVEN** the existing `insta` snapshot test suite for `scryrs-cli`
- **WHEN** a command appears as an example in the Quickstart section
- **THEN** that same command has a corresponding unit test with snapshot verification
- **AND** the example's expected output is consistent with the snapshot

### Requirement: Examples avoid speculative future behavior

Examples SHALL NOT reference commands, flags, output fields, or behavior that does not exist in the current v0 CLI contract.

#### Scenario: No speculative examples

- **WHEN** a user reads the Quickstart section
- **THEN** every command shown is a valid v0 CLI command that produces a documented exit code
- **AND** no example references commands listed in the v0 contract as "out of scope"
- **AND** no example shows output containing fields not present in the current JSON envelope
