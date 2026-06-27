# Tasks: graph-build

## 1. Spec Reconciliation

- [x] 1.1 Update `openspec/specs/graph-contract/spec.md`: revise the "Contract foundation adds no graph build surface" scenario to clarify the scope boundary — `crates/scryrs-graph` the crate remains build-pipeline-free, while the CLI command in `crates/scryrs-cli` is a separate consumer using the graph crate's public API.

## 2. Graph Builder Module

- [x] 2.1 Create `crates/scryrs-cli/src/graph.rs` with the `write_graph_json(out, err, path)` function following the `hotspots.rs` structural pattern.
- [x] 2.2 Implement input loading: resolve repo root, load `.scryrs/hotspots.json` via `serde_json::from_str`, fail exit 2 on missing or malformed.
- [x] 2.3 Implement docs scanning: enumerate `.devagent/docs/docs/*.md`/`*.mdx` files (sorted), load `_nav.json`, warn on stderr and continue if docs directory is missing or empty.
- [x] 2.4 Implement node synthesis: hotspot entries → `GraphNode` with id `{subjectKind}:{subject}`, label from subject, kind from subjectKind, evidence link with `sourceKind: "local_trace_row"` and `rowIds` from `evidence.rowIds`.
- [x] 2.5 Implement doc node synthesis: doc pages → `GraphNode` with id `doc_page:<slug>`, kind `"doc_page"`, label from page slug, evidence link with `sourceKind: "doc_reference"` and `docRef` set to the page slug. Create synthetic `docs_root` node if docs exist.
- [x] 2.6 Implement edge synthesis: `_nav.json` groups → `contains` edges from parent nav group nodes to child page nodes, plus `contains` edges from `docs_root` to top-level nav groups.
- [x] 2.7 Implement assembly: populate `KnowledgeGraph` container, call `validate()`, call `to_document(None)`.
- [x] 2.8 Implement output: write JSON to stdout and `.scryrs/graph.json`. Exit 1 on artifact write failure.

## 3. CLI Surface Updates

- [x] 3.1 Update `dispatch.rs`: add `"graph"` to the known-command allowlist guard, add `graph` clap subcommand with required `PATH` argument and usage-error handling.
- [x] 3.2 Update `help_text.rs`: add `scryrs graph build <PATH>` entry to the COMMANDS section with a description and output contract summary.
- [x] 3.3 Update `help_json.rs`: add a `graph` command entry to the machine-readable surface document.
- [x] 3.4 Register `mod graph;` in `crates/scryrs-cli/src/lib.rs`.

## 4. Tests

- [x] 4.1 Remove `"graph"` from the `previously_stubbed_commands_exit_2()` test in `dispatch_tests.rs`.
- [x] 4.2 Add dispatch test: `graph build` missing PATH exits 2 with contract error format.
- [x] 4.3 Add dispatch test: `graph build` with missing `.scryrs/hotspots.json` exits 2 with descriptive error.
- [x] 4.4 Add dispatch test: `graph build` with valid hotspot and docs exits 0, produces valid JSON on stdout.
- [x] 4.5 Add dispatch test: `graph build` repeated runs produce byte-identical JSON output for unchanged inputs.
- [x] 4.6 Add dispatch test: `graph build` with empty docs directory exits 0 (warning on stderr, hotspot-only graph on stdout).
- [x] 4.7 Add dispatch test: `graph build` with malformed `.scryrs/hotspots.json` exits 2.
- [x] 4.8 Add unit tests in `graph.rs` for: node ID derivation, evidence link conversion, nav hierarchy edge generation, empty inputs, sorted ordering.
- [x] 4.9 Update `insta` snapshots for `--help` and `--help-json` to include the new graph command.

## 5. Documentation Updates

- [x] 5.1 Update `.devagent/docs/docs/graph.md`: move `scryrs graph build` from Deferred to Shipped; remove "No `scryrs graph build` command exists" statement; document the command.
- [x] 5.2 Update `README.md`: bump command count from six to seven; add `scryrs graph build <PATH>` example.
- [x] 5.3 Update `.devagent/docs/docs/roadmap.mdx`: note partial delivery of Phase 5 graph build.
