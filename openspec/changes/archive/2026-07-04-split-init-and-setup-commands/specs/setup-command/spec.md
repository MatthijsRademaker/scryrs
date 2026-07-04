## ADDED Requirements

### Requirement: `scryrs setup <mode>` configures runtime transport separately from hook install

The CLI SHALL provide a native `scryrs setup <mode>` command that scaffolds runtime trace-transport configuration, where `mode` is a required positional argument accepting `local` or `live`. `setup` SHALL be independent of `init`: it SHALL NOT install or require any harness hook, and `init` SHALL NOT be a prerequisite for running `setup`. `setup` SHALL be the only command that writes `scryrs.json` and the `.scryrs/` config scaffold; `init` SHALL NOT write either.

#### Scenario: setup requires an explicit mode

- **WHEN** `scryrs setup` is invoked without a mode argument
- **THEN** the exit code is 2
- **AND** stderr states that a mode (`local` or `live`) is required
- **AND** no files are written

#### Scenario: unknown mode fails loudly

- **WHEN** `scryrs setup remote` (or any unsupported mode) is invoked
- **THEN** the exit code is 2
- **AND** stderr lists the supported modes (`live`, `local`) in stable order
- **AND** no files are written

#### Scenario: setup runs without a prior init

- **GIVEN** no harness hook has been installed in the target project
- **WHEN** `scryrs setup local` is invoked
- **THEN** the command succeeds based only on its own config-scaffolding work
- **AND** it does not require or install any hook

### Requirement: `scryrs setup local` scaffolds the local store only

`scryrs setup local` SHALL scaffold the single-machine local trace store under the target project's `.scryrs/` directory (`.scryrs/scryrs.db` and `.scryrs/.gitignore`), preserving any existing store. It SHALL remain manifest-agnostic: it SHALL NOT create or modify `scryrs.json`, and SHALL NOT require or emit any remote/network configuration.

#### Scenario: local setup creates the store

- **WHEN** `scryrs setup local` completes successfully in a project with no prior `.scryrs/`
- **THEN** `.scryrs/scryrs.db` and `.scryrs/.gitignore` exist under the project
- **AND** no `scryrs.json` is created or modified
- **AND** stdout next-step text does not mention Docker or a remote ingest URL

#### Scenario: local setup is idempotent

- **GIVEN** a project that already has an initialized `.scryrs/scryrs.db`
- **WHEN** `scryrs setup local` is invoked again
- **THEN** the exit code is 0
- **AND** the existing store contents are preserved unchanged

### Requirement: `scryrs setup live` requires only ingest URL and workspace ID

`scryrs setup live` SHALL create-or-merge the target project's `scryrs.json` `remote` section as the committed source of truth for live ingest. The only required inputs SHALL be `ingest_url` and `workspace_id`, resolved via the standard precedence (`CLI flag > env > .scryrs/.env > scryrs.json remote`). `repository_id` SHALL be derived from the Git remote origin when not supplied and SHALL NOT be written to committed config; `agent_id` SHALL be left to runtime autogeneration and SHALL NOT be written to committed config. `docker_network` SHALL NOT be required by core `setup live`. Missing or invalid required inputs SHALL exit 2 before any files are written.

#### Scenario: live setup writes only the committed shared constants

- **WHEN** `scryrs setup live --ingest-url http://scryrs:8081 --workspace-id myproj` succeeds
- **THEN** `scryrs.json` `remote` contains `ingest_url` and `workspace_id`
- **AND** `scryrs.json` `remote` does not contain `repository_id` or `agent_id`
- **AND** the managed values are not duplicated into `.scryrs/.env`

#### Scenario: live setup does not require a docker network

- **WHEN** `scryrs setup live` is invoked with a valid `ingest_url` and `workspace_id` but no `docker_network`
- **THEN** the command succeeds
- **AND** no `docker_network` value is required and none is written to committed config

#### Scenario: missing required live input fails without partial writes

- **WHEN** `scryrs setup live` is invoked with a missing or empty `ingest_url` or `workspace_id` and no resolvable override
- **THEN** the exit code is 2
- **AND** stderr contains deterministic remediation guidance naming the missing field
- **AND** no `scryrs.json` or `.scryrs/` changes are written

