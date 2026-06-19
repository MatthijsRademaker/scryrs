## 1. Publish v0 CLI Contract Design Note

- [ ] 1.1 Create `.devagent/docs/docs/cli-v0-contract.md` documenting: binary name (`scryrs`), single command (`components`), global flags (`--help`/`-h`, `--version`/`-V`), accepted inputs (`components`, optional `--format json`), agent-facing intent (capability/build introspection only), output contract (stable text format, JSON shape with `schemaVersion` and `components` array), stdout/stderr rules, exit code policy (0/1/2), and fail-fast rules for unsupported paths.
- [ ] 1.2 Add the v0 contract note to `.devagent/docs/docs/_nav.json` under the Documentation section so it is discoverable alongside Architecture.

## 2. Narrow CLI Dispatch to v0 Surface

- [ ] 2.1 Remove the `is_known_stub()` function (definition and dispatch arm) from `crates/scryrs-cli/src/lib.rs` so all unrecognized commands hit the unknown-command path (stderr message + exit 2).
- [ ] 2.2 Narrow `write_help()` in `crates/scryrs-cli/src/lib.rs` to show only the v0 surface: `scryrs components [--format json]` with global help/version flags. Remove all stub command lines (`trace`, `hotspots`, `propose`, `graph`, `route`, `adapters`) and the scaffold-placeholder caveat text.
- [ ] 2.3 Verify existing `components`, `components --format json`, `--help`, `-h`, `--version`, `-V`, and unknown-command dispatch arms are unchanged and continue to pass existing tests.

## 3. Lock Contract with Tests

- [ ] 3.1 Add a test verifying that a previously-recognized stub command (e.g., `trace`) now returns exit 2 and writes an error to stderr (confirming `is_known_stub()` removal).
- [ ] 3.2 Add a test verifying that `--help` output no longer contains any stub command names (`trace`, `hotspots`, `propose`, `graph`, `route`, `adapters`).
- [ ] 3.3 Ensure all existing tests pass (`help_returns_success`, `unknown_command_returns_usage_error`, `components_can_be_emitted_as_json`).

## 4. Update OpenSpec Change Artifacts

- [ ] 4.1 Update `openspec/changes/task-1613d7cf-54a6-44ff-acb3-780736980aa3/proposal.md` from placeholder to real content (Why, What Changes, Impact).
- [ ] 4.2 Update `openspec/changes/task-1613d7cf-54a6-44ff-acb3-780736980aa3/design.md` (create if absent) with the design decisions documented above.
- [ ] 4.3 Create `openspec/changes/task-1613d7cf-54a6-44ff-acb3-780736980aa3/specs/cli-v0-contract/spec.md` with frozen v0 CLI contract requirements.