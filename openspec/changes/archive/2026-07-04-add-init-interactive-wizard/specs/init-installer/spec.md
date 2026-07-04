## MODIFIED Requirements

### Requirement: Init supports explicit local and live setup modes

The `init` subcommand SHALL accept `--mode local|live` with `live` as the default. Local mode SHALL preserve the existing local-install flow. Live mode SHALL use deterministic CLI inputs, environment variables, `.scryrs/.env`, existing `scryrs.json remote`, or interactive wizard answers for `ingest_url`, `workspace_id`, and required `docker_network`, with optional `repository_id` and optional `agent_id`. When `repository_id` is omitted, the installer SHALL derive it from Git remote origin using the normalized repository identity contract required by live ingest. When `agent_id` is omitted, the installer SHALL NOT require it and SHALL leave it to runtime autogeneration; `agent_id` is not a required live-mode input. Missing or invalid live-mode configuration SHALL exit 2 before the installer writes hook files, `.scryrs/` artifacts, or `scryrs.json` when `--no-interactive` is present, when terminal IO is unavailable, when the user cancels the wizard, or when validation still fails after wizard input.

#### Scenario: Default init validates or collects complete live bootstrap inputs before writing

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` is invoked without `--mode`
- **THEN** the installer resolves live-mode bootstrap inputs
- **AND** the command succeeds only after `ingest_url`, `workspace_id`, and `docker_network` are all valid and `repository_id` is resolvable
- **AND** the command may collect missing required live values through the interactive wizard when terminal IO is available and `--no-interactive` is not present
- **AND** no partial files are written before validation completes

#### Scenario: agent_id is not a required live-mode input

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked without an `agent_id` value
- **THEN** the command does not fail for a missing `agent_id`
- **AND** no `agent_id` is written into committed config
- **AND** `agent_id` is left to runtime autogeneration

#### Scenario: Invalid live mode fails without partial writes in promptless mode

- **WHEN** `scryrs init --agent claude-code --mode live --no-interactive` is invoked with a missing or empty required live-mode field (`ingest_url`, `workspace_id`, or `docker_network`)
- **THEN** the exit code is 2
- **AND** stderr contains deterministic validation guidance
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

#### Scenario: Invalid live mode fails without partial writes when prompting is unavailable

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked with a missing or empty required live-mode field (`ingest_url`, `workspace_id`, or `docker_network`)
- **AND** stdin or stdout is not a terminal
- **THEN** the exit code is 2
- **AND** stderr contains deterministic validation guidance
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

#### Scenario: Local mode remains an explicit opt-in

- **WHEN** `scryrs init --agent pi --mode local` or `scryrs init --agent claude-code --mode local` is invoked
- **THEN** the installer behaves as the existing local-mode path
- **AND** no live bootstrap scaffold is required
- **AND** no network-specific next-step text is emitted

#### Scenario: No-interactive flag is accepted for local mode without changing local behavior

- **WHEN** `scryrs init --agent pi --mode local --no-interactive` is invoked
- **THEN** the installer behaves as the existing local-mode path
- **AND** no live-init wizard is started
- **AND** no `scryrs.json` file is created or modified
