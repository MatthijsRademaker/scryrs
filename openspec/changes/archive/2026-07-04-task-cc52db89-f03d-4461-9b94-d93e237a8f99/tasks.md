## 1. Add --help-json support to argument parser

- [x] 1.1 Add `--help-json` and `-hj` to the global flag matches in `run_with_writers` — emit the surface document to stdout and return 0 <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->
- [x] 1.2 Ensure `--help-json` placed after a command (e.g., `hotspots --help-json`) falls through to the command's argument parser and exits 2 (unchanged behavior — no per-command introspection in v0) <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->

## 2. Implement surface document serialization

- [x] 2.1 Write `fn write_cli_surface(out: &mut impl Write) -> io::Result<()>` that serializes a `serde_json::Value` to stdout via `serde_json::to_writer` <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 2.2 Write `fn cli_surface_doc() -> serde_json::Value` that constructs the surface document with all required sections: `surfaceVersion`, `binary`, `commands`, `globalFlags`, `rootBehavior`, `exitCodes` <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 2.3 Populate the `commands` array with the `hotspots` entry including `name`, `description`, `arguments` (PATH: required, string), and `output` object (mimeType: "application/json", fields for schemaVersion, command, status) <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 2.4 Populate the `globalFlags` array with entries for `help` (-h/--help), `version` (-V/--version), and `help-json` (-hj/--help-json), each with `name`, `short`, `long`, `description`, and `action` <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 2.5 Populate the `rootBehavior` object with `action: "help"` and `exitCode: 0` <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 2.6 Populate the `exitCodes` object with entries for 0 (Success), 1 (I/O error), and 2 (Usage error) <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->

## 3. Update CLI contract documentation

- [x] 3.1 Add `--help-json` surface description to `.devagent/docs/docs/cli-v0-contract.md` — document the flag, the surface document format, each field's semantics, and how agents should use it <!-- completed outside OpenSpec: .devagent/docs/docs/cli-v0-contract.md -->
- [x] 3.2 Document the `surfaceVersion` field and the versioning policy (semver for the surface document format) <!-- completed outside OpenSpec: .devagent/docs/docs/cli-v0-contract.md -->

## 4. Add tests

- [x] 4.1 Add test: `--help-json` outputs valid JSON to stdout and exits 0 with empty stderr <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.2 Add test: `-hj` short flag works identically to `--help-json` <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.3 Add test: surface document contains all required top-level fields (`surfaceVersion`, `binary`, `commands`, `globalFlags`, `rootBehavior`, `exitCodes`) <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.4 Add test: `commands` array has exactly one entry with name `"hotspots"` and correct argument/output metadata <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.5 Add test: `globalFlags` array has exactly three entries (help, version, help-json) with correct short/long/action values <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.6 Add test: `exitCodes` object has keys "0", "1", "2" with correct descriptions <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.7 Add test: `rootBehavior` has `action: "help"` and `exitCode: 0` <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.8 Add test: `--help-json` does not interfere with existing `--help`, `--version`, `hotspots <PATH>`, hotspots without PATH, bare invocation, or unknown commands — all existing tests still pass <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->
- [x] 4.9 Add test: `--help-json` is idempotent — calling it twice produces identical output <!-- completed outside OpenSpec: crates/scryrs-cli/tests/dispatch_tests.rs -->

## 5. Validation

- [x] 5.1 Run `cargo test -p scryrs-cli` to confirm all tests pass <!-- completed outside OpenSpec: crates/scryrs-cli/ -->
- [x] 5.2 Run `cargo check -p scryrs-cli` to confirm no warnings <!-- completed outside OpenSpec: crates/scryrs-cli/ -->
- [x] 5.3 Manually verify `scryrs --help-json` output is well-formed JSON with correct structure <!-- completed outside OpenSpec: crates/scryrs-cli/src/help_json.rs -->
- [x] 5.4 Manually verify `scryrs -hj` produces identical output to `scryrs --help-json` <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->
- [x] 5.5 Manually verify all existing behavior is unchanged (`--help`, `--version`, `hotspots /tmp`, bare invocation) <!-- completed outside OpenSpec: crates/scryrs-cli/ -->
