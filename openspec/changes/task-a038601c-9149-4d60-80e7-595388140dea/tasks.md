## 1. Route Manifest Schema Contract

- [ ] 1.1 Define `ROUTE_SCHEMA_VERSION = "1.0.0"` constant in `crates/scryrs-types/src/lib.rs`
- [ ] 1.2 Define `RouteManifestDocument` struct with `schemaVersion`, `metadata: GraphMetadata`, and `routes: Vec<RouteEntry>`
- [ ] 1.3 Define `RouteEntry` struct with `id`, `subjectKind`, `subject`, `label`, `target`, `kind`, `evidenceLinks: Vec<EvidenceLink>`, optional `grouping` (groupId, groupLabel), and optional metadata
- [ ] 1.4 Apply `#[serde(rename_all = "camelCase")]`, `#[serde(skip_serializing_if)]` for optional/empty fields matching existing graph contract conventions
- [ ] 1.5 Remove the `RouteHint` placeholder struct (insufficient for v1 manifest)
- [ ] 1.6 Add serde round-trip unit tests for `RouteManifestDocument`, `RouteEntry`, and `RouteEntry` with/without grouping

## 2. Route Manifest Generator

- [ ] 2.1 Create `crates/scryrs-cli/src/route.rs` module, gated behind `#[cfg(feature = "graph")]`
- [ ] 2.2 Implement `write_route_json(out, err, path)` with same signature pattern as `write_graph_json`
- [ ] 2.3 Load and deserialize `.scryrs/graph.json` into `KnowledgeGraphDocument`
- [ ] 2.4 Validate loaded graph `schemaVersion` equals `GRAPH_SCHEMA_VERSION`; exit 2 with contract error on mismatch
- [ ] 2.5 Build parent lookup map from `contains` edges (for each edge where `relationship == "contains"`, map `target_node_id → source_node_id`)
- [ ] 2.6 For each graph node, construct a `RouteEntry` with fields derived from node properties and parent lookup
- [ ] 2.7 Sort `RouteEntry` vector by `id` ascending (preserving `(subjectKind, subject)` identity ordering)
- [ ] 2.8 Sort `evidenceLinks` within each entry by the documented tie-break chain
- [ ] 2.9 Serialize `RouteManifestDocument` as single-line JSON to stdout
- [ ] 2.10 Write artifact to `.scryrs/routes.json`; exit 1 on write failure with contract error on stderr
- [ ] 2.11 Implement disabled-feature fallback (exit 2, "unavailable" message) following `graph.rs` pattern

## 3. CLI Dispatch Integration

- [ ] 3.1 Register `route` subcommand in clap `Command` chain: `Command::new("route").about("...").arg(Arg::new("PATH").required(true))`
- [ ] 3.2 Add `"route"` to the known-command allowlist in `dispatch.rs` pre-clap check
- [ ] 3.3 Add dispatch arm: `Some(("route", m))` routes to `write_route_json(&mut out, &mut err, path)`
- [ ] 3.4 Add `MissingRequiredArgument` error handler for `"route"` with three-line contract format
- [ ] 3.5 Add `UnknownArgument` error handler for `"route"` with three-line contract format
- [ ] 3.6 Add `scryrs route <PATH>` entry to `help_text.rs` `write_help()` output
- [ ] 3.7 Add `route` entry to `help_json.rs` `cli_surface_doc()` with argument specs and output contract fields

## 4. Test Suite

- [ ] 4.1 Remove `"route"` from `previously_stubbed_commands_exit_2` array in `dispatch_tests.rs`
- [ ] 4.2 Add test: `route_command_produces_help_output` (verify `--help` lists route)
- [ ] 4.3 Add test: `route_missing_graph_exits_2` (no `.scryrs/graph.json` produces exit 2 with contract error)
- [ ] 4.4 Add test: `route_malformed_graph_exits_2` (invalid JSON produces exit 2 with contract error)
- [ ] 4.5 Add test: `route_repeated_runs_produce_byte_identical_output` (two runs over same graph input produce identical stdout)
- [ ] 4.6 Add test: `route_identity_boundary_preserves_distinct_subjects` (graph nodes `file:auth`, `search:auth`, `symbol:auth` with no linking edges produce three distinct `RouteEntry` items; verify no collapsing by shared label text)
- [ ] 4.7 Add test: `route_doc_pages_include_grouping_from_contains_edges` (doc page with `contains` edge from parent group node carries `grouping` field)
- [ ] 4.8 Add test: `route_hotspot_nodes_remain_ungrouped_in_v1` (hotspot nodes have no `grouping` field when no cross-domain edges exist)
- [ ] 4.9 Add test: `route_artifact_written_to_routes_json` (verify `.scryrs/routes.json` exists after successful run)
- [ ] 4.10 Update help text `insta` snapshots for new route command entry

## 5. Documentation and README

- [ ] 5.1 Add `scryrs route <PATH>` to the public command list in `README.md`
- [ ] 5.2 Remove `route` from the unknown-command section of `README.md`
- [ ] 5.3 Mark route-manifest artifact as delivered in `.devagent/docs/docs/roadmap.mdx` Phase 5 section

## 6. Validation and Cleanup

- [ ] 6.1 Run `cargo test -p scryrs-types` — all new schema tests pass
- [ ] 6.2 Run `cargo test -p scryrs-cli --features graph` — all route and dispatch tests pass
- [ ] 6.3 Run `cargo test -p scryrs-graph` — no regressions (crate unchanged)
- [ ] 6.4 Run `cargo clippy --all-targets` — no new warnings
- [ ] 6.5 Run `cargo fmt --all -- --check` — formatting unchanged
- [ ] 6.6 Verify `scryrs --help-json` output includes route entry with correct field structure