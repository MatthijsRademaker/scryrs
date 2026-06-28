# route-explain Specification

## Purpose
TBD - created by archiving change task-ba92d62e-feb8-4aed-b4e8-c027a6c228c5. Update Purpose after archive.
## Requirements
### Requirement: Route explain CLI surface exists and is discoverable

The system SHALL expose `scryrs route explain <PATH> --query <TEXT>` as a documented CLI command reachable through the existing `route` command namespace. The parent `route` command SHALL remain backward-compatible: `scryrs route <PATH>` SHALL continue to generate the route manifest unchanged.

#### Scenario: Route explain appears in help output

- **GIVEN** the explain command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output includes a `scryrs route explain <PATH> --query <TEXT>` entry
- **AND** it includes a copy-paste example and interpretation notes
- **AND** it does NOT contain the phrase "deferred" in reference to route explain

#### Scenario: Route explain appears in help-json output

- **GIVEN** the explain command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output includes an `explain` subcommand entry under the `route` command
- **AND** the `routeHintOutput` description references the explain command as available
- **AND** no "deferred" language appears for route explain

#### Scenario: Existing route manifest generation is preserved

- **GIVEN** the explain command is registered
- **WHEN** the user runs `scryrs route <PATH>`
- **THEN** the command generates a route manifest from `.scryrs/graph.json`
- **AND** `.scryrs/routes.json` is written
- **AND** exit code, stdout, and stderr behavior are unchanged from the pre-explain contract

#### Scenario: Explain is accepted by dispatch

- **GIVEN** the explain command is registered
- **WHEN** the user runs `scryrs route explain /some/path --query "auth"`
- **THEN** the command is routed to the explain handler
- **AND** it does not produce an "unknown command" error

### Requirement: Explain consumes only the route manifest artifact

The explain handler SHALL load `.scryrs/routes.json` from the resolved repository root, deserialize it as `RouteManifestDocument`, and validate its `schemaVersion` against `ROUTE_SCHEMA_VERSION`. It SHALL NOT inspect `.scryrs/graph.json`, `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, or any other artifact directory.

#### Scenario: Explain loads routes.json and validates schema version

- **GIVEN** a repository root containing `.scryrs/routes.json` with `schemaVersion` equal to `ROUTE_SCHEMA_VERSION`
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the manifest is deserialized successfully
- **AND** processing continues to query matching

#### Scenario: Explain does not inspect graph artifact

- **GIVEN** a repository root where `.scryrs/graph.json` is absent but `.scryrs/routes.json` is valid
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the command succeeds (exit 0) without any error about missing graph
- **AND** it never attempts to read `.scryrs/graph.json`

### Requirement: Fail-fast for missing, malformed, or schema-mismatched route manifests

The explain handler SHALL exit with code 2 and emit explicit three-line diagnostic messages to stderr when `.scryrs/routes.json` is absent, contains invalid JSON, or has a `schemaVersion` not equal to `ROUTE_SCHEMA_VERSION`.

#### Scenario: Missing routes.json exits 2

- **GIVEN** a repository root where `.scryrs/routes.json` does not exist
- **WHEN** the user runs `scryrs route explain <PATH> --query "auth"`
- **THEN** the command exits with code 2
- **AND** an error message on stderr states the route artifact is not found
- **AND** the error follows the three-line contract format (error, usage, see-help)

#### Scenario: Malformed routes.json exits 2

- **GIVEN** a repository root where `.scryrs/routes.json` exists but contains invalid JSON
- **WHEN** the user runs `scryrs route explain <PATH> --query "auth"`
- **THEN** the command exits with code 2
- **AND** an error message on stderr describes the parse failure

#### Scenario: Schema version mismatch exits 2

- **GIVEN** `.scryrs/routes.json` contains `schemaVersion` not equal to `ROUTE_SCHEMA_VERSION`
- **WHEN** the user runs `scryrs route explain <PATH> --query "auth"`
- **THEN** the command exits with code 2
- **AND** an error message on stderr states the schema version mismatch

### Requirement: Required arguments fail with exit 2

The explain handler SHALL require both `PATH` and `--query` arguments. Missing `PATH` SHALL exit 2 with "missing required PATH argument." Missing `--query` SHALL exit 2 with "missing required --query argument."

#### Scenario: Missing PATH exits 2

- **GIVEN** the explain command
- **WHEN** the user runs `scryrs route explain --query "auth"` (no PATH)
- **THEN** the command exits with code 2
- **AND** an error message on stderr states the PATH argument is missing

#### Scenario: Missing --query exits 2

- **GIVEN** the explain command
- **WHEN** the user runs `scryrs route explain /some/path` (no --query)
- **THEN** the command exits with code 2
- **AND** an error message on stderr states the --query argument is missing

### Requirement: Query matching is deterministic and model-free

The explain handler SHALL perform case-insensitive substring matching of the `--query` value against `RouteEntry.label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. Matching SHALL be deterministic and independent of platform locale. The handler SHALL NOT use model-based ranking, semantic retrieval, fuzzy matching, or hidden heuristics.

