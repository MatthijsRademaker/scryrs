## 1. Install snapshot testing infrastructure

- [ ] 1.1 Add `insta` dev-dependency to `crates/scryrs-cli/Cargo.toml` (no feature flags needed for basic snapshot operations)

## 2. Replace help text substring tests with snapshot tests

- [ ] 2.1 Add `insta::assert_snapshot!()` file-snapshot test for `--help` output — capture full stdout from `run_with_writers(["--help"], out, err)`, assert exit 0, assert empty stderr
- [ ] 2.2 Convert `-h` test to verify byte-for-byte identity with `--help` output (not a separate snapshot) — keep exit 0 and empty stderr assertions
- [ ] 2.3 Convert bare invocation test to verify byte-for-byte identity with `--help` output — keep exit 0 and empty stderr assertions
- [ ] 2.4 Remove the old substring assertions (`output.contains("USAGE")`, `output.contains("EXAMPLES")`, etc.) from the help-output tests — they are redundant with the snapshot

## 3. Replace --help-json structural tests with a single snapshot test

- [ ] 3.1 Add `insta::assert_snapshot!()` file-snapshot test for `--help-json` output — capture full stdout from `run_with_writers(["--help-json"], out, err)`, assert exit 0, assert empty stderr
- [ ] 3.2 Verify `-hj` produces byte-for-byte identical output to `--help-json` (not a separate snapshot)
- [ ] 3.3 Verify `--help-json` is idempotent — two calls produce identical output
- [ ] 3.4 Remove the old structural field-level assertions (`surfaceVersion`, `binary`, `commands`, `globalFlags`, `rootBehavior`, `exitCodes`, individual command/flag/field checks) — they are redundant with the snapshot

## 4. Replace hotspots placeholder substring test with inline snapshot

- [ ] 4.1 Add `insta::assert_snapshot!(output, @"...")` inline-snapshot test for `hotspots /tmp` output — capture stdout, assert exit 0, assert empty stderr
- [ ] 4.2 Remove the old substring assertions for `schemaVersion`, `command`, `status` — they are redundant with the inline snapshot

## 5. Add smoke tests for the public run() entrypoint

- [ ] 5.1 Add smoke test: `run(["--help"])` exits 0, stdout non-empty
- [ ] 5.2 Add smoke test: `run(["--version"])` exits 0, stdout non-empty
- [ ] 5.3 Add smoke test: `run(["hotspots", "/tmp"])` exits 0, stdout non-empty
- [ ] 5.4 Add smoke test: `run([])` (bare invocation) exits 0, stdout non-empty
- [ ] 5.5 Add smoke test: `run(["unknown"])` exits 2, stderr non-empty
- [ ] 5.6 Add smoke test: `run(["hotspots"])` (missing PATH) exits 2, stderr non-empty
- [ ] 5.7 Add smoke test: `run(["--help-json"])` exits 0, stdout non-empty

## 6. Document local check path

- [ ] 6.1 Add a "Local Development Testing" section to `.devagent/docs/docs/cli-v0-contract.md` covering: how to run tests (`cargo test -p scryrs-cli`), how to view snapshot diffs, how to update snapshots (`cargo insta review` or `cargo insta test --accept -p scryrs-cli`), and how to install `cargo-insta` if needed (`cargo install cargo-insta`)

## 7. Validation

- [ ] 7.1 Run `cargo check -p scryrs-cli` to confirm no compile errors after all changes
- [ ] 7.2 Run `cargo test -p scryrs-cli` — initial run will create snapshot files and fail (snapshots need acceptance); verify the diffs show expected output
- [ ] 7.3 Run `cargo insta test --accept -p scryrs-cli` to accept the initial snapshots
- [ ] 7.4 Run `cargo test -p scryrs-cli` again to confirm all tests pass with accepted snapshots
- [ ] 7.5 Verify all existing error-path tests still pass unchanged (unknown command, missing PATH, extra args)
- [ ] 7.6 Verify `cargo clippy -p scryrs-cli` passes with no warnings on the changed code
