## MODIFIED Requirements

### Requirement: Workspace live bootstrap is scaffolded under `.scryrs/`

Live-mode consumer bootstrap SHALL be workspace-local. When `scryrs init --agent <name>` succeeds in live mode, whether configured by flags/env/manifests or by the interactive live-init wizard, the installer SHALL scaffold managed runtime files under the target project's `.scryrs/` directory instead of requiring the operator to check out the scryrs source repository and run the repo-root Compose stack there. Committed live config (transport identity and the Docker network name) SHALL be the single source of truth in the project's `scryrs.json`; the scaffolded `.scryrs/.env` SHALL exist as an overrides-only file and SHALL NOT be the source of truth for managed bootstrap values.

#### Scenario: Live init creates managed workspace bootstrap files

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` succeeds in a consumer project in live mode
- **THEN** `.scryrs/compose.yml` and `.scryrs/.env` exist under that project
- **AND** `scryrs.json` carries the committed live identity and `remote.docker_network`
- **AND** the workspace-local scaffold plus committed manifest are sufficient input for later `scryrs up`
- **AND** the operator is not required to copy or invoke the scryrs repository's root `docker-compose.yml`

#### Scenario: Wizard-assisted live init creates same bootstrap files

- **WHEN** `scryrs init --agent pi` starts the live-init wizard in a consumer project
- **AND** the user supplies missing required live values and confirms the summary
- **THEN** `.scryrs/compose.yml` and `.scryrs/.env` exist under that project
- **AND** `scryrs.json` carries the committed live identity and `remote.docker_network`
- **AND** `.scryrs/.env` remains an overrides-only file rather than the source of truth for wizard answers

#### Scenario: Second harness install reuses committed bootstrap

- **GIVEN** a consumer project already has a successful live bootstrap with committed `scryrs.json` live config and a managed `.scryrs/compose.yml`
- **WHEN** `scryrs init --agent <other-supported-harness>` is run with equivalent live bootstrap values
- **THEN** the existing committed `scryrs.json` live config and managed `.scryrs/compose.yml` are preserved
- **AND** the overrides-only `.scryrs/.env` is not rewritten with managed values
- **AND** only the newly requested harness installation work is applied
