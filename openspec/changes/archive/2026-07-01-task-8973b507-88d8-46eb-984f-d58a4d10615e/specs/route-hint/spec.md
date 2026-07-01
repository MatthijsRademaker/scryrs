## MODIFIED Requirements

### Requirement: Route hint items carry structured identity, target, rank, and evidence

Each `RouteHintItem` SHALL include `routeId` (the source route entry id), `target` (normalized load target), `label` (human-readable label), `rank` (1-based ordinal from manifest sort order), `relevance` (optional; omitted by plain `hints_from_manifest` projection and populated for `scryrs route explain` query matches), `reason` (deterministic template text, with explain-specific query-match suffix when applicable), and `evidence` (provenance links copied from the source `RouteEntry`).

#### Scenario: Route hint fields match the source route entry for plain projection

- **GIVEN** a `RouteManifestDocument` with one route entry: `id = "file:src/main.rs"`, `label = "src/main.rs"`, `target = "file:src/main.rs"`, `subjectKind = "file"`, and one `EvidenceLink` with `sourceKind = "local_trace_row"`
- **WHEN** `hints_from_manifest` projects this entry
- **THEN** the hint item's `routeId` is `"file:src/main.rs"`
- **AND** `target` is `"file:src/main.rs"`
- **AND** `label` is `"src/main.rs"`
- **AND** `rank` is `1`
- **AND** `relevance` is absent from the serialized JSON
- **AND** `reason` contains `"Route 'src/main.rs' (file:src/main.rs): 1 evidence link(s), subject kind file"`
- **AND** `evidence` contains the same evidence link with `sourceKind = "local_trace_row"`

#### Scenario: Rank is a 1-based ordinal derived from manifest entry ordering

- **GIVEN** a manifest with three route entries sorted by id ascending: `"file:aaa.rs"`, `"file:zzz.rs"`, `"search:routing"`
- **WHEN** the hint producer projects all entries
- **THEN** the hint for `"file:aaa.rs"` has `rank = 1`
- **AND** the hint for `"file:zzz.rs"` has `rank = 2`
- **AND** the hint for `"search:routing"` has `rank = 3`

#### Scenario: Plain projection omits relevance

- **GIVEN** any valid route manifest
- **WHEN** `hints_from_manifest` projects hints
- **THEN** every `RouteHintItem.relevance` is `None`
- **AND** the serialized JSON excludes the `relevance` field entirely

#### Scenario: Explain matches populate deterministic relevance

- **GIVEN** a route entry matched by `scryrs route explain`
- **WHEN** the explain handler serializes the matched hint
- **THEN** `RouteHintItem.relevance` is present as a numeric `u32`
- **AND** the value equals `tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`

#### Scenario: Reason is a deterministic template citing entry identity and evidence count

- **GIVEN** a route entry with `label = "auth"`, `id = "search:auth"`, `subjectKind = "search"`, and 3 `evidenceLinks`
- **WHEN** the plain hint producer generates the reason
- **THEN** `reason` equals `"Route 'auth' (search:auth): 3 evidence link(s), subject kind search"`

### Requirement: Route-hint contract is documented for consumers

The CLI help surface (`--help` and `--help-json`), the CLI contract documentation (`cli-v0-contract.md`), and the route-manifest documentation (`route-manifests.md`) SHALL document the route-hint schema shape, its evidence sources, `rank` as the manifest ordinal, plain-projection `relevance` omission, and explain-match `relevance` population. Those surfaces SHALL NOT describe explain `relevance` as always deferred/`None` or explain ranking as manifest-order only.

#### Scenario: Help text scopes relevance by context

- **GIVEN** the route-hint contract is defined
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output explains that `rank` is the manifest ordinal
- **AND** it explains that plain route-hint projection omits `relevance`
- **AND** it explains that `scryrs route explain` populates deterministic `relevance` for matches

#### Scenario: Help JSON documents conditional relevance population

- **GIVEN** the route-hint contract is defined
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output documents the `RouteHintDocument` fields
- **AND** the `relevance` field description distinguishes plain projection from explain output

#### Scenario: Consumer documentation distinguishes rank from relevance

- **GIVEN** the route-hint contract documentation exists
- **WHEN** a consumer reads `cli-v0-contract.md` or `route-manifests.md`
- **THEN** the documentation states that `rank` is the deterministic manifest ordinal
- **AND** it states that explain `relevance` is a deterministic packed score for matched hints
- **AND** it states that plain projection still omits `relevance`
