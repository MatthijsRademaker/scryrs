## 1. Add Quickstart section to README

- [ ] 1.1 Read current `README.md` and existing snapshot files to determine exact placement and representative output
- [ ] 1.2 Add Quickstart header and prerequisites subsection (Rust 1.85+ or Docker)
- [ ] 1.3 Add "Build from source" subsection with copy-paste `cargo build -p scryrs-cli` and `cargo run -p scryrs-cli -- --help` commands
- [ ] 1.4 Add "Explore the CLI surface" subsection with `--help`, `--version`, and `--help-json` examples showing representative expected output
- [ ] 1.5 Add "Run the placeholder command" subsection with `scryrs hotspots <PATH>` example showing the JSON placeholder envelope with explanation
- [ ] 1.6 Add "Error paths" subsection showing at least one error example (e.g., missing PATH argument) with expected stderr output and exit code explanation
- [ ] 1.7 Add "Current limitations" subsection documenting: single command, placeholder output, no engine behavior, no speculative future commands
- [ ] 1.8 Add "Troubleshooting" subsection covering common issues (unknown command errors, missing PATH, build failures)

## 2. Verify examples match tested CLI behavior

- [ ] 2.1 Verify `--help` example output matches the existing `insta` help flag snapshot
- [ ] 2.2 Verify `--version` example output matches the existing version flag test behavior
- [ ] 2.3 Verify `--help-json` example output matches the existing `insta` help-json flag snapshot
- [ ] 2.4 Verify `hotspots <PATH>` example output matches the existing `hotspots` inline snapshot
- [ ] 2.5 Verify error path example matches the existing missing-PATH error test behavior
- [ ] 2.6 Confirm no example references commands outside the v0 contract (no `components`, `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`)

## 3. Final validation

- [ ] 3.1 Run `cargo test -p scryrs-cli` to confirm all tests pass unchanged
- [ ] 3.2 Run `scripts/check` to confirm no lint or formatting issues
- [ ] 3.3 Read the updated `README.md` end-to-end to verify flow: a first-time user can follow from prerequisites through to running the CLI
