## Context

Graph build currently materializes hotspot-backed nodes, docs nodes plus `contains` edges from `_nav.json`, and accepted `semantic_graph_grouping` decisions. The canonical graph-build spec still carries a `No cross-domain edges in v1` scenario, and route generation still ignores every non-`contains` edge. This task introduces the first deterministic cross-domain relationships while preserving the existing model boundaries: no LLM inference, no node merging, and no graph-build logic inside `crates/scryrs-graph`.

## Goals / Non-Goals

### Goals

- Derive at least two deterministic cross-domain edge rules from explicit evidence.
- Preserve exact node identity for hotspot, docs, and accepted-group nodes.
- Attach evidence links to every derived edge and keep graph output deterministic.
- Keep accepted semantic grouping and contains-based route grouping working unchanged.
- Expose cross-domain edge context to route consumers through an additive route-manifest field.
- Document the shipped rules, examples, and the local-trace limitation.

### Non-Goals

- No LLM semantic inference, fuzzy matching, embeddings, or heuristic similarity scoring.
- No node merging or collapsing of `file:*`, `search:*`, `symbol:*`, `document:*`, or `doc_page:*` identities.
- No generic all-pairs session co-occurrence graph.
- No changes to `crates/scryrs-graph` beyond continued use of its existing validation/materialization API.
- No dashboard visualization, retrieval-policy, or proposal/adapter work.

## Decisions

### Decision 1: Same-session co-occurrence is the v1 evidence boundary

Cross-domain derivation uses `session_id` equality from local trace events as the only session-chain rule in v1. It does not add adjacency windows, bounded row-distance rules, or inferred chains.

### Decision 2: Ship exactly two named relationship rules with fixed direction

The graph adds only these v1 relationships:

- `symbol_inspected_during_file_context` from `file:<path>` to `symbol:<name>`
- `search_result` from `search:<query>` to a matching `document:<doc_ref>` and/or `doc_page:<slug>` target

The direction is rule-defined, not inferred from labels or alphabetical ordering.

### Decision 3: Search/document matching uses exact normalized equality

Normalize the search query, `DocRetrieved.doc_ref`, and docs-page slug by lowercasing, stripping one leading `/`, and stripping a trailing `.md` or `.mdx` extension before comparison. A `search_result` edge is allowed only when the normalized values are exactly equal.

### Decision 4: Cross-domain derivation is optional and local-trace-backed

Graph build attempts derivation only when the local `.scryrs/scryrs.db` trace store is available through `TraceQuery::iter_events_with_ids_ordered()`. When the DB is absent, or when the available evidence does not satisfy a rule, graph build succeeds and emits no derived cross-domain edges.

### Decision 5: Derived edges aggregate deterministically

A derived edge is keyed by `(relationship, source_node_id, target_node_id)`. Repeated observations merge into one edge with stable ID `{relationship}_{sourceNodeId}_{targetNodeId}` and aggregated evidence links from both sides. Existing graph materialization continues to sort evidence links deterministically.

### Decision 6: Search targets remain distinct when both node kinds exist

If the same normalized query matches both an existing `document:*` hotspot node and an existing `doc_page:*` docs node, graph build emits separate `search_result` edges to each target. This preserves subject identity instead of choosing one representation or collapsing them.

### Decision 7: Route exposure is additive and does not change grouping semantics

`RouteEntry` gains an optional `relatedEdges` field in the wire contract. Each item contains `relationship`, `targetRouteId`, and `evidenceLinks`. Route generation projects outgoing non-`contains` graph edges into `relatedEdges` on the source route entry only. `grouping` remains reserved for explicit `contains` parents.

## Conflict Resolution

- **Route exposure shape**: Adopt the architect/lead-dev additive route-entry field rather than deferring route exposure, because the task acceptance criteria require route manifests to expose edge effects.
- **Normalization rule**: Adopt the lead-dev normalization rule (`lowercase + strip leading slash + strip .md/.mdx`) to eliminate ambiguity while staying inside the no-fuzzy-inference boundary.
- **Relationship direction**: Use rule-specific directed relationships instead of a symmetric `co_observed_with` label, following the accepted architect decision and the reviewer’s requirement to pin direction.
- **Missing DB behavior**: Use silent deterministic skip when `.scryrs/scryrs.db` is absent, matching the accepted architect decision and avoiding hard failure for hotspot/docs-only builds.
- **Document vs. doc_page target ambiguity**: Emit separate edges when both independently satisfy the rule, because the task requires preserved subject identity and forbids node merging.

## Risks

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| Same-session false positives in long sessions | Unrelated events in one session can still produce a deterministic but weak edge | Limit v1 to the two named event-type pairs and document the boundary clearly |
| Route-manifest wire-contract expansion | Downstream consumers must tolerate a new additive field | Make `relatedEdges` optional/omittable and keep existing route identity/grouping fields unchanged |
| No cross-domain edges without local trace DB | Live/exported hotspot artifacts alone lack session context | Document that local trace evidence is required for v1 derivation and that missing DB means zero derived edges |
| Duplicate observations | Repeated qualifying evidence could create unstable duplicate edges | Key edges by `(relationship, source, target)` and merge evidence before materialization |

## Traceability

- Task: `158a5326-006e-4baf-b0b7-7a7399d2c189`
- Dossier: `2026-07-05T05:06:31.536Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `snapshotId=initial`