## 1. CLI Dispatch Restructuring

- [ ] 1.1 Restructure `route` clap subcommand in `dispatch.rs` as a parent command with `.subcommand_required(false)` that accepts either bare `<PATH>` (existing) or `explain` subcommand.
- [ ] 1.2 Add pre-clap intercept for `route explain` before clap processes args (following `proposals` pattern at `dispatch.rs:82`).
- [ ] 1.3 Wire the intercept to `route_explain::execute_route_explain(out, err, args)`.
- [ ] 1.4 Ensure existing `scryrs route <PATH>` behavior is unchanged (path resolution, graph.json loading, manifest generation, exit codes).
- [ ] 1.5 Ensure `scryrs route` without PATH still exits 2 with "missing required PATH argument".

## 2. New Module: `crates/scryrs-cli/src/route_explain.rs`

- [ ] 2.1 Create `route_explain.rs` with `pub(crate) fn execute_route_explain(out, err, args) -> i32`.
- [ ] 2.2 Implement `--help` / `-h` handler that writes explain usage text (copy-paste example, field descriptions, interpretation notes).
- [ ] 2.3 Parse `PATH` and `--query` from args (manual parsing following route.rs convention).
- [ ] 2.4 Resolve `PATH` to absolute repo root, load `.scryrs/routes.json`, parse as `RouteManifestDocument`.
- [ ] 2.5 Validate `schema_version` matches `ROUTE_SCHEMA_VERSION`. Exit 2 on mismatch.
- [ ] 2.6 Exit 2 with explicit diagnostics for: missing routes.json, malformed JSON, schema version mismatch, missing `--query`, missing `PATH`.
- [ ] 2.7 Call `scryrs_runtime::explain_hints(&manifest, &query)` and serialize result as single-line JSON to stdout.
- [ ] 2.8 Return exit 0 on success.

## 3. New Runtime Function: `explain_hints` in `scryrs-runtime`

- [ ] 3.1 Add `pub fn explain_hints(manifest: &RouteManifestDocument, query: &str) -> RouteHintDocument` to `crates/scryrs-runtime/src/lib.rs`.
- [ ] 3.2 Internally call `hints_from_manifest(manifest)` to obtain the full ordered hints array.
- [ ] 3.3 For each entry in the manifest, check for case-insensitive substring match of `query` against `label`, `subject`, `id`, `target`, `kind`, and `evidence_links[].subject`.
- [ ] 3.4 Classify each match: exact string match to any field → tier 3; prefix match → tier 2; substring match → tier 1.
- [ ] 3.5 Filter to only matched entries. Discard unmatched entries entirely.
- [ ] 3.6 Sort matched entries by match tier descending, then by original manifest ordinal rank ascending.
- [ ] 3.7 Extend each matching hint's `reason` field to append `"; query match on {comma-separated field names}"`.
- [ ] 3.8 Return `RouteHintDocument { schema_version: HINT_SCHEMA_VERSION, hints }` with empty `hints` array on zero matches.

## 4. Help Text and Help JSON Updates

- [ ] 4.1 Update `help_text.rs` to replace "The `scryrs route explain` command is deferred" with `scryrs route explain <PATH> --query <TEXT>` usage, example, and interpretation notes.
- [ ] 4.2 Update `help_json.rs` to add `explain` subcommand entry under `route` with arguments, output contract, and remove "deferred" language.
- [ ] 4.3 Update `help_json.rs` `routeHintOutput.description` to note explain is available and reference the explain subcommand.
- [ ] 4.4 Bump `SURFACE_VERSION` if the schema mandates it for surface changes.

## 5. Documentation Updates

- [ ] 5.1 Update `.devagent/docs/docs/route-manifests.md` to replace "Deferred" table entry with explain command documentation, copy-paste example, match-field table, and interpretation notes.
- [ ] 5.2 Update `.devagent/docs/docs/cli-v0-contract.md` route-hint section to replace "deferred for future enhancement" with explain command contract (arguments, exit codes, output format).
- [ ] 5.3 Verify all cross-references and links remain valid after changes.

## 6. Tests and Snapshots

- [ ] 6.1 Add `route_explain_help_flag_prints_help_and_exits_0` test.
- [ ] 6.2 Add `route_explain_missing_query_exits_2` test.
- [ ] 6.3 Add `route_explain_missing_path_exits_2` test.
- [ ] 6.4 Add `route_explain_missing_routes_json_exits_2` test.
- [ ] 6.5 Add `route_explain_malformed_routes_json_exits_2` test.
- [ ] 6.6 Add `route_explain_schema_version_mismatch_exits_2` test.
- [ ] 6.7 Add `route_explain_successful_match_produces_hints` test.
- [ ] 6.8 Add `route_explain_deterministic_repeatability` test.
- [ ] 6.9 Add `route_explain_zero_match_emits_empty_hints_exits_0` test.
- [ ] 6.10 Add `route_explain_help_json_includes_explain_entry` test.
- [ ] 6.11 Add `route_explain_help_text_includes_explain_command` test.
- [ ] 6.12 Verify existing `route` tests still pass unchanged.
- [ ] 6.13 Update insta snapshots after all help text changes (`INSTA_UPDATE=always cargo test`).