## Context

This task is a contract-foundation change, not a graph builder. Today `crates/scryrs-types` contains only a placeholder `GraphNode { id, title }` with no serde support, edges, evidence links, metadata, or schema versioning, and `crates/scryrs-graph` only stores `Vec<GraphNode>`. That blocks downstream route-manifest and proposal work because consumers cannot trace graph entities back to hotspot subjects, local hotspot evidence row IDs, live hotspot/server evidence row IDs, docs references, or recorded evidence through a stable machine-readable shape.

The repository already separates shared DTO ownership from crate-specific logic: `crates/scryrs-types` owns cross-crate contracts, while feature crates own containers and validation logic. The graph foundation should follow that pattern.

## Goals / Non-Goals

**Goals**

- Define a versioned graph wire contract with explicit top-level metadata, node, edge, evidence-link, and metadata-extension fields.
- Make nodes and edges traceable to hotspot subject identity, local `trace_events.id` row IDs, live `server_trace_events.id` row IDs, docs references, and recorded evidence descriptors.
- Keep the contract deterministic and docs-framework agnostic.
- Make the contract suitable for later route-manifest generation through stable IDs, directed relationships, labels, tags, aliases, and evidence-backed explanations.
- Keep shared DTOs in `crates/scryrs-types` and graph-specific container or validation logic in `crates/scryrs-graph`.

**Non-Goals**

- No `scryrs graph build`, route-manifest generation, docs crawling, adapter integration, server endpoints, dashboard work, or runtime retrieval behavior.
- No changes to hotspot scoring, trace ingestion, live hotspot accumulation, or existing hotspot/server contracts.
- No docs-framework-specific fields in the core graph schema.
- No LLM proposal generation or automatic documentation edits.

## Decisions

### 1. Shared DTOs live in `crates/scryrs-types`, and the placeholder `GraphNode` is replaced in place

Additive graph DTOs belong in `crates/scryrs-types` alongside `HotspotEntry`, `HotspotSignal`, and the live server contracts. The scaffold `GraphNode` is not preserved; it is replaced by the real graph contract types.

### 2. The graph document is a versioned top-level envelope

Define `GRAPH_SCHEMA_VERSION` independently from `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and `LIVE_HOTSPOT_SCHEMA_VERSION`. The serialized graph envelope is `KnowledgeGraphDocument { schemaVersion, metadata, nodes, edges }`.

`GraphMetadata` is kept minimal and deterministic: it includes optional `repositoryId` for server-context compatibility plus an optional bounded metadata map. Wall-clock timestamps are excluded from the core contract so identical graph inputs can materialize the same document shape without timestamp churn.

### 3. Nodes and edges are route-ready but not route-scored

`GraphNode` carries a stable `id`, human-readable `label`, optional `description`, string-backed `kind`, `tags`, `aliases`, `evidenceLinks`, and optional metadata. `GraphEdge` carries a stable `id`, directed `sourceNodeId` and `targetNodeId`, string-backed `relationship`, optional `label`, `tags`, `evidenceLinks`, and optional metadata.

This provides the identity and filtering surface later route-manifest work needs without introducing ranking, weight, or runtime routing semantics in this task.

### 4. Evidence uses a flat link shape with explicit source provenance

`EvidenceLink` is a flat DTO with a closed `EvidenceSourceKind` serialized as snake_case strings. Supported source kinds are `hotspot_subject`, `local_trace_row`, `server_trace_row`, `doc_reference`, and `recorded_evidence`.

Each evidence link carries `sourceKind`, `subject`, `rowIds`, and optional `docRef`, `description`, `score`, and metadata. `rowIds` preserves ordered local or server evidence rows when applicable; `docRef` carries document references; `description` carries recorded evidence descriptors; `score` is optional snapshot data only and not part of stable identity.

### 5. Metadata extensions are bounded and namespaced

`GraphMetadata`, `GraphNode`, `GraphEdge`, and `EvidenceLink` may each carry an optional string-keyed metadata map with JSON values. Keys must be namespaced via reverse-domain notation or an explicit snake_case namespace prefix. The metadata map is additive only; it cannot replace first-class fields, and docs-framework-specific prefixes such as `rspress_`, `docusaurus_`, and `vitepress_` are excluded from the core contract.

### 6. Deterministic ordering is part of the contract

The contract documents exact ordering rules instead of leaving them implicit:

- graph `nodes` sort by `id` ascending
- graph `edges` sort by `id` ascending
- node `tags`, node `aliases`, and edge `tags` sort lexicographically ascending
- `evidenceLinks` sort by `(sourceKind, subject, docRef, description, rowIds, score)` ascending, treating missing string fields as empty values and preserving `rowIds` as an ordered list comparison

Deterministic ordering applies to serialized arrays; metadata maps are additive annotations and not ordering keys.

### 7. `crates/scryrs-graph` owns structure validation and deterministic materialization

`KnowledgeGraph` in `crates/scryrs-graph` grows from node-only storage to node-plus-edge storage. It validates that every edge references existing node IDs and materializes a deterministically ordered `KnowledgeGraphDocument`. It does not build the graph from source data; it only holds, validates, and serializes the contract.

## Conflict Resolution

- **Evidence source strictness**: use a closed `EvidenceSourceKind` enum serialized as snake_case strings for known provenance kinds, while keeping `GraphNode.kind` and `GraphEdge.relationship` as string-backed fields for additive adapter evolution.
- **Metadata shape**: use namespaced JSON-value metadata maps, not framework-specific first-class fields.
- **Top-level metadata**: keep `repositoryId` optional for server compatibility and omit required wall-clock timestamps from the core contract to preserve deterministic output expectations.

## Risks

- Replacing the placeholder `GraphNode` requires coordinated updates in `crates/scryrs-graph`, but the current graph crate is scaffold-level and has no mature external contract.
- If future adapters need new evidence kinds, `EvidenceSourceKind` will require an additive enum update.
- Route-manifest readiness can drift into ranking semantics if implementation adds scores, weights, or priorities beyond the accepted `EvidenceLink.score` snapshot field.
- Invalid edges can silently corrupt downstream consumers unless `KnowledgeGraph` rejects dangling `sourceNodeId` or `targetNodeId` references.

## Traceability

- Task source: `task:8f74e9a5-6308-4945-b2d2-f75a6525c567`
- Exploration dossier: `dossier:2026-06-27T05:08:57.479Z`
- Accepted decisions: `decision:1-swarm-architect-recommendation`, `decision:1-swarm-lead-dev-recommendation`, `decision:1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
