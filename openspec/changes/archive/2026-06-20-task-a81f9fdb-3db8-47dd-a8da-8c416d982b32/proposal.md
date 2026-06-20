## Why

The v0 CLI contract (`scryrs hotspots <PATH>`, help text, `--help-json` surface document, exit codes) is frozen and implemented, but the test suite relies entirely on substring assertions — it detects when a keyword like `USAGE` or `EXAMPLES` is missing but cannot detect silent drift in formatting, wording, ordering, whitespace, or structural completeness. An accidental rewrite of the help text that preserves those keywords passes the suite. A change that drops one field from `--help-json` while keeping the field names string-asserted also passes. The contract is only as frozen as its tests are precise. Now — while the surface is still single-command and stable — is the last moment to lock it with exact-match golden tests before multi-command expansion makes the snapshot base larger and the update process more costly.

## What Changes

1. **Add golden/snapshot tests for `--help` output** — replace substring assertions with exact output matching against a committed snapshot (help text is the primary human-facing contract; every line break, section heading, and example matters).
2. **Add golden/snapshot tests for `--help-json` output** — replace substring structural checks with exact JSON matching against a committed snapshot (the surface document IS the machine-readable contract; field names, values, ordering, and nesting all matter).
3. **Add golden/snapshot tests for `hotspots <PATH>` output** — the single-line JSON envelope is low-risk but a snapshot here keeps the pattern consistent across all CLI outputs.
4. **Add process-level smoke tests** — test the public `run()` entrypoint (which wires `std::env::args()` to `run_with_writers`), catching arg-collection wiring issues that the library-level tests cannot. These tests verify exit codes from the real entrypoint, not just the internal writer function.
5. **Document the local check path** — add a test-running section to the CLI contract design note (`cli-v0-contract.md`) or a contributing-notes file, describing how to run tests, view snapshots, and update them when the contract intentionally changes.
6. **Install snapshot testing infrastructure** — add `insta` (preferred) or hand-rolled golden file helpers. If `insta`, add `cargo-insta` installation note to the local check docs.

## Capabilities

### New Capabilities

- `cli-golden-tests`: Exact-match snapshot verification of CLI output (help text, `--help-json` surface document, `hotspots` placeholder JSON). Snapshot files are committed and reviewed alongside contract changes.
- `cli-smoke-checks`: Process-level smoke tests exercising the public `run()` entrypoint, verifying exit code propagation and arg wiring from env-args to the writer-based logic.

### Modified Capabilities
<!-- No existing capability changes requirements — this is purely a testing and documentation change. -->

## Impact

- **Code changes**: `crates/scryrs-cli/src/lib.rs` only — replace substring assertions in existing tests with snapshot assertions; add `run()` entrypoint smoke tests.
- **Dependency addition**: `insta` dev-dependency in `crates/scryrs-cli/Cargo.toml` (if insta approach chosen).
- **New files**: Snapshot files under `crates/scryrs-cli/src/snapshots/` (if insta) or golden files under `crates/scryrs-cli/tests/` (if hand-rolled).
- **Docs change**: `.devagent/docs/docs/cli-v0-contract.md` updated with local test check documentation.
- **No contract changes**: CLI output, exit codes, error messages, and all existing behavior remain identical. This is a test-only and docs-only change.
- **No engine crate changes**: `scryrs-types`, `scryrs-core`, and all other workspace crates untouched.
