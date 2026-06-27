## Context

scryrs emits deterministic graph artifacts at `.scryrs/graph.json` via the `scryrs graph <PATH>` command. This artifact contains hotspot-backed subject nodes (with node IDs like `file:src/main.rs`, `search:routing`, `symbol:MyStruct`) and doc-page nodes (with IDs like `doc_page:graph`) connected by `contains` edges from navigation hierarchy. The current graph build v1 explicitly avoids cross-domain edges — no edges connect hotspot nodes to doc-page nodes.

The task backlog requires a route manifest artifact derived from explicit graph evidence. The manifest must be machine-readable, stable, and evidence-backed. It must not silently collapse distinct subject identities (e.g., `file:auth`, `search:auth`, `symbol:auth` must remain separate routes unless explicit graph edges justify grouping).

Three refinement round contributors (architect, lead-dev, reviewer) produced convergent recommendations with only minor surface-level differences resolved by the lead-dev's binding decision to use singular `route` to match the existing dispatch_tests stub.

## Goals

- **G1**: Emit a stable, versioned route manifest artifact at `.scryrs/routes.json` (and stdout) from `.scryrs/graph.json` input.
- **G2**: Every route entry carries explicit evidence backlinks to graph/hotspot/doc provenance.
- **G3**: Preserve `(subjectKind, subject)` identity boundaries by default — one route entry per graph node.
- **G4**: Derive grouping only from explicit `contains` edges in the graph; no LLM inference, no semantic collapsing.
- **G5**: Produce byte-identical output for identical input with deterministic ordering and no wall-clock timestamps.
- **G6**: Fail fast on missing or malformed `.scryrs/graph.json` with exit code 2 and contract three-line error format.
- **G7**: Keep `crates/scryrs-graph` unchanged (confirmed by graph-contract spec).

## Non-Goals

- No LLM semantic clustering, synonym merging, or inferred domain grouping.
- No runtime retrieval UX, `scryrs route explain`, dashboard route visualization, or human-facing prose.
- No expansion of graph build scope to invent cross-domain edges.
- No route-manifest generation in `crates/scryrs-graph`.
- No server endpoint or live route query API.

## Decisions

### D1: CLI command surface — `scryrs route <PATH>`

**Choice**: Singular `route` with required PATH argument, mirroring `scryrs graph <PATH>`.

**Rationale**: The dispatch_tests.rs already stubs `"route"` as a future command at line 317. All existing top-level commands use singular form (`graph`, `hotspots`, `record`, `hook`, `init`, `dashboard`, `server`). The artifact file is `routes.json` (plural, descriptive), consistent with `graph.json` and `hotspots.json`.

**Conflict resolution**: The architect recommended `scryrs routes` (plural, two contributors). The lead-dev and reviewer both recommended `scryrs route` (singular) to match the existing dispatch stub and command naming convention. The lead-dev's binding decision is adopted per the retry context tier resolution. Artifact naming remains `routes.json` (plural, descriptive).

**Accepted by**: 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation, 1-swarm-reviewer-recommendation (two of three explicitly state `route` singular).

### D2: Input source — `.scryrs/graph.json`

**Choice**: Read the canonical graph artifact rather than rebuilding from hotspots/docs.

**Rationale**: This keeps the artifact pipeline explicit (hotspots → graph → routes), avoids duplicating graph-build logic, and decouples route generation from graph build changes. The graph crate's `to_document()` already produces deterministically ordered JSON that the route generator can consume directly.

**Accepted by**: All three contributors.

### D3: Route manifest schema in `scryrs-types`

**Choice**: Define `RouteManifestDocument` envelope, `RouteEntry` items, and `ROUTE_SCHEMA_VERSION = "1.0.0"`. Reuse `EvidenceLink`, `EvidenceSourceKind`, and `GraphMetadata` from the existing graph contract.

**Schema summary**:
- `RouteManifestDocument { schemaVersion, metadata: GraphMetadata, routes: [RouteEntry] }`
- `RouteEntry { id, subjectKind, subject, label, target, kind, evidenceLinks: [EvidenceLink], grouping?: { groupId, groupLabel }, metadata? }`

