## 1. Shared contract and manifest-loading plumbing

- [ ] 1.1 Add `RouteBundleDocument` and `BUNDLE_SCHEMA_VERSION` to `crates/scryrs-types/src/lib.rs`, with `schemaVersion`, `query`, `limit`, and `targets` reusing the existing `RouteHintItem` field shape.
- [ ] 1.2 Extract shared `.scryrs/routes.json` resolution, JSON parsing, and schema-version validation into `crates/scryrs-cli/src/route_common.rs` (or equivalent shared helper), and update `route_explain.rs` to use it without changing existing diagnostics or exit codes.
- [ ] 1.3 Mirror the existing runtime feature-gating pattern in the new bundle handler so bundle and explain build under the same assumptions.

## 2. CLI implementation and surfaced behavior

- [ ] 2.1 Add `crates/scryrs-cli/src/route_bundle.rs` to parse `PATH`, required `--query`, and required positive `--limit`, load the route manifest, call explain-derived ranking, truncate with the requested limit, and serialize a single-line bundle JSON document.
- [ ] 2.2 Extend `crates/scryrs-cli/src/dispatch.rs` so `route bundle`, `route bundle --help`, and the existing `route explain` paths are all intercepted and dispatched correctly without regressing `scryrs route <PATH>` manifest generation.
- [ ] 2.3 Update `crates/scryrs-cli/src/help_text.rs` and `crates/scryrs-cli/src/help_json.rs` so the new command is discoverable, the bundle output contract is described, and bundle-vs-explain guidance is explicit. Bump `SURFACE_VERSION` if the public help-json surface changes.

## 3. Docs and verification

- [ ] 3.1 Update `.devagent/docs/docs/route-manifests.md` and `.devagent/docs/docs/cli-v0-contract.md` to document the bundle schema, limit behavior, preserved evidence fields, and when agents should call `bundle` versus `explain`.
- [ ] 3.2 Add or refresh CLI tests and snapshots covering successful bundle output, stable ordering before truncation, zero-match output, missing or invalid or non-positive limits, missing or malformed or schema-mismatched `.scryrs/routes.json`, manifest-only reads, and help/help-json discoverability.
