## Why

Graph build currently emits exact hotspot nodes, doc-page nodes, structural `contains` edges, and accepted semantic grouping, but it does not derive deterministic context edges between related subjects. Route consumers therefore see useful grouping only where explicit `contains` edges exist, while otherwise-related file, symbol, search, and documentation nodes remain isolated.

This change delivers the next conservative graph step required by Graph Product 05: derive explainable cross-domain edges from explicit trace and documentation evidence only, without LLM inference and without merging node identities.

## What Changes

- Update graph build to derive cross-domain edges when local trace evidence is available from `.scryrs/scryrs.db`.
- Ship exactly two v1 rules:
  - `file -> symbol` with relationship `symbol_inspected_during_file_context` when `FileOpened` and `SymbolInspected` evidence for those exact subjects appears in the same session.
  - `search -> document` or `search -> doc_page` with relationship `search_result` when `SearchRun` and `DocRetrieved` evidence appears in the same session and the normalized search query exactly matches the normalized `doc_ref` or docs-page slug.
- Preserve existing graph node identities and accepted semantic grouping. Derived edges add context only, deduplicate by `(relationship, source, target)`, and aggregate evidence links from both sides.
- Extend route manifests with an additive `relatedEdges` summary on `RouteEntry` so route consumers can see cross-domain adjacency without changing one-route-per-node identity or contains-based grouping.
- Replace the current "No cross-domain edges in v1" graph expectations with positive deterministic rule coverage, and document the shipped rules, normalization, and local-trace limitation.

## Impact

- Affects `crates/scryrs-cli/src/graph.rs`, `crates/scryrs-cli/src/route.rs`, `crates/scryrs-types/src/lib.rs`, OpenSpec graph/route specs, and graph/route documentation.
- Leaves `crates/scryrs-graph` unchanged as a pure container/validation crate.
- Local graph builds with `.scryrs/scryrs.db` gain cross-domain context. Graph builds without the local trace store still succeed, but they emit no derived cross-domain edges.