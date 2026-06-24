## 1. Define the shared graph contract in `scryrs-types`

- [ ] 1.1 Add `GRAPH_SCHEMA_VERSION = "0.1.0"` as a graph-specific schema constant independent from the existing trace-event and hotspot-report version constants.
- [ ] 1.2 Replace the placeholder `GraphNode` with explicit serde-ready `GraphEnvelope`, `GraphMetadata`, `GraphNode`, `GraphEdge`, and `EvidenceLink` contract types.
- [ ] 1.3 Model `EvidenceLink` as an internally tagged enum with `HotspotSubject`, `TraceRows`, `DocumentRef`, and reserved `LiveSignal` variants.
- [ ] 1.4 Keep the contract docs-framework agnostic by representing document evidence through generic document references rather than framework-specific fields.

## 2. Update `scryrs-graph` to consume and validate the contract

- [ ] 2.1 Update graph-crate types and helpers to consume the new shared graph contract instead of the placeholder node-only shape.
- [ ] 2.2 Add validated constructors or helpers that reject produced nodes with empty `evidence`.
- [ ] 2.3 Add validated constructors or helpers that reject produced edges with empty `evidence`.
- [ ] 2.4 Keep the graph crate schema-only in this task: no builders, no route manifests, no emitted graph artifacts, and no new CLI surfaces.

## 3. Verify determinism and hotspot evidence compatibility

- [ ] 3.1 Add serialization round-trip coverage for a graph containing at least one node, one edge, and mixed evidence kinds.
- [ ] 3.2 Add validation coverage proving empty evidence is rejected for produced nodes and edges.
- [ ] 3.3 Add fixture or example coverage showing how `HotspotEntry.subjectKind`, `HotspotEntry.subject`, and `HotspotEvidence.rowIds` map into graph evidence links.
- [ ] 3.4 Verify deterministic ordering expectations for serialized nodes, edges, and trace-row evidence.

## 4. Publish the OpenSpec contract artifacts

- [ ] 4.1 Add `specs/graph-contract/spec.md` describing the versioned graph envelope, explicit evidence-link variants, hotspot evidence mapping, and schema-only scope boundary.
- [ ] 4.2 Publish proposal and design text that records the crate-boundary, versioning, Live Hotspot compatibility, and non-goal decisions for this foundation task.
