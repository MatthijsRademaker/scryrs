# cli-foundation-closure Specification

## Purpose
TBD - created by archiving change task-93754ce0-28cf-4a1b-b755-909aa7018c19. Update Purpose after archive.

## RECONCILIATION — Phase 2 Hotspot Materialization (2026-06-21)

The Phase 2 hotspot materialization has been fully implemented and delivered. The following requirement is **superseded** by the Phase 2 contractual specs:

- **Superseded**: Requirement "Single placeholder command operates correctly" and its scenarios asserting that `write_hotspots_json()` emits a placeholder JSON envelope `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}` and that the function has "no backend wiring" — these asserted that the hotspots command was a string-literal stub with no engine calls. The live implementation in `crates/scryrs-cli/src/lib.rs` now opens `.scryrs/scryrs.db` via `TraceQuery`, runs the full scoring engine in `crates/scryrs-core/src/scoring.rs`, and emits a real `HotspotsReport` (schemaVersion 1.0.0) with runMetadata, entries, evidence, and a `.scryrs/hotspots.json` artifact file.

**Canonical Phase 2 contract** (supersedes the above):
- [`openspec/specs/hotspot-report/spec.md`](../hotspot-report/spec.md) — `HotspotsReport` schema, scoring dimensions, exit codes, artifact file.
- Live implementation in `crates/scryrs-cli/src/lib.rs` (function `write_hotspots_json`) and `crates/scryrs-core/src/scoring.rs` (function `score_hotspots`).

**Closure change traceability**: `openspec/changes/task-56573ced-fdeb-49b2-aea6-41b30f19d2bf/specs/phase-2-closure/spec.md` documents the full evidence matrix mapping code/test artifacts to Phase 2 deliverables.

All other requirements in this spec remain valid for CLI foundation closure and are not affected by this reconciliation.
## Requirements
### Requirement: Binary target exists and is buildable

The native Rust binary target `scryrs` SHALL exist at `crates/scryrs-cli` and SHALL be the public entrypoint for the v0 CLI surface.

#### Scenario: Binary target is defined
- **GIVEN** the repository at the current baseline
- **WHEN** inspecting `crates/scryrs-cli/Cargo.toml`
- **THEN** a `[[bin]]` section exists with `name = "scryrs"` and `path = "src/main.rs"`

#### Scenario: Binary entrypoint delegates to library
- **GIVEN** the binary entrypoint at `crates/scryrs-cli/src/main.rs`
- **WHEN** the binary is invoked
- **THEN** execution delegates to `scryrs_cli::run(std::env::args().skip(1))`
- **AND** the process exits with the returned status code

### Requirement: Single placeholder command operates correctly

The `scryrs hotspots <PATH>` command SHALL emit a deterministic versioned JSON envelope and exit 0. All behavioral scenarios are defined in the prior change spec at `openspec/changes/task-9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486/specs/cli-v0-contract/spec.md`.

#### Scenario: Placeholder command emits JSON and exits 0
- **GIVEN** the CLI library at `crates/scryrs-cli/src/lib.rs`
- **WHEN** `run_with_writers(["hotspots", "/tmp"], ...)` is invoked
- **THEN** the `write_hotspots_json()` function emits `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}` to stdout
- **AND** the exit code is 0
- **AND** the unit test `hotspots_with_path_emits_json_and_exits_0` passes

#### Scenario: Placeholder command has no backend wiring
- **GIVEN** the `write_hotspots_json()` function in `crates/scryrs-cli/src/lib.rs`
- **WHEN** the function is called
- **THEN** it writes a string literal only
- **AND** no `scryrs-core`, `scryrs-llm`, or any engine crate function is called

### Requirement: Unsupported invocations fail loudly

All unsupported invocations SHALL produce a non-zero exit code with a descriptive error message on stderr. Full behavioral scenarios are defined in the prior change spec at `openspec/changes/task-9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486/specs/cli-v0-contract/spec.md`.