#### Scenario: live setup merges into an existing manifest

- **GIVEN** a project `scryrs.json` with unrelated top-level keys
- **WHEN** `scryrs setup live` succeeds
- **THEN** only the `remote` section is updated
- **AND** unrelated manifest keys are preserved unchanged

#### Scenario: conflicting committed manifest value fails loudly

- **GIVEN** a project `scryrs.json` whose `remote.ingest_url` or `remote.workspace_id` conflicts with the resolved inputs
- **WHEN** `scryrs setup live` is invoked
- **THEN** the exit code is 2
- **AND** stderr reports the conflicting field with remediation guidance
- **AND** no partial files are written

### Requirement: Self-hosted live-server compose scaffolding is opt-in

`scryrs setup live` SHALL NOT scaffold `.scryrs/compose.yml` or require a `docker_network` value by default. Scaffolding the managed self-hosted live-server compose stack SHALL be an explicit opt-in (e.g. a `--with-compose` flag), and only that opt-in path SHALL require `docker_network`. Deployments that join an existing network or target an externally managed live server SHALL be able to complete `setup live` without compose scaffolding.

#### Scenario: core live setup does not scaffold compose

- **WHEN** `scryrs setup live` succeeds without the compose opt-in
- **THEN** `.scryrs/compose.yml` is not created
- **AND** no `docker_network` value is required

#### Scenario: compose opt-in scaffolds the managed stack and requires the network

- **WHEN** `scryrs setup live` is invoked with the compose opt-in and a resolvable `docker_network`
- **THEN** `.scryrs/compose.yml` is scaffolded for a server reachable to peers as `http://scryrs:8081`
- **AND** `remote.docker_network` is recorded for later `scryrs up`

#### Scenario: compose opt-in without a network fails loudly

- **WHEN** the compose opt-in is requested but no `docker_network` resolves from any layer
- **THEN** the exit code is 2
- **AND** stderr explains that compose scaffolding requires a Docker network

### Requirement: `scryrs setup live` is refused inside the scryrs source checkout

`scryrs setup live` SHALL refuse to run inside the scryrs source checkout, because live setup configures consumer-project remote state. `scryrs setup local` SHALL remain permitted inside the source checkout for dogfooding.

#### Scenario: live setup refused in source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs setup live` is invoked
- **THEN** the exit code is 2
- **AND** stderr explains that live setup must target a consumer project, not the scryrs source repository
- **AND** no `scryrs.json` changes are written in the source checkout

#### Scenario: local setup dogfooding remains allowed in source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs setup local` is invoked
- **THEN** the local store scaffolding proceeds at the checkout root

### Requirement: Interactive live-config collection belongs to `setup live`, not `init`

When interactive collection of missing live configuration is available, it SHALL occur in `scryrs setup live` and SHALL NOT occur in `scryrs init`. `scryrs init` SHALL never prompt, because it collects no configuration. Non-interactive `setup live` (no TTY, or an explicit non-interactive opt-out) SHALL preserve deterministic exit-2 fail-fast on missing required inputs.

#### Scenario: init never prompts

- **WHEN** `scryrs init --agent pi` is invoked in an interactive terminal with no live config present
- **THEN** the command installs the hook and exits 0 without prompting for any config

#### Scenario: non-interactive live setup fails fast rather than prompting

- **WHEN** `scryrs setup live` is invoked without a TTY (or with the non-interactive opt-out) and required inputs are missing
- **THEN** the exit code is 2
- **AND** no interactive prompt is shown
- **AND** stderr contains deterministic remediation guidance

### Requirement: `scryrs setup` is discoverable via dispatch, help, and help-json

The `setup` command SHALL be reachable through CLI dispatch and SHALL appear in `scryrs --help` and `scryrs --help-json`. The machine-readable entry SHALL document the required `mode` positional and the live-mode inputs (`ingest_url`, `workspace_id`, and the compose opt-in inputs).

#### Scenario: help lists setup with its modes

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the `setup` command
- **AND** it documents the `local` and `live` modes

#### Scenario: help-json exposes setup inputs

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array contains a `setup` entry
- **AND** that entry documents the required `mode` positional and the live-mode configuration inputs
