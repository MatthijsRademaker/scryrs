# Graph

The knowledge graph provides explainable, reusable routing context that agents and docs systems can consume directly — so future sessions load the right context without rediscovering relationships that previous sessions already mapped.

## What a Scryrs Graph Represents

A **scryrs graph** is route-ready knowledge connecting docs, concepts, and evidence-backed relationships. It captures what the codebase knows — files, symbols, doc pages, domain terms — as stable nodes, and records how those nodes relate to each other through directed edges that carry evidence provenance.

The core insight: **agents repeatedly navigate the same relationships across sessions** (opening the same files in the same order, searching for the same connections, failing to find the same linked concepts). A knowledge graph makes those relationships explicit and machine-readable so routing infrastructure can tell future agents what context to load before they rediscover it.

Every relationship in the graph is backed by evidence. A node isn't just a label — it's a concept with evidence links tracing back to hotspot subjects, trace row IDs, doc references, or recorded evidence descriptors. An edge isn't just "A relates to B" — it's a directed assertion with a relationship kind and the same evidence provenance chain. When a route manifest tells an agent to load a particular document, the agent can follow the evidence chain back to the specific sessions and trace events that proved the relationship exists.

## Core Concepts

### Nodes

A **node** is a stable, identifiable concept. Nodes represent the entities the codebase knows about: files, symbols (functions, types, traits), doc pages, domain terms, and other named concepts.

Each node carries:

| Field | Purpose |
|-------|---------|
| `id` | Stable unique identifier — the lookup key other nodes and edges reference |
| `label` | Human-readable display name for filtering and UI presentation |
| `description` | Optional longer explanation of what this node represents |
| `kind` | String-backed node category for downstream grouping and filtering (e.g. `"file"`, `"symbol"`, `"doc_page"`, `"domain_term"`) |
| `tags` | Searchable, filterable tags, deterministically sorted at materialization time |
| `aliases` | Alternative names for the same concept, deterministically sorted — enables matching when agents refer to the same thing under different names |
| `evidenceLinks` | Provenance chain tracing back to hotspots, trace rows, docs, or recorded evidence |
| `metadata` | Additive namespaced extension map (reverse-domain or snake_case keys) for future adapter-specific data without modifying the core contract |

Nodes are the stable "what" in the graph. Edges provide the "how they relate."

### Directed Edges

An **edge** is a relationship assertion between two nodes. Direction is meaningful: `sourceNodeId → targetNodeId` expresses a directional relationship like "depends on," "documents," "implements," or "references."

Each edge carries:

| Field | Purpose |
|-------|---------|
| `id` | Stable unique edge identifier |
| `sourceNodeId` | The node the relationship originates from |
| `targetNodeId` | The node the relationship points to |
| `relationship` | String-backed relationship kind — e.g. `"depends_on"`, `"documents"`, `"references"`, `"implements"` |
| `label` | Optional human-readable label |
| `tags` | Searchable, filterable tags, deterministically sorted |
| `evidenceLinks` | Same evidence provenance chain as nodes |
| `metadata` | Additive namespaced extension map |

Edges must reference existing nodes: every `sourceNodeId` and `targetNodeId` must match a node `id` present in the graph. The graph container validates this at materialization time and rejects documents with dangling edge references.

### Evidence Links

An **evidence link** attaches provenance to a node or edge. It answers "why does this node/edge exist in the graph?" by pointing back to the evidence that proved it.

Every evidence link carries a `sourceKind` from a closed set of five provenance categories:

| Source Kind | What it captures |
|-------------|-----------------|
| `hotspot_subject` | A hotspot subject identity — evidence that this concept appeared as a hotspot (score, subject name, no row-level granularity) |
| `local_trace_row` | One or more local `trace_events` row IDs — concrete agent actions that demonstrate this node or relationship |
| `server_trace_row` | One or more live `server_trace_events` row IDs — same evidence pattern, but from live server-owned persistence |
| `doc_reference` | A documentation citation — evidence that docs describe or reference this concept, anchored by `docRef` |
| `recorded_evidence` | A recorded evidence descriptor — human-authored evidence anchored by `description` |

