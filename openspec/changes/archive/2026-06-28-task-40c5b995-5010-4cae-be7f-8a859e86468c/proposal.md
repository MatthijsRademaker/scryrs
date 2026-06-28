## Why

`scryrs-runtime` currently defines an ad-hoc `RouteHint { target, reason }` struct with only two fields — no label, rank, relevance, or evidence-backed citations. The acceptance criteria for the runtime routing contract require explicit explanation fields that cite deterministically-derived evidence. The existing `RouteManifestDocument` already carries all the data needed (`id`, `subjectKind`, `label`, `target`, `evidenceLinks`, `grouping`), so the missing piece is a stable projection contract and a deterministic producer function.

This task fills that gap by defining a versioned route-hint wire contract in `scryrs-types` and a deterministic, model-free hint producer in `scryrs-runtime`. It enables runtime consumers to ask what to load and why, with explicit evidence citations, without collapsing distinct route identities like `file:auth`, `search:auth`, and `symbol:auth` or smuggling in opaque model-based ranking.

## What Changes

### Shared contract (`crates/scryrs-types/src/lib.rs`)

- Define `HINT_SCHEMA_VERSION = "1.0.0"` as an independent version constant, following the existing pattern (`ROUTE_SCHEMA_VERSION`, `PROPOSAL_SCHEMA_VERSION`, etc.).
- Define `RouteHintDocument` as the top-level versioned envelope with `schemaVersion` and `hints: Vec<RouteHintItem>`.
- Define `RouteHintItem` with fields: `routeId` (the source route entry id), `target`, `label`, `rank` (u32, 1-based ordinal), `relevance` (optional u32, deferred), `reason`, and `evidence` (copied `Vec<EvidenceLink>` from the source `RouteEntry`).

### Runtime producer (`crates/scryrs-runtime/src/lib.rs`)

- Replace the current ad-hoc `RouteHint { target, reason }` struct with a deterministic `hints_from_manifest(manifest: &RouteManifestDocument) -> RouteHintDocument` function.
- Project each `RouteEntry` into one `RouteHintItem`. Rank is the 1-based ordinal position within the manifest's sort order (by `id` ascending). Relevance is `None` (deferred for future enhancement).
- Reason is a deterministic template string: `"Route '{label}' ({id}): {N} evidence link(s), subject kind {subjectKind}"`.
- Evidence is copied directly from the source `RouteEntry.evidenceLinks` — no new types, no opening of `.scryrs/proposals/` or `.scryrs/accepted/`.
- Preserve identity boundaries: `file:auth`, `search:auth`, and `symbol:auth` produce three distinct `RouteHintItem` values with distinct `routeId` fields.

### Tests (`crates/scryrs-runtime/src/lib.rs`)

- Replace existing `RouteHint` placeholder tests with tests for `hints_from_manifest`.
- Add identity-preservation regression test mirroring the existing `route_identity_boundary_preserves_distinct_subjects` test in `dispatch_tests.rs`.
- Add determinism test verifying repeated calls produce identical output.
- Add field-level tests for rank, reason template, and evidence copying.

### Documentation

- Update `crates/scryrs-cli/src/help_text.rs`: add route-hint contract mention under `scryrs route <PATH>`, noting that `scryrs route explain` is deferred.
- Update `crates/scryrs-cli/src/help_json.rs`: add `routeHintOutput` documentation under the `route` command entry with field descriptions and the deferred-ranking policy statement.
- Update `.devagent/docs/docs/cli-v0-contract.md`: add a route-hint contract section with a JSON example and explicit deferred-ranking language.
- Update `.devagent/docs/docs/route-manifests.md`: move route hints from the Deferred column to a new "Route Hint Contract" section describing the schema, identity preservation, and deterministic/deferred ranking.

### Non-changes

- No `scryrs route explain` CLI command ships in this task — deferred to a follow-up.
- No mutation of graph, route, or proposal artifacts during hint generation.
- No model-based ranking, fuzzy retrieval, or hidden heuristics.
- No opening of `.scryrs/proposals/`, `.scryrs/accepted/`, or `.scryrs/rejected/` during hint production.

## Impact

- **Affected crates**: `scryrs-types` (new types), `scryrs-runtime` (replacement logic + tests), `scryrs-cli` (help text, help-json), none others.
- **Affected specs**: New `route-hint` capability spec; no changes to `route-manifest`, `graph-contract`, or `graph-build` specs.
- **Affected docs**: `cli-v0-contract.md`, `route-manifests.md`.
- **Breaking change**: The existing `RouteHint` struct in `scryrs-runtime` is replaced. No external consumer depends on it yet (the crate is a foundation library with no published API consumers), so this is a non-breaking internal change.
- **Risk**: Low — the change is additive schema work and a deterministic projection with existing inputs. No network, concurrency, or I/O surface.