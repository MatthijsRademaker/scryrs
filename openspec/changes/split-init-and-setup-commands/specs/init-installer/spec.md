## MODIFIED Requirements

### Requirement: Init subcommand is discoverable via CLI dispatch, help, and help-json

The `init` subcommand SHALL remain accessible through CLI dispatch, help text, and help-json. As a hook-install-only command, its discovery surfaces SHALL describe only harness hook installation via `--agent`. The machine-readable `init` entry SHALL include `--agent` and SHALL NOT include `--mode` or any live-mode remote-configuration arguments. Help SHALL direct operators to `scryrs setup <mode>` for transport configuration.

#### Scenario: Help documents init as hook install only

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the `init` command described as harness hook installation
- **AND** it does not document a `--mode` option on `init`
- **AND** it points to `scryrs setup` for local/live transport configuration

#### Scenario: Help-json exposes init with --agent only

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array contains an `init` entry
- **AND** that entry includes `--agent`
- **AND** that entry does not include `--mode` or live-mode remote-configuration arguments

### Requirement: Installer does not create or depend on scryrs.json

The `init` installer SHALL be manifest-agnostic in all cases. It SHALL NOT read, create, or modify `scryrs.json`, and SHALL NOT create the `.scryrs/` config scaffold. All transport configuration (the `scryrs.json` `remote` section and `.scryrs/` scaffolding) is owned exclusively by `scryrs setup`.

#### Scenario: init never creates a manifest

- **WHEN** `scryrs init --agent <name>` completes successfully
- **THEN** no `scryrs.json` file is created or modified

#### Scenario: init never creates the .scryrs scaffold

- **WHEN** `scryrs init --agent <name>` completes successfully
- **THEN** no `.scryrs/` directory, store, or config files are created by `init`

### Requirement: Successful install prints deterministic next-step text

On successful installation, the `init` installer SHALL print deterministic, hook-focused next-step instructions to stdout. The text SHALL confirm the hook was installed, direct the operator to configure transport with `scryrs setup <mode>` (when they have not already), and to reload their agent harness. It SHALL NOT print mode-specific (local vs live) configuration guidance, a remote ingest URL, or `scryrs up` guidance — those belong to `scryrs setup`.

#### Scenario: init prints hook-focused next steps

- **WHEN** `scryrs init --agent pi` completes successfully
- **THEN** stdout confirms the Pi hook was installed
- **AND** stdout directs the operator to run `scryrs setup <mode>` to configure transport
- **AND** stdout does not include a remote ingest URL or `scryrs up` guidance

### Requirement: Self-install into scryrs source checkout is refused

The `init` installer SHALL continue to detect the scryrs source checkout by ancestry markers. `pi` hook installation inside the source checkout SHALL remain permitted for dogfooding (writing `.pi/extensions/scryrs/index.ts` at the resolved checkout root). Claude Code consumer config installation SHALL remain refused inside the source checkout. `init` no longer has a live mode, so it carries no live-mode refusal; refusal of live transport setup inside the source checkout is owned by `scryrs setup live`.

#### Scenario: Pi hook dogfooding remains allowed in source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** installation proceeds against `.pi/extensions/scryrs/index.ts` at the checkout root

#### Scenario: Claude Code consumer config remains refused in source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** no `.claude/` consumer config files are created or modified inside the source checkout

## REMOVED Requirements

### Requirement: Init supports explicit local and live setup modes

**Reason**: Mode selection and live-config validation are moved out of `init` into the new `scryrs setup <mode>` command. `init` is now hook-install only and has no `--mode`.

**Migration**: Replace `scryrs init --agent <name> --mode local` with `scryrs init --agent <name>` followed by `scryrs setup local`. Replace `scryrs init --agent <name> --mode live --ingest-url ... --workspace-id ...` with `scryrs init --agent <name>` followed by `scryrs setup live --ingest-url ... --workspace-id ...`. See the `setup-command` capability for the new validation contract.

### Requirement: Live-mode init writes project remote config without local-store scaffolding

**Reason**: Writing the `scryrs.json` `remote` section and the live `.scryrs/` scaffold is now the responsibility of `scryrs setup live`, not `init`. The compose/`docker_network` scaffold also becomes opt-in rather than mandatory.

**Migration**: Use `scryrs setup live` to write `scryrs.json` remote config (`ingest_url` + `workspace_id` required; `docker_network`/compose now opt-in). See the `setup-command` and `workspace-live-bootstrap` capabilities for the relocated and updated behavior.
