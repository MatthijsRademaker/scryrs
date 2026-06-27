## Why

The repository currently has only a scaffold-level graph foundation: `crates/scryrs-types` exposes a placeholder `GraphNode { id, title }`, and `crates/scryrs-graph` stores only a node list. There is no versioned graph document, no directed edges, no explicit evidence-link shape, no deterministic ordering rules, and no top-level metadata that can represent either local hotspot evidence or live server evidence provenance. Later route-manifest and proposal consumers need a stable machine-readable contract that can trace every node or edge back to hotspot subjects, local `trace_events.id` rows, live `server_trace_events.id` rows, docs references, or other recorded evidence.

## What Changes

1. **New graph contract capability**: add a `graph-contract` OpenSpec delta covering a versioned `KnowledgeGraphDocument`, top-level metadata, node and edge DTOs, evidence-link fields, deterministic ordering, bounded metadata extensions, and scope limits.
2. **Shared wire-contract types in `crates/scryrs-types`**: add `GRAPH_SCHEMA_VERSION`, `KnowledgeGraphDocument`, `GraphMetadata`, `GraphNode`, `GraphEdge`, `EvidenceLink`, and `EvidenceSourceKind`, replacing the current placeholder `GraphNode` without preserving the scaffold API.
3. **Graph container updates in `crates/scryrs-graph`**: extend `KnowledgeGraph` from node-only storage to node-plus-edge storage with structural validation and deterministic document materialization.
4. **Verification**: add serialization, ordering, edge-validation, and local/live evidence-link compatibility tests.
5. **Explicit scope boundary**: do not add `scryrs graph build`, route-manifest generation, docs crawling, adapter integration, server routes, dashboard work, or runtime retrieval behavior.

## Capabilities

### New Capabilities

- `graph-contract`: defines the versioned knowledge graph wire contract, route-ready node and edge fields, evidence provenance model, deterministic ordering rules, and compatibility with both local hotspot reports and Live Hotspot Foundation evidence.

### Modified Capabilities

- (none)

## Impact

- **Shared contracts**: `crates/scryrs-types/src/lib.rs` becomes the source of truth for graph DTOs and `GRAPH_SCHEMA_VERSION`.
- **Graph crate**: `crates/scryrs-graph/src/lib.rs` grows from a node scaffold into a container that holds nodes and directed edges under the new contract.
- **Compatibility surface**: local `HotspotEntry.evidence.rowIds`, live `HotspotSignal.evidenceRowIds`, and server-side `server_trace_events` / `hotspot_accumulators` evidence remain unchanged but become valid inputs to graph `EvidenceLink`s.
- **No new surfaces**: this change does not add CLI commands, build steps, adapters, server endpoints, dashboard routes, or runtime route retrieval.
