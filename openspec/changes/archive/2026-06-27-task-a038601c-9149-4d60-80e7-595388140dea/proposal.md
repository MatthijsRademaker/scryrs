## Why

scryrs now emits deterministic knowledge graph artifacts at `.scryrs/graph.json`, containing hotspot-backed subject nodes and doc-page nodes with structural `contains` edges. However, the system lacks a machine-readable route manifest artifact. This blocks future runtimes from preloading relevant context without re-discovering subjects through full graph traversal.

The task is scoped as an evidence-backed export layer over existing graph facts — not a semantic router. Every emitted route must trace back to explicit graph node/edge identity and/or underlying hotspot/doc evidence. The route manifest must preserve `(subjectKind, subject)` identity boundaries by default; higher-level grouping is only emitted when explicit graph structure (specifically `contains` edges) justifies it.

## What Changes

- **`crates/scryrs-types`**: Add `ROUTE_SCHEMA_VERSION = "1.0.0"` constant and define `RouteManifestDocument`, `RouteMetadata`, and `RouteEntry` types with camelCase wire serialization. Reuse existing `EvidenceLink`, `EvidenceSourceKind`, and `GraphMetadata` for provenance. Remove the `RouteHint` placeholder struct.
- **`crates/scryrs-cli/src/route.rs`** (new): Implement route manifest generator that reads `.scryrs/graph.json` as input, walks graph nodes/edges deterministically, emits one `RouteEntry` per graph node with evidence backlinks, applies grouping from explicit `contains` edges, and writes `RouteManifestDocument` JSON to stdout and `.scryrs/routes.json`.
- **`crates/scryrs-cli/src/dispatch.rs`**: Register `scryrs route <PATH>` as a live subcommand (like `graph`), add it to the known-command allowlist, add subcommand definition with required PATH argument, add dispatch arm routing to the route generator, add `MissingRequiredArgument` and `UnknownArgument` error handlers.
- **`crates/scryrs-cli/src/help_text.rs`**: Add `scryrs route <PATH>` entry to the help output.
- **`crates/scryrs-cli/src/help_json.rs`**: Add `route` entry to the `--help-json` surface document with argument specs and output contract.
- **`crates/scryrs-cli/src/dispatch_tests.rs`**: Remove `"route"` from the `previously_stubbed_commands_exit_2` array. Add tests for: missing/malformed graph input (exit 2), route manifest determinism (byte-identical reruns), identity-boundary preservation (`file:auth`/`search:auth`/`symbol:auth` non-collapse), and artifact file write.
- **`README.md`**: Add `scryrs route <PATH>` to the public command list; remove `route` from the unknown-command documentation.
- **`crates/scryrs-graph`**: Unchanged — the graph crate remains a pure container/contract layer.

## Impact

- **Affected crates**: `scryrs-types`, `scryrs-cli`
- **Unaffected crates**: `scryrs-graph` (explicitly excluded by graph-contract spec)
- **Feature gate**: Route manifest generation is gated behind the existing `graph` feature (which `route.rs` depends on for reading graph artifacts)
- **Schema compatibility**: `ROUTE_SCHEMA_VERSION = "1.0.0"` is independently versioned, no backward-compat burden on existing graph/hotspot/trace contracts
- **CLI breaking change**: `route` moves from stubbed (exit 2) to live command; any script that previously expected `scryrs route` to exit 2 must be updated
- **Deterministic contract**: Route manifests follow the same byte-stable, no-timestamp, sorted-output contract as graph manifests