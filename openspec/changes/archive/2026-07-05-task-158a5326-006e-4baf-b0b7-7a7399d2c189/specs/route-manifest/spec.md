## MODIFIED Requirements

### Requirement: Route entries carry structured identity, target, evidence, and related edge context

Each `RouteEntry` SHALL include `id` (the graph node ID), `subjectKind` (the node kind), `subject` (raw subject value), `label` (human-readable label), `target` (the stable source graph node ID preserved for identity and matching), `kind` (node kind repeated), and `evidenceLinks` (provenance backlinks). Optional fields SHALL include `loadTarget`, `grouping`, `relatedEdges`, and `metadata`.

When present, `loadTarget` SHALL be a structured object with `kind` and optional `reference`. `file` routes SHALL use a repository-relative file `reference`; `doc_page` routes SHALL use a canonical docs `reference` in the form `project-docs/<slug>`; non-loadable kinds SHALL use `kind = "non_loadable"` with no `reference`.

When present, `relatedEdges` SHALL be an array of summaries for outgoing non-`contains` graph edges from the source node. Each summary SHALL include `relationship`, `targetRouteId`, and `evidenceLinks`. `targetRouteId` SHALL equal the related route entry `id` for the edge target. `relatedEdges` SHALL be omitted when no outgoing non-`contains` edges exist for the route.

#### Scenario: File route keeps node identity and exposes a loadable file reference

- **GIVEN** a graph node with `id = "file:src/main.rs"`, `kind = "file"`, `label = "src/main.rs"`, and an `EvidenceLink` with `sourceKind = "local_trace_row"`
- **WHEN** the route generator emits a route entry for this node
- **THEN** the entry's `id` is `"file:src/main.rs"`
- **AND** `subjectKind` is `"file"`
- **AND** `subject` is `"src/main.rs"`
- **AND** `label` is `"src/main.rs"`
- **AND** `target` is `"file:src/main.rs"`
- **AND** `loadTarget.kind` is `"file"`
- **AND** `loadTarget.reference` is `"src/main.rs"`
- **AND** `evidenceLinks` contains the same evidence link with `sourceKind = "local_trace_row"`

#### Scenario: Docs routes normalize to a canonical docs reference

- **GIVEN** a `doc_page` route whose usable docs reference is either `"graph"` or `"project-docs/graph"`
- **WHEN** the route generator emits a route entry
- **THEN** `loadTarget.kind` is `"doc_page"`
- **AND** `loadTarget.reference` is exactly `"project-docs/graph"`

#### Scenario: Route entry for doc pages preserves doc_reference evidence

- **GIVEN** a graph node with `id = "doc_page:graph"`, `kind = "doc_page"`, and an `EvidenceLink` with `sourceKind = "doc_reference"` and `docRef = "graph"`
- **WHEN** the route generator emits a route entry
- **THEN** the entry's `evidenceLinks` contains one link with `sourceKind = "doc_reference"`
- **AND** that link's `docRef` is `"graph"`
- **AND** `loadTarget.reference` is `"project-docs/graph"`

#### Scenario: Non-loadable subject kinds remain explicit

- **GIVEN** a graph node with `id = "search:auth"`, `kind = "search"`, and label `"auth"`
- **WHEN** the route generator emits a route entry
- **THEN** `target` remains `"search:auth"`
- **AND** `loadTarget.kind` is `"non_loadable"`
- **AND** `loadTarget` has no `reference`

#### Scenario: Related edges expose cross-domain adjacency on the source route

- **GIVEN** `.scryrs/graph.json` contains an edge with `relationship = "search_result"`, `sourceNodeId = "search:graph"`, and `targetNodeId = "doc_page:graph"`
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the route entry for `search:graph` includes one `relatedEdges` item with `relationship = "search_result"`
- **AND** `targetRouteId = "doc_page:graph"`
- **AND** `evidenceLinks` matches the source graph edge evidence

#### Scenario: Routes without outgoing non-contains edges omit relatedEdges

- **GIVEN** a graph node with no outgoing non-`contains` edges
- **WHEN** the route generator emits its route entry
- **THEN** the entry omits `relatedEdges`

