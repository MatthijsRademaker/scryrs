## Why

The code quality backlog item targets maintainability debt concentrated in `crates/scryrs-cli/src/lib.rs`. At ~3335 lines, this single file mixes command dispatch, help text generation, help-json surface generation, `record` command implementation, `hotspots` command implementation, and five separate test modules (`tests`, `record_tests`, `smoke`, `hotspot_integration_tests`, `init_tests`). This directly violates the repository file-scope guidance (AGENTS.md rule 12: prefer small files with one clear responsibility, split before ~1000 lines).

A secondary hotspot exists in `crates/scryrs-types/src/lib.rs` (~1000 lines), which mixes trace-event wire contracts, hotspot-report types, and placeholder graph/proposal/route types in a single file. However, this crate is at the soft size limit rather than grossly exceeding it, and its internal boundaries are already clear. Per refinement consensus, the types crate is deferred to a follow-up pass to keep this change surgical and verifiable.

There is also concrete duplication: `CWD_GUARD` / `with_cwd` is defined identically in both `init_tests` (lib.rs:2841) and `tests/hotspot_e2e.rs:15`, creating two parallel copies that must be kept in sync manually.

## What Changes

### Primary: Structural split of `crates/scryrs-cli/src/lib.rs`

Production code (~580 lines) is separated into responsibility-focused sibling modules:

- **`dispatch.rs`** — `run`, `run_with_writers`, `run_with_io` entrypoints and the clap dispatch match block (lines 42-266)
- **`help_text.rs`** — `write_help` function producing the human-readable help text (lines 268-332)
- **`help_json.rs`** — `cli_surface_doc` and `write_cli_surface` for `--help-json` output (lines 738-829)
- **`hotspots.rs`** — `write_hotspots_json` and `write_empty_success_report` (lines 335-511), plus the non-core stub variant
- **`record.rs`** — `execute_record` and its non-core stub variant (lines 568-735)
- **`chrono.rs`** — `chrono_now` and `days_to_ymd` timestamp helpers (lines 512-547)
- **`store_override.rs`** — the thread-local store path override used by record tests (lines 17-37), extracted as `pub(crate)`
- **`lib.rs`** — reduced to a thin entrypoint (~50-100 lines) declaring modules and re-exporting the public API (`pub use dispatch::*`)

Test code (~2750 lines) is extracted to sibling test files declared with `#[cfg(test)]` or `#[cfg(all(test, feature = "core"))]` guards in `lib.rs`:

- **`dispatch_tests.rs`** — mod `tests` (lines 831-1205): dispatch/contract tests, insta snapshots for help/help-json
- **`record_tests.rs`** — mod `record_tests` (lines 1207-1853): stdin/file ingestion, rejection, CWD-based persistence
- **`smoke_tests.rs`** — mod `smoke` (lines 1854-1939): public entrypoint no-panic smoke
- **`hotspot_integration_tests.rs`** — mod `hotspot_integration_tests` (lines 1941-2832): typed `TraceEvent` fixture builders, store-populate-and-score tests
- **`init_tests.rs`** — mod `init_tests` (lines 2833-3335): CWD-scoped init tests with hook installation verification

### Secondary: Extract shared test-support helpers

- **`test_support.rs`** (or `tests/support/mod.rs`) — single canonical location for `CWD_GUARD` and `with_cwd`, used by both `init_tests` and `tests/hotspot_e2e.rs`
- The duplicated CWD_GUARD/with_cwd in `init_tests` and `hotspot_e2e.rs` is removed; both consumers import from the shared module
- Hotspot fixture builders remain separate: the typed `TraceEvent` struct builders in `hotspot_integration_tests` and the raw JSON string builders in `hotspot_e2e.rs` serve different pipeline stages (direct store population vs. CLI stdin ingestion) and should not be unified behind an intermediate abstraction (AGENTS.md Rule 8)

### Required infrastructure changes

- **Snapshot path migration**: Moving `tests` and `smoke` modules out of `lib.rs` breaks insta snapshot `source:` metadata in `src/snapshots/`. Snapshot files are relocated to `tests/snapshots/` alongside existing e2e snapshots, and test functions use `insta::with_settings!({snapshot_path => ...})` or rely on the default module-based snapshot naming to pick up the new paths.
- **`store_override` visibility**: Extracted to `src/store_override.rs` with `pub(crate)` visibility so `record_tests` can still access it via `crate::store_override::set()`.
- **Feature-gate preservation**: `record.rs`, `hotspots.rs`, `chrono.rs`, `store_override.rs` and their test counterparts carry `#[cfg(feature = "core")]` annotations as needed; test module declarations in `lib.rs` preserve `#[cfg(all(test, feature = "core"))]` gates.

### Explicitly deferred

- `crates/scryrs-types/src/lib.rs` split — deferred to a follow-up pass. The file is at the soft limit (~1000 lines) but not grossly exceeding it, and its internal trace-event/hotspot/placeholder boundaries are already clear. A separate focused change will handle the split with less cross-crate coordination risk.

## Impact

- **Affected source files**: `crates/scryrs-cli/src/lib.rs` (rewritten as thin entrypoint); 7 new production modules; 5 new test modules; 1 new test-support module; `crates/scryrs-cli/tests/hotspot_e2e.rs` (updated imports)
- **Affected snapshot files**: `crates/scryrs-cli/src/snapshots/` (relocated to `crates/scryrs-cli/tests/snapshots/`); snapshot `source:` metadata updated to reflect new module paths
- **Public API**: No change. `scryrs-cli` continues to export `run`, `run_with_writers`, `run_with_io` from `lib.rs`.
- **CLI contract**: No change. `scryrs --help`, `scryrs --help-json`, `scryrs record`, `scryrs hotspots`, and `scryrs init` produce identical output.
- **Risk profile**: Low. The change is purely structural — no function signatures, logic, or output contracts are modified. Risks are mechanical (snapshot paths, feature-gate propagation, store_override visibility) and addressed in the design.
- **Verification**: Full Docker-backed test suite (`scripts/test`) and optional full suite (`scripts/test --full`) must pass with identical test results.