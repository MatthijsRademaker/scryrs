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

Matched entries SHALL be ordered deterministically by the full sort tuple `(best_match_tier DESC, total_evidence_score DESC, evidence_count DESC, manifest_index ASC, route_id ASC)`. `best_match_tier` remains exact (tier 3) before prefix (tier 2) before substring (tier 1), determined across `label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. `total_evidence_score` SHALL be the saturating sum of `EvidenceLink.score.unwrap_or(0)` across the route's evidence links, with absent scores contributing zero. `evidence_count` SHALL be `RouteEntry.evidenceLinks.len()`. The full sort tuple SHALL be authoritative for ordering; the serialized `relevance` value SHALL NOT be used as the sort key. Only entries that match at least one field SHALL appear in the output.

#### Scenario: Exact match outranks higher-evidence prefix match

- **GIVEN** entry A matches the query exactly
- **AND** entry B matches the same query by prefix only but has a higher evidence score
- **WHEN** `scryrs route explain <PATH> --query <TEXT>` runs
- **THEN** entry A appears before entry B
- **AND** match tier outranks evidence score

#### Scenario: Higher evidence score wins within the same tier

- **GIVEN** two entries match within the same best-match tier
- **AND** entry A has a higher `total_evidence_score` than entry B
- **WHEN** the explain handler sorts matches
- **THEN** entry A appears before entry B

#### Scenario: Higher evidence count wins when tier and score tie

- **GIVEN** two entries match within the same best-match tier
- **AND** both entries have the same `total_evidence_score`
- **AND** entry A has more evidence links than entry B
- **WHEN** the explain handler sorts matches
- **THEN** entry A appears before entry B

#### Scenario: Stable tie-breaks preserve deterministic order

- **GIVEN** two entries have the same best-match tier, `total_evidence_score`, and `evidence_count`
- **WHEN** the explain handler sorts matches
- **THEN** the entry earlier in manifest order appears first
- **AND** `route_id` remains the final defensive deterministic tie-break

#### Scenario: Best match tier across fields determines ranking

- **GIVEN** entry A matches `id` exactly and `label` by substring
- **AND** entry B matches `subject` by prefix only
- **WHEN** the explain handler computes match tiers
- **THEN** entry A is tier 3 and entry B is tier 2
- **AND** entry A appears before entry B

### Requirement: Output is a valid RouteHintDocument with extended reason

The explain handler SHALL emit a `RouteHintDocument` with `schemaVersion` equal to `HINT_SCHEMA_VERSION`. Each matched `RouteHintItem` SHALL populate `loadTarget` from the source route entry and `relevance` with a deterministic packed `u32` value computed as `tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`. The packed value SHALL be documented as a display-friendly monotonic derivative of the authoritative sort tuple, not the ordering primitive. Each `RouteHintItem.reason` SHALL use the same base template as plain projection, including `, load target <kind>`, and SHALL append query-match provenance in the format `; query match on {fields}` where `{fields}` is a comma-separated, alphabetically sorted list of matched field names. Unmatched entries SHALL be excluded entirely.

#### Scenario: Output schema matches RouteHintDocument contract

- **GIVEN** a valid route manifest with matching entries
- **WHEN** `scryrs route explain <PATH> --query "auth"` runs
- **THEN** the stdout output is valid single-line JSON
- **AND** it deserializes as `RouteHintDocument` with `schemaVersion = HINT_SCHEMA_VERSION`
- **AND** every returned `RouteHintItem` has `routeId`, `target`, `loadTarget`, `label`, `rank`, `relevance`, `reason`, and `evidence` fields

#### Scenario: Relevance is populated from match tier and evidence

- **GIVEN** a matched entry with best-match tier `2`, `total_evidence_score = 15`, and `evidence_count = 3`
- **WHEN** the explain handler serializes the hint
- **THEN** `relevance` equals `2000015003`

#### Scenario: Reason cites load target kind and query-match provenance

- **GIVEN** a route entry with `label = "auth"`, `subject = "authentication"`, and `loadTarget.kind = "file"` that matches query `auth` on both fields
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"load target file; query match on label, subject"`
- **AND** the base template is preserved before the match suffix

#### Scenario: Reason for evidence-only match

- **GIVEN** a route entry where only `evidence_links[].subject` matches the query and `loadTarget.kind = "non_loadable"`
- **WHEN** the explain handler generates the reason
- **THEN** `reason` contains `"load target non_loadable; query match on evidence.subject"`
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

The existing `pub fn hints_from_manifest(manifest: &RouteManifestDocument) -> RouteHintDocument` function in `crates/scryrs-runtime` SHALL remain available and deterministic. Its plain-projection semantics remain read-only, query-free, and manifest-order based. It SHALL now copy `loadTarget` from each source route entry and use the base reason template `Route '<label>' (<route_id>): <count> evidence link(s), subject kind <subject_kind>, load target <load_target_kind>` without an explain query-match suffix or populated `relevance`.

#### Scenario: hints_from_manifest reason template includes load target kind

- **GIVEN** a route entry with `label = "auth"`, `id = "search:auth"`, `subjectKind = "search"`, 3 `evidenceLinks`, and `loadTarget.kind = "non_loadable"`
- **WHEN** `hints_from_manifest` is called
- **THEN** `reason` is `"Route 'auth' (search:auth): 3 evidence link(s), subject kind search, load target non_loadable"`
- **AND** the projected hint copies `loadTarget.kind = "non_loadable"`

### Requirement: Explain is documented for consumers

The CLI help (`--help`), CLI machine-readable surface (`--help-json`), route-manifest documentation (`route-manifests.md`), and CLI contract documentation (`cli-v0-contract.md`) SHALL document the explain command, its arguments, output format, match fields, returned `loadTarget`, reason text including load target kind, deterministic tie-break chain, packed relevance formula, and zero-match contract. Those surfaces SHALL distinguish `rank` as the manifest ordinal from explain `relevance` as the deterministic packed score, and SHALL state that plain `hints_from_manifest` output still omits `relevance`.

#### Scenario: Help text includes load-target context alongside ranking documentation

- **GIVEN** the explain command is implemented
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output documents exact > prefix > substring matching
- **AND** it documents evidence score, evidence count, and stable tie-breaks
- **AND** it documents how explain `relevance` is derived
- **AND** it documents that explain reasons mention load target kind

#### Scenario: Help-json distinguishes plain projection, explain matches, and loadTarget

- **GIVEN** the route explain contract is implemented
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output describes the full explain ranking chain
- **AND** the route-hint field description states that plain projection omits `relevance`
- **AND** the explain command description states that query matches populate it with the packed deterministic score
- **AND** it states that the packed score is not the authoritative sort key
- **AND** it documents the returned `loadTarget` field and its kinds

#### Scenario: Consumer docs no longer describe target alone as the loadable address

- **GIVEN** the explain command is implemented
- **WHEN** a consumer reads `route-manifests.md` or `cli-v0-contract.md`
- **THEN** the documentation describes explain ordering by match tier, evidence score, evidence count, and stable tie-breaks
- **AND** it explains that `target` remains the stable node-id string
- **AND** it explains that `loadTarget` carries the file/docs/non-loadable address context

