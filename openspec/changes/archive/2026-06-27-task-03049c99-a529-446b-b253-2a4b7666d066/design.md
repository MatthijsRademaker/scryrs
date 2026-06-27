# Design: graph-build

## Context

The scryrs codebase already provides:
- `scryrs-types`: `KnowledgeGraphDocument`, `GraphNode`, `GraphEdge`, `EvidenceLink`, `EvidenceSourceKind` (five source kinds), and `GRAPH_SCHEMA_VERSION`.
- `scryrs-graph`: `KnowledgeGraph` container with `add_node()`, `add_edge()`, `validate()` (dangling-edge rejection), and `to_document()` (deterministic materialization with id-sorted nodes/edges and tie-break-chained evidence links).
- `scryrs-cli`: Command dispatch with a six-command allowlist, `hotspots.rs` as a proven I/O pattern (resolve path → load inputs → process → serialize → stdout + artifact file → exit code).
- `scryrs-cli/Cargo.toml`: `graph` feature already a default dependency on `scryrs-graph`.

The architecture docs explicitly assign `scryrs-graph` the role of "knowledge graph foundation" that "owns routing graph concepts separate from trace collection" — it is a pure container/contract crate. The CLI crate owns command surfaces and I/O.

The current state of the codebase explicitly prohibits a `graph build` command:
- `crates/scryrs-graph/src/lib.rs:22` documents the container does not implement graph build or CLI commands.
- `crates/scryrs-cli/src/dispatch.rs:65` does not include `"graph"` in the known-command allowlist.
- `crates/scryrs-cli/src/dispatch_tests.rs:313-317` tests that `"graph"` exits 2 as an unknown command.
- `openspec/specs/graph-contract/spec.md:148` has a scenario asserting no `scryrs graph build` command is introduced.
- `.devagent/docs/docs/graph.md:179` states no graph build command exists.

## Goals / Non-Goals

### Goals

1. Add a discoverable `scryrs graph build <PATH>` CLI command accepted by the dispatcher, documented in help/help-json/README, and no longer treated as an unknown command.
2. Build one merged repository knowledge graph from existing hotspot artifacts (`.scryrs/hotspots.json`) plus docs structure (`.devagent/docs/docs/` including `_nav.json`).
3. Preserve hotspot provenance by carrying `evidence.rowIds` into graph `EvidenceLink` entries with `sourceKind: "local_trace_row"`.
4. Guarantee deterministic output for identical inputs using stable ID derivation, sorted iteration, and the existing `KnowledgeGraph::to_document()` materialization rules.
5. Keep the implementation explainable and local-only; no network calls, no LLM inference, no content analysis.

### Non-Goals

- No route-manifest generation, route explanation, or runtime retrieval behavior.
- No proposal drafting, docs mutation, or adapter publishing.
- No changes to hotspot scoring semantics or live-server contracts.
- No LLM inference or non-deterministic enrichment.
- No expansion into a generic graph query/edit surface.
- No cross-domain edge derivation between hotspot nodes and doc nodes (deferred to follow-on work).

## Decisions

### D1: Builder lives in `crates/scryrs-cli/src/graph.rs`

**Decision**: The graph build pipeline is a new `graph` module in the CLI crate — not a feature added to `scryrs-graph`.

**Rationale**: `scryrs-graph` is a pure container/contract crate that should remain free of I/O, file reading, CLI dispatch, and input parsing. The `hotspots.rs` module in the CLI crate provides the proven structural pattern: resolve path, load inputs, process, serialize, write stdout + artifact, return exit code. The architecture docs confirm the CLI crate owns command surfaces.

**Sources**: `crates/scryrs-graph/src/lib.rs:22`, `crates/scryrs-cli/src/hotspots.rs`, `.devagent/docs/docs/architecture.mdx`

### D2: Input contract — hotspots required, docs tolerated-missing

**Decision**: Missing or unparseable `.scryrs/hotspots.json` is a hard error (exit 2). Missing or empty `.devagent/docs/docs/` directory is tolerated — the build produces a graph with hotspot-derived nodes only, prints a warning to stderr, and exits 0.

