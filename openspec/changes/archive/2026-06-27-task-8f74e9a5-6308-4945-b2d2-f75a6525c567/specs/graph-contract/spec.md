## ADDED Requirements

### Requirement: Knowledge graph documents are versioned and self-describing

The system SHALL define a `KnowledgeGraphDocument` wire contract for graph consumers. Each serialized graph document SHALL include `schemaVersion`, `metadata`, `nodes`, and `edges`. `schemaVersion` SHALL equal `GRAPH_SCHEMA_VERSION` and SHALL be versioned independently from `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and `LIVE_HOTSPOT_SCHEMA_VERSION`.

#### Scenario: Serialized document carries the required top-level fields

- **GIVEN** a knowledge graph document produced under this contract
- **WHEN** a consumer inspects the serialized JSON
- **THEN** it contains `schemaVersion`
- **AND** it contains `metadata`
- **AND** it contains `nodes` as a JSON array
- **AND** it contains `edges` as a JSON array

#### Scenario: Graph schema version is independent from trace and hotspot versions

- **GIVEN** the workspace already defines `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and `LIVE_HOTSPOT_SCHEMA_VERSION`
- **WHEN** the graph contract is versioned
- **THEN** it defines a separate `GRAPH_SCHEMA_VERSION`
- **AND** every serialized graph document uses `GRAPH_SCHEMA_VERSION` for `schemaVersion`

#### Scenario: Top-level metadata stays compatible with local and server contexts

- **GIVEN** a graph document emitted from a local repository or a server-owned repository context
- **WHEN** the consumer inspects `metadata`
- **THEN** `metadata.repositoryId` MAY be present for server-owned graphs
- **AND** `metadata.repositoryId` MAY be absent for local-only graphs
- **AND** the presence or absence of `repositoryId` does not change the graph schema version

### Requirement: Metadata extensions are bounded and namespaced

The contract SHALL use optional metadata maps on `GraphMetadata`, `GraphNode`, `GraphEdge`, and `EvidenceLink`. Each metadata map SHALL be a string-keyed object with JSON values. Keys SHALL use reverse-domain notation or an explicit snake_case namespace prefix. Metadata SHALL be additive only and SHALL NOT replace first-class contract fields.

#### Scenario: Namespaced metadata extends the contract additively

- **GIVEN** a graph producer needs additive metadata on a node, edge, document, or evidence link
- **WHEN** the metadata is serialized
- **THEN** it is emitted as a string-keyed object with JSON values
- **AND** its keys use reverse-domain notation or an explicit snake_case namespace prefix
- **AND** core fields such as `id`, `relationship`, and `sourceKind` remain first-class fields rather than being duplicated in metadata

#### Scenario: Docs-framework-specific metadata does not become part of the core contract

- **GIVEN** an adapter wants to attach framework-specific information
- **WHEN** the core graph contract is defined
- **THEN** docs-framework-specific keys such as `rspress_*`, `docusaurus_*`, and `vitepress_*` are not first-class graph fields
- **AND** the core contract remains docs-framework agnostic

### Requirement: Graph nodes and edges carry explicit route-ready identity

`GraphNode` SHALL include a stable `id`, human-readable `label`, optional `description`, string-backed `kind`, `tags`, `aliases`, `evidenceLinks`, and optional metadata. `GraphEdge` SHALL include a stable `id`, directed `sourceNodeId` and `targetNodeId`, string-backed `relationship`, optional `label`, `tags`, `evidenceLinks`, and optional metadata.

#### Scenario: Node fields support route-manifest filtering and explanation

- **GIVEN** a graph node representing a hotspot-backed subject
- **WHEN** a route or proposal consumer inspects the node
- **THEN** the node exposes a stable `id`
- **AND** the node exposes a human-readable `label`
- **AND** the node MAY expose `description`
- **AND** the node MAY expose `kind`
- **AND** the node exposes `tags` and `aliases` for downstream filtering or matching
- **AND** the node exposes `evidenceLinks` for provenance

#### Scenario: Edge fields represent directed relationships

- **GIVEN** a graph edge connecting two nodes
- **WHEN** a consumer inspects the edge
- **THEN** the edge exposes a stable `id`
- **AND** the edge exposes `sourceNodeId` and `targetNodeId`
- **AND** the edge exposes a string-backed `relationship`
- **AND** the edge MAY expose `label` and `tags`
- **AND** the edge exposes `evidenceLinks` for provenance
- **AND** the edge direction is from `sourceNodeId` to `targetNodeId`

