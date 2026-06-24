## ADDED Requirements

### Requirement: Graph contract envelope is versioned and independent

The system SHALL define a `GraphEnvelope` JSON contract with `schemaVersion`, `metadata`, `nodes`, and `edges` fields. `schemaVersion` SHALL be set from an independent `GRAPH_SCHEMA_VERSION` constant with value `"0.1.0"` and SHALL NOT reuse trace-event or hotspot-report schema version constants.

#### Scenario: Envelope carries required top-level fields

- **GIVEN** a serialized knowledge graph contract
- **WHEN** a consumer inspects the top-level object
- **THEN** it contains `schemaVersion`
- **AND** it contains `metadata`
- **AND** it contains `nodes` as a JSON array
- **AND** it contains `edges` as a JSON array

#### Scenario: Graph schema version is independent

- **GIVEN** `SCHEMA_VERSION` and `HOTSPOT_SCHEMA_VERSION` already exist for other contracts
- **WHEN** the graph contract is defined
- **THEN** it defines `GRAPH_SCHEMA_VERSION`
- **AND** `GRAPH_SCHEMA_VERSION` is `"0.1.0"`
- **AND** the envelope `schemaVersion` field uses `GRAPH_SCHEMA_VERSION`, not the other version constants

### Requirement: Graph metadata is explicit and deterministic

The system SHALL define `GraphMetadata` as explicit graph-level metadata for the serialized contract. `GraphMetadata` SHALL include deterministic count fields `nodeCount` and `edgeCount`. This task SHALL NOT require a wall-clock `generatedAt` or other runtime timestamp in graph metadata.

#### Scenario: Metadata reports serialized counts

- **GIVEN** a graph envelope containing 2 nodes and 1 edge
- **WHEN** a consumer inspects `metadata`
- **THEN** `metadata.nodeCount` is `2`
- **AND** `metadata.edgeCount` is `1`

#### Scenario: Metadata stays deterministic without wall-clock data

- **GIVEN** the same logical graph serialized twice
- **WHEN** the graph contract is compared across both serializations
- **THEN** `metadata` does not require a wall-clock timestamp field
- **AND** no `generatedAt` field is needed for this task

### Requirement: Nodes and edges expose stable routing fields

The system SHALL define explicit `GraphNode` and `GraphEdge` shapes suitable for later route-manifest generation. `GraphNode` SHALL include `id`, `kind`, `title`, and `evidence`. `GraphEdge` SHALL include `id`, `sourceId`, `targetId`, `kind`, `label`, and `evidence`. These fields SHALL be JSON-serializable with deterministic camelCase names where applicable.

#### Scenario: Node exposes stable identity and title

- **GIVEN** a graph node representing a hotspot subject
- **WHEN** a consumer inspects the node
- **THEN** the node contains a stable `id`
- **AND** the node contains a machine-readable `kind`
- **AND** the node contains a human-readable `title`
- **AND** the node contains an `evidence` array

#### Scenario: Edge exposes stable relationship endpoints

- **GIVEN** a graph edge describing a relationship between two nodes
- **WHEN** a consumer inspects the edge
- **THEN** the edge contains a stable `id`
- **AND** the edge contains `sourceId` and `targetId`
- **AND** the edge contains a machine-readable `kind`
- **AND** the edge contains a human-readable `label`
- **AND** the edge contains an `evidence` array

### Requirement: Produced nodes and edges require evidence

Every produced `GraphNode` and `GraphEdge` SHALL carry at least one `EvidenceLink`. The public graph-construction helpers in `scryrs-graph` SHALL enforce this invariant through validated construction instead of relying on empty-vector serde rejection.

#### Scenario: Producer rejects node without evidence

- **GIVEN** graph-construction code attempts to create a node with `evidence = []`
- **WHEN** the validated constructor or helper is called
- **THEN** construction fails
- **AND** the node is not produced as a successful graph result

#### Scenario: Producer rejects edge without evidence

- **GIVEN** graph-construction code attempts to create an edge with `evidence = []`
- **WHEN** the validated constructor or helper is called
- **THEN** construction fails
- **AND** the edge is not produced as a successful graph result

### Requirement: Evidence links are explicit, source-typed, and docs-framework agnostic

The system SHALL define `EvidenceLink` as an internally tagged enum with explicit variants for `HotspotSubject`, `TraceRows`, `DocumentRef`, and `LiveSignal`. `HotspotSubject` SHALL carry `subjectKind` and `subject`. `TraceRows` SHALL carry `rowIds: Vec<u64>`. `DocumentRef` SHALL carry a framework-agnostic `docRef` string. `LiveSignal` SHALL carry a reserved generic `sourceId` string for future Live Hotspot compatibility.

