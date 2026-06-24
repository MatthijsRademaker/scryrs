## Context

The task is a foundation-only graph contract change. Current workspace types still expose a placeholder `GraphNode { id, title }`, and `scryrs-graph` remains a node-only scaffold with no explicit edge, evidence, metadata, or schema-version contract. At the same time, the roadmap requires graph and route work to remain behind Live Hotspot contract stabilization, so this change must stop at schema definition and validation.

The implementation therefore needs to do two things at once: publish a precise OpenSpec contract and land matching shared Rust types that later route-manifest and proposal work can consume without introducing any graph builder, emitted artifact, CLI command, or live runtime behavior.

## Goals

1. Define a versioned, machine-readable graph envelope with explicit node, edge, evidence-link, and metadata fields.
2. Ensure every produced node and edge can cite at least one evidence link back to hotspot subjects, trace rows, document references, or a reserved live-signal source.
3. Keep the contract deterministic, JSON-serializable, and docs-framework agnostic.
4. Make the contract suitable for later route-manifest generation by carrying stable node IDs, edge IDs, edge endpoints, relationship kinds, and human-readable node or edge labels.
5. Document the mapping from existing `HotspotEntry.subjectKind`, `HotspotEntry.subject`, and `HotspotEvidence.rowIds` into graph evidence links.

## Non-Goals

- Do not implement graph building, route-manifest generation, or emitted graph artifacts.
- Do not add any `scryrs graph` or `scryrs route` CLI surface.
- Do not implement live hotspot signal production, server APIs, or proposal-engine behavior.
- Do not couple the contract to Rspress, Markdown frontmatter specifics, or any single documentation framework.
- Do not preserve the placeholder graph node shape for backwards compatibility.

## Decisions

### D1: Shared contract ownership lives in `scryrs-types`

The version constant and serializable contract types belong in `crates/scryrs-types`, which already owns cross-crate wire contracts such as trace events and hotspot reports. `crates/scryrs-graph` consumes those types and owns graph-specific construction helpers and validation.

### D2: Graph schema version is independent and starts at `0.1.0`

The graph contract introduces a new `GRAPH_SCHEMA_VERSION = "0.1.0"` that is independent of both trace-event `SCHEMA_VERSION` and hotspot-report `HOTSPOT_SCHEMA_VERSION`. The accepted refinement evidence resolved this as a pre-artifact contract version, not a delivered build artifact version.

### D3: Evidence links use an internally tagged enum

`EvidenceLink` is an internally tagged, source-typed enum with explicit variants for:

- `HotspotSubject { subjectKind, subject }`
- `TraceRows { rowIds }`
- `DocumentRef { docRef }`
- `LiveSignal { sourceId }`

This is the most explicit way to satisfy the evidence-traceability requirement while keeping document evidence framework-agnostic and reserving Live Hotspot compatibility without locking a future identifier format.

### D4: Nodes and edges carry stable routing fields plus non-empty evidence

`GraphNode` carries stable identity, kind, title, and evidence fields. `GraphEdge` carries stable identity, source and target endpoints, kind, label, and evidence fields. Produced nodes and edges must never be emitted without evidence; that invariant is enforced through validated construction in `scryrs-graph` rather than through serde alone.

### D5: Graph metadata stays deterministic in this foundation task

The contract includes explicit graph-level metadata, but it remains deterministic and builder-free. `GraphMetadata` carries deterministic count fields such as `nodeCount` and `edgeCount`, and the contract does not require a wall-clock `generatedAt` field before a real graph builder exists.

### D6: Serialization order is deterministic for consumer comparison

The contract is intended for machine comparison and later route-manifest generation. Producers emit nodes and edges in deterministic `id` order, and trace-row evidence preserves the existing hotspot-report ordering semantics (`timestamp ASC, id ASC`).

### D7: Scope stays schema-only

This change stops at contract definition, shared types, graph-crate validation, and tests. It explicitly does not add builders, runtime artifact emission, CLI surfaces, dashboard views, or live-server behavior.

## Conflict Resolution

- **Schema version**: resolved to `0.1.0` based on accepted refinement decisions that treated the graph contract as pre-artifact and independent from the hotspot-report version.
- **Contract placement**: resolved in favor of `scryrs-types` for shared serde contracts and `scryrs-graph` for validation helpers, matching the architecture boundary used elsewhere in the workspace.
- **Live Hotspot identifier shape**: resolved by reserving a generic `sourceId` field in the `LiveSignal` variant instead of guessing the final Phase 4 signal identifier contract.
- **Metadata determinism**: resolved by omitting wall-clock metadata such as `generatedAt` from this task and keeping metadata limited to deterministic graph-level fields.

## Risks

| Risk | Mitigation |
| --- | --- |
| Live Hotspot work may later choose a different identifier model. | Keep the reserved live evidence variant generic with `sourceId` and document that Phase 4 can refine semantics later without expanding this task now. |
| Replacing the placeholder shared `GraphNode` can break scaffold code if migration is incomplete. | Update `scryrs-graph` in the same change to consume the new shared contract types. |
| The non-empty evidence invariant cannot be enforced by serde alone. | Enforce it through validated graph-construction helpers or constructors in `scryrs-graph` and cover rejection with tests. |
| Later graph builders may need additional metadata once artifact emission exists. | Keep this version intentionally minimal and deterministic; future builder work can evolve the schema from the explicit `0.1.0` baseline. |

## Traceability

- Task: `51ad98d8-3e67-44d6-a16f-e04fe9cf7a47`
- Dossier: `2026-06-24T07:09:44.125Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Validated round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`
