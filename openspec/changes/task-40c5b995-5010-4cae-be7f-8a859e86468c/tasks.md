## 1. Define route-hint wire contract in scryrs-types

- [ ] 1.1 Add `HINT_SCHEMA_VERSION = "1.0.0"` constant to `crates/scryrs-types/src/lib.rs` alongside existing contract version constants.
- [ ] 1.2 Define `RouteHintItem` struct with `routeId: String`, `target: String`, `label: String`, `rank: u32`, `relevance: Option<u32>`, `reason: String`, and `evidence: Vec<EvidenceLink>`. Derive `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. Use `#[serde(rename_all = "camelCase")]` following existing contract conventions.
- [ ] 1.3 Define `RouteHintDocument` struct with `schemaVersion: String` and `hints: Vec<RouteHintItem>`. Same derive and serde conventions.
- [ ] 1.4 Add serialization round-trip tests for `RouteHintItem` and `RouteHintDocument` in `crates/scryrs-types/src/lib.rs`.

## 2. Implement deterministic hint producer in scryrs-runtime

- [ ] 2.1 Replace the existing `RouteHint` struct, `explain_route` function, and all six placeholder tests in `crates/scryrs-runtime/src/lib.rs` with new deterministic logic.
- [ ] 2.2 Implement `pub fn hints_from_manifest(manifest: &RouteManifestDocument) -> RouteHintDocument`: iterate `manifest.routes` in order, project each `RouteEntry` into one `RouteHintItem`. Set `rank` to 1-based index, `relevance` to `None`, `reason` to `format!("Route '{}' ({}): {} evidence link(s), subject kind {}", entry.label, entry.id, entry.evidence_links.len(), entry.subject_kind)`, `evidence` to `entry.evidence_links.clone()`, `routeId` to `entry.id.clone()`, `target` to `entry.target.clone()`, `label` to `entry.label.clone()`.
- [ ] 2.3 Generate `RouteHintDocument` with `schemaVersion: HINT_SCHEMA_VERSION.to_string()` and the populated `hints` vector.

## 3. Add runtime tests

- [ ] 3.1 Add test: `hints_from_manifest_preserves_identity_boundaries` — constructs a `RouteManifestDocument` with three routes (`file:auth`, `search:auth`, `symbol:auth`), calls `hints_from_manifest`, asserts `hints.len() == 3` and three distinct `routeId` values.
- [ ] 3.2 Add test: `hints_from_manifest_assigns_ordinal_rank` — verifies rank fields are 1-based and sequential.
- [ ] 3.3 Add test: `hints_from_manifest_copies_evidence_links` — verifies evidence arrays are preserved from source entries.
- [ ] 3.4 Add test: `hints_from_manifest_reason_template` — verifies reason includes label, id, subject kind, and evidence count.
- [ ] 3.5 Add test: `hints_from_manifest_is_deterministic` — calls twice with same input, asserts byte-identical output.
- [ ] 3.6 Add test: `hints_from_manifest_empty_manifest` — verifies empty manifest produces empty hints vector.
- [ ] 3.7 Add test: `hints_from_manifest_relevance_is_none` — verifies all relevance fields are `None`.

## 4. Update CLI contract documentation

- [ ] 4.1 Update `crates/scryrs-cli/src/help_text.rs`: add a note under the `scryrs route <PATH>` entry documenting the route-hint contract shape, that `scryrs route explain` is deferred, and that ranking is deterministic/deferred. Keep existing route command description intact.
- [ ] 4.2 Update `crates/scryrs-cli/src/help_json.rs`: add a `routeHintOutput` section under the `route` command entry in `cli_surface_doc()` with fields describing `RouteHintDocument` output shape, a JSON example, and explicit deferred-ranking policy statement.
- [ ] 4.3 Update `.devagent/docs/docs/cli-v0-contract.md`: add a route-hint contract section under the Route command section with a JSON example, field descriptions, and deferred-ranking language.
- [ ] 4.4 Update `.devagent/docs/docs/route-manifests.md`: move "runtime explanation" from the Deferred column to a new "Route Hint Contract" section describing the schema, identity preservation, deterministic rank/relevance derivation, and the `scryrs route explain` deferral.

## 5. Validation

- [ ] 5.1 Run `cargo test -p scryrs-runtime` to confirm all new tests pass.
- [ ] 5.2 Run `cargo test -p scryrs-types` to confirm serialization round-trip tests pass.
- [ ] 5.3 Run `cargo test -p scryrs-cli` to confirm CLI contract tests pass and snapshot updates are accepted.
- [ ] 5.4 Run `cargo check --workspace` to confirm no compilation errors across dependent crates.
- [ ] 5.5 Verify `scryrs --help` and `scryrs --help-json` output includes the route-hint contract documentation.