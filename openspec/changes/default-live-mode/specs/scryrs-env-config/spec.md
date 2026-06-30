## ADDED Requirements

### Requirement: Remote config is loaded from a project-local `.scryrs/.env` file

The CLI SHALL load remote-ingest configuration from a `.scryrs/.env` dotenv file located under the resolved project root. The file SHALL use standard `KEY=value` dotenv syntax and SHALL recognize the keys `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, and `SCRYRS_REMOTE_TIMEOUT_MS`. Blank lines and lines beginning with `#` SHALL be ignored. A missing `.scryrs/.env` file SHALL NOT be an error by itself; it simply contributes no values to resolution.

#### Scenario: Values are read from `.scryrs/.env`

- **GIVEN** a `.scryrs/.env` file containing `SCRYRS_REMOTE_INGEST_URL=http://scryrs-server:8081`
- **AND** no corresponding process environment variable is set
- **WHEN** the CLI resolves remote configuration
- **THEN** the ingest URL resolves to `http://scryrs-server:8081`

#### Scenario: Comments and blank lines are ignored

- **GIVEN** a `.scryrs/.env` file containing comment lines starting with `#` and blank lines
- **WHEN** the CLI parses the file
- **THEN** comment and blank lines do not contribute key/value pairs
- **AND** valid `KEY=value` lines are still parsed

#### Scenario: Missing env file is not an error

- **GIVEN** no `.scryrs/.env` file exists under the project root
- **WHEN** the CLI resolves remote configuration
- **THEN** no error is raised for the absent file
- **AND** resolution proceeds using the remaining configuration layers

### Requirement: Remote config resolution follows a single deterministic precedence

The CLI SHALL resolve each remote-ingest field using the precedence order, highest first: explicit CLI flags, then process environment variables, then `.scryrs/.env`, then the `scryrs.json` `remote` section. `repository_id` SHALL fall back to the normalized Git remote-origin identity contract when unresolved by any layer. `timeout_ms` SHALL default to `3000` when unresolved.

#### Scenario: Process environment overrides the env file

- **GIVEN** `.scryrs/.env` sets `SCRYRS_REMOTE_INGEST_URL` to one value
- **AND** the process environment sets `SCRYRS_REMOTE_INGEST_URL` to a different value
- **WHEN** the CLI resolves the ingest URL
- **THEN** the process environment value wins

#### Scenario: CLI flag overrides all other layers

- **GIVEN** `.scryrs/.env`, process environment, and `scryrs.json` `remote` all provide a workspace identity
- **WHEN** `--workspace-id` is passed on the command line
- **THEN** the flag value is used over all other layers

#### Scenario: Env file overrides the manifest remote section

- **GIVEN** `scryrs.json` `remote` provides an ingest URL
- **AND** `.scryrs/.env` provides a different ingest URL
- **AND** no process environment variable or flag is set for it
- **THEN** the `.scryrs/.env` value is used

### Requirement: Absent required live configuration fails fast with guidance

When a command runs in the live default and the ingest URL or any required identity field cannot be resolved from any layer, the command SHALL exit with code `2` before any network call or filesystem write, and SHALL print deterministic stderr guidance that names the missing field(s) and describes both remediation paths: populating `.scryrs/.env` and selecting local mode explicitly.

#### Scenario: Missing ingest URL under the live default is guided

- **GIVEN** no ingest URL resolves from any configuration layer
- **WHEN** a command runs in the live default
- **THEN** the command exits with code `2`
- **AND** stderr names the missing configuration
- **AND** stderr describes populating `.scryrs/.env` and selecting local mode

#### Scenario: No partial side effects on guided failure

- **GIVEN** required live configuration is incomplete
- **WHEN** a command fails fast under the live default
- **THEN** no network submission is attempted
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written
