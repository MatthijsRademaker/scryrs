## ADDED Requirements

### Requirement: `scryrs hotspots` supports explicit live artifact export

`scryrs hotspots <PATH>` SHALL support `--mode live|local`, `--server-url <URL>`, and `--repository-id <ID>`. When `--mode live` is selected, the command SHALL resolve server URL and repository ID through this precedence chain: explicit flags, then process environment (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`), then `.scryrs/.env`, then `scryrs.json remote`. The live export SHALL query `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`.

#### Scenario: Explicit live flags trigger the cumulative live query

- **GIVEN** `scryrs server` has accepted events for repository `repo-a`
- **WHEN** the operator runs `scryrs hotspots <PATH> --mode live --server-url http://localhost:8081 --repository-id repo-a`
- **THEN** the command queries `GET http://localhost:8081/v1/repositories/repo-a/hotspots?window=cumulative`
- **AND** the command does not require any downstream live-specific flags after export

#### Scenario: Live mode reuses existing remote config precedence

- **GIVEN** `--mode live` is selected
- **AND** `SCRYRS_REMOTE_INGEST_URL` and `SCRYRS_REPOSITORY_ID` are set in the process environment
- **AND** `.scryrs/.env` and `scryrs.json remote` contain different values for the same fields
- **WHEN** `scryrs hotspots <PATH>` runs without explicit `--server-url` or `--repository-id`
- **THEN** the process-environment values win
- **AND** the command uses those resolved values for the live hotspot query

#### Scenario: Local mode remains the default path

- **GIVEN** no live mode is selected
- **WHEN** the operator runs `scryrs hotspots <PATH>`
- **THEN** the command follows the existing local SQLite-backed behavior
- **AND** no live server query is attempted

### Requirement: Live export materializes a schema-compatible `HotspotsReport`

A successful live export SHALL write a schema-compatible `HotspotsReport` to `<PATH>/.scryrs/hotspots.json` and emit the same JSON to stdout. The exported report SHALL keep `schemaVersion: "1.0.0"`, `command: "hotspots"`, and the live response `entries` unchanged. `repositoryPath` SHALL identify the resolved export target path. `storePath` SHALL be `live:<server_url>/v1/repositories/<repository_id>/hotspots?window=cumulative`. `generatedAt` SHALL be copied from the live response. `runMetadata` SHALL be derived from live entries with `analyzedSubjectCount = entries.len()`, `analyzedEventCount = sum(evidence.rowIds.len())`, `storeSchemaVersion = 0`, `firstEventId = 0`, and `lastEventId = 0`.

#### Scenario: Successful live export writes the existing artifact shape

- **GIVEN** the live endpoint returns a valid `LiveHotspotsResponse` for repository `repo-a`
- **WHEN** `scryrs hotspots <PATH> --mode live ...` succeeds
- **THEN** `<PATH>/.scryrs/hotspots.json` contains a valid `HotspotsReport`
- **AND** `entries` exactly match the live response entries
- **AND** `generatedAt` matches the live response `generatedAt`
- **AND** stdout matches the artifact contents byte-for-byte

#### Scenario: Live source identity is explicit to operators

- **GIVEN** a successful live export
- **WHEN** the command finishes
- **THEN** `storePath` records the full live query source as a `live:<query_url>` descriptor
- **AND** stderr states the live server URL and repository ID used for the export

### Requirement: Live export validates the remote response before replacing the artifact

Live export SHALL fail loudly and exit non-zero when the server is unreachable, times out, returns a non-2xx status, returns malformed JSON, returns the wrong live schema version, or returns a `repositoryId` that does not match the requested repository. The command SHALL validate those conditions before replacing `<PATH>/.scryrs/hotspots.json` and SHALL write the artifact via temp-file plus atomic rename so failures never leave a partial replacement.

#### Scenario: Transport failure does not replace the artifact

- **GIVEN** `<PATH>/.scryrs/hotspots.json` already exists
- **AND** the configured live server is unreachable
- **WHEN** `scryrs hotspots <PATH> --mode live ...` runs
- **THEN** the command exits non-zero with a clear `scryrs hotspots:` error
- **AND** the existing artifact remains unchanged

#### Scenario: Malformed or mismatched responses fail before write

- **GIVEN** the live server responds with malformed JSON, the wrong `schemaVersion`, or a different `repositoryId`
- **WHEN** `scryrs hotspots <PATH> --mode live ...` runs
- **THEN** the command exits non-zero with a clear `scryrs hotspots:` error
- **AND** no new hotspot artifact is created or partially written

### Requirement: Live and local hotspot sources never merge implicitly

When `--mode live` is selected, `scryrs hotspots` SHALL materialize the hotspot artifact exclusively from the live server response. The live path SHALL NOT open `.scryrs/scryrs.db`, score local events, or merge local-only subjects into the exported entries, even when local hotspot inputs already exist.

#### Scenario: Existing local SQLite data does not affect live export

- **GIVEN** `<PATH>/.scryrs/scryrs.db` exists and contains local-only hotspot subjects
- **AND** the live server returns a different ranked entry set for the requested repository
- **WHEN** `scryrs hotspots <PATH> --mode live ...` succeeds
- **THEN** the exported `entries` match only the live server response
- **AND** no local-only subject appears in the exported artifact unless it was present in the live response

### Requirement: Downstream commands accept the live-exported artifact unchanged

After a successful live export, existing downstream commands SHALL consume `<PATH>/.scryrs/hotspots.json` without any live-specific flags or schema changes.

#### Scenario: Graph, route, and propose run on the exported artifact

- **GIVEN** `scryrs hotspots <PATH> --mode live ...` has written `<PATH>/.scryrs/hotspots.json`
- **WHEN** the operator runs `scryrs graph <PATH>`, then `scryrs route <PATH>`, then `scryrs propose <PATH>`
- **THEN** each command succeeds using the exported artifact
- **AND** none of the downstream commands require a live-only compatibility flag

### Requirement: CLI help describes live export mode

The CLI help surface SHALL document the explicit live export mode for `scryrs hotspots`, including the live-mode flags and the fact that live export materializes a standalone artifact rather than merging with local SQLite evidence.

#### Scenario: `hotspots` help lists live export inputs

- **WHEN** the operator runs `scryrs hotspots --help`
- **THEN** the help text lists `--mode`, `--server-url`, and `--repository-id`
- **AND** the help text explains that live export writes `.scryrs/hotspots.json` from the server response without merging local SQLite data

#### Scenario: `--help-json` includes the live export contract

- **WHEN** the operator runs `scryrs --help-json`
- **THEN** the hotspots command description includes the live-mode flags
- **AND** the documented output remains the existing `HotspotsReport` artifact shape