### Requirement: Evidence links encode provenance across local, live, docs, and recorded evidence

The contract SHALL use a flat `EvidenceLink` shape with a closed `EvidenceSourceKind` enum serialized as snake_case strings. Supported source kinds SHALL include `hotspot_subject`, `local_trace_row`, `server_trace_row`, `doc_reference`, and `recorded_evidence`. Each evidence link SHALL carry `sourceKind`, `subject`, `rowIds`, and optional `docRef`, `description`, `score`, and metadata.

#### Scenario: Local hotspot evidence preserves ordered trace row IDs

- **GIVEN** a local hotspot entry with `evidence.rowIds = [5, 12, 23]` for a subject
- **WHEN** a graph producer emits a matching evidence link
- **THEN** `sourceKind` is `local_trace_row`
- **AND** `subject` identifies the hotspot subject
- **AND** `rowIds` is `[5, 12, 23]`
- **AND** the row order matches the local hotspot evidence order

#### Scenario: Live hotspot evidence distinguishes server-owned row IDs

- **GIVEN** a live hotspot or signal payload with ordered `server_trace_events` row IDs
- **WHEN** a graph producer emits a matching evidence link
- **THEN** `sourceKind` is `server_trace_row`
- **AND** `subject` identifies the hotspot subject
- **AND** `rowIds` preserves the ordered server row IDs
- **AND** consumers can distinguish the server row-ID domain from local `trace_events.id`

#### Scenario: Doc and recorded evidence use explicit non-row fields

- **GIVEN** a graph node or edge cites a documentation reference or a recorded evidence descriptor
- **WHEN** the consumer inspects the evidence links
- **THEN** a doc reference link uses `sourceKind = doc_reference` and `docRef`
- **AND** a recorded evidence link uses `sourceKind = recorded_evidence` and `description`
- **AND** `rowIds` is empty for evidence sources that do not refer to trace rows

#### Scenario: Score snapshots are optional and non-identifying

- **GIVEN** an evidence link carries a hotspot score snapshot
- **WHEN** a consumer compares provenance identity across runs
- **THEN** `score` is treated as auxiliary snapshot data
- **AND** stable linkage continues to depend on source kind, subject, row IDs, `docRef`, or `description` rather than on `score`

### Requirement: Graph serialization is deterministic

Graph documents SHALL serialize deterministically for the same logical input. The `nodes` array SHALL sort by `id` ascending. The `edges` array SHALL sort by `id` ascending. Node `tags`, node `aliases`, and edge `tags` SHALL sort lexicographically ascending. `evidenceLinks` SHALL sort by `(sourceKind, subject, docRef, description, rowIds, score)` ascending, treating missing string fields as empty values and comparing `rowIds` lexicographically as ordered lists.

#### Scenario: Repeated materialization preserves node and edge ordering

- **GIVEN** the same set of graph nodes and graph edges
- **WHEN** the graph document is materialized twice
- **THEN** `nodes` appear in the same `id`-ascending order in both outputs
- **AND** `edges` appear in the same `id`-ascending order in both outputs

#### Scenario: Evidence links use a documented tie-break chain

- **GIVEN** multiple evidence links attached to the same node or edge
- **WHEN** they are serialized
- **THEN** they are ordered by `sourceKind`, then `subject`, then `docRef`, then `description`, then `rowIds`, then `score`
- **AND** the same logical input produces the same evidence-link order across repeated serialization

### Requirement: The graph crate validates structure without implementing a build pipeline

`crates/scryrs-graph` SHALL provide a `KnowledgeGraph` container that stores nodes and directed edges under the shared contract, validates structural references, preserves evidence links, and materializes a deterministic `KnowledgeGraphDocument`. This foundation SHALL NOT implement graph build, CLI graph commands, route-manifest generation, docs crawling, adapter integration, server endpoints, dashboard work, or runtime retrieval behavior.

#### Scenario: Dangling edges are rejected

- **GIVEN** a graph edge whose `sourceNodeId` or `targetNodeId` does not match any node in the graph
- **WHEN** the graph is validated or materialized
- **THEN** the graph is rejected as invalid
- **AND** the invalid edge is not silently preserved as successful output

#### Scenario: Contract foundation adds no graph build surface

- **WHEN** this change is implemented
- **THEN** no `scryrs graph build` command is introduced
- **AND** no route-manifest generator is introduced
- **AND** no docs crawler, server endpoint, or runtime retrieval behavior is introduced
