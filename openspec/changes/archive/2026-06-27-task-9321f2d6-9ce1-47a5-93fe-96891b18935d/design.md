## Context

scryrs includes a `crates/scryrs-graph` crate and graph wire-contract types in `crates/scryrs-types`, but developers and product readers lack a discoverable domain explanation of what the graph achieves. Existing graph coverage is fragmented across six product pages (vision.md, architecture.mdx, roadmap.mdx, hotspots.md, trace-hook-contract.md, cli-v0-contract.md) without a single page that answers the product question: what problem does the knowledge graph solve, and how does it work in product terms?

The `hotspots.md` page provides a proven structural precedent for closing this kind of gap: lead with the domain problem, define the concept in plain language, explain the implemented flow, distinguish current from future scope, and cross-link to related pages.

Executable truth for current graph behavior lives in three sources:
- `crates/scryrs-types/src/lib.rs` — defines `GRAPH_SCHEMA_VERSION`, `GraphNode`, `GraphEdge`, `EvidenceLink`, `EvidenceSourceKind`, `GraphMetadata`, `KnowledgeGraphDocument`
- `crates/scryrs-graph/src/lib.rs` — provides `KnowledgeGraph` container with add/validate/to_document and explicitly documents what it does NOT implement (graph build, CLI commands, route-manifest generation, docs crawling, adapter integration, server endpoints, runtime retrieval)
- `openspec/specs/graph-contract/spec.md` — canonical requirements including deterministic serialization and explicit non-goal of no build pipeline

## Goals / Non-Goals

### Goals

1. Create `.devagent/docs/docs/graph.md` with a domain-first structure following the `hotspots.md` pattern
2. Explain the graph in product terms: what problem it solves, what a scryrs graph represents, how evidence becomes explainable routing context
3. Clearly separate current implementation (contract + container) from deferred future work (graph build, route manifests, runtime retrieval)
4. Add nav entry in `_nav.json` under "Technical" alongside Hotspots
5. Add cross-links from vision.md, architecture.mdx, roadmap.mdx, hotspots.md, and trace-hook-contract.md
6. Verify all claims against the three truth sources above
7. Keep the change docs-only — no Rust code or OpenSpec spec modifications

### Non-Goals

- Do not add or imply a shipped `scryrs graph` CLI command, graph builder, route-manifest generator, docs crawler, or runtime retrieval feature
- Do not change Rust code, graph schema behavior, scoring behavior, or roadmap sequencing
- Do not rewrite architecture pages into deep implementation detail
- Do not describe route manifest schemas, proposal workflows, or adapter integration patterns
- Do not cross-link from CLI v0 Contract (it only lists "graph" as an unimplemented command with zero domain context)

## Decisions

1. **Page name: `graph.md` (/graph)** — matching the crate name `scryrs-graph` and existing page naming conventions. Rejected alternatives: `knowledge-graph.md` (longer, inconsistent with single-word page names like `hotspots.md`, `vision.md`), `routing-graph.md` (implies routing is shipped, which it is not).

2. **Structure: follow `hotspots.md` pattern** — Problem statement → What a graph represents → Core concepts → Current implementation boundary → Related pages. This pattern is proven: it makes the domain explanation the reader's first encounter, with technical depth accessible via cross-references.

3. **Nav placement: "Technical" section alongside Hotspots** — keeps graph adjacent to its closest conceptual neighbor. Both graph and hotspots share `EvidenceSourceKind` types and the Observe→Detect→Promote→Route product loop. Hotspots already lives under "Technical" in `_nav.json`.

4. **Route manifests: brief future-scope note only** — no substantive subsection since no route manifest implementation exists today. Route manifests are Phase 5+ future work, not current or near-term.

5. **Inline JSON example: include a small illustrative sketch** — one node, one edge, one evidence link, annotated for domain understanding. For full wire-contract details, reference `openspec/specs/graph-contract/spec.md` prominently.

6. **Cross-link scope: vision.md, architecture.mdx, roadmap.mdx, hotspots.md, trace-hook-contract.md** — all five pages mention graph work. `trace-hook-contract.md` explicitly says traces are persisted for "graph building" and already has a Related Pages section. CLI v0 Contract excluded because it only lists `graph` as an unimplemented command name with zero domain signal.

7. **Implementation boundary section: modeled on roadmap.md hard non-goals pattern** — an explicit table listing shipped vs deferred so readers cannot misread the page as describing existing end-user graph features.

## Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Scope creep into route-manifest or proposal-engine descriptions | Medium | Every conceptual flow section explicitly labels build/manifests/runtime as "Future Phase 5+" |
| Contract-detail duplication causing drift from graph-contract spec | Medium | Include only illustrative JSON sketch; use prominent callout to graph-contract spec for full wire contract |
| Stale cross-links if page location changes | Low | Use stable path `/graph`; all cross-links use relative `./graph.md` |
| Readers assume shipped features from contract/container existence | Medium | Lead with implementation-status table distinguishing shipped foundation from deferred pipeline |
| Navigation ambiguity (graph as product concept vs crate reference) | Low | Place under "Technical" alongside Hotspots; page opens with domain problem, not crate docs |

## Traceability

All decisions trace to round-1 validated outputs and the exploration dossier:
- Page structure: swarm-architect recommendation (round 1) following hotspots.md precedent mapped in dossier
- Page name /graph: swarm-architect and swarm-lead-dev recommendations (round 1)
- Nav placement: swarm-lead-dev recommendation (round 1), reviewers agree
- JSON example scope: swarm-architect accepted decision #1
- Cross-link targets: swarm-lead-dev accepted decision #2 (minimum set) + trace-hook-contract conditional
- Implementation boundary: swarm-architect non-blocking blocker #2, swarm-lead-dev risk #4
- Truth verification: dossier acceptance criterion 6, all three reviewers corroborate