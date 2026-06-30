## MODIFIED Requirements

### Requirement: Manifest remote defaults are subordinate to environment overrides

When a `remote` object is present in `scryrs.json`, its values SHALL act as the lowest-precedence defaults for CLI transport resolution and MAY be overridden, in increasing precedence, by `.scryrs/.env` values, then by `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, and `SCRYRS_REMOTE_TIMEOUT_MS` in the process environment, then by explicit CLI flags. The `scryrs.json` `remote` section is committed defaults; per-machine identity is expected to live in the gitignored `.scryrs/.env`. Because live transport is the CLI default, an empty or omitted `ingest_url` across all layers SHALL NOT silently activate local mode; the command SHALL fail fast with guidance unless local mode is explicitly selected.

#### Scenario: Environment overrides manifest remote values

- **GIVEN** a manifest `remote` section provides one set of values
- **AND** one or more corresponding `SCRYRS_REMOTE_*` environment variables are set
- **WHEN** the CLI resolves remote configuration
- **THEN** the environment values take precedence for those fields

#### Scenario: Env file overrides manifest remote values

- **GIVEN** a manifest `remote` section provides one set of values
- **AND** `.scryrs/.env` provides different values for the same fields
- **AND** no corresponding process environment variable or CLI flag is set
- **WHEN** the CLI resolves remote configuration
- **THEN** the `.scryrs/.env` values take precedence over the manifest defaults

#### Scenario: Unresolved ingest URL does not silently activate local mode

- **GIVEN** no `ingest_url` resolves from any configuration layer
- **AND** `--mode local` is not selected
- **WHEN** a CLI command resolves transport
- **THEN** the command fails fast with configuration guidance rather than running local mode
