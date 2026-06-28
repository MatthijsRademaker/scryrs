# Route Manifests

Route manifests are scryrs' machine-readable loading map. They flatten graph nodes into deterministic route entries that future runtimes can consume directly, without reinterpreting hotspot and docs evidence every session.

## What a Route Manifest Represents

A **route manifest** is not ranking logic and not retrieval output. It is stable artifact derived from `.scryrs/graph.json` that preserves node identity, normalized load target, optional doc grouping, and evidence backlinks. Later runtime code can use that artifact to decide what context to load and explain why.

## Core Concepts

### Top-level document

`RouteManifestDocument` is versioned envelope with three fields:

| Field | Purpose |
| --- | --- |
| `schemaVersion` | Route-manifest contract version. Current value: `1.0.0` (`ROUTE_SCHEMA_VERSION`). |
| `metadata` | Repository-level context copied from graph artifact. |
| `routes` | Deterministically ordered `RouteEntry` array. |

### Route entries

Each **route entry** is one graph node rendered as one route target.

| Field | Purpose |
| --- | --- |
| `id` | Stable graph node ID. |
| `subjectKind` | Source node kind such as `file`, `search`, or `doc_page`. |
| `subject` | Raw subject value from source node. |
| `label` | Human-readable display label. |
| `target` | Normalized load target. In v1 this equals source graph node `id`. |
| `kind` | Repeated node kind for additive downstream evolution. |
| `evidenceLinks` | Provenance backlinks copied from source graph node. |
| `grouping` | Optional parent grouping, only from explicit `contains` edge. |
| `metadata` | Optional extension map. |

### Grouping

Grouping is intentionally narrow in v1. `grouping` appears only when route source node is target of explicit `contains` edge in graph. That includes both docs navigation hierarchy and accepted semantic grouping already materialized during graph build:

- `docs_root -> Technical`
- `Technical -> doc_page:graph`
- `domain_term:auth -> file:auth`

In those cases the child route entry carries the parent identity and label:

- `groupId = "technical"`, `groupLabel = "Technical"`
- `groupId = "domain_term:auth"`, `groupLabel = "Auth"`

Hotspot-backed nodes such as `file:src/main.rs` remain ungrouped unless graph already contains an explicit parent edge.

## Deterministic Identity Boundary

Route generation preserves exact source identity. One graph node becomes one route entry.

That means these remain distinct in manifest:

- `file:auth`
- `search:auth`
- `symbol:auth`

Shared text is not enough to merge entries. Higher-level grouping needs explicit graph evidence first.

## Current Command and Artifact

`scryrs route <PATH>` resolves `<PATH>` to repository root, loads `.scryrs/graph.json`, validates `GRAPH_SCHEMA_VERSION`, then emits single-line `RouteManifestDocument` JSON to stdout and writes same bytes to `.scryrs/routes.json`. It does not inspect `.scryrs/accepted/`, `.scryrs/rejected/`, or `.scryrs/proposals/` directly.

Output rules:

- `routes` sort by `id` ascending.
- `evidenceLinks` within each route sort by `(sourceKind, subject, docRef, description, rowIds, score)` ascending.
- Output contains no wall-clock timestamps, random IDs, or hidden ranking fields.
- Missing, malformed, or schema-mismatched `.scryrs/graph.json` fails fast with exit code `2`.

## Current Implementation Boundary

| Shipped | Deferred |
| --- | --- |
| `RouteManifestDocument`, `RouteEntry`, `RouteGrouping` contracts in `crates/scryrs-types/src/lib.rs` | `scryrs route explain ...` runtime explanation command |
| `scryrs route <PATH>` CLI command in `crates/scryrs-cli/src/route.rs` | Runtime ranking and retrieval decisions |
| One route entry per graph node | Inferred semantic grouping from shared labels alone |
| Grouping only from explicit `contains` edges, including accepted semantic grouping already materialized in `.scryrs/graph.json` | Any graph mutation during route generation |
| Deterministic artifact output at `.scryrs/routes.json` | |

## Why Route Manifests Exist

Graph artifacts are rich but general-purpose. Route manifests are narrower: they keep only what future retrieval needs to load context predictably and explainably. That split matters:

- **graph** = reusable knowledge structure
- **route manifest** = retrieval-ready projection of that structure
- **runtime retrieval** = future decision layer that consumes route manifest

Keeping those layers separate prevents hidden ranking logic from leaking into graph build.

## Related Pages

- [Graph](./graph.md) — graph nodes, edges, and evidence that feed route manifests
- [Proposals](./proposals.md) — review-first proposal flow, including semantic grouping candidates that may later become explicit graph evidence
- [Architecture](./architecture.mdx) — crate boundaries for `scryrs-types`, `scryrs-cli`, `scryrs-graph`, and `scryrs-runtime`
- [Product Roadmap](./roadmap.mdx) — Phase 5 shipped scope vs. deferred runtime retrieval
