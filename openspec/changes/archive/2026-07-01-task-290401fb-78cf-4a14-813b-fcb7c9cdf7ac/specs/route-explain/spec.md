## MODIFIED Requirements

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
