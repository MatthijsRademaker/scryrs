## MODIFIED Requirements

### Requirement: init installs the trace hook and the agent context skill

`scryrs init --agent <NAME>` SHALL install both the trace hook and the scryrs agent context skill for the named harness. Installation SHALL remain idempotent and config-free: `init` SHALL NOT write `scryrs.json` and SHALL NOT create the `.scryrs/` scaffold. `scryrs setup <local|live>` SHALL remain the only command that writes `scryrs.json` and the `.scryrs/` scaffold.

The installed skill copy SHALL be non-canonical. The canonical skill source SHALL live in the repository, and the installed copy SHALL NOT be treated as the leading source or edited directly by agents — matching the ownership model already documented for the Pi trace hook.

#### Scenario: init installs both hook and skill

- **WHEN** `scryrs init --agent <NAME>` runs for a supported harness
- **THEN** the trace hook is installed
- **AND** the agent context skill is installed into the harness's skill location

#### Scenario: init remains config-free

- **WHEN** `scryrs init --agent <NAME>` runs
- **THEN** no `scryrs.json` is written
- **AND** no `.scryrs/` scaffold is created

#### Scenario: init remains idempotent

- **WHEN** `scryrs init --agent <NAME>` runs twice
- **THEN** the second run leaves exactly one installed hook and one installed skill
- **AND** it exits 0

#### Scenario: setup keeps sole ownership of configuration

- **WHEN** trace transport is configured
- **THEN** only `scryrs setup <local|live>` writes `scryrs.json` and the `.scryrs/` scaffold

#### Scenario: Installed skill copy is non-canonical

- **WHEN** the installed skill copy is inspected
- **THEN** it is identifiable as an installed runtime artifact rather than the canonical source

#### Scenario: A locally modified installed skill is not silently overwritten

- **GIVEN** an installed skill copy has been modified locally
- **WHEN** `scryrs init --agent <NAME>` runs again
- **THEN** the local modification is not silently discarded

#### Scenario: init help states both installed artifacts

- **WHEN** `scryrs init --help` and `scryrs --help-json` are read
- **THEN** they state that `init` installs the trace hook and the agent context skill

### Requirement: doctor reports skill install status

`scryrs doctor` SHALL report agent context skill install status alongside the existing hook status report.

#### Scenario: doctor reports an installed skill

- **GIVEN** the agent context skill is installed
- **WHEN** `scryrs doctor` runs
- **THEN** it reports the skill as installed

#### Scenario: doctor reports a missing skill

- **GIVEN** the trace hook is installed but the agent context skill is not
- **WHEN** `scryrs doctor` runs
- **THEN** it reports the missing skill as an actionable finding

#### Scenario: doctor --json includes skill status

- **WHEN** `scryrs doctor --json` runs
- **THEN** the JSON output includes the skill install status