Each evidence link also carries:

- `subject` — the subject identity string (e.g., hotspot subject name)
- `rowIds` — ordered row IDs from the source data store (empty for `doc_reference` and `recorded_evidence`)
- `docRef` — documentation reference string (present for `doc_reference`)
- `description` — recorded evidence description (present for `recorded_evidence`)
- `score` — optional hotspot score snapshot (auxiliary data, not part of stable provenance identity)

### Deterministic Output

Graph output is deterministic: the same nodes and edges always produce the same serialized `KnowledgeGraphDocument`. The ordering rules are:

- **Nodes** sort by `id` ascending.
- **Edges** sort by `id` ascending.
- **Tags and aliases** within each node/edge sort lexicographically ascending.
- **Evidence links** sort by a documented tie-break chain: `sourceKind` → `subject` → `docRef` → `description` → `rowIds` → `score`, all ascending (missing string fields treated as empty values, `rowIds` compared lexicographically as ordered lists).

Given the same input, the graph materializes to the same JSON every time — there is no randomness, no model inference, and no time-dependent behavior beyond the `schemaVersion` field carried as `GRAPH_SCHEMA_VERSION`.

## How the Graph Fits the Product Loop

The graph is the foundational structure that makes **Route** possible in scryrs' Observe → Detect → Promote → Route product loop:

```text
Observe              Detect               Promote               Route
                                                                  ↑
Agent hooks          Hotspot analysis     Knowledge proposals    │
capture traces  →    scores repeated  →   turn evidence      ───┘
                     agent attention       into durable docs    (future)
                         │
                         ↓
                    Graph organizes
                    evidence into structured
                    relationships
```

- **Observe → Detect:** Traces and hotspots provide the raw evidence. Every time an agent opens a file, searches a term, or fails to find a concept, scryrs records a trace event. Hotspot analysis identifies the subjects that consume the most repeated agent effort.
- **Detect → Graph:** The graph organizes that evidence into structured, queryable relationships. Hotspot evidence (score, row IDs, subject identity) becomes evidence links on graph nodes. Doc references and recorded evidence complement trace-based evidence for concepts not yet appearing in agent sessions.
- **Graph → Route (Future Phase 5+):** Route manifests will consume graph relationships to tell agents what context to load for a given task. When an agent asks about authentication, the routing layer can trace the graph from the `auth` node, follow `depends_on` and `documents` edges, and load the right set of files and docs — with evidence chains explaining why each recommendation exists.

**Important:** Graph build, route manifests, and runtime retrieval are **Future Phase 5+** work. Today, the graph foundation provides the contract and container so evidence can be organized — the pipeline that populates the graph from hotspots, docs, and traces, and the routing infrastructure that consumes it, are explicitly deferred.

## Illustrated JSON Example

