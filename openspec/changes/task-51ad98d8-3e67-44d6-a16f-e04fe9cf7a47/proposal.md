## Why

The current graph surface is still scaffold-level: shared types only expose a placeholder `GraphNode { id, title }`, and `scryrs-graph` has no explicit edge, evidence, metadata, or schema-version contract. This task exists to freeze the machine-readable graph shape before later route-manifest and proposal work builds on it.

Roadmap guardrails also matter. Phase 5 needs graph and route contracts, but graph and route behavior remains deferred until Live Hotspot contracts stabilize. The safe scope here is therefore contract-first: define the schema, evidence-link model, and compatibility mapping now, without adding graph builders, CLI commands, emitted artifacts, or live-server behavior.

## What Changes

1. Add a `graph-contract` OpenSpec capability under this change that defines a versioned `GraphEnvelope` with explicit `schemaVersion`, `metadata`, `nodes`, and `edges` fields.
2. Define the shared graph contract in `crates/scryrs-types` with a new independent `GRAPH_SCHEMA_VERSION = "0.1.0"`, replacing the placeholder graph node shape with explicit `GraphNode`, `GraphEdge`, `GraphMetadata`, and `EvidenceLink` types.
3. Model `EvidenceLink` as an internally tagged, source-typed enum with explicit variants for hotspot subject references, trace row references, document references, and reserved live-signal references using a generic `sourceId`.
4. Update `crates/scryrs-graph` to consume the shared contract types and own graph-specific validation helpers or constructors that reject produced nodes or edges with empty `evidence`.
5. Add representative serialization and validation tests for a graph containing at least one node, one edge, and mixed evidence kinds, while intentionally adding no graph build pipeline, route-manifest generation, CLI surface, `.scryrs/graph.json`, or live hotspot implementation.

## Impact

- Later graph, route, and proposal consumers get a stable, explainable contract whose nodes and edges can trace back to hotspot subjects, document references, and recorded trace evidence.
- The change is intentionally contract-only: it affects shared types, graph crate integration, tests, and OpenSpec artifacts, but does not add any new runtime surface.
- The placeholder graph node contract can be replaced directly; no backwards-compatibility layer is required.
- Live Hotspot compatibility is reserved without locking the project into a premature signal identifier format.
