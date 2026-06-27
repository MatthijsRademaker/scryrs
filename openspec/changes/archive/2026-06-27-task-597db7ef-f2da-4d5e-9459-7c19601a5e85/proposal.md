## Why

The scryrs repository ships deterministic hotspot (`.scryrs/hotspots.json`) and graph (`.scryrs/graph.json`) artifacts plus a versioned `ProposalDocument` inbox contract, but there is no generator. The `propose` command is explicitly listed as unsupported in dispatch, help text, help JSON, and README. The `scryrs-curator` crate contains only a single-entry placeholder (`propose_from_hotspot`) that targets `docs_note` alone.

The roadmap's Phase 6 requires deterministic, evidence-citing proposal artifacts that materialize knowledge candidates for review without silently mutating source-of-truth files. The proposal contract defers command generation; this change closes that gap by implementing a deterministic proposal engine and CLI command that generates review-only `ProposalDocument` files for five target types: `docs_note`, `adr`, `skill`, `memory_patch`, and `semantic_graph_grouping`.

## What Changes

- **New CLI command**: `scryrs propose <PATH>` mirrors the existing `graph`/`route` pattern — a flat command with a single positional PATH argument, feature-gated behind the existing `curator` feature.
- **Expanded `scryrs-curator` engine**: Replaces the single-hotspot placeholder with a deterministic multi-target generation function `generate_proposals(graph, hotspots) -> Vec<ProposalDocument>` that applies concrete heuristics per target type.
- **CLI surface updates**: `propose` appears in `--help`, `--help-json`, and README. The previously-stubbed unknown-command test is updated to reflect the new command.
- **Deterministic proposal generation**: Given identical hotspot and graph inputs, repeated runs produce the same set of `.scryrs/proposals/{id}.json` files with stable content-addressed IDs. `createdAt` is derived from the hotspot artifact's `generatedAt` for determinism.
- **Non-mutation guarantee**: The command writes only under `.scryrs/proposals/`. It does not create or modify `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or any source-of-truth ADR, skill, or memory files.
- **Crate dependency**: `scryrs-curator` gains a `serde_json` dependency for deserializing `KnowledgeGraphDocument` from disk.

## Impact

- **Affected crates**: `scryrs-cli` (new `propose.rs` module, updated dispatch, help_text, help_json), `scryrs-curator` (expanded engine, new `serde_json` dep), `README.md` (command surface listing).
- **Affected tests**: `dispatch_tests::previously_stubbed_commands_exit_2` must remove `"propose"` from the unknown-command list. New integration tests in `propose.rs` for artifact loading, generation, determinism, and non-mutation assertions.
- **No breaking changes**: All existing commands, contracts, and artifact paths remain unchanged. The `curator` feature is already in `default` features.
- **risk: ADR generation quality**: V1 hotspot-only heuristics may produce weak ADR candidates. Mitigation: narrow ADR rule requiring multiple subject kinds with high scores; document that ADR proposals improve with richer graph data.
- **risk: Semantic grouping false positives**: Simple stem-based node matching may propose spurious groupings. Mitigation: require ≥2 distinct subject kinds in the candidate group plus at least one shared hotspot-backed evidence link.
- **no effect**: Graph build, route generation, live-hotspot APIs, hotspot scoring, dashboard, or server behavior are unchanged.