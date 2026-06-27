## Why

Graph documentation exists only as incidental mentions across product pages (vision, architecture, roadmap, hotspots, trace-hook contract) and as the formal graph-contract OpenSpec — but never as a discoverable domain explanation of what the scryrs knowledge graph achieves, what problem it solves, how evidence becomes explainable routing context, or which graph pieces are implemented today versus explicitly deferred.

This is the same gap the `hotspots.md` page closed for hotspot analysis. The graph needs the same treatment: a domain-first page that answers "what problem does this solve and how does it work in product terms?" before diving into crate topology or wire-contract details.

## What Changes

- **New page** `.devagent/docs/docs/graph.md` — a dedicated domain-first documentation page following the proven `hotspots.md` structural pattern (Problem → Concept → How it works → Current implementation boundary → Related pages).
- **Nav entry** in `.devagent/docs/docs/_nav.json` adds `{"text": "Graph", "link": "/graph"}` under the `"Technical"` section, keeping graph adjacent to its closest conceptual neighbor (Hotspots).
- **Cross-links** added to existing pages that already mention graph work:
  - `vision.md` Related Pages — adds `[Graph](./graph.md)`
  - `architecture.mdx` Related Pages — adds `[Graph](./graph.md)`
  - `roadmap.mdx` Related Pages — adds `[Graph](./graph.md)`
  - `hotspots.md` Related Pages — adds `[Graph](./graph.md)`
  - `trace-hook-contract.md` Related Pages — adds `[Graph](./graph.md)`

## Impact

- **Docs-only change.** No Rust code, graph schema behavior, roadmap sequencing, or canonical OpenSpec specs are modified.
- **Discoverability improvement.** Readers encountering graph mentions in any existing product page now have a single domain-first destination that explains the concept.
- **Scope risk managed.** The new page explicitly labels current shipped scope (KnowledgeGraphDocument contract + KnowledgeGraph container/validation/materialization) and deferred scope (graph build, route manifests, docs crawling, runtime retrieval) so readers do not assume features exist that are not yet built.
- **Docs site builds cleanly.** All new and modified markdown files are compatible with the Rspress build, with no broken links.