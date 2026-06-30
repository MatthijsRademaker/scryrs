## MODIFIED Requirements

### Requirement: Workspace live bootstrap is scaffolded under `.scryrs/`

Live-mode consumer bootstrap SHALL be workspace-local. When `scryrs init --agent <name>` succeeds in live mode, the installer SHALL scaffold managed runtime files under the target project's `.scryrs/` directory instead of requiring the operator to check out the scryrs source repository and run the repo-root Compose stack there. Committed live config (transport identity and the Docker network name) SHALL be the single source of truth in the project's `scryrs.json`; the scaffolded `.scryrs/.env` SHALL exist as an overrides-only file and SHALL NOT be the source of truth for managed bootstrap values.

#### Scenario: Live init creates managed workspace bootstrap files

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` succeeds in a consumer project in live mode
- **THEN** `.scryrs/compose.yml` and `.scryrs/.env` exist under that project
- **AND** `scryrs.json` carries the committed live identity and `remote.docker_network`
- **AND** the workspace-local scaffold plus committed manifest are sufficient input for later `scryrs up`
- **AND** the operator is not required to copy or invoke the scryrs repository's root `docker-compose.yml`

#### Scenario: Second harness install reuses committed bootstrap

- **GIVEN** a consumer project already has a successful live bootstrap with committed `scryrs.json` live config and a managed `.scryrs/compose.yml`
- **WHEN** `scryrs init --agent <other-supported-harness>` is run with equivalent live bootstrap values
- **THEN** the existing committed `scryrs.json` live config and managed `.scryrs/compose.yml` are preserved
- **AND** the overrides-only `.scryrs/.env` is not rewritten with managed values
- **AND** only the newly requested harness installation work is applied

### Requirement: `scryrs up` is a thin workspace-local compose launcher

The `scryrs up` command SHALL start the live server from the workspace-managed `.scryrs/compose.yml` only. It SHALL resolve the external Docker network name from the precedence chain `CLI flag > SCRYRS_DOCKER_NETWORK env > .scryrs/.env > scryrs.json remote.docker_network`, and SHALL inject the resolved value into the Compose process environment so that the compose file's `${SCRYRS_DOCKER_NETWORK}` substitution resolves. It SHALL not install hooks, infer or rewrite runtime identity, or mutate unrelated workspace state.

#### Scenario: `scryrs up` launches with the network resolved from the manifest

- **GIVEN** a consumer project contains a valid `.scryrs/compose.yml` and a `scryrs.json` declaring `remote.docker_network`, with no overriding `.scryrs/.env` or environment value
- **WHEN** `scryrs up` is invoked from that project
- **THEN** the command resolves the external network name from `scryrs.json` `remote.docker_network`
- **AND** it launches the compose stack with `SCRYRS_DOCKER_NETWORK` set to the resolved value
- **AND** it uses the workspace-managed bootstrap files rather than the scryrs repository root artifacts

#### Scenario: Override layers take precedence over the committed network

- **GIVEN** a project whose `scryrs.json` declares one `remote.docker_network` value
- **AND** `.scryrs/.env` or `SCRYRS_DOCKER_NETWORK` provides a different value
- **WHEN** `scryrs up` resolves the external network
- **THEN** the higher-precedence override value is used for the Compose launch

#### Scenario: `scryrs up` fails loudly when the network cannot be resolved

- **WHEN** `scryrs up` is invoked in a project where no external network value resolves from any layer, or the managed compose scaffold is missing
- **THEN** the command exits non-zero with deterministic remediation guidance
- **AND** it does not invoke Compose with an unresolved `${SCRYRS_DOCKER_NETWORK}`
- **AND** it does not attempt unrelated hook installation or live-config mutation
