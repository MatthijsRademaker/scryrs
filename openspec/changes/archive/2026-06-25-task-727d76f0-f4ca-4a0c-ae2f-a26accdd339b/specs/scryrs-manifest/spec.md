## ADDED Requirements

### Requirement: Manifest may declare remote ingest defaults

The manifest SHALL allow an optional top-level `remote` object for CLI remote-ingest defaults. When present, the object SHALL use fields `ingest_url`, `repository_id`, `workspace_id`, `agent_id`, and `timeout_ms`. The `remote` object is configuration only; it does not register a direct HTTP tool surface and does not change the manifest's hook-interface scope.

#### Scenario: Remote defaults are parseable

- **WHEN** a consumer reads a `scryrs.json` file that includes `remote`
- **THEN** the `remote` object parses as ordinary JSON configuration
- **AND** its fields are limited to remote-ingest defaults rather than a callable tool surface

#### Scenario: Remote defaults do not imply agent-callable HTTP behavior

- **WHEN** a developer inspects the manifest's `remote` section
- **THEN** they see remote transport defaults for the CLI
- **AND** they do not see the manifest described as a direct HTTP integration surface

### Requirement: Manifest remote defaults are subordinate to environment overrides

When a `remote` object is present in `scryrs.json`, its values SHALL act as defaults for CLI transport resolution and MAY be overridden by `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, and `SCRYRS_REMOTE_TIMEOUT_MS`. An empty or omitted `ingest_url` SHALL NOT activate remote mode.

#### Scenario: Environment overrides manifest remote values

- **GIVEN** a manifest `remote` section provides one set of values
- **AND** one or more corresponding `SCRYRS_REMOTE_*` environment variables are set
- **WHEN** the CLI resolves remote configuration
- **THEN** the environment values take precedence for those fields

#### Scenario: Empty ingest URL does not activate remote mode

- **WHEN** the manifest omits `remote.ingest_url` or sets it to an empty value
- **THEN** the manifest alone does not activate remote ingest mode
- **AND** the CLI remains in local mode unless a non-empty ingest URL is provided by a higher-precedence source
