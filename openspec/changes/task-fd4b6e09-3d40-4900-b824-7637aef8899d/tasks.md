## 1. Wire installed-hook e2e into verify-trace-capture

- [ ] 1.1 Add `--init-only` flag parsing to `scripts/verify-trace-capture` (matching existing `--claude-only`/`--pi-only` filter pattern)
- [ ] 1.2 Add installed-hook e2e as a third fixture phase in `scripts/verify-trace-capture`, invoked after existing source-hook fixtures when running full lane, or independently via `--init-only`
- [ ] 1.3 Ensure the installed-hook phase uses the same `FIXTURE_NODE_IMAGE` (default `node:22`) and scryrs binary from the build step
- [ ] 1.4 Wire the scryrs binary PATH correctly (same `PATH=/workspace/target/release:...` pattern as existing fixtures)
- [ ] 1.5 Verify the full lane (`scripts/verify-trace-capture` with no flags) runs all three fixtures and exits 0 on success
- [ ] 1.6 Verify `--init-only` runs only installed-hook e2e and skips source-hook fixtures
- [ ] 1.7 Verify error propagation: if installed-hook e2e fails, the lane exits non-zero and reports which fixture failed

## 2. Reconcile Claude Code settings.json schema

- [ ] 2.1 Audit which `.claude/settings.json` hook config form is canonical for current Claude Code versions (flat `"hook"` string form vs. nested `"type": "command"` command-block form)
- [ ] 2.2 If the flat `"hook"` string form is canonical: update `init.rs` next-steps text to emit the flat form and update the collision-error JSON block to match
- [ ] 2.3 If the nested command-block form is canonical: update `hooks/claude-code/README.md` to use the command-block form
- [ ] 2.4 If both forms are intentionally supported: document this explicitly in both `init.rs` next-steps text and `hooks/claude-code/README.md`
- [ ] 2.5 Update Rust unit tests in `crates/scryrs-cli/src/lib.rs` that assert on the next-steps or collision-error JSON content to match the chosen canonical form
- [ ] 2.6 Verify that `cargo test` passes with updated next-steps text

## 3. Update verification documentation

- [ ] 3.1 Add `installed-hook-e2e.mjs` to the fixture tree in `scripts/verification/README.md` (in the Architecture section, alongside claude-code-e2e.mjs and pi-hook-e2e.mjs)
- [ ] 3.2 Document what `installed-hook-e2e.mjs` proves (init output is functional, not just files created)
- [ ] 3.3 Document the new `--init-only` flag in `scripts/verify-trace-capture` usage section
- [ ] 3.4 Add a note in the README about the Pi version-gated assumption (single-file index.ts sufficiency, documented Pi versions tested)

## 4. Verify end-to-end

- [ ] 4.1 Run `scripts/verify-trace-capture --init-only` and confirm installed-hook e2e passes for both Claude Code and Pi
- [ ] 4.2 Run `scripts/verify-trace-capture` (full lane) and confirm all three fixtures pass
- [ ] 4.3 Confirm that `cargo test --workspace` passes (no regressions from next-steps text changes)
- [ ] 4.4 Confirm that the init installer contract remains unchanged: no `.claude/settings.json` auto-creation, collision refusal still exits 2, next-step text is deterministic