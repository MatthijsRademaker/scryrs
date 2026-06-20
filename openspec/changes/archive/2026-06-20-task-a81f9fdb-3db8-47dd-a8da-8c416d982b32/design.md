## Context

The `scryrs` CLI crate (`crates/scryrs-cli/src/lib.rs`) has a frozen v0 contract: one placeholder command (`scryrs hotspots <PATH>`), global flags (`--help`, `--version`, `--help-json`), deterministic JSON output, and a documented exit-code policy (0/1/2). The current test suite uses `run_with_writers()` with `Vec<u8>` buffers and substring assertions (`assert!(output.contains("USAGE"))`, etc.). This approach:

- Passes even when output drifts significantly, as long as key substrings survive.
- Provides no diff on failure — a reviewer cannot see what changed.
- Has no mechanism for automated snapshot updates when output intentionally changes.
- Lacks process-level coverage — the public `run()` entrypoint (which captures `std::env::args()`) and the `main()` binary entrypoint are untested.

The existing help text is ~25 lines of formatted prose. The `--help-json` surface document is a structured JSON object with ~6 top-level fields. The `hotspots` placeholder output is a single line of JSON. All three are deterministic and stable — ideal candidates for snapshot testing.

No snapshot or golden-test infrastructure currently exists in the workspace. The workspace `Cargo.toml` has no `insta` or similar dev-dependency.

## Goals / Non-Goals

**Goals:**

- Replace substring assertions for `--help` output with exact-match snapshot tests.
- Replace substring structural checks for `--help-json` output with exact-match snapshot tests.
- Add exact-match snapshot tests for `hotspots <PATH>` placeholder JSON output.
- Add smoke tests exercising the public `run()` entrypoint (not just `run_with_writers`), verifying exit code propagation through the env-args → writers pipeline.
- Document the local test check path (how to run tests, view/update snapshots).
- Keep existing error-message tests as substring assertions — error messages are short, explicit, and the exact wording is stable but the informational value of "contains 'missing'" + exit code 2 is sufficient.

**Non-Goals:**

- No integration tests that invoke the compiled binary as a subprocess. The `run()` entrypoint tests provide sufficient coverage at lower cost.
- No snapshot tests for error messages — they are short, stable, and adequately covered by substring + exit-code assertions.
- No change to the `--help-json` surface document format or CLI contract.
- No change to exit codes or error message wording.
- No changes to any crate outside `scryrs-cli`.
- No implementation of `xtask ci-fast` beyond documenting the test command.

## Decisions

### D1: Use `insta` for snapshot testing

**Decision**: Add `insta` as a dev-dependency in `crates/scryrs-cli/Cargo.toml` and use its file-snapshot mode for help text and surface document, inline-snapshot mode for the single-line `hotspots` JSON.

**Rationale**: `insta` is the de facto standard for Rust snapshot testing. It provides:

- File-based snapshots that are committed alongside source code.
- `cargo insta review` for interactive snapshot review/accept/reject workflow.
- `cargo insta test --accept` for batch updating all changed snapshots.
- Inline snapshots (`assert_snapshot!(output, @"expected text")`) for small outputs, file snapshots (`assert_snapshot!(output)`) for large outputs.
- Diff output on failure showing exactly what changed.
- Deterministic, zero-false-positive test results.

Alternatives considered:

- **Hand-rolled golden files**: Requires writing file-read helpers, update scripts, and diff utilities. Adds boilerplate without the review workflow. Rejected because `insta`'s review workflow is the primary value.
- **Inline `assert_eq!` with raw strings**: Help text is ~25 lines — inline strings in the test are unwieldy to read and update. Surface document JSON is similarly large. Rejected for readability.
- **`similar-asserts`**: Provides diff output on failure but no snapshot management. Rejected because it solves only half the problem.

**Sources**: Rust ecosystem standard, current workspace has no snapshot infra.

### D2: Help text — file snapshot via `insta::assert_snapshot!`

**Decision**: The `--help` output test captures the full help text via `insta::assert_snapshot!()` (file snapshot mode). The snapshot file is saved at `crates/scryrs-cli/src/snapshots/scryrs_cli__tests__help_output.snap`.

**Rationale**: Help text is ~25 lines of formatted prose. An inline snapshot would bloat the test file. A file snapshot is readable, diffable, and cleanly separates the test logic from the expected output. The snapshot name encodes the test path for discoverability.

**Contract**: The help text snapshot is the source of truth for the human-facing contract. Any intentional change to help text requires updating the snapshot. Any unintentional change is caught as a test failure with a visible diff.

### D3: `--help-json` surface document — file snapshot via `insta::assert_snapshot!`

**Decision**: The `--help-json` output is captured as a file snapshot, producing a `.snap` file with the complete JSON document.

**Rationale**: The surface document is a structured JSON object. A file snapshot lets reviewers see the exact JSON structure in diff view. Field ordering, whitespace, and value changes are all visible. The snapshot doubles as documentation of the machine-readable contract.

**Contract**: The `--help-json` snapshot is the source of truth for the machine-facing contract. Any addition, removal, or rename of a CLI surface element requires updating this snapshot.

### D4: `hotspots <PATH>` placeholder — inline snapshot via `insta::assert_snapshot!` with string literal

**Decision**: The single-line JSON envelope `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}` is tested with `insta::assert_snapshot!(output, @"...")` inline snapshot.

**Rationale**: The output is a single line. An inline snapshot keeps it visible in the test source. File snapshots for single-line outputs add indirection without benefit. If the output grows (additional fields), it can be promoted to a file snapshot.

### D5: Smoke tests exercise `run()`, not the compiled binary

**Decision**: Smoke tests call the public `run()` function (which takes `IntoIterator<Item = Into<String>>` and delegates to `run_with_writers`). They do NOT invoke the binary as a subprocess.