#### Scenario: Malformed file subject fails loudly

- **GIVEN** a graph node whose `file:` subject is empty, absolute, or parent-traversing
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the command exits with code `2`
- **AND** stderr explains that file routes must resolve to a non-empty repository-relative path without parent traversal

#### Scenario: Doc page without a usable docs reference fails loudly

- **GIVEN** a `doc_page` graph node that cannot produce a usable `DocReference`-derived docs reference
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the command exits with code `2`
- **AND** stderr explains that doc_page routes must provide a canonical docs reference

### Requirement: Grouping is derived only from explicit contains edges

Route entries SHALL carry an optional `grouping` field ONLY when the source graph node is the target of a `contains` edge from a parent group node. That includes parent group nodes materialized from accepted `semantic_graph_grouping` review decisions during graph build. Non-`contains` edges SHALL populate `relatedEdges` only and SHALL NOT create grouping, merge route entries, or change one-entry-per-node behavior. Route generation SHALL continue to consume `.scryrs/graph.json` only and SHALL NOT read `.scryrs/accepted/`, `.scryrs/rejected/`, or `.scryrs/proposals/` directly.

#### Scenario: Accepted semantic grouping appears through normal graph consumption

- **GIVEN** `.scryrs/graph.json` contains a node `domain_term:auth`
- **AND** `.scryrs/graph.json` contains a `contains` edge from `domain_term:auth` to `file:auth`
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the route entry for `file:auth` includes `grouping.groupId = "domain_term:auth"`
- **AND** `grouping.groupLabel` equals the parent node label
- **AND** route generation does not inspect proposal or review-artifact directories

#### Scenario: Cross-domain edges do not create grouping

- **GIVEN** `.scryrs/graph.json` contains `file:src/auth.rs`, `symbol:Authenticator`, and a `symbol_inspected_during_file_context` edge from the file node to the symbol node
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the route entry for `symbol:Authenticator` does not gain a `grouping` field from that cross-domain edge
- **AND** the route entry for `file:src/auth.rs` remains a separate route entry with `relatedEdges` instead of grouping

### Requirement: Route manifest output is deterministic

The generator SHALL produce byte-identical JSON output for identical `.scryrs/graph.json` input. The `routes` array SHALL sort by `id` ascending. Evidence links within each entry SHALL sort by `(sourceKind, subject, docRef, description, rowIds, score)` ascending. `relatedEdges`, when present, SHALL sort by `(relationship, targetRouteId)` ascending, with each `relatedEdges[*].evidenceLinks` using the same evidence-link ordering rule. The output SHALL NOT include wall-clock timestamps, random identifiers, or non-deterministic iteration.

#### Scenario: Repeated runs produce identical JSON

- **GIVEN** the same `.scryrs/graph.json` artifact
- **WHEN** `scryrs route <PATH>` is run twice
- **THEN** the stdout output is byte-identical across both runs
- **AND** `.scryrs/routes.json` is byte-identical across both runs

#### Scenario: Routes are sorted by id ascending

- **GIVEN** graph nodes with IDs `"file:zzz.rs"`, `"file:aaa.rs"`, `"search:routing"`
- **WHEN** the route generator sorts routes
- **THEN** the routes array order is `"file:aaa.rs"`, `"file:zzz.rs"`, `"search:routing"` (lexicographic by node ID)

#### Scenario: Related edges are sorted deterministically

- **GIVEN** a route entry has outgoing non-`contains` edges to `doc_page:graph` with relationship `"search_result"` and to `symbol:Authenticator` with relationship `"symbol_inspected_during_file_context"`
- **WHEN** the route generator serializes `relatedEdges`
- **THEN** the `relatedEdges` array is sorted by `relationship`, then `targetRouteId`
- **AND** repeated runs preserve the same order

#### Scenario: No wall-clock timestamps in output

- **GIVEN** a valid route manifest has been produced
- **WHEN** the output is inspected
- **THEN** no fields contain wall-clock timestamps
- **AND** the document's `metadata` does not include generation time fields
