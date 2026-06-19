## 1. Publish CLI contract design note

- [ ] 1.1 Create `.devagent/docs/docs/cli-v0-contract.md` with the frozen v0 CLI contract.
- [ ] 1.2 Add `cli-v0-contract` entry to `.devagent/docs/docs/_nav.json` under a new "Technical" section.

## 2. Narrow CLI surface to single placeholder command

- [ ] 2.1 Update argument parsing in `crates/scryrs-cli/src/lib.rs:` to accept only `scryrs hotspots <PATH>` (required PATH argument), `-h`/`--help`, `-V`/`--version`, and bare invocation (help).
- [ ] 2.2 Remove `components` command implementation: delete `write_components_text`, `write_components_json`, and `descriptors` functions (or gate behind off-by-default feature flag `_dev`).
- [ ] 2.3 Remove `is_known_stub` function and all stub-command handling from argument matching.
- [ ] 2.4 Update `write_help` output to list only `scryrs hotspots <PATH>` as the available command.
- [ ] 2.5 Implement the `hotspots` placeholder command handler: emit a versioned JSON envelope to stdout (`{"schemaVersion": "<VERSION>", "command": "hotspots", "status": "placeholder"}`).
- [ ] 2.6 Ensure missing PATH argument produces usage error on stderr and exits 2.
- [ ] 2.7 Ensure unknown commands produce usage error on stderr and exit 2.

## 3. Update tests

- [ ] 3.1 Remove or update tests for `components` JSON/text output (`components_can_be_emitted_as_json`).
- [ ] 3.2 Add test: `hotspots` with PATH argument prints JSON to stdout and exits 0.
- [ ] 3.3 Add test: `hotspots` without PATH argument prints usage error to stderr and exits 2.
- [ ] 3.4 Add test: unknown command prints error to stderr and exits 2.
- [ ] 3.5 Add test: bare invocation prints help to stdout and exits 0.
- [ ] 3.6 Add test: `--help` prints help to stdout and exits 0.
- [ ] 3.7 Add test: `--version` prints version to stdout and exits 0.

## 4. Update README

- [ ] 4.1 Replace `components` examples in "Feature split" section with `scryrs hotspots <PATH>` as the primary v0 surface.
- [ ] 4.2 Remove or update `cargo run -p scryrs-cli -- components` and multi-feature examples to reflect single-command contract.
- [ ] 4.3 Ensure README no longer advertises `components` or any second real command.

## 5. Validation

- [ ] 5.1 Run `cargo test -p scryrs-cli` to confirm all updated tests pass.
- [ ] 5.2 Run `cargo check -p scryrs-cli` to confirm no dead code warnings for removed functions.
- [ ] 5.3 Verify `scryrs --help` output shows only `scryrs hotspots <PATH>`.
- [ ] 5.4 Verify `scryrs hotspots /tmp` emits valid JSON to stdout.
- [ ] 5.5 Verify `scryrs components` exits 2 with error on stderr.
- [ ] 5.6 Verify `scryrs trace` exits 2 with error on stderr.