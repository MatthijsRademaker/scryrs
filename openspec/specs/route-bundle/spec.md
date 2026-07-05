# route-bundle Specification

## Purpose
TBD - created by archiving change task-b122a458-4c9e-425d-b58b-04ba8aebe405. Update Purpose after archive.
## Requirements
### Requirement: Route bundle CLI surface exists and is discoverable

The system SHALL expose `scryrs route bundle <PATH> --query <TEXT> --limit <N>` as a documented CLI command under the existing `route` namespace. The parent `route` command SHALL remain backward-compatible: `scryrs route <PATH>` SHALL continue to generate the route manifest unchanged.

#### Scenario: Route bundle appears in help output

- **GIVEN** the bundle command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output includes a `scryrs route bundle <PATH> --query <TEXT> --limit <N>` entry
- **AND** it describes bundle as a bounded context-loading plan
- **AND** it distinguishes bundle from `scryrs route explain`

#### Scenario: Route bundle appears in help-json output

- **GIVEN** the bundle command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output includes a `bundle` subcommand entry under the `route` command
- **AND** the machine-readable description distinguishes bundle from `explain`

#### Scenario: Existing route manifest generation is preserved

- **GIVEN** the bundle command is registered
- **WHEN** the user runs `scryrs route <PATH>`
- **THEN** the command still generates `.scryrs/routes.json`
- **AND** its stdout, stderr, and exit-code behavior remain unchanged

#### Scenario: Bundle is accepted by dispatch

- **GIVEN** the bundle command is registered
- **WHEN** the user runs `scryrs route bundle /some/path --query "auth" --limit 5`
- **THEN** the command is routed to the bundle handler
- **AND** it does not produce an unknown-command error

### Requirement: Bundle consumes only the route manifest and explain-derived ranking

The bundle handler SHALL resolve `PATH`, load `.scryrs/routes.json`, deserialize it as `RouteManifestDocument`, validate its `schemaVersion` against `ROUTE_SCHEMA_VERSION`, and derive matches from `scryrs_runtime::explain_hints` or a shared extraction of the same logic. It SHALL NOT inspect `.scryrs/graph.json`, source files, docs files, proposal artifacts, or any other project content.

#### Scenario: Bundle succeeds with routes.json only

- **GIVEN** a repository root containing a valid `.scryrs/routes.json`
- **AND** `.scryrs/graph.json` is absent
- **WHEN** `scryrs route bundle <PATH> --query "auth" --limit 5` runs
- **THEN** the command succeeds using `.scryrs/routes.json` only
- **AND** it does not error about missing graph data

#### Scenario: Bundle uses explain ordering as its source of truth

- **GIVEN** a route manifest with multiple matches for the same query
- **WHEN** `scryrs route explain <PATH> --query <TEXT>` and `scryrs route bundle <PATH> --query <TEXT> --limit 5` are both run
- **THEN** the bundle's `targets` order matches the explain result order before truncation
- **AND** bundle does not apply a second ranking algorithm

### Requirement: Required arguments and manifest failures are fail-fast

The bundle handler SHALL require `PATH`, `--query`, and `--limit`. `--limit` SHALL be a positive integer. Missing `PATH`, missing `--query`, missing `--limit`, non-numeric limit values, zero, negative values, missing `.scryrs/routes.json`, malformed route JSON, and route schema mismatches SHALL exit with code 2 and emit explicit diagnostics consistent with the existing route-explain error contract.

#### Scenario: Missing limit exits 2

- **GIVEN** the bundle command
- **WHEN** the user runs `scryrs route bundle /some/path --query "auth"`
- **THEN** the command exits with code 2
- **AND** stderr states that `--limit` is required

#### Scenario: Zero limit exits 2

- **GIVEN** the bundle command
- **WHEN** the user runs `scryrs route bundle /some/path --query "auth" --limit 0`
- **THEN** the command exits with code 2
- **AND** stderr states that `--limit` must be positive

#### Scenario: Missing routes.json exits 2

- **GIVEN** a repository root where `.scryrs/routes.json` does not exist
- **WHEN** the user runs `scryrs route bundle <PATH> --query "auth" --limit 5`
- **THEN** the command exits with code 2
- **AND** stderr reports that the route artifact is not found

