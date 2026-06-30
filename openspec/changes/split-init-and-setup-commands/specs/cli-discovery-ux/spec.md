## MODIFIED Requirements

### Requirement: Help text documents workspace live bootstrap commands

The `scryrs --help` output SHALL document the workspace-local live bootstrap workflow as a two-step flow: `scryrs init --agent <NAME>` installs the harness hook, and `scryrs setup live` configures live transport. The help SHALL include the `setup` command and SHALL show `scryrs up` as the follow-up step for operators who self-host the managed live server (the `setup live` compose opt-in). It SHALL NOT present `scryrs init` as taking a `--mode` or live remote-configuration inputs.

#### Scenario: Help output lists `scryrs setup`

- **WHEN** `scryrs --help` is invoked
- **THEN** the command list includes `scryrs setup`
- **AND** the description states that it configures local or live trace transport

#### Scenario: Help output lists `scryrs up`

- **WHEN** `scryrs --help` is invoked
- **THEN** the command list includes `scryrs up`
- **AND** the description states that it starts the workspace-managed live-server Compose stack

#### Scenario: Help output shows the init-then-setup live flow

- **WHEN** `scryrs --help` is invoked
- **THEN** the examples or usage guidance show a live bootstrap flow of `scryrs init --agent <NAME>` followed by `scryrs setup live`
- **AND** that flow shows `scryrs up` as the follow-up step for self-hosting the managed server
- **AND** the guidance does not show `scryrs init` with a `--mode` or live remote-configuration inputs