#### Scenario: Query matches route entry label

- **GIVEN** a route manifest containing a route entry with `label = "Authentication"`
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the route entry is included in the output hints
- **AND** the match is case-insensitive ("auth" matches "Authentication")

#### Scenario: Query matches evidence link subject

- **GIVEN** a route manifest containing a route entry with `evidenceLinks[0].subject = "auth_handler"`
- **WHEN** `scryrs route explain <PATH> --query "handler"` runs
- **THEN** the route entry is included in the output hints
- **AND** the reason field notes a match on evidence subject

#### Scenario: Query does not match any field

- **GIVEN** a route manifest where no field contains the query text
- **WHEN** `scryrs route explain <PATH> --query "nonexistent"` runs
- **THEN** the command exits 0
- **AND** the output is a valid `RouteHintDocument` with an empty `hints` array

#### Scenario: Matching is locale-independent

- **GIVEN** the same route manifest and query
- **WHEN** `scryrs route explain <PATH> --query "straße"` runs on two different platforms
- **THEN** both runs produce byte-identical output

### Requirement: Matching is tiered with documented tie-break

Matched entries SHALL be ordered deterministically: exact match (tier 3) before prefix match (tier 2) before substring match (tier 1). Within each tier, entries SHALL follow the manifest entry order (by `id` ascending). An entry's best match tier across all searched fields determines its tier. Only entries that match at least one field SHALL appear in the output.

#### Scenario: Exact match appears before substring match

- **GIVEN** a route manifest with entry A `label = "auth"` and entry B `label = "authentication"`
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** entry A appears before entry B in the output (exact > prefix/substring)

#### Scenario: Same-tier entries preserve manifest order

- **GIVEN** a route manifest with entries sorted by id: `file:aaa.rs` (matches by substring), `file:zzz.rs` (matches by substring)
- **WHEN** `scryrs route explain <PATH> --query "file"` runs
- **THEN** `file:aaa.rs` appears before `file:zzz.rs` (manifest order tie-break)

#### Scenario: Best match tier across fields determines ranking

- **GIVEN** entry A matches `id` exactly and `label` by substring, entry B matches `subject` by prefix only
- **WHEN** the explain handler computes match tiers
- **THEN** entry A is tier 3 (exact on any field), entry B is tier 2 (prefix on any field)
- **AND** entry A appears before entry B

### Requirement: Output is a valid RouteHintDocument with extended reason

The explain handler SHALL emit a `RouteHintDocument` with `schemaVersion` equal to `HINT_SCHEMA_VERSION`. Each `RouteHintItem.reason` SHALL append query-match provenance in the format `; query match on {fields}` where `{fields}` is a comma-separated, alphabetically sorted list of matched field names. Unmatched entries SHALL be excluded entirely.

#### Scenario: Output schema matches RouteHintDocument contract

- **GIVEN** a valid route manifest with matching entries
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the stdout output is valid single-line JSON
- **AND** it deserializes as `RouteHintDocument` with `schemaVersion = HINT_SCHEMA_VERSION`
- **AND** every `RouteHintItem` has `routeId`, `target`, `label`, `rank`, `reason`, and `evidence` fields

#### Scenario: Reason cites query-match provenance

