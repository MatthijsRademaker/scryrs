# Design: Hotspot Artifact and E2E Verification

## Context

The `scryrs hotspots <PATH>` command already produces stdout and `.scryrs/hotspots.json` artifact output through `write_hotspots_json()` and `write_empty_success_report()` in `crates/scryrs-cli/src/lib.rs:335-507`. The `scryrs record --stdin` command persists accepted events to `.scryrs/scryrs.db` via `execute_record()` at `lib.rs:646-688`. Both use the canonical `.scryrs/scryrs.db` path resolved relative to the repository working directory.

However, the integration test suite has a critical gap: all existing hotspot tests (`lib.rs:1978-2503`) seed SQLite directly through `populate_store()` which opens `EventStore` directly — bypassing the `record` CLI entirely. All existing record tests use the private `store_override` helper (`lib.rs:16-31`) to redirect writes to a temp directory — but `hotspots` never reads that override because `write_hotspots_json` resolves `.scryrs/scryrs.db` relative to its `<PATH>` argument. The two halves of the pipeline are tested in isolation but never together.

Additionally, no `insta` snapshots exist for any hotspot output. The `crates/scryrs-cli/src/snapshots/` directory contains only `help_flag_prints_help_and_exits_0.snap` and `help_json_flag_outputs_valid_json_and_exits_0.snap`. The existing `insta` crate with JSON support (configured in `Cargo.toml:44-46`) is the project's preferred drift-detection mechanism.

Existing hotspot test fixtures use only `FileOpened` events (`make_file_opened` at `lib.rs:1948-1958`). The 7 subject-bearing event families defined in `scryrs_types::TraceEventPayload` (file, search, symbol, command, document, edit, failed lookup) and their associated scoring weights are covered by unit tests in `crates/scryrs-core/src/scoring.rs` but not through the CLI integration surface.

The change is pure verification hardening: add tests that prove what the production code already does, with zero production code changes.

## Goals / Non-Goals

### Goals

1. **Prove the public CLI pipeline end-to-end**: `scryrs record --stdin → .scryrs/scryrs.db → scryrs hotspots <PATH> → .scryrs/hotspots.json`
2. **Catch contract drift**: `insta` snapshot assertions for normalized hotspot stdout and artifact JSON so accidental changes to the output contract fail CI
3. **Cover all 7 subject-bearing event families plus failure**: prove that scoring, ranking, and evidence tracking work correctly for `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, `FailedLookup`, and `Outcome::Failure` events through the CLI
4. **Verify missing/empty store behavior**: retain existing targeted tests for missing-store exit codes and empty-store outputs; ensure the E2E test does not regress them
5. **Zero production code changes**: all changes are additive test code

### Non-Goals

- Changing hotspot scoring weights, ranking rules, or schema versions
- Adding new persisted artifact types beyond `.scryrs/hotspots.json`
- Changing `scryrs record` to accept new CLI arguments (e.g., a repository path)
- Adding graph, proposal, adapter, runtime, or LLM behavior
- Refactoring existing production code or test infrastructure
- Replacing existing targeted error-path tests

## Decisions

### D1: Separate integration test binary at `crates/scryrs-cli/tests/hotspot_e2e.rs`

**Choice**: Add a new `tests/hotspot_e2e.rs` integration test binary rather than adding E2E tests inside `src/lib.rs`.

**Rationale**:
- `src/lib.rs` is already ~3100 lines; adding E2E test code would further bloat the file
- The separate binary avoids interaction with `store_override` state, which is thread-local and scoped to `src/lib.rs` test modules
- Follows Rust convention (`tests/` directory) for integration tests that exercise the public CLI
- The separate binary must implement its own CWD serialization (simple `Mutex`-guarded pattern, ~15 lines, proven in `init_tests`)

**Alternatives considered**:
- Adding to `hotspot_integration_tests` in `lib.rs` (architect's recommendation): simpler to access `run_with_writers`/`run_with_io`, but grows an already-large file and risks interaction with `store_override`
- Creating `tests/common/mod.rs` for shared helpers: adds a module for only one consumer; the E2E binary can self-contain its helpers

### D2: Programmatic multi-event fixture (not checked-in JSONL)

**Choice**: Build the multi-event fixture in Rust test code using the same helper-pattern as `make_file_opened` rather than checking in `.jsonl` fixture files.

**Rationale**:
- Stays in sync with `TraceEvent`/`TraceEventPayload` types at compile time — if a type changes, the fixture code fails to compile rather than producing silent false passes
- Self-contained: no external asset management, no path resolution across `tests/` and `fixtures/`
- Consistent with existing pattern: all current test fixtures are programmatic (e.g., `make_file_opened`, `make_valid_event_json`)

### D3: Volatile-field normalization via test-side helper function

**Choice**: Add a `normalize_hotspot_json(json: &str) -> String` helper in the test code that parses the JSON, replaces `generatedAt` with `"<GENERATED_AT>"`, `repositoryPath` with `"<REPO>"`, `storePath` with `"<STORE>"`, then re-serializes before passing to `insta::assert_snapshot!` and `insta::assert_json_snapshot!`.

**Rationale**:
- Volatile fields (`generatedAt` is a wall-clock timestamp, `repositoryPath` and `storePath` are absolute paths) differ on every test run and would break snapshot comparisons
- Test-side normalization keeps the normalization logic close to the tests that need it, without adding `#[cfg(test)]` seams in production code
- The normalization is simple string replacement on parsed JSON values — not a schema-level change

