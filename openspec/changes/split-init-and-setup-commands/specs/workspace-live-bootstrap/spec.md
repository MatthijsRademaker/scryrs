## MODIFIED Requirements

### Requirement: Workspace live bootstrap is scaffolded under `.scryrs/`

Live-mode consumer bootstrap SHALL be workspace-local and SHALL be triggered by `scryrs setup live` (no longer by `scryrs init --mode live`). When `scryrs setup live` succeeds, the installer SHALL write committed live config (transport identity) into the project's `scryrs.json` as the single source of truth. Scaffolding the managed self-hosted runtime files (`.scryrs/compose.yml`, the overrides-only `.scryrs/.env`, the Docker network name) SHALL be an explicit opt-in for operators who self-host the live server via `scryrs up`; it SHALL NOT be a mandatory part of core `setup live`. The scaffolded `.scryrs/.env`, when created, SHALL exist as an overrides-only file and SHALL NOT be the source of truth for managed bootstrap values.

#### Scenario: Core live setup writes committed identity without compose scaffolding

- **WHEN** `scryrs setup live` succeeds in a consumer project without the compose opt-in
- **THEN** `scryrs.json` carries the committed live identity (`remote.ingest_url`, `remote.workspace_id`)
- **AND** `.scryrs/compose.yml` is not created and no `remote.docker_network` is required
- **AND** the operator is not required to copy or invoke the scryrs repository's root `docker-compose.yml`

#### Scenario: Compose opt-in creates managed workspace bootstrap files

- **WHEN** `scryrs setup live` is invoked with the self-host compose opt-in and a resolvable Docker network
- **THEN** `.scryrs/compose.yml` and `.scryrs/.env` exist under that project
- **AND** `scryrs.json` carries the committed live identity and `remote.docker_network`
- **AND** the workspace-local scaffold plus committed manifest are sufficient input for later `scryrs up`

#### Scenario: Second harness install reuses committed bootstrap

- **GIVEN** a consumer project already has a successful live bootstrap with committed `scryrs.json` live config
- **WHEN** `scryrs init --agent <other-supported-harness>` is run to install an additional hook
- **THEN** the existing committed `scryrs.json` live config and any managed `.scryrs/compose.yml` are preserved
- **AND** the overrides-only `.scryrs/.env` is not rewritten with managed values
- **AND** only the newly requested harness hook installation work is applied

### Requirement: `scryrs up` is a thin workspace-local compose launcher

The `scryrs up` command SHALL start the live server from the workspace-managed `.scryrs/compose.yml` only, scaffolded by `scryrs setup live --with-compose` (the self-host opt-in). It SHALL resolve the external Docker network name from the precedence chain `CLI flag > SCRYRS_DOCKER_NETWORK env > .scryrs/.env > scryrs.json remote.docker_network`, and SHALL inject the resolved value into the Compose process environment so that the compose file's `${SCRYRS_DOCKER_NETWORK}` substitution resolves. It SHALL not install hooks, infer or rewrite runtime identity, or mutate unrelated workspace state.

#### Scenario: `scryrs up` launches with the network resolved from the manifest

- **GIVEN** a consumer project contains a valid `.scryrs/compose.yml` and a `scryrs.json` declaring `remote.docker_network`, with no overriding `.scryrs/.env` or environment value
- **WHEN** `scryrs up` is invoked from that project
- **THEN** the command resolves the external network name from `scryrs.json` `remote.docker_network`
- **AND** it launches the compose stack with `SCRYRS_DOCKER_NETWORK` set to the resolved value
- **AND** it uses the workspace-managed bootstrap files rather than the scryrs repository root artifacts

#### Scenario: `scryrs up` fails loudly when the compose scaffold is missing

- **WHEN** `scryrs up` is invoked in a project where no managed `.scryrs/compose.yml` exists (the self-host compose opt-in was never run)
- **THEN** the command exits non-zero with deterministic remediation guidance pointing at `scryrs setup live --with-compose`
- **AND** it does not attempt unrelated hook installation or live-config mutation
