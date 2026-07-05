## 1. Specs and contracts

- [ ] 1.1 Update the graph-build change spec to replace the v1 cross-domain prohibition with the shipped deterministic rules, evidence requirements, skip behavior, and identity-preservation requirements.
- [ ] 1.2 Update the route-manifest change spec to add additive `relatedEdges` exposure and deterministic ordering rules.

## 2. Graph derivation

- [ ] 2.1 Add a cross-domain derivation step in `crates/scryrs-cli/src/graph.rs` after accepted semantic grouping is loaded and before `kg.to_document()`.
- [ ] 2.2 Implement `symbol_inspected_during_file_context` from `file:<path>` to `symbol:<name>` using same-session `FileOpened` and `SymbolInspected` evidence.
- [ ] 2.3 Implement `search_result` from `search:<query>` to each matching `document:<doc_ref>` and `doc_page:<slug>` target using same-session `SearchRun` and `DocRetrieved` evidence plus exact normalization.
- [ ] 2.4 Deduplicate derived edges by `(relationship, source_node_id, target_node_id)`, use stable edge IDs, aggregate evidence links from both sides, and leave accepted `contains` edges untouched.
- [ ] 2.5 Silently skip cross-domain derivation when `.scryrs/scryrs.db` is absent or when a rule’s required nodes/evidence are missing.

## 3. Route manifest exposure

- [ ] 3.1 Extend `RouteEntry` with optional `relatedEdges` wire-contract support in `crates/scryrs-types/src/lib.rs`.
- [ ] 3.2 Update `crates/scryrs-cli/src/route.rs` to project outgoing non-`contains` graph edges into `relatedEdges` without changing one-route-per-node identity or contains-based grouping.

## 4. Verification and documentation

- [ ] 4.1 Replace `no_cross_domain_edges_in_v1` coverage with positive graph tests for both rules, duplicate aggregation, missing-DB skip, no-match/no-target cases, and repeated-output determinism.
- [ ] 4.2 Add route tests for `relatedEdges` projection and unchanged contains-grouping behavior.
- [ ] 4.3 Update graph and route documentation with the shipped rule examples, relationship names, normalization rule, and local-trace limitation.