#### Scenario: Hotspot subject evidence is explicit

- **GIVEN** a graph node derived from a hotspot subject
- **WHEN** a consumer inspects one of its evidence links
- **THEN** the evidence link can be identified as `HotspotSubject`
- **AND** it carries `subjectKind`
- **AND** it carries `subject`

#### Scenario: Trace row evidence preserves trace-row references

- **GIVEN** a graph node or edge backed by recorded trace evidence
- **WHEN** a consumer inspects a `TraceRows` evidence link
- **THEN** the link contains `rowIds`
- **AND** each row ID refers to a recorded trace row identifier

#### Scenario: Document references stay framework agnostic

- **GIVEN** a graph node or edge cites documentation evidence
- **WHEN** a consumer inspects a `DocumentRef` evidence link
- **THEN** it contains a `docRef` string
- **AND** the `docRef` does not depend on any single docs framework concept

#### Scenario: Live Hotspot compatibility is reserved without locking identifier shape

- **GIVEN** future Live Hotspot evidence becomes available
- **WHEN** a consumer inspects a `LiveSignal` evidence link
- **THEN** it contains a generic `sourceId`
- **AND** the graph contract does not assume a finalized live signal identifier format in this task

### Requirement: Hotspot-report evidence maps directly into graph evidence links

The graph contract SHALL document and preserve the mapping from existing hotspot outputs into evidence links. `HotspotEntry.subjectKind` and `HotspotEntry.subject` SHALL map to `EvidenceLink::HotspotSubject`. `HotspotEvidence.rowIds` SHALL map to `EvidenceLink::TraceRows.rowIds` without changing their ordering semantics.

#### Scenario: Existing hotspot subject becomes hotspot-subject evidence

- **GIVEN** a hotspot entry with `subjectKind = "file"` and `subject = "src/main.rs"`
- **WHEN** graph evidence links are created from that entry
- **THEN** one evidence link can be represented as `HotspotSubject { subjectKind: "file", subject: "src/main.rs" }`

#### Scenario: Existing hotspot rowIds preserve report ordering

- **GIVEN** a hotspot entry with `evidence.rowIds` ordered by `timestamp ASC, id ASC`
- **WHEN** graph evidence links are created from that entry
- **THEN** the `TraceRows.rowIds` field uses the same ordered row ID list
- **AND** consumers can join those row IDs back to recorded trace evidence

### Requirement: Contract serialization is deterministic and route-manifest suitable

The graph contract SHALL be JSON-serializable and suitable for later route-manifest generation. Producers SHALL emit `nodes` in deterministic `id` order and `edges` in deterministic `id` order. The contract SHALL provide the stable node identifiers, edge endpoints, relationship kinds, human-readable node and edge labels, and evidence-backed relationships required by later routing consumers.

#### Scenario: Repeated serialization stays comparable

- **GIVEN** the same logical graph contents
- **WHEN** the graph contract is serialized twice
- **THEN** `nodes` appear in the same deterministic order
- **AND** `edges` appear in the same deterministic order
- **AND** consumers can compare the results without docs-framework-specific interpretation

#### Scenario: Routing consumers can resolve endpoints and reasons

- **GIVEN** a downstream route-manifest generator reads the graph contract
- **WHEN** it inspects a relationship
- **THEN** it can resolve the source node via `sourceId`
- **AND** it can resolve the target node via `targetId`
- **AND** it can use `kind` and `label` to interpret the relationship
- **AND** it can inspect `evidence` to understand why the relationship exists

### Requirement: This task adds no graph build or runtime surface

This change SHALL remain schema-only. It SHALL NOT add graph build commands, emitted graph artifacts, route manifests, dashboard graph views, live hotspot server behavior, or proposal-engine behavior.

#### Scenario: No new graph CLI or artifact is introduced

- **WHEN** this task is implemented
- **THEN** no `scryrs graph` command is added
- **AND** no `.scryrs/graph.json` artifact is emitted
- **AND** no `.scryrs/routes.json` artifact is emitted

#### Scenario: No live hotspot or proposal behavior is introduced

- **WHEN** this task is implemented
- **THEN** no live hotspot signal production or server API is added
- **AND** no proposal-engine behavior depends on the graph contract yet
