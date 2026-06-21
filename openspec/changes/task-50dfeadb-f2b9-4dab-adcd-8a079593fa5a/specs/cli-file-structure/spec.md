## ADDED Requirements

### Requirement: CLI crate lib.rs is a thin entrypoint

`crates/scryrs-cli/src/lib.rs` SHALL be reduced to a thin entrypoint file that declares modules and re-exports the public API, with no production command implementations or test modules inlined.

#### Scenario: lib.rs is under 100 lines
- **WHEN** the refactor is complete
- **THEN** `crates/scryrs-cli/src/lib.rs` contains only module declarations (`mod dispatch; mod help_text;` etc.), crate-wide `use` imports, and public re-exports (`pub use dispatch::run;` etc.)
- **AND** the file does not exceed ~100 lines

#### Scenario: Public API is preserved
- **WHEN** external consumers import from `scryrs_cli`
- **THEN** `scryrs_cli::run`, `scryrs_cli::run_with_writers`, and `scryrs_cli::run_with_io` remain directly importable
- **AND** the `main.rs` binary entrypoint compiles without changes

### Requirement: Production code is separated into responsibility-focused modules

Production CLI logic SHALL be split into dedicated modules in `crates/scryrs-cli/src/`, each with a single clear responsibility.

#### Scenario: Dispatch module exists
- **WHEN** the refactor is complete
- **THEN** `src/dispatch.rs` contains the `run`, `run_with_writers`, `run_with_io` entrypoints and the clap dispatch match block

#### Scenario: Help text module exists
- **WHEN** the refactor is complete
- **THEN** `src/help_text.rs` contains the `write_help` function producing human-readable help output

#### Scenario: Help JSON module exists
- **WHEN** the refactor is complete
- **THEN** `src/help_json.rs` contains `cli_surface_doc`, `write_cli_surface`, and the `SURFACE_VERSION` constant

#### Scenario: Hotspots module exists
- **WHEN** the refactor is complete
- **THEN** `src/hotspots.rs` contains `write_hotspots_json` and `write_empty_success_report`, including both `#[cfg(feature = "core")]` and `#[cfg(not(feature = "core"))]` variants

#### Scenario: Record module exists
- **WHEN** the refactor is complete
- **THEN** `src/record.rs` contains both `execute_record` variants (`#[cfg(feature = "core")]` and `#[cfg(not(feature = "core"))]`)

#### Scenario: Chrono module exists
- **WHEN** the refactor is complete
- **THEN** `src/chrono.rs` contains `chrono_now` and `days_to_ymd`, gated by `#[cfg(feature = "core")]`

#### Scenario: Store override module exists
- **WHEN** the refactor is complete
- **THEN** `src/store_override.rs` contains the thread-local store path override with `pub(crate)` visibility, gated by `#[cfg(feature = "core")]`

#### Scenario: No source file exceeds line-count guideline
- **WHEN** the refactor is complete
- **THEN** no file in `crates/scryrs-cli/src/` exceeds ~1000 lines

### Requirement: Test modules are extracted to separate files

All test code currently inline in `lib.rs` SHALL be moved to dedicated test files under `src/`, declared as submodules in `lib.rs` with appropriate `#[cfg(test)]` gates.

#### Scenario: Dispatch tests are separate
- **WHEN** the refactor is complete
- **THEN** `src/dispatch_tests.rs` contains all tests from the former `mod tests` block
- **AND** it is declared as `#[cfg(test)] mod dispatch_tests;` in `lib.rs`
- **AND** the insta snapshot tests for `--help` and `--help-json` continue to pass

#### Scenario: Record tests are separate
- **WHEN** the refactor is complete
- **THEN** `src/record_tests.rs` contains all tests from the former `mod record_tests` block
- **AND** it is declared as `#[cfg(all(test, feature = "core"))] mod record_tests;` in `lib.rs`
- **AND** it uses `crate::store_override::set()` instead of `super::store_override::set()`

#### Scenario: Smoke tests are separate
- **WHEN** the refactor is complete
- **THEN** `src/smoke_tests.rs` contains all tests from the former `mod smoke` block
- **AND** all smoke tests continue to pass without process termination

#### Scenario: Hotspot integration tests are separate
- **WHEN** the refactor is complete
- **THEN** `src/hotspot_integration_tests.rs` contains all tests from the former `mod hotspot_integration_tests` block
- **AND** the typed `TraceEvent` fixture builders remain in this file (not unified with JSON string builders)
- **AND** it is declared as `#[cfg(all(test, feature = "core"))] mod hotspot_integration_tests;` in `lib.rs`

#### Scenario: Init tests are separate
- **WHEN** the refactor is complete
- **THEN** `src/init_tests.rs` contains all tests from the former `mod init_tests` block
- **AND** it imports `CWD_GUARD` and `with_cwd` from `crate::test_support` instead of defining them inline

### Requirement: Shared CWD guard is extracted to a single test-support module

The duplicated `CWD_GUARD` / `with_cwd` helpers SHALL be unified in a single `test_support` module accessible to both unit tests and integration tests.

#### Scenario: Test support module exists
- **WHEN** the refactor is complete
- **THEN** `src/test_support.rs` contains a `CWD_GUARD` static Mutex and `with_cwd` helper function, both marked `pub(crate)` and gated by `#[cfg(test)]`

#### Scenario: Init tests use shared CWD guard
- **WHEN** the refactor is complete
- **THEN** `init_tests.rs` imports `CWD_GUARD` and `with_cwd` from `crate::test_support`
- **AND** the inline `CWD_GUARD` definition is removed from `init_tests`

#### Scenario: E2E hotspot tests use shared CWD guard
- **WHEN** the refactor is complete
- **THEN** `tests/hotspot_e2e.rs` imports `CWD_GUARD` and `with_cwd` from `scryrs_cli::test_support`
- **AND** the inline `CWD_GUARD` definition is removed from `hotspot_e2e.rs`

### Requirement: CLI contract output is preserved

All externally observable CLI behavior SHALL remain identical after the refactor.

#### Scenario: Help output unchanged
- **WHEN** `scryrs --help` or `scryrs -h` is invoked after the refactor
- **THEN** the output is byte-identical to the pre-refactor output

#### Scenario: Help-json output unchanged
- **WHEN** `scryrs --help-json` or `scryrs -hj` is invoked after the refactor
- **THEN** the output is byte-identical to the pre-refactor output

#### Scenario: Record output unchanged
- **WHEN** `scryrs record --stdin` or `scryrs record --file <PATH>` is invoked after the refactor
- **THEN** the stdout summary, stderr diagnostics, and exit codes are identical to pre-refactor behavior

#### Scenario: Hotspots output unchanged
- **WHEN** `scryrs hotspots <PATH>` is invoked after the refactor
- **THEN** the stdout JSON report, stderr errors, artifact file (`.scryrs/hotspots.json`), and exit codes are identical to pre-refactor behavior

#### Scenario: Init output unchanged
- **WHEN** `scryrs init --agent <NAME>` is invoked after the refactor
- **THEN** the stdout instructions, stderr errors, hook file artifacts, and exit codes are identical to pre-refactor behavior

#### Scenario: Docker-backed test suite passes
- **WHEN** `scripts/test` is executed after the refactor
- **THEN** all tests pass with the same results as before the refactor