### Requirement: Bundle output is a versioned bounded plan

The bundle handler SHALL emit a `RouteBundleDocument` with `schemaVersion` equal to `BUNDLE_SCHEMA_VERSION`. Each successful output SHALL include `query`, `limit`, and `targets`. Each `targets` entry SHALL preserve the explain-derived `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence` fields. The command SHALL serialize valid single-line JSON.

#### Scenario: Bundle output schema matches the contract

- **GIVEN** a valid route manifest with matching entries
- **WHEN** `scryrs route bundle <PATH> --query "auth" --limit 5` runs
- **THEN** stdout is valid single-line JSON
- **AND** it deserializes as `RouteBundleDocument`
- **AND** `schemaVersion` equals `BUNDLE_SCHEMA_VERSION`
- **AND** each target includes `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence`

#### Scenario: Bundle includes query and limit metadata

- **GIVEN** the user runs `scryrs route bundle <PATH> --query "auth" --limit 5`
- **WHEN** the bundle is serialized
- **THEN** the top-level document includes `query = "auth"`
- **AND** it includes `limit = 5`

### Requirement: Bundle size is bounded and stable

Successful bundle output SHALL satisfy `targets.len() <= limit`. If more matches exist than the requested limit, the handler SHALL emit exactly the first `limit` explain-ordered matches. Non-loadable targets SHALL remain explicit in the output and SHALL count toward the limit.

#### Scenario: Bundle truncates after stable explain ordering

- **GIVEN** a query that produces more than five explain matches
- **WHEN** the user runs `scryrs route bundle <PATH> --query <TEXT> --limit 5`
- **THEN** the bundle contains exactly five `targets`
- **AND** they are the first five entries from the explain-ordered result

#### Scenario: Non-loadable targets are preserved

- **GIVEN** an explain-ranked match whose `loadTarget.kind` is `non_loadable`
- **WHEN** that match falls within the requested limit
- **THEN** the bundle includes it in `targets`
- **AND** it still carries `loadTarget.kind = non_loadable`
- **AND** it counts toward `targets.len()`

#### Scenario: Zero matches emit an empty valid bundle

- **GIVEN** a valid route manifest where no route entry matches the query
- **WHEN** `scryrs route bundle <PATH> --query "zzz_nonexistent" --limit 5` runs
- **THEN** the command exits with code 0
- **AND** stdout is a valid `RouteBundleDocument`
- **AND** `targets` is an empty array
- **AND** stderr is empty

### Requirement: Bundle is read-only and does not mutate context

The bundle handler SHALL be a read-only operation. It SHALL NOT create, modify, or delete `.scryrs/routes.json`, `.scryrs/graph.json`, `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, or any other filesystem artifact. It SHALL NOT automatically load files, docs, or mutate runtime context; the bundle is a plan only.

#### Scenario: Bundle leaves artifacts unchanged

- **GIVEN** a repository root with `.scryrs/routes.json`
- **WHEN** `scryrs route bundle <PATH> --query "auth" --limit 5` runs
- **THEN** `.scryrs/routes.json` remains byte-identical
- **AND** no new files are created under `.scryrs/`

### Requirement: Bundle is documented for consumers

The CLI help surfaces (`--help`, `route bundle --help`, and `--help-json`), `route-manifests.md`, and `cli-v0-contract.md` SHALL document the bundle command, its required arguments, its bounded `RouteBundleDocument` output, its reuse of explain ordering, its inclusion of route ID, load target, reason, relevance, and evidence per target, and when agents should call bundle versus explain.

#### Scenario: Help surfaces explain bundle versus explain usage

- **GIVEN** the bundle command is implemented
- **WHEN** the user reads CLI help or help-json output
- **THEN** the documentation states that `bundle` is the bounded context-loading plan
- **AND** it states that `explain` remains the unbounded diagnostic explanation of route matches

#### Scenario: Developer docs explain the bounded bundle contract

- **GIVEN** the bundle command is implemented
- **WHEN** a consumer reads `route-manifests.md` or `cli-v0-contract.md`
- **THEN** the documentation describes the bundle envelope fields and limit semantics
- **AND** it explains that bundle reuses explain ordering and preserves evidence-backed target metadata