**Alternatives considered**:
- `insta` dynamic redaction: possible but ties redaction to `insta` internals; a standalone function is test-framework-agnostic
- `#[cfg(test)]` seam in `chrono_now()` and path formatting: would require production-code changes, violating the zero-code-change constraint

### D4: Extend existing `hotspot_integration_tests` with full fixture for inline assertions

**Choice**: In addition to the new E2E binary, extend the existing `hotspot_integration_tests` in `lib.rs` with a new test that includes events from all 7 subject-bearing families plus a failure case, and asserts expected scores, ranking order, and evidence fields.

**Rationale**:
- The existing `hotspot_integration_tests` module has fast, narrow tests that exercise `populate_store` (bypassing `record`) — these complement the E2E binary by providing focused coverage
- The reviewer's accepted decision explicitly recommends extending these tests with the full fixture
- Adding the full fixture to existing tests proves the scoring contract with inline assertions, while the E2E binary proves the pipeline with snapshots

### D5: CWD serialization via per-binary `Mutex`

**Choice**: The E2E binary defines its own `static CWD_GUARD: Mutex<()>` and `with_cwd` function, duplicating the proven pattern from `init_tests` at `lib.rs:2595-2618`.

**Rationale**:
- `init_tests::with_cwd` is private to that module — a separate binary cannot access it
- A `tests/common/mod.rs` shared helper module would be the "right" factoring but is only needed by one binary; duplication of ~15 lines is acceptable
- The pattern is well-understood and proven to prevent `std::env::set_current_dir` races in parallel test execution

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| CWD serialization race in parallel test execution | Low | Tests flake with spurious failures | Duplicate the proven `Mutex` + `with_cwd` pattern; Rust test runner serializes `tests/*.rs` binaries |
| Snapshot volatility from incomplete normalization | Low | Snapshots fail on every CI run | Normalize all known volatile fields (`generatedAt`, `repositoryPath`, `storePath`); update normalization if new volatile fields are added |
| Fixture fragility when scoring weights change | Medium | Snapshot and inline assertions fail | This is by design — the snapshot exists to detect contract drift. Updating snapshots is a deliberate act via `cargo insta review` |
| `lib.rs` growth from extended hotspot_integration_tests | Low | File size increases marginally | The extension is additive (~50 lines of fixture definitions), not a refactor; the E2E binary offloads the bulk of new test code |

## Traceability

- **Task**: `6dad120d-e43c-44d1-a257-d56de11ce553` (Hotspot Foundation 04)
- **Canonical spec**: `openspec/specs/hotspot-report/spec.md` (artifact file requirement, scoring contract, ranking rules)
- **Prior art**: `openspec/changes/archive/2026-06-21-task-649bd576-c00f-40d7-8edb-79e22b1783d5/` (Hotspot Foundation 03 — already added artifact-write logic)
- **Accepted decisions**: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- **Dossier**: `2026-06-21T13:38:36.157Z` exploration dossier
- **Blockers**: None unresolved. Two non-blocking reviewer claims (no E2E pipeline coverage, no snapshot drift detection) are addressed by this design