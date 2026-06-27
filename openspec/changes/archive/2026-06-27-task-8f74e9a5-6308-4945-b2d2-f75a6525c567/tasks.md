## 1. Shared graph contract in `crates/scryrs-types`

- [x] 1.1 Add `GRAPH_SCHEMA_VERSION` as an independent public constant.
- [x] 1.2 Replace the placeholder `GraphNode` with serde-decorated camelCase DTOs for `KnowledgeGraphDocument`, `GraphMetadata`, `GraphNode`, `GraphEdge`, `EvidenceLink`, and `EvidenceSourceKind`.
- [x] 1.3 Add route-ready node and edge fields: stable IDs, labels, optional descriptions, string-backed node kind or edge relationship, tags, aliases where applicable, evidence links, and bounded metadata.
- [x] 1.4 Define the flat `EvidenceLink` field rules for `hotspot_subject`, `local_trace_row`, `server_trace_row`, `doc_reference`, and `recorded_evidence`, including ordered `rowIds` handling and optional `docRef`, `description`, and `score`.
- [x] 1.5 Define the bounded metadata-map convention shared by graph metadata, nodes, edges, and evidence links.

## 2. Graph container and validation in `crates/scryrs-graph`

- [x] 2.1 Extend `KnowledgeGraph` to store both nodes and directed edges under the new DTO contract.
- [x] 2.2 Add structural validation so every edge references existing node IDs.
- [x] 2.3 Add deterministic document materialization that sorts nodes, edges, tags, aliases, and evidence links according to the contract.
- [x] 2.4 Expose the container and materialization behavior without adding graph-build, CLI, server, adapter, or runtime-retrieval functionality.

## 3. Verification

- [x] 3.1 Add JSON round-trip tests for `KnowledgeGraphDocument` and the new graph DTOs.
- [x] 3.2 Add deterministic-ordering tests covering nodes, edges, tags, aliases, and evidence links.
- [x] 3.3 Add validation tests that reject dangling edge references.
- [x] 3.4 Add compatibility tests or fixtures showing evidence links can be constructed from local `HotspotEntry.evidence.rowIds` and live `HotspotSignal.evidenceRowIds` / server evidence rows without changing the existing hotspot contracts.

## 4. Scope guard

- [x] 4.1 Verify the change adds no graph build pipeline, no `scryrs graph` CLI command, no route-manifest generator, no docs crawler, no server endpoint, and no runtime retrieval behavior.