**Rationale**: The existing `EvidenceLink` type and `EvidenceSourceKind` enum are purpose-built for attaching provenance to graph entities. Reusing them avoids defining parallel evidence types. The `RouteHint` placeholder struct is insufficient (no versioning, no evidence, no grouping) and is replaced entirely.

**Accepted by**: All three contributors.

### D4: Route generation algorithm

**Choice**: One route entry per graph node, derived from graph node properties. Doc-page routes carry an optional `grouping` field from explicit `contains` edges when a parent group node exists. Hotspot nodes remain ungrouped unless explicit graph edges connect them to a parent (not possible in v1 since no cross-domain edges exist).

**Algorithm**:
1. Load and deserialize `.scryrs/graph.json` into `KnowledgeGraphDocument`.
2. Validate `schemaVersion` matches `GRAPH_SCHEMA_VERSION`.
3. Build a parent lookup map from `contains` edges: for each edge with `relationship == "contains"`, map `target_node_id → (source_node_id, source_node_label)`.
4. For each graph node, emit a `RouteEntry` with fields derived from node properties and parent lookup.
5. Sort routes by `id` ascending.
6. Sort evidence links within each route by the documented tie-break chain.
7. Serialize as camelCase JSON to stdout and `.scryrs/routes.json`.

**Accepted by**: All three contributors.

### D5: Feature gating

**Choice**: Gate route manifest generation behind the existing `graph` feature flag.

**Rationale**: Route manifests depend on graph artifacts. Gating under `graph` avoids Cargo.toml churn and is semantically correct. Conditionally compile `route.rs` behind `#[cfg(feature = "graph")]`.

**Accepted by**: 1-swarm-reviewer-recommendation.

### D6: Error handling contract

**Choice**: Mirror the graph command's error behavior. Exit code 2 for missing/malformed graph.json with three-line contract error on stderr. Exit code 1 for artifact write failure.

**Accepted by**: All three contributors (implied by "matching graph patterns").

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Schema overfitting to v1's sparse edges | Medium | Low | Keep `kind` as string-backed, use optional `grouping` field; schema version bump when richer edges arrive |
| dispatch_tests stub change breaks CI | High | Low | Remove `"route"` from `previously_stubbed_commands_exit_2` array; update help snapshots in lockstep |
| Incompatible graph schema version silently produces wrong routes | Low | High | Validate loaded graph's `schemaVersion` matches `GRAPH_SCHEMA_VERSION`; exit 2 on mismatch |
| Route entry `target` field ambiguity | Medium | Low | Use graph node ID as target; runtime consumers resolve target shape from `subjectKind` and `target` fields |
| Deterministic sort order masks identity-boundary tests | Low | Medium | Sort by node `id` which encodes `(subjectKind, subject)`; test explicitly for distinct IDs before sort comparison |

## Traceability

- **Task**: a038601c-9149-4d60-80e7-595388140dea (backlog story, scenarios, acceptance criteria)
- **Dossier**: 2026-06-27T16:14:09.884Z (problem framing, goals, non-goals, assumptions)
- **Round 1 Architect**: decision 1-swarm-architect-recommendation (schema design, pipeline separation, CLI surface)
- **Round 1 Lead Dev**: decision 1-swarm-lead-dev-recommendation (binding CLI choice, grouping rules, command naming)
- **Round 1 Reviewer**: decision 1-swarm-reviewer-recommendation (feature gating, schema validation, dispatch updates)
- **Existing specs**: graph-contract (node/edge identity, evidence model, graph crate boundary), graph-build (v1 no cross-domain edges, CLI artifact conventions)
- **Code evidence**: graph.rs (artifact pipeline pattern), dispatch.rs (command registration), dispatch_tests.rs (route stub, determinism tests), scryrs-types (existing EvidenceLink, GraphNode, GraphEdge), scryrs-graph (explicit route exclusion)