**Rationale**: The `run()` function is the public API that `main()` calls. Its signature accepts any string iterator type, so tests can pass `Vec<&str>` directly — no compilation needed. Testing via subprocess (`std::process::Command`) would add:

- Compilation cost for each test run (or require a pre-built binary).
- Platform-specific path/fork behavior.
- No additional coverage over `run()` — `main()` is 3 lines of delegation.

The key gap that `run()` tests fill compared to `run_with_writers` tests: `run()` internally calls `io::stdout().lock()` and `io::stderr().lock()`, so it verifies that the argument iterator is collected and forwarded correctly. This catches bugs where argument normalization or collection logic diverges between the two entrypoints.

**Smoke test coverage**: For each smoke test, verify exit code and that stdout/stderr are writable (not panicking). The exact output content is tested by the snapshot tests — smoke tests focus on the wiring layer.

### D6: Error-message tests remain as substring assertions

**Decision**: The existing tests for error messages (`missing required PATH argument`, `unknown command:`, `unexpected argument after PATH`) are kept as substring assertions. No snapshot tests are added for error paths.

**Rationale**: Error messages are short (2-3 lines), explicit, and the substring + exit-code combination is sufficient to catch drift. The exact wording of "missing required PATH argument" is visible in the source code and error-message tests serve as documentation of the error contract more than drift detection. Snapshot-testing error messages would create noise for minimal gain.

**Risk**: If error message wording changes, the existing substring tests still pass as long as the key phrases survive. This is acceptable — error message wording is intentionally stable and any change would be caught by human review.

### D7: Local check documentation lives in `cli-v0-contract.md`

**Decision**: A "Local Development Testing" section is added to `.devagent/docs/docs/cli-v0-contract.md` documenting:

- How to run tests: `cargo test -p scryrs-cli`
- How to view snapshot diffs: standard `cargo test` output shows diffs
- How to update snapshots after intentional changes: `cargo insta test --accept -p scryrs-cli` or `cargo insta review`
- How to install `cargo-insta` if needed: `cargo install cargo-insta`

**Rationale**: The CLI contract doc is the canonical reference for the CLI surface. Adding test documentation here keeps it co-located with the contract it verifies. The README is a broader document and adding test instructions there dilutes its focus. The `xtask ci-fast` command is still a stub and should be the subject of a future change to wire up the full CI pipeline.

## Risks / Trade-offs

| Risk | Severity | Mitigation |
|------|----------|------------|
| R1: `insta` adds a dev-dependency and `cargo-insta` CLI tool requirement. Developers who don't install `cargo-insta` can still run tests and see diffs on failure — they just can't auto-accept snapshots. | Low | Document that `cargo-insta` is optional for test execution, only needed for snapshot updates. The `insta` crate itself has no runtime dependencies on the CLI tool. |
| R2: Snapshot files grow stale if the CLI surface changes and snapshots aren't updated. | Low | The test fails immediately on drift. The developer must update the snapshot, which forces explicit acknowledgment of the output change. This is the intended behavior. |
| R3: File snapshots create review noise when intentionally changing help text — the snapshot diff is added to every PR. | Low | This IS the value. The snapshot diff IS the review artifact. A PR that changes help text without updating the snapshot test would be caught in CI. |
| R4: Inline snapshot for `hotspots` output is less discoverable than a file snapshot. | Low | The output is a single line. If it grows, promote to file snapshot. |
| R5: Smoke tests via `run()` don't catch `main()` panics or `std::process::exit` behavior. | Low | `main()` is 3 lines: `let code = scryrs_cli::run(...); std::process::exit(code);`. A panic in `run()` propagates through both `run()` and `main()` identically. The `exit()` call is a standard library function. Coverage gap is negligible. |
| R6: Existing substring tests and new snapshot tests overlap — both test the same output. | Low | Existing substring tests should be removed for outputs that get snapshot coverage to avoid dual-maintenance. Error-message tests remain unchanged. |

## Migration Plan

1. Add `insta` dev-dependency to `crates/scryrs-cli/Cargo.toml`.
2. Replace the `help_flag_prints_help_and_exits_0` test with a snapshot test.
3. Replace `short_help_flag_prints_help_and_exits_0` — either remove (redundant) or add a lighter assertion that `-h` produces identical output to `--help`.
4. Replace `bare_invocation_prints_help_and_exits_0` — bare invocation should produce identical output to `--help`. Either deduplicate or snapshot independently.
5. Replace `help_json_flag_outputs_valid_json_and_exits_0`, `surface_doc_contains_all_required_top_level_fields`, `commands_array_has_exactly_one_entry_for_hotspots`, `global_flags_array_has_exactly_three_entries`, `exit_codes_object_has_correct_keys_and_descriptions`, and `root_behavior_has_action_help_and_exit_code_0` — all replaced by a single snapshot test for `--help-json` output.
6. Replace `hotspots_with_path_emits_json_and_exits_0` with an inline snapshot.
7. Add smoke tests for the `run()` entrypoint: `--help`, `--version`, `hotspots /tmp`, bare invocation, hotspots without PATH, unknown command, and `--help-json`.
8. Remove redundant substring assertions that overlap with new snapshots.
9. Update `.devagent/docs/docs/cli-v0-contract.md` with local check documentation.
10. Run `cargo test -p scryrs-cli` to generate initial snapshots, verify diffs, and accept.

## Open Questions

- Should `-h` and bare invocation share the same snapshot (proving identical output), or each have their own? If they share, a single snapshot for help text suffices and `-h` test becomes "verify it matches `--help` output". If separate, any formatting drift between the two paths is detected individually. **Preferred**: single snapshot for help text output, with a separate test asserting `-h` produces identical output to `--help`.
