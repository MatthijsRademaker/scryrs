## 1. Create domain-first graph documentation page

- [ ] 1.1 Create `.devagent/docs/docs/graph.md` following the `hotspots.md` structural pattern
- [ ] 1.2 Write domain problem statement: agents and docs systems need explainable, reusable routing context instead of rediscovering relationships every session
- [ ] 1.3 Define what a scryrs graph represents: route-ready knowledge connecting docs, concepts, and evidence-backed relationships
- [ ] 1.4 Explain core concepts (nodes, directed edges, evidence links, evidence source kinds) in plain language
- [ ] 1.5 Describe how graph fits the Observe → Detect → Promote → Route product loop as the foundational structure that makes Route possible
- [ ] 1.6 Include a small annotated JSON example of `KnowledgeGraphDocument` (one node, one edge, one evidence link) for reader comprehension
- [ ] 1.7 Add Current Implementation Boundary section with explicit table distinguishing shipped from deferred
- [ ] 1.8 Add Future Scope section briefly noting graph build, route manifests, and runtime retrieval as Phase 5+ work
- [ ] 1.9 Add Related Pages section cross-linking to vision.md, architecture.mdx, roadmap.mdx, hotspots.md, trace-hook-contract.md, and the graph-contract spec

## 2. Update docs navigation

- [ ] 2.1 Add `{"text": "Graph", "link": "/graph"}` entry to `.devagent/docs/docs/_nav.json` under the `"Technical"` section after Hotspots

## 3. Add cross-links from existing product docs

- [ ] 3.1 Add `[Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context` to `vision.md` Related Pages
- [ ] 3.2 Add `[Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context` to `architecture.mdx` Related Pages
- [ ] 3.3 Add `[Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context` to `roadmap.mdx` Related Pages
- [ ] 3.4 Add `[Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context` to `hotspots.md` Related Pages
- [ ] 3.5 Add `[Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context` to `trace-hook-contract.md` Related Pages

## 4. Verify documentation claims against truth sources

- [ ] 4.1 Verify all graph type claims (nodes, edges, evidence links, source kinds, schema version) against `crates/scryrs-types/src/lib.rs`
- [ ] 4.2 Verify all implementation boundary claims (shipped: contract + container; deferred: build/manifests/runtime) against `crates/scryrs-graph/src/lib.rs`
- [ ] 4.3 Verify all contract claims (deterministic ordering, validation behavior, explicit non-goals) against `openspec/specs/graph-contract/spec.md`
- [ ] 4.4 Verify all product-loop and roadmap claims against `vision.md` and `roadmap.mdx`

## 5. Verify docs site builds cleanly

- [ ] 5.1 Run `bun run build` in `.devagent/docs/` and confirm exit code 0
- [ ] 5.2 Confirm no broken link warnings in build output
- [ ] 5.3 Confirm new `/graph` route is present in generated site

## 6. Verify docs-only scope

- [ ] 6.1 Confirm no Rust source files, Cargo configuration, or OpenSpec specs (outside this change) are modified
- [ ] 6.2 Confirm all changed files are under `.devagent/docs/docs/`