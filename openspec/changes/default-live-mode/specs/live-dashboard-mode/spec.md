## MODIFIED Requirements

### Requirement: Live dashboard mode is explicit and CLI-activated

Live mode SHALL be the default for `scryrs dashboard`. The dashboard SHALL run in live mode unless local mode is explicitly selected via `--mode local`. In the live default, `--server-url` and `--repository-id` SHALL be resolved from CLI flags, then process environment, then `.scryrs/.env`, then the `scryrs.json` `remote` section (mapping `ingest_url` to the server URL and `repository_id` to the repository identity). When the dashboard runs in the live default and the server URL or repository identity cannot be resolved from any layer, startup SHALL fail with a clear configuration error that names the missing values and describes populating `.scryrs/.env` and selecting `--mode local`. Live mode SHALL remain read-only and SHALL NOT merge server data with local `.scryrs` artifacts.

#### Scenario: Default dashboard activates live mode

- **GIVEN** `scryrs dashboard` is invoked without `--mode local`
- **AND** the server URL and repository identity resolve from `.scryrs/.env`, environment, or flags
- **WHEN** the dashboard backend starts
- **THEN** it runs in live mode
- **AND** `/api/meta` returns `mode: "live"`
- **AND** `/api/meta` returns the resolved `repositoryId`

#### Scenario: Local mode requires explicit opt-in

- **GIVEN** `scryrs dashboard --mode local` is invoked
- **WHEN** the dashboard backend starts
- **THEN** it runs in local mode
- **AND** `/api/meta` returns `mode: "local"`
- **AND** existing local hotspot, session, and event behavior remains available

#### Scenario: Live default without resolvable config fails loudly

- **GIVEN** `scryrs dashboard` is invoked without `--mode local`
- **AND** neither the server URL nor the repository identity can be resolved from any layer
- **WHEN** startup validation runs
- **THEN** the command exits with a non-zero status
- **AND** stderr names the missing configuration and describes populating `.scryrs/.env` and selecting `--mode local`

#### Scenario: Live mode does not mix local artifacts with server state

- **GIVEN** the dashboard is running in live mode
- **AND** local `.scryrs` artifacts exist in the repository
- **WHEN** live dashboard API endpoints are queried
- **THEN** only live server data is used
- **AND** local `.scryrs` files are not merged or used as fallback
