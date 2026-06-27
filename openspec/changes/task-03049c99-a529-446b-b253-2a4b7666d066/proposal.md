# Proposal: graph-build

## Why

The repository already ships the complete graph wire contract (`KnowledgeGraphDocument`, `GraphNode`, `GraphEdge`, `EvidenceLink`) in `scryrs-types`, and the deterministic `KnowledgeGraph` container with `validate()` and `to_document()` in `scryrs-graph`. The `graph` feature is already a default dependency in `scryrs-cli/Cargo.toml`. What is missing is the pipeline that populates a graph from real inputs — hotspot artifacts and documentation structure. Without this build step, downstream features such as route manifests and proposal generation have no structured knowledge to consume.

This task adds `scryrs graph build <PATH>` — the first CLI command for graph artifact construction. It merges local hotspot evidence from `.scryrs/hotspots.json` with the repository docs structure under `.devagent/docs/docs/`, synthesizes nodes and structural nav-hierarchy edges, converts hotspot `evidence.rowIds` into `EvidenceLink` entries, and writes a deterministic `KnowledgeGraphDocument` to both stdout and `.scryrs/graph.json`.

The implementation is scoped to the CLI crate, leaves `scryrs-graph` as a pure container/contract crate, and does not introduce route manifests, proposal generation, LLM inference, or docs mutation.

## What Changes

### Code

- **New module `crates/scryrs-cli/src/graph.rs`** — the `graph build` builder, mirroring the `hotspots.rs` structural pattern:
  - Accept a required `PATH` argument, resolve it to an absolute repo root.
  - Load `.scryrs/hotspots.json`; missing or unparseable hotspot input exits 2 with an explicit error.
  - Scan `.devagent/docs/docs/` for `.md`/`.mdx` files and load `_nav.json` for navigation hierarchy. Missing or empty docs directory emits a stderr warning but produces a hotspot-only graph (exit 0).
  - Convert every hotspot entry (all five subject kinds: `file`, `search`, `symbol`, `command`, `document`) into a `GraphNode` with id `{subjectKind}:{subject}`, label set to the subject, kind set to the subject kind, and an `EvidenceLink` with `sourceKind: "local_trace_row"` and `rowIds` from `evidence.rowIds`.
  - Create `doc_page` nodes for each discovered doc page. Node id is `doc_page:<slug>` where slug is the kebab-case page name derived from the nav link or filename.
  - Create `contains` edges from each nav group node to its child page nodes per `_nav.json` hierarchy.
  - Add a synthetic `docs_root` node if docs exist, connected via `contains` edges to top-level nav groups.
  - Assemble nodes and edges into a `KnowledgeGraph`, call `validate()`, then `to_document(None)`.
  - Write the document as a single-line JSON to stdout and as `.scryrs/graph.json`. Artifact write failure exits 1.
  - All iteration is explicitly sorted for deterministic output (filesystem traversal, nav entries, hotspot entries).

- **CLI dispatch updates in `crates/scryrs-cli/src/`**:
  - `dispatch.rs`: Add `"graph"` to the known-command allowlist guard; add a `graph` clap subcommand with required `PATH` argument and usage-error handling.
  - `help_text.rs`: Add `scryrs graph build <PATH>` entry to COMMANDS section.
  - `help_json.rs`: Add a `graph` command entry to the machine-readable surface document with argument, output, and exit code descriptions.
  - `lib.rs`: Register `mod graph;` and wire the module.

- **Test changes**:
  - `dispatch_tests.rs`: Remove `"graph"` from `previously_stubbed_commands_exit_2()`; add tests for: `graph build` with valid hotspot+docs (exit 0, output contract), missing hotspot (exit 2), invalid hotspot JSON (exit 2), missing PATH (exit 2), empty docs (exit 0 with warning), and repeated-output determinism (byte-identical JSON across two runs).
  - New inline `#[cfg(test)]` module in `graph.rs` for builder unit tests.
  - Update `insta` snapshots for `--help` and `--help-json` to include the new graph command.

### Specs

- **`specs/graph-build/spec.md`**: New capability specification for the graph build command — input contract, node/edge derivation rules, evidence preservation, output contract, determinism guarantees, and error behavior.
- **`specs/graph-contract/spec.md`**: Revise the "Contract foundation adds no graph build surface" scenario to clarify the scope boundary: `crates/scryrs-graph` the crate remains build-pipeline-free (no internal build pipeline, no CLI commands, no route-manifest generation), while the CLI command in `crates/scryrs-cli` is a separate consumer that assembles and materializes graph documents through the graph crate's public API.

### Docs

- `.devagent/docs/docs/graph.md`: Remove "`scryrs graph` CLI command" from the Deferred table; add it to the Shipped column; update the "No `scryrs graph build` command exists" statement to document the shipped command.
- `README.md`: Update command count from six to seven; add `scryrs graph build <PATH>` to the documented commands.
- `.devagent/docs/docs/roadmap.mdx`: Note that graph build from docs+hotspots is partially delivered.

## Impact

- **Affected crates**: `scryrs-cli` (new module, dispatch, help, tests). `scryrs-graph` and `scryrs-types` are unchanged — consumed as-is through their public API.
- **Feature gates**: The `graph` feature is already a default dependency. No new optional features required.
- **Backwards compatibility**: None broken. Previously `scryrs graph` was an unknown command (exit 2); now it routes to `graph build`. Existing known commands are unchanged.
- **Insta snapshots**: `--help` and `--help-json` test snapshots will need updating for the new command entry.
- **External API**: No server endpoints, no live mode, no network calls. Purely local, file-based operation.