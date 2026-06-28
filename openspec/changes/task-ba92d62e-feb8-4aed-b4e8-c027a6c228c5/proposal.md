## Why

The repository already ships deterministic route-manifest generation (`scryrs route <PATH>`) and a reusable route-hint projection (`hints_from_manifest`), but there is no user-facing `scryrs route explain` command. Current help, help-json, and documentation all explicitly mark explain as deferred. As a result, future agent users must inspect `.scryrs/routes.json` manually or rediscover graph relationships on every session.

This change adds a deterministic, model-free `scryrs route explain <PATH> --query <TEXT>` CLI command that reads only the route manifest artifact, applies documented query filtering over manifest fields, and returns evidence-backed route recommendations in the existing `RouteHintDocument` wire shape.

## What Changes

- **New CLI subcommand**: `scryrs route explain <PATH> --query <TEXT>` added via the existing `proposals`-style pre-clap intercept pattern in `dispatch.rs`. The parent `route` command is restructured with `.subcommand_required(false)` to preserve backward compatibility for `scryrs route <PATH>` (manifest generation).
- **New module**: `crates/scryrs-cli/src/route_explain.rs` implements CLI dispatch, artifact loading (`.scryrs/routes.json` → `RouteManifestDocument`), schema validation, and exit-code handling mirroring the fail-fast pattern in `route.rs`.
- **New runtime function**: `explain_hints(manifest, query)` added in `crates/scryrs-runtime/src/lib.rs` composes with `hints_from_manifest`, then applies deterministic case-insensitive substring matching over `RouteEntry.label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`. Matching uses a documented tie-break: exact match > prefix match > substring match, with manifest entry order (by id ascending) as final tie-break.
- **Extended reason template**: Each `RouteHintItem.reason` appends `"; query match on {fields}"` (comma-separated list of matched field names) when explain mode is active. The core `hints_from_manifest` function is unchanged.
- **No-match contract**: Zero matches emits a valid `RouteHintDocument` with empty `hints` array and exits 0.
- **Documentation updates**: Help text, help-json, route-manifest docs, and CLI-v0-contract docs replace all "deferred" mentions with complete explain usage, example, and interpretation notes.
- **Test and snapshot coverage**: New dispatch tests cover success, determinism, missing/malformed/schema-mismatch routes.json, missing `--query`, missing `PATH`, zero-match, and help/help-json discoverability. Existing help snapshots are updated via `cargo insta`.

## Impact

- **Affected crates**: `scryrs-cli` (dispatch, new module, help text, help json, tests, snapshots), `scryrs-runtime` (new `explain_hints` function), `scryrs-types` (no schema changes — reuses existing contracts).
- **Affected docs**: `.devagent/docs/docs/route-manifests.md`, `.devagent/docs/docs/cli-v0-contract.md`.
- **Risk**: CLI dispatch restructuring must not break `scryrs route <PATH>`. Mitigated by `.subcommand_required(false)` and a pre-clap intercept that only catches `route explain` before clap processes args.
- **No breaking changes**: Existing `RouteManifestDocument`, `RouteHintDocument`, `RouteHintItem` schemas are preserved. `hints_from_manifest` API is unchanged.