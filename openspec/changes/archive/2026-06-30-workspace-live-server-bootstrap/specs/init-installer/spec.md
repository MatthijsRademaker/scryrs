## MODIFIED Requirements

### Requirement: Init supports explicit local and live setup modes

The `init` subcommand SHALL accept `--mode local|live` with `live` as the default. Local mode SHALL preserve the existing local-install flow. Live mode SHALL use deterministic CLI inputs for `ingest_url`, `workspace_id`, `agent_id`, required `docker_network`, and optional `repository_id`; when `repository_id` is omitted, the installer SHALL derive it from Git remote origin using the normalized repository identity contract required by live ingest. Missing or invalid live-mode configuration SHALL exit 2 before the installer writes hook files, `.scryrs/` artifacts, or `scryrs.json`.

#### Scenario: Default init validates complete live bootstrap inputs before writing

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` is invoked without `--mode`
- **THEN** the installer resolves live-mode bootstrap inputs
- **AND** the command succeeds only after `ingest_url`, `workspace_id`, `agent_id`, and `docker_network` are all valid
- **AND** no partial files are written before that validation completes

#### Scenario: Invalid live mode fails without partial writes

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked with a missing or empty required live-mode field
- **THEN** the exit code is 2
- **AND** stderr contains deterministic validation guidance
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

#### Scenario: Local mode remains an explicit opt-in

- **WHEN** `scryrs init --agent pi --mode local` or `scryrs init --agent claude-code --mode local` is invoked
- **THEN** the installer behaves as the existing local-mode path
- **AND** no live bootstrap scaffold is required
- **AND** no network-specific next-step text is emitted

### Requirement: Live-mode init writes project remote config without local-store scaffolding

In live mode, the installer SHALL install the same harness hook transport, create `.scryrs/`, `.scryrs/.gitignore`, `.scryrs/hooks/`, `.scryrs/.env`, and `.scryrs/compose.yml` under the target project, and create-or-merge only the `remote` section of the target project's `scryrs.json`. It SHALL preserve unrelated manifest keys. It SHALL NOT create `.scryrs/scryrs.db`, SHALL NOT dual-write, and SHALL NOT add direct HTTP logic to hook artifacts.

#### Scenario: Live mode scaffolds managed env and compose files

- **WHEN** `scryrs init --agent pi --mode live` succeeds
- **THEN** `.scryrs/`, `.scryrs/.gitignore`, `.scryrs/hooks/`, `.scryrs/.env`, and `.scryrs/compose.yml` exist under the target project
- **AND** `.scryrs/scryrs.db` does not exist
- **AND** the scaffolded `.scryrs/.env` includes the resolved live ingest identity and Docker network value

#### Scenario: Live mode merges into an existing manifest

- **GIVEN** a target project already has a `scryrs.json` file with unrelated top-level keys
- **WHEN** `scryrs init --agent claude-code --mode live` succeeds
- **THEN** the installer updates only the `remote` section
- **AND** unrelated manifest keys remain unchanged
- **AND** the workspace-local `.scryrs/compose.yml` and `.scryrs/.env` are created or preserved as managed bootstrap files

### Requirement: Pre-existing target files trigger loud refusal

The installer SHALL preserve existing managed files deterministically. For file-based harness installs, identical existing managed content SHALL be treated as a successful no-op, while divergent existing content SHALL still fail loudly instead of being overwritten.

#### Scenario: Identical Pi runtime copy is accepted as no-op

- **GIVEN** `.pi/extensions/pi-trace/index.ts` already exists and is byte-identical to the embedded Pi hook source
- **WHEN** `scryrs init --agent pi` is invoked again
- **THEN** the exit code is 0
- **AND** the existing file content is preserved unchanged
- **AND** the installer continues with the rest of the requested init work

#### Scenario: Divergent Pi runtime copy is refused

- **GIVEN** `.pi/extensions/pi-trace/index.ts` already exists and differs from the embedded Pi hook source
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the exit code is 2
- **AND** stderr reports the existing path conflict with remediation guidance
- **AND** the divergent file is not overwritten

#### Scenario: Pi target directory already exists but target file does not

- **GIVEN** `.pi/extensions/pi-trace/` already exists but contains no `index.ts`
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer creates `index.ts` inside the existing directory
- **AND** the exit code is 0

### Requirement: Successful install prints deterministic next-step text

On successful installation, the installer SHALL print deterministic, mode-specific next-step instructions to stdout. Local mode SHALL keep the current local guidance. Live mode SHALL explain that the project is configured for remote ingest, identify the configured live-server endpoint, and direct the operator to launch the managed workspace compose stack with `scryrs up`.

#### Scenario: Local mode keeps current next steps

- **WHEN** `scryrs init --agent claude-code --mode local` completes successfully
- **THEN** stdout contains the existing local next-step guidance
- **AND** it does not mention Docker or a remote ingest URL

#### Scenario: Live mode prints workspace-bootstrap next steps

- **WHEN** `scryrs init --agent claude-code --mode live` completes successfully
- **THEN** stdout states that live remote ingest was configured
- **AND** stdout includes the remote server endpoint for the project
- **AND** stdout tells the operator to start the managed live server with `scryrs up`
- **AND** stdout does not tell the operator to check out the scryrs source repository and run the root `docker-compose.yml`
