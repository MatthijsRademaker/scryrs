## ADDED Requirements

### Requirement: Live init prompts for missing required values in interactive terminals

When `scryrs init` runs in live mode and required live bootstrap values are missing after applying the existing precedence chain, the CLI SHALL start an interactive wizard if stdin and stdout are terminals and `--no-interactive` is not present. The wizard SHALL collect only the missing required live values needed to complete bootstrap: `ingest_url`, `workspace_id`, and `docker_network`.

#### Scenario: Interactive missing live config starts wizard

- **WHEN** `scryrs init --agent pi` runs in live mode with missing `ingest_url`, `workspace_id`, or `docker_network`
- **AND** stdin and stdout are terminals
- **AND** `--no-interactive` is not present
- **THEN** the CLI starts a live-init wizard instead of immediately emitting the missing-config error
- **AND** the wizard prompts for each missing required value

#### Scenario: Complete live config skips wizard

- **WHEN** `scryrs init --agent claude-code` runs in live mode and `ingest_url`, `workspace_id`, and `docker_network` all resolve before prompting
- **THEN** the command does not start the wizard
- **AND** the existing live-init validation and write flow runs normally

#### Scenario: Resolved values are not re-prompted

- **WHEN** `scryrs init --agent pi` runs in live mode with `workspace_id` resolved from flags, environment, `.scryrs/.env`, or `scryrs.json`
- **AND** other required live values are missing
- **THEN** the wizard does not require re-entry of `workspace_id`
- **AND** the resolved value participates in the final confirmation summary

### Requirement: Non-interactive live init remains promptless and deterministic

The CLI SHALL NOT prompt when `--no-interactive` is present or when either stdin or stdout is not a terminal. In those cases, missing required live-mode config SHALL retain the existing exit-2 fail-fast behavior and deterministic remediation guidance.

#### Scenario: No-interactive flag preserves fail-fast behavior

- **WHEN** `scryrs init --agent pi --no-interactive` runs in live mode with missing required live config
- **THEN** the command exits 2
- **AND** stderr reports the missing live config guidance
- **AND** no prompt text is written to stdout
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

#### Scenario: Non-terminal stdin does not prompt

- **WHEN** `scryrs init --agent claude-code` runs in live mode with missing required live config
- **AND** stdin is not a terminal
- **THEN** the command exits 2 with deterministic missing-config guidance
- **AND** the command does not block waiting for input

#### Scenario: Non-terminal stdout does not prompt

- **WHEN** `scryrs init --agent claude-code` runs in live mode with missing required live config
- **AND** stdout is not a terminal
- **THEN** the command exits 2 with deterministic missing-config guidance
- **AND** the command does not emit interactive wizard controls

### Requirement: Wizard validates input before live init writes files

The wizard SHALL validate collected values before the installer performs any filesystem writes. Empty required values SHALL be rejected in the wizard. Invalid values SHALL be re-prompted or cancelled before manifest, `.scryrs/`, or hook writes occur.

#### Scenario: Empty required value is rejected

- **WHEN** the live-init wizard asks for `workspace_id`
- **AND** the user submits an empty value
- **THEN** the wizard rejects the value
- **AND** the installer does not write hook files, `.scryrs/` artifacts, or `scryrs.json`

#### Scenario: Invalid ingest URL is rejected

- **WHEN** the live-init wizard asks for `ingest_url`
- **AND** the user submits a value that live-mode validation rejects as unusable
- **THEN** the wizard rejects the value or the final validation exits 2
- **AND** no partial install files are written before validation succeeds

### Requirement: Wizard confirms committed live config before writing

Before live init writes files, the wizard SHALL show a confirmation summary of the final live bootstrap values that will be committed to `scryrs.json remote`. The summary SHALL distinguish committed shared config from local overrides and SHALL require explicit confirmation before continuing.

#### Scenario: Confirmation accepted continues live init

- **WHEN** the live-init wizard has collected missing required values
- **AND** the user confirms the final summary
- **THEN** the installer proceeds with the existing live-mode validation and write flow
- **AND** successful init writes `scryrs.json remote` with `ingest_url`, `workspace_id`, and `docker_network`

#### Scenario: Confirmation rejected writes nothing

- **WHEN** the live-init wizard has collected missing required values
- **AND** the user rejects the final summary or cancels the wizard
- **THEN** the command exits 2
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

### Requirement: Wizard output is human-oriented and machine output remains stable

Interactive wizard output SHALL be human-oriented stdout/stderr text only for terminal sessions. Machine-readable surfaces such as `scryrs --help-json` SHALL document the `--no-interactive` flag but SHALL NOT depend on prompt rendering.

#### Scenario: Help-json documents no-interactive without prompt transcript

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `init` command entry includes `--no-interactive`
- **AND** the help-json output does not include an interactive prompt transcript
