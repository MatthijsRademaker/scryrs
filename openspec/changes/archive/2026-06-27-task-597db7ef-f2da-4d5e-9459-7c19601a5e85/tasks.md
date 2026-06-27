## 1. Curator Engine Foundation

- [x] 1.1 Add `serde_json` dependency to `scryrs-curator/Cargo.toml`.
- [x] 1.2 Define `pub fn generate_proposals(graph: &KnowledgeGraphDocument, hotspots: &[HotspotEntry], generated_at: &str) -> Vec<ProposalDocument>` as the curator entry point.
- [x] 1.3 Implement `docs_note` rule: generate one `ProposalDocument` per hotspot entry with markdown content describing subject and score, citing `EvidenceSourceKind::HotspotSubject` evidence.
- [x] 1.4 Implement `skill` rule: generate proposals for hotspot entries with `Failure` outcome event counts > 0, citing relevant hotspot evidence.
- [x] 1.5 Implement `memory_patch` rule: generate proposals for hotspot entries with failure-ratio ≥ 0.5 (failure outcome count ≥ total outcome count / 2) and score ≥ 4, with structured JSON `proposedContent`.
- [x] 1.6 Implement `adr` rule: detect subject strings appearing across ≥2 distinct subject kinds with aggregate score ≥ 10, generate one ADR proposal per cluster.
- [x] 1.7 Implement `semantic_graph_grouping` rule: detect graph node families with ≥2 distinct subject kinds sharing the same subject stem, with at least one shared hotspot-backed evidence link, generate grouping proposals with `sourceNodeIds`, `targetGroupNodeId`, and `targetGroupLabel`.
- [x] 1.8 Set `createdAt` on every generated proposal to the hotspot report's `generatedAt` field.
- [x] 1.9 Remove the old `propose_from_hotspot()` placeholder function and its tests (or repurpose them to test the new engine).
- [x] 1.10 Add unit tests for each deterministic rule pack: verify threshold behavior, empty-input handling, and evidence citation fields.

## 2. CLI Command Registration

- [x] 2.1 Create `crates/scryrs-cli/src/propose.rs` with `#[cfg(feature = "curator")]` gating.
- [x] 2.2 Implement `pub(crate) fn write_proposals(out, err, path) -> i32` following the pattern of `write_graph_json`/`write_route_json`.
- [x] 2.3 Resolve PATH to absolute repo root; exit 2 on resolution failure with `scryrs propose: cannot resolve path '...'` error.
- [x] 2.4 Load `.scryrs/hotspots.json`; exit 2 with `scryrs propose: hotspots artifact not found at ...` if missing; exit 2 with `scryrs propose: malformed hotspots file: ...` if parse fails.
- [x] 2.5 Load `.scryrs/graph.json`; exit 2 with `scryrs propose: graph artifact not found at ...` if missing; exit 2 with `scryrs propose: malformed graph file: ...` if parse fails.
- [x] 2.6 Delegate to `scryrs-curator::generate_proposals()`, validate each returned `ProposalDocument` via `.validate()`, ensure `.scryrs/proposals/` directory exists (create with `create_dir_all`).
- [x] 2.7 Write each validated proposal as `.scryrs/proposals/{id}.json` using `inbox_filename()`. Report count to stdout following the `graph`/`route` pattern.
- [x] 2.8 Register `propose` in the CLI dispatch allowlist (the `first != "propose"` guard in `dispatch.rs`).
- [x] 2.9 Add `propose` subcommand to clap `Command` builder: `Command::new("propose").about("...").disable_help_flag(true).disable_version_flag(true).arg(Arg::new("PATH").required(true).value_name("PATH"))`.
- [x] 2.10 Add `Some(("propose", m))` match arm in the subcommand dispatcher, gated behind `#[cfg(feature = "curator")]`, calling `write_proposals`.
- [x] 2.11 Add `"propose"` to the `attempted_command` guard list for error routing.
- [x] 2.12 Add `Some("propose")` match arm for `MissingRequiredArgument` and `UnknownArgument` error kinds, following the three-line `scryrs propose: ...` format.
- [x] 2.13 Declare `mod propose;` in `crates/scryrs-cli/src/lib.rs` (ungated; the file content is gated internally).

## 3. Help and Documentation Surface

- [x] 3.1 Add `propose` command entry to `help_text.rs` in the COMMANDS section following the `graph`/`route` style: `scryrs propose <PATH> — Generate reviewable knowledge proposals from hotspot and graph evidence.`
- [x] 3.2 Add `propose` entry to `help_json.rs` in alphabetical sorted position (between `init` and `record`), with PATH argument and output description.
- [x] 3.3 Update `README.md` to list `propose` among supported commands; remove `propose` from the "unsupported" wording in Current Limitations.
- [x] 3.4 Document `createdAt` determinism rule in help text: note that timestamp derives from hotspot `generatedAt`.

## 4. Test Updates and New Tests

- [x] 4.1 Remove `"propose"` from the `previously_stubbed_commands_exit_2` test array in `dispatch_tests.rs`.
- [x] 4.2 Add integration test: valid hotspots + graph produce proposal files under `.scryrs/proposals/`.
- [x] 4.3 Add integration test: missing hotspots → exit 2, no proposal files written.
- [x] 4.4 Add integration test: malformed hotspots → exit 2, no proposal files written.
- [x] 4.5 Add integration test: deterministic rerun — same inputs produce identical proposal set (same filenames, same content ids).
- [x] 4.6 Add integration test: source-of-truth non-mutation — verify `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/` are unmodified after `propose`.
- [x] 4.7 Add integration test: upsert behavior — running twice with same inputs overwrites same files; running with different inputs adds new files without removing old ones.
- [x] 4.8 Add integration test: semantic grouping proposals carry `sourceNodeIds`, `targetGroupNodeId`, `targetGroupLabel` and serialize as `targetType = semantic_graph_grouping`.
- [x] 4.9 Add integration test: `propose` command appears in `--help` and `--help-json` output.
- [x] 4.10 Ensure all tests pass with `cargo test -p scryrs-cli --features curator` and `cargo test -p scryrs-curator`.
