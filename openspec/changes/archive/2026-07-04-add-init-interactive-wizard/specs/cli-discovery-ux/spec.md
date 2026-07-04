## MODIFIED Requirements

### Requirement: Help text documents workspace live bootstrap commands

The `scryrs --help` output SHALL document the workspace-local live bootstrap workflow, including the `scryrs up` command, the live init inputs needed to scaffold a managed external-network live server, the default interactive wizard behavior for missing live inputs, and the `--no-interactive` opt-out for promptless automation.

#### Scenario: Help output lists `scryrs up`

- **WHEN** `scryrs --help` is invoked
- **THEN** the command list includes `scryrs up`
- **AND** the description states that it starts the workspace-managed live-server Compose stack

#### Scenario: Help output includes live bootstrap example

- **WHEN** `scryrs --help` is invoked
- **THEN** the examples or usage guidance show a live bootstrap flow that includes `scryrs init --agent <NAME>` with external-network configuration
- **AND** that flow shows `scryrs up` as the follow-up step to start the managed server

#### Scenario: Help output documents interactive live init

- **WHEN** `scryrs --help` is invoked
- **THEN** the `init` guidance explains that missing live bootstrap values may be collected by an interactive wizard when terminal IO is available
- **AND** the guidance explains that `--no-interactive` disables prompts and preserves fail-fast validation

#### Scenario: Help-json exposes no-interactive flag

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array contains an `init` entry
- **AND** that entry includes a `--no-interactive` argument
- **AND** the argument description states that prompts are disabled and missing live config fails fast
