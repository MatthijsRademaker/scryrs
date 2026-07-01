# route-hint Specification

## Purpose
TBD - created by archiving change task-40c5b995-5010-4cae-be7f-8a859e86468c. Update Purpose after archive.
## Requirements
### Requirement: Route hint documents are versioned and self-describing

The system SHALL define a `RouteHintDocument` wire contract with an independent `HINT_SCHEMA_VERSION`. Each serialized route-hint document SHALL include `schemaVersion` and `hints`. `schemaVersion` SHALL equal `HINT_SCHEMA_VERSION` and SHALL be versioned independently from `ROUTE_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, `PROPOSAL_SCHEMA_VERSION`, `REVIEW_DECISION_SCHEMA_VERSION`, `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and `LIVE_HOTSPOT_SCHEMA_VERSION`.

#### Scenario: Serialized hint document carries the required top-level fields

- **GIVEN** a route-hint document produced under this contract
- **WHEN** a consumer inspects the serialized JSON
- **THEN** it contains `schemaVersion` equal to `"1.0.0"`
- **AND** it contains `hints` as a JSON array

#### Scenario: Hint schema version is independent from route and graph versions

- **GIVEN** the workspace already defines `ROUTE_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, and `PROPOSAL_SCHEMA_VERSION`
- **WHEN** the route-hint contract is versioned
- **THEN** it defines a separate `HINT_SCHEMA_VERSION`
- **AND** every serialized hint document uses `HINT_SCHEMA_VERSION` for `schemaVersion`

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

### Requirement: Hint generation preserves subject identity boundaries

Each `RouteEntry` SHALL produce exactly one `RouteHintItem`. Distinct `routeId` values SHALL remain distinct in the hints array. Hints SHALL NOT be merged, deduplicated, or collapsed based on shared label text, subject text, or any other heuristic. In particular, route entries `file:auth`, `search:auth`, and `symbol:auth` SHALL remain three distinct hint items.

#### Scenario: Distinct route identities remain distinct hints

- **GIVEN** a `RouteManifestDocument` containing three routes: `file:auth`, `search:auth`, and `symbol:auth`
- **WHEN** `hints_from_manifest` is called
- **THEN** the hints array contains exactly three entries
- **AND** their `routeId` values are `"file:auth"`, `"search:auth"`, and `"symbol:auth"`
- **AND** no entry is silently omitted

#### Scenario: Every route entry produces exactly one hint item

- **GIVEN** a `RouteManifestDocument` with N route entries
- **WHEN** the hint producer runs
- **THEN** the emitted `hints` array contains exactly N entries
- **AND** each hint's `routeId` matches a distinct route entry `id`

### Requirement: Hint generation is deterministic

The hint producer SHALL produce byte-identical output for identical `RouteManifestDocument` input. The `hints` array SHALL follow the same order as the source `routes` array (sorted by `id` ascending). The output SHALL NOT include non-deterministic fields, wall-clock timestamps, random identifiers, or iteration-order-dependent content.

#### Scenario: Repeated runs produce identical output

- **GIVEN** the same `RouteManifestDocument` input
- **WHEN** `hints_from_manifest` is called twice
- **THEN** the returned `RouteHintDocument` values are equal
- **AND** their serialized JSON is byte-identical

#### Scenario: Hints follow manifest entry order

- **GIVEN** a manifest with routes sorted by id ascending: `"file:aaa.rs"`, `"file:zzz.rs"`, `"search:routing"`
- **WHEN** the hint producer runs
- **THEN** the hints array order is hint for `"file:aaa.rs"`, then `"file:zzz.rs"`, then `"search:routing"`

### Requirement: Hint generation does not mutate graph, route, or proposal artifacts

The hint producer SHALL consume `RouteManifestDocument` as a read-only input. It SHALL NOT read or write `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, or any other filesystem artifact.

#### Scenario: Hint generation is a pure function over the manifest

- **GIVEN** a `RouteManifestDocument` value in memory
- **WHEN** `hints_from_manifest(manifest)` is called
- **THEN** the function returns a `RouteHintDocument` without performing any filesystem I/O
- **AND** no `.scryrs/` artifacts are created, modified, or deleted

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