**Rationale**: Hotspot evidence is the primary graph seed; a graph without evidence is useless for downstream routing. Docs are a structural enhancement layer — a graph without doc nodes is still a valid and useful artifact. This matches the task acceptance criteria "fail with explicit non-zero errors when required inputs are missing or malformed" while keeping docs as a non-blocking input.

**Sources**: Accepted decision 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation; dossier acceptance criteria

### D3: All five hotspot subject kinds become graph nodes

**Decision**: Hotspot entries of all five subject kinds (`file`, `search`, `symbol`, `command`, `document`) are converted to graph nodes in v1. No filtering.

**Rationale**: Filtering would silently drop evidence, contradicting the acceptance criterion "Evidence links are preserved in output." The reviewer flagged this as a risk if not all kinds are included. All five kinds carry row-ID evidence that downstream consumers may need.

**Sources**: 1-swarm-lead-dev-recommendation, 1-swarm-reviewer-blocker-3 (RISK 3)

### D4: Node ID scheme — `{subjectKind}:{subject}` for hotspots, `doc_page:<slug>` for docs

**Decision**: Hotspot node IDs follow the format `{subjectKind}:{subject}` (e.g., `file:src/main.rs`, `symbol:handle_request`). Doc page node IDs follow the format `doc_page:<slug>` where slug is the kebab-case page name derived from the nav link (e.g., `doc_page:graph`, `doc_page:hotspots`). A synthetic `docs_root` node is created as the root of the doc hierarchy.

**Rationale**: The `{subjectKind}:{subject}` pattern is already used internally by the hotspot engine (`format!("{kind}:{subj}")` in `hotspots.rs:88`). The `doc_page:` prefix provides a stable namespace distinct from hotspot subject kinds. Slugs from nav links are deterministic and human-readable; they avoid filesystem-path dependencies that could vary across environments.

**Sources**: `crates/scryrs-cli/src/hotspots.rs:88`, `.devagent/docs/docs/_nav.json`, 1-swarm-architect-recommendation

### D5: V1 edge vocabulary — only `contains` from nav hierarchy

**Decision**: V1 produces only `contains` edges derived from the `_nav.json` navigation hierarchy (nav group → child page). No cross-domain edges between hotspot nodes and doc nodes. No heading-level or page-content analysis.

**Rationale**: `_nav.json` provides an explicit, machine-readable, deterministic hierarchical structure. Cross-domain edge derivation requires heuristics that introduce non-determinism risk (fuzzy matching, substring heuristics) or coupling to content shape. The lead-dev explicitly recommended ONLY structural `contains` edges for v1. The architect included exact-match `documents` edges as well, but the more conservative lead-dev position is chosen for v1 to eliminate edge-derivation risks. Cross-domain edges can be added in a follow-on task with a separately designed deterministic association algorithm.

**Sources**: 1-swarm-lead-dev-recommendation, 1-swarm-architect-recommendation, dossier non-goals

### D6: Output contract — dual stdout + `.scryrs/graph.json`

**Decision**: The graph document is written both to stdout as a single-line JSON and to `.scryrs/graph.json` as an artifact file. Artifact write failure is fatal (exit 1), mirroring the hotspot pattern.

**Rationale**: The hotspots command already writes to both stdout and `.scryrs/hotspots.json`. Consistency across commands reduces consumer surprise. The roadmap explicitly mentions `/.scryrs/graph.json`. stdout output enables piping; artifact output enables file-based consumption.

**Sources**: `crates/scryrs-cli/src/hotspots.rs:133-149`, `.devagent/docs/docs/roadmap.mdx:165`, 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation

### D7: Spec reconciliation — update graph-contract spec scenario

**Decision**: The graph-contract spec scenario "Contract foundation adds no graph build surface" is revised to clarify the scope boundary: `crates/scryrs-graph` the crate remains build-pipeline-free, while the CLI command in `crates/scryrs-cli` is a separate build consumer using the graph crate's public API. The requirement title and body remain unchanged; only the scope-narrowing scenario is updated.