#### Scenario: Unknown command exits 2
- **GIVEN** any command name not matching `hotspots`, `--help`, `-h`, `--version`, or `-V`
- **WHEN** `run_with_writers` is invoked with that command
- **THEN** the process exits with code 2
- **AND** `unknown command: <name>` is written to stderr
- **AND** the unit tests `unknown_command_exits_2_with_error`, `components_command_exits_2`, and `previously_stubbed_commands_exit_2` all pass

#### Scenario: Missing PATH exits 2
- **GIVEN** `hotspots` is invoked without a PATH argument
- **WHEN** `run_with_writers(["hotspots"], ...)` is invoked
- **THEN** the process exits with code 2
- **AND** `error: missing required PATH argument` is written to stderr
- **AND** the unit test `hotspots_without_path_exits_2_with_error` passes

#### Scenario: Extra arguments exit 2
- **GIVEN** `hotspots` is invoked with more than one argument after PATH
- **WHEN** `run_with_writers(["hotspots", "path", "extra"], ...)` is invoked
- **THEN** the process exits with code 2
- **AND** `error: unexpected argument after PATH` is written to stderr
- **AND** the unit test `hotspots_with_extra_args_exits_2_with_error` passes

### Requirement: Help, version, and bare invocation behave correctly

Global flags and bare invocation SHALL produce appropriate output and exit 0. Full behavioral scenarios are defined in the prior change spec at `openspec/changes/task-9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486/specs/cli-v0-contract/spec.md`.

#### Scenario: Help flag exits 0
- **WHEN** `--help` or `-h` is passed
- **THEN** help text listing `scryrs hotspots <PATH>` is written to stdout
- **AND** the process exits with code 0
- **AND** the unit tests `help_flag_prints_help_and_exits_0` and `short_help_flag_prints_help_and_exits_0` pass

#### Scenario: Version flag exits 0
- **WHEN** `--version` or `-V` is passed
- **THEN** the version string is written to stdout
- **AND** the process exits with code 0
- **AND** the unit tests `version_flag_prints_version_and_exits_0` and `short_version_flag_prints_version_and_exits_0` pass

#### Scenario: Bare invocation exits 0
- **WHEN** `scryrs` is invoked with no arguments
- **THEN** help text is written to stdout
- **AND** the process exits with code 0
- **AND** the unit test `bare_invocation_prints_help_and_exits_0` passes

### Requirement: No reachable behavior beyond stub path

No backend wiring, hotspot engine execution, indexing, LLM calls, or hidden fallback command surface SHALL be reachable through the public binary entrypoint.

#### Scenario: All match arms exhaust known entry paths
- **GIVEN** the `run_with_writers` match expression in `crates/scryrs-cli/src/lib.rs`
- **WHEN** any argument vector is provided
- **THEN** execution reaches only `write_help`, version output, `write_hotspots_json`, or an error branch
- **AND** no `unsafe` blocks, `std::process::Command` invocations, or external HTTP calls are used

#### Scenario: Backend dependencies are feature-gated
- **GIVEN** `crates/scryrs-cli/Cargo.toml`
- **WHEN** the default features are enabled
- **THEN** all backend crate dependencies (`scryrs-core`, `scryrs-graph`, `scryrs-curator`, `scryrs-llm`, etc.) are declared as `optional = true`
- **AND** the stub path above is satisfied without any backend crate being invoked

### Requirement: Closure record is traceable to prior implementation

The OpenSpec artifacts for this task SHALL cite the prior change (task-9b98b3fd) as the implementation vehicle and SHALL cross-reference its evidence.

#### Scenario: Proposal cites prior change
- **GIVEN** `openspec/changes/task-93754ce0-28cf-4a1b-b755-909aa7018c19/proposal.md`
- **WHEN** a reader reviews the closure record
- **THEN** the proposal cites `task-9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486` as the implementation change
- **AND** the proposal cites specific code, test, and doc files as evidence

#### Scenario: Residual drift is tracked separately
- **GIVEN** the closure record
- **WHEN** a reader reviews outstanding work items
- **THEN** the stale `cargo run -p scryrs-cli -- components` examples in `.devagent/docs/docs/architecture.mdx` are acknowledged
- **AND** the record states these are not absorbed into this task

