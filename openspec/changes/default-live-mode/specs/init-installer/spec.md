## MODIFIED Requirements

### Requirement: Init supports explicit local and live setup modes

The `init` subcommand SHALL accept `--mode local|live` with `live` as the default. Live mode SHALL be the behavior when `--mode` is omitted. Live mode SHALL resolve `ingest_url`, `workspace_id`, `agent_id`, and optional `repository_id` through the shared remote-config resolution precedence (CLI flags, then process environment, then `.scryrs/.env`, then `scryrs.json` `remote`); when `repository_id` is omitted, the installer SHALL derive it from Git remote origin using the normalized repository identity contract required by live ingest. Local mode SHALL preserve the existing local-install flow and SHALL be selected only by explicit `--mode local`. Missing or invalid live-mode configuration SHALL exit `2` before the installer writes hook files, `.scryrs/` artifacts, or `scryrs.json`, and SHALL print deterministic guidance describing how to populate `.scryrs/.env` and how to select local mode.

#### Scenario: Default init is live

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` is invoked without `--mode`
- **THEN** the installer runs the live-mode path
- **AND** remote configuration is resolved through the shared precedence chain

#### Scenario: Local mode requires explicit opt-in

- **WHEN** `scryrs init --agent claude-code --mode local` is invoked
- **THEN** the installer behaves as the existing local-mode path
- **AND** no remote configuration is required
- **AND** no network-specific next-step text is emitted

#### Scenario: Live default with complete config validates before writing

- **WHEN** `scryrs init --agent pi` is invoked and complete live configuration resolves from `.scryrs/.env`, environment, or flags
- **THEN** the installer validates the remote identity set before writing any files
- **AND** the command succeeds only after the full live configuration is valid

#### Scenario: Live default without resolvable config fails with guidance

- **WHEN** `scryrs init --agent claude-code` is invoked and a required live-mode field cannot be resolved from any layer
- **THEN** the exit code is `2`
- **AND** stderr names the missing configuration and describes populating `.scryrs/.env` and selecting `--mode local`
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

### Requirement: Live-mode init writes project remote config without local-store scaffolding

In live mode, the installer SHALL install the same harness hook transport, create `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` under the target project, create-or-merge only the `remote` section of the target project's `scryrs.json`, and create-or-merge a `.scryrs/.env` template containing the `SCRYRS_REMOTE_*` keys without overwriting any values an operator has already set. `.scryrs/.gitignore` SHALL cover `.env`. It SHALL preserve unrelated manifest keys. It SHALL NOT create `.scryrs/scryrs.db`, SHALL NOT dual-write, and SHALL NOT add direct HTTP logic to hook artifacts.

#### Scenario: Live mode merges into an existing manifest

- **GIVEN** a target project already has a `scryrs.json` file with unrelated top-level keys
- **WHEN** `scryrs init --agent claude-code` succeeds in the live default
- **THEN** the installer updates only the `remote` section
- **AND** unrelated manifest keys remain unchanged

#### Scenario: Live mode scaffolds a gitignored env template without clobbering values

- **WHEN** `scryrs init --agent pi` succeeds in the live default
- **THEN** `.scryrs/.env` exists with the `SCRYRS_REMOTE_*` keys
- **AND** `.scryrs/.gitignore` covers `.env`
- **AND** any pre-existing `.scryrs/.env` values are preserved

#### Scenario: Live mode skips local trace-store creation

- **WHEN** `scryrs init --agent pi` succeeds in the live default
- **THEN** `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` exist under the target project
- **AND** `.scryrs/scryrs.db` does not exist

### Requirement: Self-install into scryrs source checkout is refused

The installer SHALL continue to detect the scryrs source checkout by ancestry markers. `pi` local-mode installation inside the source checkout remains permitted for dogfooding. Live mode SHALL be refused inside the scryrs source checkout because live init configures consumer-project remote state. Because live is the default, a bare `scryrs init` invoked inside the source checkout SHALL emit the live-mode refusal and SHALL direct the operator to run `--mode local` (Pi) from the source checkout or run live init from a consumer project.

#### Scenario: Local-mode Pi dogfooding remains allowed

- **GIVEN** the current directory is inside the scryrs source checkout
- **WHEN** `scryrs init --agent pi --mode local` is invoked
- **THEN** the installer proceeds with local-mode Pi installation

#### Scenario: Live default is refused in the source checkout with guidance

- **GIVEN** the current directory is inside the scryrs source checkout
- **WHEN** `scryrs init --agent pi` is invoked without `--mode`
- **THEN** the exit code is `2`
- **AND** stderr explains that live mode is not allowed in the scryrs source repository
- **AND** stderr directs the operator to use `--mode local` here or run live init from a consumer project
