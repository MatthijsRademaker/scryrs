## ADDED Requirements

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