- **GIVEN** a route entry with `label = "auth"` and `subject = "authentication"` that matches query "auth" on both fields
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"; query match on label, subject"`
- **AND** the base template (identity, evidence count, subject kind) is preserved before the match suffix

#### Scenario: Reason for evidence-only match

- **GIVEN** a route entry where only `evidence_links[].subject` matches the query
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"; query match on evidence.subject"`
- **AND** no other field names are listed

### Requirement: Empty result is valid output

When the query matches zero route entries, the explain handler SHALL emit a valid `RouteHintDocument` with an empty `hints` array and exit 0. No error or warning SHALL appear on stderr for zero-match results.

#### Scenario: Zero matches emits empty hints, exits 0

- **GIVEN** a valid route manifest where no entry matches the query
- **WHEN** `scryrs route explain <PATH> --query "zzz_nonexistent"` runs
- **THEN** the command exits with code 0
- **AND** stdout contains `{"schemaVersion":"1.0.0","hints":[]}`
- **AND** stderr is empty

### Requirement: Explain is deterministic and repeatable

The explain handler SHALL produce byte-identical stdout output for identical inputs (same `.scryrs/routes.json` and same `--query` value). Output SHALL NOT include wall-clock timestamps, random identifiers, or iteration-order-dependent content.

#### Scenario: Repeated runs produce byte-identical output

- **GIVEN** the same `.scryrs/routes.json` artifact and the same `--query` value
- **WHEN** `scryrs route explain <PATH> --query "auth"` is run twice
- **THEN** the stdout output is byte-identical across both runs

### Requirement: Explain does not mutate artifacts

The explain handler SHALL be a read-only operation. It SHALL NOT create, modify, or delete `.scryrs/routes.json`, `.scryrs/graph.json`, `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, or any other filesystem artifact.

#### Scenario: Explain leaves the route manifest unchanged

- **GIVEN** a repository root with `.scryrs/routes.json`
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** `.scryrs/routes.json` is unmodified (byte-identical to pre-invocation)
- **AND** no new files are created under `.scryrs/`

### Requirement: `hints_from_manifest` API is unchanged

The existing `pub fn hints_from_manifest(manifest: &RouteManifestDocument) -> RouteHintDocument` function in `crates/scryrs-runtime` SHALL remain unchanged. Its reason template, rank assignment, evidence copying, and determinism guarantees SHALL be preserved exactly as shipped in Runtime Foundation 01.

#### Scenario: hints_from_manifest reason template is preserved

- **GIVEN** a route entry with `label = "auth"`, `id = "search:auth"`, `subjectKind = "search"`, 3 evidence links
- **WHEN** `hints_from_manifest` is called
- **THEN** `reason` is `"Route 'auth' (search:auth): 3 evidence link(s), subject kind search"` (no query-match suffix)

### Requirement: Explain is documented for consumers

The CLI help (`--help`), CLI machine-readable surface (`--help-json`), route-manifest documentation (`route-manifests.md`), and CLI contract documentation (`cli-v0-contract.md`) SHALL all document the explain command, its arguments, output format, match fields, tiered ordering, and no-match contract. A copy-paste example with interpretation notes SHALL be included.

#### Scenario: Help text includes example and interpretation

- **GIVEN** the explain command is implemented
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output under route includes a `scryrs route explain <PATH> --query <TEXT>` entry
- **AND** a copy-paste example is shown (e.g., `scryrs route explain . --query "authentication"`)
- **AND** a note explains how to interpret rank, reason, and evidence fields

#### Scenario: Route-manifest docs describe explain

- **GIVEN** the explain command is implemented
- **WHEN** a consumer reads `.devagent/docs/docs/route-manifests.md`
- **THEN** the shipped-vs-deferred table no longer lists explain as deferred
- **AND** the document describes explain usage, match fields, tiered ordering, and no-match behavior

#### Scenario: CLI-v0-contract docs describe explain

- **GIVEN** the explain command is implemented
- **WHEN** a consumer reads `.devagent/docs/docs/cli-v0-contract.md`
- **THEN** the route-hint section describes the explain command contract
- **AND** it includes arguments, output format, exit codes, and example JSON

