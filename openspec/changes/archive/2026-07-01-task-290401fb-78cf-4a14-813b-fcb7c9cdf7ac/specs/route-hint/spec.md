## MODIFIED Requirements

### Requirement: Route hint items carry structured identity, target, rank, and evidence

Each `RouteHintItem` SHALL include `routeId` (the source route entry id), `target` (the stable source route target string), `loadTarget` (optional structured load target copied from the source `RouteEntry`), `label` (human-readable label), `rank` (1-based ordinal from manifest sort order), `relevance` (optional; omitted by plain `hints_from_manifest` projection and populated for `scryrs route explain` query matches), `reason` (deterministic template text, with load-target kind in the base template and explain-specific query-match suffix when applicable), and `evidence` (provenance links copied from the source `RouteEntry`).

#### Scenario: Route hint fields match the source route entry for plain projection

- **GIVEN** a `RouteManifestDocument` with one route entry: `id = "file:src/main.rs"`, `label = "src/main.rs"`, `target = "file:src/main.rs"`, `loadTarget.kind = "file"`, `loadTarget.reference = "src/main.rs"`, `subjectKind = "file"`, and one `EvidenceLink` with `sourceKind = "local_trace_row"`
- **WHEN** `hints_from_manifest` projects this entry
- **THEN** the hint item's `routeId` is `"file:src/main.rs"`
- **AND** `target` is `"file:src/main.rs"`
- **AND** `loadTarget.kind` is `"file"`
- **AND** `loadTarget.reference` is `"src/main.rs"`
- **AND** `label` is `"src/main.rs"`
- **AND** `rank` is `1`
- **AND** `relevance` is absent from the serialized JSON
- **AND** `reason` contains `"Route 'src/main.rs' (file:src/main.rs): 1 evidence link(s), subject kind file, load target file"`
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

#### Scenario: Reason is a deterministic template citing entry identity, evidence count, and load target kind

- **GIVEN** a route entry with `label = "auth"`, `id = "search:auth"`, `subjectKind = "search"`, 3 `evidenceLinks`, and `loadTarget.kind = "non_loadable"`
- **WHEN** the plain hint producer generates the reason
- **THEN** `reason` equals `"Route 'auth' (search:auth): 3 evidence link(s), subject kind search, load target non_loadable"`

#### Scenario: Non-loadable routes stay explicit in hints

- **GIVEN** a manifest with one route entry `id = "domain_term:auth"`, `target = "domain_term:auth"`, and `loadTarget.kind = "non_loadable"`
- **WHEN** `hints_from_manifest` projects this entry
- **THEN** the hint item's `loadTarget.kind` is `"non_loadable"`
- **AND** the serialized hint does not invent a file path or docs reference

### Requirement: Route-hint contract is documented for consumers

The CLI help surface (`--help` and `--help-json`), the CLI contract documentation (`cli-v0-contract.md`), and the route-manifest documentation (`route-manifests.md`) SHALL document the route-hint schema shape, its evidence sources, optional `loadTarget`, `rank` as the manifest ordinal, plain-projection `relevance` omission, explain-match `relevance` population, and reason strings that mention load target kind. Those surfaces SHALL state that file hints carry repository-relative file references, doc-page hints carry canonical `project-docs/<slug>` references, and non-loadable hints remain explicit.

#### Scenario: Help text scopes relevance and load target by context

- **GIVEN** the route-hint contract is defined
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output explains that `rank` is the manifest ordinal
- **AND** it explains that plain route-hint projection omits `relevance`
- **AND** it explains that `scryrs route explain` populates deterministic `relevance` for matches
- **AND** it documents that reason strings mention load target kind

#### Scenario: Help JSON documents conditional relevance population and load target

- **GIVEN** the route-hint contract is defined
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output documents the `RouteHintDocument` fields
- **AND** the `relevance` field description distinguishes plain projection from explain output
- **AND** the `loadTarget` field description distinguishes `file`, `doc_page`, and `non_loadable`

#### Scenario: Consumer documentation distinguishes rank, relevance, and load target

- **GIVEN** the route-hint contract documentation exists
- **WHEN** a consumer reads `cli-v0-contract.md` or `route-manifests.md`
- **THEN** the documentation states that `rank` is the deterministic manifest ordinal
- **AND** it states that explain `relevance` is a deterministic packed score for matched hints
- **AND** it states that plain projection still omits `relevance`
- **AND** it states that file hints use repository-relative references and doc-page hints use `project-docs/<slug>` references
