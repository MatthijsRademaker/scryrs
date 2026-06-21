## 1. Extract production modules from lib.rs

- [x] 1.1 Create `crates/scryrs-cli/src/dispatch.rs`: move `run`, `run_with_writers`, `run_with_io`, and the clap dispatch match block (lib.rs lines 42-266). Keep imports local to the module. Add `pub` visibility on the three entrypoints.
- [x] 1.2 Create `crates/scryrs-cli/src/help_text.rs`: move `write_help` (lib.rs lines 268-332). Keep imports local.
- [x] 1.3 Create `crates/scryrs-cli/src/help_json.rs`: move `cli_surface_doc`, `write_cli_surface`, and `SURFACE_VERSION` (lib.rs lines 738-829). Keep imports local.
- [x] 1.4 Create `crates/scryrs-cli/src/hotspots.rs`: move `write_hotspots_json`, `write_empty_success_report`, and the `#[cfg(not(feature = "core"))]` stub variant (lib.rs lines 335-511, 549-566). Gate the file with `#[cfg(any(feature = "core", not(feature = "core")))]` to allow both variants to coexist, or use separate `#[cfg(feature = "core")]` and `#[cfg(not(feature = "core"))]` blocks.
- [x] 1.5 Create `crates/scryrs-cli/src/record.rs`: move both `execute_record` variants — `#[cfg(feature = "core")]` (lib.rs lines 568-726) and `#[cfg(not(feature = "core"))]` (lib.rs lines 728-735). Handle feature-gate blocks appropriately.
- [x] 1.6 Create `crates/scryrs-cli/src/chrono.rs`: move `chrono_now` and `days_to_ymd` (lib.rs lines 512-547). Gate the file with `#[cfg(feature = "core")]`.
- [x] 1.7 Create `crates/scryrs-cli/src/store_override.rs`: move the `store_override` module (lib.rs lines 17-37) as a standalone `pub(crate)` module gated by `#[cfg(feature = "core")]`.
- [x] 1.8 Rewrite `crates/scryrs-cli/src/lib.rs` as a thin entrypoint (~50-100 lines): declare all production and test modules, re-export `run`, `run_with_writers`, `run_with_io` from `dispatch`, keep `mod init` and `use` imports that are crate-wide.

## 2. Extract test modules from lib.rs

- [x] 2.1 Create `crates/scryrs-cli/src/dispatch_tests.rs`: move `mod tests` (lib.rs lines 831-1205). Gate the file with `#[cfg(test)]`. Update `use super::*` imports to reference the new module structure. Add `mod dispatch_tests;` in lib.rs gated by `#[cfg(test)]`.
- [x] 2.2 Create `crates/scryrs-cli/src/record_tests.rs`: move `mod record_tests` (lib.rs lines 1207-1853). Gate the file with `#[cfg(all(test, feature = "core"))]`. Update `super::store_override::set()` to `crate::store_override::set()`. Add `mod record_tests;` in lib.rs gated by `#[cfg(all(test, feature = "core"))]`.
- [x] 2.3 Create `crates/scryrs-cli/src/smoke_tests.rs`: move `mod smoke` (lib.rs lines 1854-1939). Gate the file with `#[cfg(test)]`. Add `mod smoke_tests;` in lib.rs gated by `#[cfg(test)]`.
- [x] 2.4 Create `crates/scryrs-cli/src/hotspot_integration_tests.rs`: move `mod hotspot_integration_tests` (lib.rs lines 1941-2832). Gate the file with `#[cfg(all(test, feature = "core"))]`. Add `mod hotspot_integration_tests;` in lib.rs gated by `#[cfg(all(test, feature = "core"))]`.
- [x] 2.5 Create `crates/scryrs-cli/src/init_tests.rs`: move `mod init_tests` (lib.rs lines 2833-3335). Gate the file with `#[cfg(test)]`. Remove duplicated CWD_GUARD/with_cwd; import from `test_support` instead. Add `mod init_tests;` in lib.rs gated by `#[cfg(test)]`.

## 3. Extract shared test-support helpers

- [x] 3.1 Create `crates/scryrs-cli/src/test_support.rs`: place canonical `CWD_GUARD` (static Mutex) and `with_cwd` function. Declared `pub` (not `pub(crate)`) to allow access from integration tests. Declare `pub mod test_support;` in lib.rs (without `#[cfg(test)]` gate, needed for integration test visibility).
- [x] 3.2 Update `crates/scryrs-cli/src/init_tests.rs`: remove inline CWD_GUARD/with_cwd; import `crate::test_support::with_cwd`.
- [x] 3.3 Update `crates/scryrs-cli/tests/hotspot_e2e.rs`: remove inline CWD_GUARD/with_cwd; import `scryrs_cli::test_support::with_cwd`.

## 4. Handle insta snapshot migration

- [x] 4.1 Moved snapshot files to `crates/scryrs-cli/src/snapshots/` (unit test default) with module-renamed filenames to match new `dispatch_tests` module path.
- [x] 4.2 See 4.1 — both snapshots relocated together to `src/snapshots/`.
- [x] 4.3 Updated `source:` metadata in both snapshots from `crates/scryrs-cli/src/lib.rs` to `crates/scryrs-cli/src/dispatch_tests.rs`.
- [x] 4.4 Insta auto-resolves the new path from `src/snapshots/` (unit test default). No `with_settings!` needed.
- [x] 4.5 Deleted the old empty `crates/scryrs-cli/src/snapshots/` directory (removed during initial relocation, recreated for new snapshots).
- [x] 4.6 Snapshot tests pass with matching content — no diffs to accept.

## 5. Verify behavior preservation

- [x] 5.1 `cargo test -p scryrs-cli` — all 85 lib tests + 3 integration tests pass (via Docker-backed `scripts/test`).
- [x] 5.2 `scripts/test` (Docker-backed) — full test suite passes (85 scryrs-cli lib tests + all other crate tests).
- [x] 5.3 `scripts/test --full` — hook verification passes (35/35 tests).
- [x] 5.4 `scryrs --help` — verified via insta snapshot assertion (byte-identical).
- [x] 5.5 `scryrs --help-json` — verified via insta snapshot assertion (byte-identical).
- [x] 5.6 No file exceeds ~1000 lines. lib.rs: 29 lines. Largest file: hotspot_integration_tests.rs at 888 lines.
- [x] 5.7 No changes to `crates/scryrs-types/` or any other crate. Only `scryrs-cli` files modified.

## 6. Create follow-up task for types crate

- [x] 6.1 Document the deferred `crates/scryrs-types/src/lib.rs` split as a follow-up task on the swarm board (swarm task `e2076309`), referencing this change's completion and the types crate structure analysis.
