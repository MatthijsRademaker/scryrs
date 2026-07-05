## MODIFIED Requirements

### Requirement: Doc pages become graph nodes with structural edges

The builder SHALL scan `.devagent/docs/docs/` for `.md` and `.mdx` files and parse `_nav.json` for navigation hierarchy. Each discovered page SHALL become a `GraphNode` with kind `"doc_page"`. Nav hierarchy SHALL produce `contains` edges. These structural docs edges remain distinct from any separately derived cross-domain relationships and SHALL NOT change doc-page node identity.

#### Scenario: Doc pages are discovered from the docs directory

- **GIVEN** the docs directory `.devagent/docs/docs/` contains `graph.md`, `hotspots.md`, and `architecture.mdx`
- **WHEN** the builder scans the docs directory
- **THEN** a `GraphNode` is created for each page
- **AND** each node has `kind: "doc_page"`

#### Scenario: Doc page node IDs derive from page slugs

- **GIVEN** a doc page at `.devagent/docs/docs/graph.md` with a nav link `"/graph"`
- **WHEN** the builder creates a graph node
- **THEN** the node's `id` is `"doc_page:graph"`

#### Scenario: Doc pages carry doc_reference evidence links

- **GIVEN** a doc page with slug `graph`
- **WHEN** the builder creates a graph node
- **THEN** the node's `evidence_links` contains one entry with `sourceKind: "doc_reference"`
- **AND** `docRef` is `"graph"`
- **AND** `rowIds` is empty

#### Scenario: Nav hierarchy produces contains edges

- **GIVEN** `_nav.json` defines a nav group with items linking to `"/graph"` and `"/hotspots"`
- **WHEN** the builder processes the nav hierarchy
- **THEN** `contains` edges are created from the nav group node to `doc_page:graph` and `doc_page:hotspots`
- **AND** a synthetic `docs_root` node exists as the root of the doc hierarchy
- **AND** `contains` edges connect `docs_root` to each top-level nav group

## ADDED Requirements

### Requirement: Graph build derives deterministic cross-domain edges from local trace evidence

Graph build SHALL attempt cross-domain derivation only when the local `.scryrs/scryrs.db` trace store is available. It SHALL use hotspot evidence row IDs plus `TraceQuery::iter_events_with_ids_ordered()` to reconstruct same-session observations and SHALL apply exactly two v1 rules after hotspot nodes, docs nodes, and accepted semantic-grouping nodes exist. It SHALL NOT use LLM inference, fuzzy matching, substring matching, embeddings, or generic all-pairs co-occurrence.

Derived edges SHALL use stable IDs in the format `{relationship}_{sourceNodeId}_{targetNodeId}` and SHALL be deduplicated by `(relationship, source_node_id, target_node_id)`. Every derived edge SHALL carry evidence links citing both sides of the relationship. If the local trace store is absent, or if a candidate relationship lacks the required nodes or exact rule match, graph build SHALL emit no derived edge for that case and SHALL continue successfully.

#### Scenario: Missing local trace store skips cross-domain derivation

- **GIVEN** `.scryrs/hotspots.json` and optional docs inputs exist
- **AND** `.scryrs/scryrs.db` is absent
- **WHEN** `scryrs graph build <PATH>` runs
- **THEN** the command exits with code `0`
- **AND** hotspot, docs, and accepted-grouping nodes and edges are still emitted normally
- **AND** no derived cross-domain edges are added

#### Scenario: File and symbol evidence link deterministically

- **GIVEN** the graph already contains `file:src/auth.rs` and `symbol:Authenticator`
- **AND** hotspot evidence row IDs for those nodes resolve to `FileOpened(path = "src/auth.rs")` and `SymbolInspected(name = "Authenticator")` events with the same `session_id`
- **WHEN** graph build derives cross-domain edges
- **THEN** exactly one edge with `relationship = "symbol_inspected_during_file_context"` is emitted from `file:src/auth.rs` to `symbol:Authenticator`
- **AND** the edge ID is `"symbol_inspected_during_file_context_file:src/auth.rs_symbol:Authenticator"`
- **AND** the edge includes evidence links citing both the file-side and symbol-side trace evidence

