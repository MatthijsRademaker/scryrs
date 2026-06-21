## 1. Extract production modules from lib.rs

- [ ] 1.1 Create `crates/scryrs-cli/src/dispatch.rs`: move `run`, `run_with_writers`, `run_with_io`, and the clap dispatch match block (lib.rs lines 42-266). Keep imports local to the module. Add `pub` visibility on the three entrypoints.
- [ ] 1.2 Create `crates/scryrs-cli/src/help_text.rs`: move `write_help` (lib.rs lines 268-332). Keep imports local.
- [ ] 1.3 Create `crates/scryrs-cli/src/help_json.rs`: move `cli_surface_doc`, `write_cli_surface`, and `SURFACE_VERSION` (lib.rs lines 738-829). Keep imports local.
- [ ] 1.4 Create `crates/scryrs-cli/src/hotspots.rs`: move `write_hotspots_json`, `write_empty_success_report`, and the `#[cfg(not(feature = "core"))]` stub variant (lib.rs lines 335-511, 549-566). Gate the file with `#[cfg(any(feature = "core", not(feature = "core")))]` to allow both variants to coexist, or use separate `#[cfg(feature = "core")]` and `#[cfg(not(feature = "core"))]` blocks.
- [ ] 1.5 Create `crates/scryrs-cli/src/record.rs`: move both `execute_record` variants — `#[cfg(feature = "core")]` (lib.rs lines 568-726) and `#[cfg(not(feature = "core"))]` (lib.rs lines 728-735). Handle feature-gate blocks appropriately.
- [ ] 1.6 Create `crates/scryrs-cli/src/chrono.rs`: move `chrono_now` and `days_to_ymd` (lib.rs lines 512-547). Gate the file with `#[cfg(feature = "core")]`.
- [ ] 1.7 Create `crates/scryrs-cli/src/store_override.rs`: move the `store_override` module (lib.rs lines 17-37) as a standalone `pub(crate)` module gated by `#[cfg(feature = "core")]`.
- [ ] 1.8 Rewrite `crates/scryrs-cli/src/lib.rs` as a thin entrypoint (~50-100 lines): declare all production and test modules, re-export `run`, `run_with_writers`, `run_with_io` from `dispatch`, keep `mod init` and `use` imports that are crate-wide.

## 2. Extract test modules from lib.rs

- [ ] 2.1 Create `crates/scryrs-cli/src/dispatch_tests.rs`: move `mod tests` (lib.rs lines 831-1205). Gate the file with `#[cfg(test)]`. Update `use super::*` imports to reference the new module structure. Add `mod dispatch_tests;` in lib.rs gated by `#[cfg(test)]`.
- [ ] 2.2 Create `crates/scryrs-cli/src/record_tests.rs`: move `mod record_tests` (lib.rs lines 1207-1853). Gate the file with `#[cfg(all(test, feature = "core"))]`. Update `super::store_override::set()` to `crate::store_override::set()`. Add `mod record_tests;` in lib.rs gated by `#[cfg(all(test, feature = "core"))]`.
- [ ] 2.3 Create `crates/scryrs-cli/src/smoke_tests.rs`: move `mod smoke` (lib.rs lines 1854-1939). Gate the file with `#[cfg(test)]`. Add `mod smoke_tests;` in lib.rs gated by `#[cfg(test)]`.
- [ ] 2.4 Create `crates/scryrs-cli/src/hotspot_integration_tests.rs`: move `mod hotspot_integration_tests` (lib.rs lines 1941-2832). Gate the file with `#[cfg(all(test, feature = "core"))]`. Add `mod hotspot_integration_tests;` in lib.rs gated by `#[cfg(all(test, feature = "core"))]`.
- [ ] 2.5 Create `crates/scryrs-cli/src/init_tests.rs`: move `mod init_tests` (lib.rs lines 2833-3335). Gate the file with `#[cfg(test)]`. Remove duplicated CWD_GUARD/with_cwd; import from `test_support` instead. Add `mod init_tests;` in lib.rs gated by `#[cfg(test)]`.

## 3. Extract shared test-support helpers

- [ ] 3.1 Create `crates/scryrs-cli/src/test_support.rs`: place canonical `CWD_GUARD` (static Mutex) and `with_cwd` function. Gate the file with `#[cfg(test)]`. Mark items `pub(crate)`. Declare `#[cfg(test)] mod test_support;` in lib.rs.
- [ ] 3.2 Update `crates/scryrs-cli/src/init_tests.rs`: remove inline CWD_GUARD/with_cwd; import `crate::test_support::{CWD_GUARD, with_cwd}`.
- [ ] 3.3 Update `crates/scryrs-cli/tests/hotspot_e2e.rs`: remove inline CWD_GUARD/with_cwd; import `scryrs_cli::test_support::{CWD_GUARD, with_cwd}`.

## 4. Handle insta snapshot migration

- [ ] 4.1 Move `crates/scryrs-cli/src/snapshots/scryrs_cli__tests__help_flag_prints_help_and_exits_0.snap` to `crates/scryrs-cli/tests/snapshots/`.
- [ ] 4.2 Move `crates/scryrs-cli/src/snapshots/scryrs_cli__tests__help_json_flag_outputs_valid_json_and_exits_0.snap` to `crates/scryrs-cli/tests/snapshots/`.
- [ ] 4.3 Update the `source:` metadata field in both relocated snapshot files to reflect the new module source path (e.g., `crates/scryrs-cli/src/dispatch_tests.rs`).
- [ ] 4.4 Add `insta::with_settings!({snapshot_path => "tests/snapshots"})` in `dispatch_tests.rs` for the help and help-json snapshot tests if needed, or verify insta auto-resolves the new path.
- [ ] 4.5 Delete the now-empty `crates/scryrs-cli/src/snapshots/` directory.
- [ ] 4.6 Run `cargo insta test --review` to accept any path-based diff in snapshot metadata.

## 5. Verify behavior preservation

- [ ] 5.1 Run `cargo test -p scryrs-cli` and confirm all tests pass with the new module structure.
- [ ] 5.2 Run `scripts/test` (Docker-backed) and confirm the full test suite passes.
- [ ] 5.3 Run `scripts/test --full` (if hook/CLI integration coverage is affected) and confirm all tests pass.
- [ ] 5.4 Manually verify `scryrs --help` produces identical output.
- [ ] 5.5 Manually verify `scryrs --help-json` produces identical output.
- [ ] 5.6 Confirm no file in `crates/scryrs-cli/src/` exceeds ~1000 lines (lib.rs should be ~50-100 lines; extracted modules should each be well under the limit).
- [ ] 5.7 Confirm no changes to `crates/scryrs-types/` or any crate other than `scryrs-cli`.

## 6. Create follow-up task for types crate

- [ ] 6.1 Document the deferred `crates/scryrs-types/src/lib.rs` split as a follow-up task on the swarm board, referencing this change's completion and the types crate structure analysis.