**Rationale**: The current text directly contradicts the task acceptance criterion. The `scryrs-graph` crate should indeed never grow an internal build pipeline, CLI commands, route manifests, or other downstream features. The scenario as originally written conflated "the graph crate" with "the scryrs CLI". The revised text makes this distinction explicit.

**Sources**: 1-swarm-architect-blocker-1, 1-swarm-lead-dev-blocker-1, 1-swarm-reviewer-blocker-1; `openspec/specs/graph-contract/spec.md:148`

### D8: Determinism — sorted iteration guaranteed

**Decision**: All input iteration that could produce non-deterministic ordering is explicitly sorted before processing. Filesystem traversal uses sorted paths. Nav entries from `_nav.json` are processed in array order (JSON preserves order). Hotspot entries are processed in array order (JSON preserves order). Materialization defers to `KnowledgeGraph::to_document()` which sorts nodes/edges by id and evidence links by tie-break chain.

**Rationale**: `std::fs::read_dir` produces unspecified order on some filesystems. `_nav.json` array order is deterministic per the JSON spec. The acceptance criterion requires byte-identical output for identical inputs. The architect flagged non-deterministic filesystem traversal as a risk.

**Sources**: 1-swarm-architect-recommendation (RISK: Non-deterministic filesystem traversal); `crates/scryrs-graph/src/lib.rs:92-120`

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Docs tree path fragility: hardcoded `.devagent/docs/docs/` and `_nav.json` assumptions may break if the Rspress config or docs root changes. | Low | Document the assumption explicitly. Follow-on work can make docs source configurable. |
| Stable ID format may conflict with future route-manifest assumptions. | Medium | The `doc_page:` and `{subjectKind}:{subject}` prefix scheme provides namespaces that route-manifest consumers can filter by. If future consumers need different ID patterns, they can be evolved with a schema version bump. |
| Insta snapshot churn: 7+ existing snapshot tests will need updating for `--help` and `--help-json`. | Low | Standard snapshot update workflow; covered in tasks. |
| Edge vocabulary may be too sparse for downstream consumers. | Medium | Explicitly documented as v1 conservative scope. Extension points are clear: exact-match `documents` edges, heading-level nodes, related-page link edges. Consumers should not depend on rich edges from v1. |

## Conflict Resolution

### Docs-missing failure mode

The swarm-reviewer recommended hard error on missing docs. The swarm-architect and swarm-lead-dev both recommended tolerating missing docs (warning on stderr, produce hotspot-only graph, exit 0). **Resolution**: Follow architect+lead-dev consensus — tolerate missing docs. Rationale: docs are a structural enhancement layer; a graph from hotspot evidence alone is still a valid and useful artifact for downstream routing.

### V1 edge vocabulary scope

The swarm-architect recommended `contains` + exact-match `documents` edges. The swarm-lead-dev recommended ONLY `contains` edges, with no cross-domain derivation. **Resolution**: Follow lead-dev's more conservative position — `contains` only. Rationale: cross-domain edges between hotspots and docs require heuristics that risk non-determinism. This can be added in a follow-on task with a separately designed deterministic association algorithm.

## Traceability

- **Task**: `03049c99-a529-446b-b253-2a4b7666d066`
- **Dossier**: `2026-06-27T11:03:59.169Z` (exploration dossier with goals, non-goals, affected areas, open questions)
- **Decisions**: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation` (round 1)
- **Artifact baseline**: `initial` snapshot, `openspec/changes` backend, ledger version 1
- **Sources**: `dispatch.rs`, `dispatch_tests.rs`, `help_text.rs`, `help_json.rs`, `hotspots.rs`, `scryrs-graph/src/lib.rs`, `scryrs-types/src/lib.rs`, `graph-contract/spec.md`, `_nav.json`, `graph.md`, `roadmap.mdx`, `README.md`, `Cargo.toml`