#### Scenario: Search and document hotspot evidence link deterministically

- **GIVEN** the graph already contains `search:graph` and `document:/graph.mdx`
- **AND** hotspot evidence row IDs for those nodes resolve to `SearchRun(query = "graph")` and `DocRetrieved(doc_ref = "/graph.mdx")` events with the same `session_id`
- **WHEN** graph build derives cross-domain edges
- **THEN** exactly one edge with `relationship = "search_result"` is emitted from `search:graph` to `document:/graph.mdx`
- **AND** the edge includes evidence links citing both the search-side and document-side evidence

#### Scenario: Search and docs page evidence link deterministically

- **GIVEN** the graph already contains `search:graph` and `doc_page:graph`
- **AND** a `SearchRun(query = "graph")` event and a `DocRetrieved(doc_ref = "/graph.mdx")` event share the same `session_id`
- **AND** the normalized search query equals the normalized docs-page slug after lowercasing, stripping one leading `/`, and stripping a trailing `.md` or `.mdx`
- **WHEN** graph build derives cross-domain edges
- **THEN** exactly one edge with `relationship = "search_result"` is emitted from `search:graph` to `doc_page:graph`
- **AND** the edge includes evidence links citing search trace evidence and docs-page/document evidence

#### Scenario: Matching document and docs page remain separate targets

- **GIVEN** the graph contains `search:graph`, `document:/graph.mdx`, and `doc_page:graph`
- **AND** same-session `SearchRun(query = "graph")` and `DocRetrieved(doc_ref = "/graph.mdx")` evidence satisfies the search rule
- **WHEN** graph build derives cross-domain edges
- **THEN** it emits one `search_result` edge to `document:/graph.mdx`
- **AND** it emits a separate `search_result` edge to `doc_page:graph`
- **AND** no nodes are merged or replaced

#### Scenario: Duplicate qualifying observations aggregate into one derived edge

- **GIVEN** multiple same-session evidence observations qualify for `search_result` from `search:graph` to `doc_page:graph`
- **WHEN** graph build derives cross-domain edges
- **THEN** only one edge exists for that `(relationship, source, target)` tuple
- **AND** the edge carries the aggregated evidence links from the qualifying observations

#### Scenario: No exact match or missing target produces no derived edge

- **GIVEN** a `SearchRun(query = "graph routing")` event and a `DocRetrieved(doc_ref = "/graph.mdx")` event share the same `session_id`
- **AND** the graph does not contain another target node whose normalized subject exactly equals `graph routing`
- **WHEN** graph build derives cross-domain edges
- **THEN** no `search_result` edge is emitted for that search node

### Requirement: Cross-domain derivation preserves node identity and accepted semantic grouping

Derived cross-domain edges SHALL add context only. They SHALL reference existing graph nodes, SHALL NOT merge or rewrite hotspot/doc-page/group node identities, and SHALL NOT remove or alter accepted `semantic_graph_grouping` nodes or their `contains` edges.

#### Scenario: Shared labels across subject kinds remain distinct

- **GIVEN** the graph contains `file:auth`, `search:auth`, and `symbol:auth`
- **WHEN** graph build derives cross-domain edges
- **THEN** those three nodes remain distinct graph nodes
- **AND** the presence or absence of derived edges does not collapse them into one identity

#### Scenario: Accepted semantic grouping remains supported alongside derived edges

- **GIVEN** accepted evidence already created `domain_term:auth` with `contains` edges to `file:auth` and `search:auth`
- **AND** same-session evidence also qualifies `file:auth` for a derived edge to `symbol:AuthService`
- **WHEN** graph build completes
- **THEN** the accepted group node and its `contains` edges remain present
- **AND** the derived cross-domain edge is added without replacing or weakening the accepted grouping structure
