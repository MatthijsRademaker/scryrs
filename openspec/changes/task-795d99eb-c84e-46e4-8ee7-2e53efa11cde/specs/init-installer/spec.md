## ADDED Requirements

### Requirement: Init supports explicit local and live setup modes

The `init` subcommand SHALL accept `--mode local|live` with `local` as the default. Local mode SHALL preserve the existing local-install flow. Live mode SHALL use deterministic CLI inputs for `ingest_url`, `workspace_id`, `agent_id`, and optional `repository_id`; when `repository_id` is omitted, the installer SHALL derive it from Git remote origin using the normalized repository identity contract required by live ingest. Missing or invalid live-mode configuration SHALL exit 2 before the installer writes hook files, `.scryrs/` artifacts, or `scryrs.json`.

#### Scenario: Default init remains local

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` is invoked without `--mode`
- **THEN** the installer behaves as the existing local-mode path
- **AND** no remote configuration is required
- **AND** no network-specific next-step text is emitted

#### Scenario: Live mode validates remote identity before writing

- **WHEN** `scryrs init --agent pi --mode live` is invoked with complete live-mode inputs
- **THEN** the installer validates the remote identity set before writing any files
- **AND** the command succeeds only after the full live configuration is valid

#### Scenario: Invalid live mode fails without partial writes

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked with a missing or empty required live-mode field
- **THEN** the exit code is 2
- **AND** stderr contains deterministic validation guidance
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

### Requirement: Live-mode init writes project remote config without local-store scaffolding

In live mode, the installer SHALL install the same harness hook transport, create `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` under the target project, and create-or-merge only the `remote` section of the target project's `scryrs.json`. It SHALL preserve unrelated manifest keys. It SHALL NOT create `.scryrs/scryrs.db`, SHALL NOT dual-write, and SHALL NOT add direct HTTP logic to hook artifacts.

#### Scenario: Live mode merges into an existing manifest

- **GIVEN** a target project already has a `scryrs.json` file with unrelated top-level keys
- **WHEN** `scryrs init --agent claude-code --mode live` succeeds
- **THEN** the installer updates only the `remote` section
- **AND** unrelated manifest keys remain unchanged

#### Scenario: Live mode skips local trace-store creation

- **WHEN** `scryrs init --agent pi --mode live` succeeds
- **THEN** `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` exist under the target project
- **AND** `.scryrs/scryrs.db` does not exist

## MODIFIED Requirements

### Requirement: Init subcommand is discoverable via CLI dispatch, help, and help-json

The `init` subcommand SHALL remain accessible through CLI dispatch, help text, and help-json, and those discovery surfaces SHALL describe both local and live setup. The machine-readable `init` entry SHALL include `--agent`, `--mode`, and the live-mode remote configuration arguments.

#### Scenario: Help output documents the mode choice

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the `init` command
- **AND** it documents `--mode local|live`
- **AND** it describes the live-mode remote configuration inputs

#### Scenario: Help-json exposes live init arguments

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array contains an `init` entry
- **AND** that entry includes `--agent` and `--mode`
- **AND** it includes the live-mode remote configuration arguments in the machine-readable surface

### Requirement: Self-install into scryrs source checkout is refused

The installer SHALL continue to detect the scryrs source checkout by ancestry markers. `pi` local-mode installation inside the source checkout remains permitted for dogfooding. Live mode SHALL be refused inside the scryrs source checkout because live init configures consumer-project remote state.

#### Scenario: Local-mode Pi dogfooding remains allowed

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent pi --mode local` is invoked
- **THEN** installation proceeds against `.pi/extensions/pi-trace/index.ts` at the checkout root

#### Scenario: Live mode is refused in the source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent pi --mode live` is invoked
- **THEN** the exit code is 2
- **AND** stderr explains that live mode must target a consumer project rather than the scryrs source repository
- **AND** no `scryrs.json` changes are written in the source checkout

### Requirement: Successful install prints deterministic next-step text

On successful installation, the installer SHALL print deterministic, mode-specific next-step instructions to stdout. Local mode SHALL keep the current local guidance. Live mode SHALL explain that the project is configured for remote ingest, identify the configured live-server endpoint, and describe the server/Docker follow-up needed to start or connect the shared service.

#### Scenario: Local mode keeps current next steps

- **WHEN** `scryrs init --agent claude-code --mode local` completes successfully
- **THEN** stdout contains the existing local next-step guidance
- **AND** it does not mention Docker or a remote ingest URL

#### Scenario: Live mode prints remote-server next steps

- **WHEN** `scryrs init --agent claude-code --mode live` completes successfully
- **THEN** stdout states that live remote ingest was configured
- **AND** stdout includes the remote server endpoint for the project
- **AND** stdout includes deterministic next steps for starting or connecting to the shared server, including the documented Docker workflow

### Requirement: Installer does not create or depend on scryrs.json

The installer SHALL remain manifest-agnostic in local mode. In live mode only, it MAY create or merge the target project's `scryrs.json` `remote` section as part of remote setup. It SHALL NOT read or depend on `scryrs.json` for ordinary local installation behavior.

#### Scenario: Local mode still does not create a manifest

- **WHEN** `scryrs init --agent <name> --mode local` completes successfully
- **THEN** no `scryrs.json` file is created or modified

#### Scenario: Live mode may create the remote manifest section

- **WHEN** `scryrs init --agent <name> --mode live` completes successfully in a project without `scryrs.json`
- **THEN** the installer creates `scryrs.json` at the project root
- **AND** the file contains the configured `remote` section needed for live ingest
- **AND** no unrelated manifest structure is required for success