Below is a small annotated `KnowledgeGraphDocument` showing one node, one edge, and one evidence link. This is a domain sketch — for the complete wire contract with all field constraints, see the canonical [graph-contract spec](https://github.com/scryrs-project/scryrs/blob/main/openspec/specs/graph-contract/spec.md) and the Rust types in `crates/scryrs-types/src/lib.rs`.

```json
{
  "schemaVersion": "1.0.0",
  "metadata": {
    "repositoryId": "github.com/scryrs-project/scryrs"
  },
  "nodes": [
    {
      "id": "node_auth_module",
      "label": "Authentication Module",
      "description": "Core authentication handlers and middleware for the scryrs server",
      "kind": "file",
      "tags": ["auth", "security", "server"],
      "aliases": ["auth", "auth-handlers"],
      "evidenceLinks": [
        {
          "sourceKind": "hotspot_subject",
          "subject": "src/auth/handlers.rs",
          "score": 42
        }
      ]
    }
  ],
  "edges": [
    {
      "id": "edge_auth_depends_middleware",
      "sourceNodeId": "node_auth_module",
      "targetNodeId": "node_middleware_layer",
      "relationship": "depends_on",
      "label": "auth handlers depend on middleware",
      "evidenceLinks": [
        {
          "sourceKind": "local_trace_row",
          "subject": "src/auth/handlers.rs",
          "rowIds": [142, 289, 356]
        }
      ]
    }
  ]
}
```

Key points illustrated:

- `schemaVersion` is `"1.0.0"` — the independent `GRAPH_SCHEMA_VERSION`.
- `metadata.repositoryId` is present for a server-owned graph; it may be absent for local-only graphs.
- The node carries `id`, `label`, `description`, `kind`, `tags`, `aliases`, and `evidenceLinks`.
- The edge carries `id`, `sourceNodeId`, `targetNodeId`, `relationship`, `label`, and `evidenceLinks`, with direction `node_auth_module → node_middleware_layer`.
- Evidence links demonstrate two different `sourceKind` values: `hotspot_subject` (with a score snapshot) and `local_trace_row` (with ordered row IDs).

> **Canonical source:** The [graph-contract spec](https://github.com/scryrs-project/scryrs/blob/main/openspec/specs/graph-contract/spec.md) and the Rust types in `crates/scryrs-types/src/lib.rs` are the authoritative references for all field constraints, validation rules, and serialization behavior. This example is illustrative only.

## Current Implementation Boundary

The graph foundation today provides the contract and the container. The build pipeline, manifests, and runtime retrieval are explicitly deferred.

| Shipped | Deferred (Future Phase 5+) |
|---------|---------------------------|
| `KnowledgeGraphDocument` wire contract (`GRAPH_SCHEMA_VERSION = "1.0.0"`) | Graph build pipeline (populating nodes and edges from hotspots, docs, traces) |
| `GraphNode`, `GraphEdge`, `EvidenceLink`, `EvidenceSourceKind` types | `scryrs graph` CLI command |
| `KnowledgeGraph` container: add nodes/edges, validate structural references, materialize deterministic document | Route-manifest generator |
| Deterministic serialization (node/edge/evidence-link ordering) | Docs crawler for automatic doc→node extraction |
| Structural validation rejecting dangling edge references | Adapter integration for publishing graph data into docs systems |
| Independent schema versioning from trace and hotspot contracts | Server endpoints for graph query or retrieval |
| | Runtime retrieval (agents consuming graph relationships to load context) |

No `scryrs graph build` command exists. No route-manifest generator exists. No docs crawler or runtime retrieval behavior exists. The graph foundation provides the contract so evidence can be organized; the pipeline that fills the graph and the routing infrastructure that reads it are Phase 5+ work.

## Future Scope

Graph build, route manifests, and runtime retrieval are Phase 5+ roadmap items. In brief:

- **Graph build** — populate graph nodes and edges from hotspot evidence, documentation structure, and recorded evidence descriptors. This is the pipeline that turns raw evidence into organized graph relationships.
- **Route manifests** — export graph relationships as route-ready manifests that routing infrastructure can consume. This is the bridge from graph structure to runtime retrieval.
- **Runtime retrieval** — agents consume route manifests to load the right context for a given task without rediscovering relationships that the graph already maps.

These features depend on stable hotspot outputs (Phase 2), live hotspot server contracts (Phase 4), and the graph foundation described on this page. See the [Product Roadmap](./roadmap.mdx) for the full delivery sequence.

## Related Pages

- [Vision & Goals](./vision.md) — product positioning, suite narrative, and the Observe → Detect → Promote → Route product loop
- [Architecture](./architecture.mdx) — crate topology including `scryrs-graph` and `scryrs-types`
- [Product Roadmap](./roadmap.mdx) — delivery sequence with Phase 5 graph and route-manifest milestones
- [Hotspots](./hotspots.md) — how hotspot evidence feeds graph nodes and edges through evidence links
- [Trace Hook Contract](./trace-hook-contract.md) — how harness hooks capture the trace events that become graph evidence
- [Graph Contract Spec](https://github.com/scryrs-project/scryrs/blob/main/openspec/specs/graph-contract/spec.md) — canonical wire-contract requirements including deterministic ordering and validation rules
