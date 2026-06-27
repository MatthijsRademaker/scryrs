## 1. Test Infrastructure

- [ ] 1.1 Add a `write_test_routes(dir: &Path, content: &str)` helper in the `#[cfg(test)]` module of `crates/scryrs-cli/src/propose.rs` that creates `.scryrs/routes.json` with the given content.
- [ ] 1.2 Add a `write_test_docs(dir: &Path)` helper that creates `.devagent/docs/docs/_nav.json` with `[]` content and `.devagent/docs/docs/vision.md` with representative markdown content.
- [ ] 1.3 Add a `compute_file_inventory(root: &Path) -> HashMap<PathBuf, String>` helper that walks the temp repo tree and returns relative file paths mapped to SHA-256 hex digests of file contents.

## 2. Verification Helper

- [ ] 2.1 Implement a `verify_proposal_writes_confined(root: &Path, protected_paths: &[&str])` helper that: (a) snapshots byte-for-byte content of every protected path before `write_proposals`, (b) computes the full file inventory before `write_proposals`, (c) runs `write_proposals`, (d) asserts byte-for-byte identity of every protected path, (e) computes the full file inventory after `write_proposals`, (f) asserts that any file added or modified post-run is under `.scryrs/proposals/`.
- [ ] 2.2 The helper SHALL accept a dynamic `protected_paths` list (e.g., `&[&str]` or `&[PathBuf]`) so future source-of-truth destinations can be added at the call site without modifying the helper.

## 3. Rewrite source_of_truth_not_mutated

- [ ] 3.1 Rewrite `source_of_truth_not_mutated()` to seed `.scryrs/routes.json` (with `{}`) and `.devagent/docs/` files (via `write_test_docs`) alongside the existing `.scryrs/graph.json` seed.
- [ ] 3.2 Call `verify_proposal_writes_confined()` with the protected paths list: `[".scryrs/graph.json", ".scryrs/routes.json", ".devagent/docs/"]`.
- [ ] 3.3 Remove the `.scryrs/hotspots.json` byte-for-byte assertion from the test (it is an input artifact). Keep the existing `write_test_hotspots` seed call since `write_proposals` requires hotspots.json as input.

## 4. Test Execution and Verification

- [ ] 4.1 Run `cargo test -p scryrs-cli --features curator source_of_truth_not_mutated` and confirm the test passes.
- [ ] 4.2 Run the full `cargo test -p scryrs-cli --features curator` suite and confirm no regressions.
- [ ] 4.3 If the test fails due to a production-code write outside `.scryrs/proposals/`, fix only the offending write instruction in `write_proposals` and re-run.

## 5. Bug Contingency (conditional — only if test reveals a write-path bug)

- [ ] 5.1 Identify the exact production-code line in `write_proposals` that writes outside `.scryrs/proposals/`.
- [ ] 5.2 Fix the offending write instruction. Do NOT change proposal heuristics, target types, inbox semantics, or curator engine behavior.
- [ ] 5.3 Re-run the strengthened test and full test suite to confirm the fix.