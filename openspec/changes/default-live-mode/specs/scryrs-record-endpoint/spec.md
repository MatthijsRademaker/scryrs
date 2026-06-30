## MODIFIED Requirements

### Requirement: Remote ingest mode is explicit and configuration-driven

The `record` command SHALL resolve transport mode before reading input or opening the local store. Remote transport SHALL be the default. The command SHALL run in remote mode unless `--mode local` is explicitly selected. Remote configuration SHALL be resolved through the shared precedence: CLI flags, then process environment variables (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, `SCRYRS_REMOTE_TIMEOUT_MS`), then `.scryrs/.env`, then the nearest ancestor `scryrs.json` `remote` section. In remote mode, `workspace_id` and `agent_id` are required values; `repository_id` SHALL resolve from explicit configuration or the normalized Git remote-origin contract defined by `live-hotspot-server-contract`. When the default is remote and the ingest URL or any required identity field cannot be resolved from any layer, the command SHALL fail before any network call with exit code `2` and SHALL print deterministic guidance describing how to populate `.scryrs/.env` and how to select `--mode local`. The command SHALL NOT silently fall back to local mode when remote configuration is incomplete. `timeout_ms` SHALL default to `3000` when not configured.

#### Scenario: Default record runs in remote mode

- **WHEN** `scryrs record` runs without `--mode local` and complete remote configuration resolves
- **THEN** remote transport is active
- **AND** accepted events are submitted to the server batch endpoint

#### Scenario: Explicit local mode keeps SQLite behavior

- **WHEN** `scryrs record --mode local` runs
- **THEN** local SQLite transport is active
- **AND** no remote network submission is attempted

#### Scenario: Missing remote config under the default fails with guidance

- **GIVEN** no `--mode local` is selected
- **AND** the ingest URL or a required identity field cannot be resolved from any layer
- **WHEN** `scryrs record` starts
- **THEN** the command reports deterministic configuration guidance to stderr
- **AND** the guidance names how to populate `.scryrs/.env` and how to select local mode
- **AND** the command exits with code `2`
- **AND** does not attempt a network call
- **AND** does not silently fall back to local mode

#### Scenario: Environment overrides env file and manifest remote defaults

- **GIVEN** `.scryrs/.env` and `scryrs.json` `remote` both provide values
- **AND** one or more `SCRYRS_REMOTE_*` process variables are also set
- **WHEN** `scryrs record` resolves remote configuration
- **THEN** the process environment values override the env file and manifest defaults for the same fields

#### Scenario: Invoking from a subdirectory finds the nearest ancestor manifest

- **GIVEN** a repository contains `scryrs.json` in an ancestor directory of the current working directory
- **WHEN** `scryrs record` runs from that subdirectory
- **THEN** the command discovers the nearest ancestor `scryrs.json`
- **AND** uses its `remote` section as the manifest-layer configuration source
