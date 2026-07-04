---
name: graph-core-changes
description: "Recipe for changing shared contract types and graph/trace domain logic in crates/scryrs-types, crates/scryrs-graph, and crates/scryrs-core. Use when: the task mentions TraceEvent, GraphNode, GraphEdge, KnowledgeGraphDocument, HotspotEntry, EvidenceLink, a schema version bump, hotspot scoring, or any struct in scryrs-types. Do not use for: CLI wiring (add-cli-command) or dashboard UI consuming the API (dashboard-ui)."
metadata:
  curated: true
  sources:
    - crates/scryrs-types/**
    - crates/scryrs-graph/**
    - crates/scryrs-core/**
    - crates/scryrs-cli/src/graph.rs
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Graph / Core / Types Changes

Dependency direction: `scryrs-types` is the leaf (contracts + schema-version constants, no internal deps); `scryrs-core` (trace/hotspot/SQLite) and `scryrs-graph` (graph container + deterministic materialization) each depend only on types and NOT on each other. The graph **build pipeline** is not in scryrs-graph — it's `crates/scryrs-cli/src/graph.rs` (`write_graph_json`). Rust types + `openspec/specs/*` contract specs are authoritative; docs pages are derived narrative.

## Read first

1. `crates/scryrs-types/src/lib.rs` — the section holding your type. Note the schema-version constants at the top and the serde attribute patterns (`rename_all = "camelCase"` on graph/route/proposal types; `skip_serializing_if` + `default` on optional/collection fields).
2. `.devagent/docs/docs/graph.md` — field tables and determinism rules (for graph changes); tells you which claims your change invalidates.

## Steps (worked for "add/change a field on a shared type" — the most common shape)

1. Change the struct in `crates/scryrs-types/src/lib.rs`. New optional fields take `#[serde(skip_serializing_if = "Option::is_none")]`; new collections take `#[serde(skip_serializing_if = "Vec::is_empty", default)]`. A new **required** field is a breaking wire change that fails deserialization of existing `.scryrs/*.json` artifacts at runtime — do it only if the task explicitly says so, and bump the type's schema-version constant.
   Verify: `grep -n "skip_serializing_if\|rename_all" crates/scryrs-types/src/lib.rs` around your type — attrs match the neighboring pattern.
2. Find and update every constructor — there is no `..Default::default()` escape hatch, so these are compile errors, but find them up front:
   Verify: `grep -rn "GraphNode {\|GraphEdge {\|<YourType> {" crates/ --include="*.rs"` — you've touched every hit (graph.rs, route.rs, propose.rs, curator, curator-llm test helpers, graph test helpers `make_node`/`make_edge`).
3. For a new collection field on a graph type: extend the determinism sorting in `crates/scryrs-graph/src/lib.rs` (`sort_node_collections` / `sort_edge_collections` / `sort_evidence_links` inside `to_document`). Unsorted collections compile fine and fail idempotency tests — or worse, pass tests and produce nondeterministic artifacts.
   Verify: `grep -n "sort_node_collections\|sort_edge_collections" crates/scryrs-graph/src/lib.rs` — your field is sorted there.
4. Add tests: a serde round-trip (camelCase name!) in the types-crate `mod tests`, and for graph changes a `to_document`-based determinism assertion in scryrs-graph mirroring the existing `to_document_sorts_*` tests.
   Verify: `grep -n "<your_field_camelCase>" crates/scryrs-types/src/lib.rs` — round-trip test references the JSON name.
5. Update the paired contract spec — every prior types change landed with one (e.g. commit `3a9a280` = types + graph + `openspec/specs/graph-contract/spec.md` as one unit): `graph-contract` for graph types, `trace-event-schema` for trace types, `proposal-contract` / `route-manifest` for those domains. Then update the derived docs page field tables (`.devagent/docs/docs/graph.md` etc. — the docs-writer skill covers structure).
   Verify: `grep -n "<yourField>" openspec/specs/graph-contract/spec.md .devagent/docs/docs/graph.md` — contract and docs mention it.
6. Check the runtime-only consumers that the compiler cannot protect (see below), then run `scripts/precommit-run`. If a golden `.snap` under `crates/scryrs-cli/` covers changed output, regenerate via `source scripts/lib/docker-verification.sh && run_rust env INSTA_UPDATE=always cargo test -p scryrs-cli --locked`.

## Easy-to-miss details

- **The dashboard TypeScript mirror desyncs silently.** `crates/scryrs-dashboard/frontend/src/shared/api/client.ts` hand-mirrors `HotspotEntry`, `HotspotsReport`, `TraceEventItem`, etc. A trace/hotspot type change passes the whole Rust build and breaks the dashboard at runtime. Update the mirror (and `crates/scryrs-dashboard/src/server.rs`'s row mapping if fields flow from SQLite).
- **String-keyed graph logic breaks at runtime, not compile time.** `GraphNode.kind` / `GraphEdge.relationship` are `String`s; curator splits `node.id` on `':'` assuming the `"{kind}:{subject}"` convention from `crates/scryrs-cli/src/graph.rs`, and branches on literal kinds (`"doc_group"`, `"doc_page"`, …). Changing an id format or kind string compiles clean and silently breaks curator/route grouping.
- **`EvidenceLink` identity excludes `score`** (custom `impl Eq` in scryrs-types; sort tie-break treats score as auxiliary). A new EvidenceLink field needs a deliberate identity/sort decision or determinism breaks.
- **Scoring must stay aligned batch↔live**: `scryrs-server` reuses `scryrs-core`'s `per_event_contribution`/`score_hotspots`. A scoring change is a two-consumer contract change.
- **camelCase vs PascalCase**: graph/route/proposal types serialize camelCase; trace payloads use PascalCase `#[serde(tag = "type")]`. Write the round-trip test against the JSON, not the Rust name.

## Self-check before reporting done

- [ ] Every `<Type> {` constructor hit from the grep updated?
- [ ] New collection fields sorted in `to_document`, with a determinism test?
- [ ] Paired `openspec/specs/*` contract spec updated in the same change?
- [ ] Dashboard TS mirror checked (and updated if trace/hotspot types moved)?
- [ ] `scripts/precommit-run` green, snapshots regenerated if help/artifact output changed?
