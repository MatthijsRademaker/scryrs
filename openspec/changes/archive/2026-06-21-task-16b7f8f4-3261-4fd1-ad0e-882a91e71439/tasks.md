## 1. Wire fast hook verification into CI

- [x] 1.1 Add a CI job or step in `.github/workflows/ci.yml` that runs `scripts/hook-test` on pull requests touching `hooks/**` paths.
- [x] 1.2 Verify the CI step fails when a hook contract is broken (JSON shaping, fail-open, or transparency regression).
- [x] 1.3 Measure wall-clock runtime of `scripts/verify-trace-capture` under CI-like conditions and document the result.
- [x] 1.4 Based on measured runtime, assign `scripts/verify-trace-capture` to a nightly lane (<10 min) or document it as manual-only with an open investigation for optimization (>10 min).

## 2. Add installed-hook end-to-end validation

- [x] 2.1 Create a test that runs `scryrs init --agent claude-code` in a temp consumer project, loads the installed `scryrs-hook.mjs` from its consumer location, and exercises record/forwarding against the real `scryrs` binary.
- [x] 2.2 Create a test that runs `scryrs init --agent pi` in a temp consumer project, loads the installed Pi hook artifact from its consumer location, and exercises record/forwarding against the real `scryrs` binary.
- [x] 2.3 Assert that installed hook artifacts are loadable, produce valid events when exercised with tool-capture inputs, and remain silent on stdout/stderr.

## 3. Add targeted unit tests for scryrs-curator and scryrs-sandbox

- [x] 3.1 Add unit tests for `scryrs-curator::propose_from_hotspot` covering: proposal title contains hotspot subject, rationale references subject and score, edge case with empty counts/evidence.
- [x] 3.2 Add unit tests for `scryrs-sandbox::ToolPolicy` covering: `read_only` constructor produces correct defaults, `can_write` returns true for allowed paths, `can_write` returns false for disallowed paths, `can_write` with empty allowlist rejects all paths.
- [x] 3.3 Ensure new tests follow existing patterns (`#[cfg(test)] mod tests`) and pass with `cargo test -p scryrs-curator` and `cargo test -p scryrs-sandbox`.

## 4. Update developer-local test lane

- [x] 4.1 Add a `--full` flag or `scripts/test-all` entrypoint that runs `scripts/hook-test` after `cargo test --workspace`.
- [x] 4.2 Update `scripts/test` help output to document the full-suite option.
- [x] 4.3 Verify the full lane exits non-zero when either Rust tests or hook tests fail.

## 5. Documentation

- [x] 5.1 Update `.devagent/docs/` or relevant project docs to reflect the new CI hook verification lane and installed-hook e2e coverage.
- [x] 5.2 Document the deferred status of `tests/golden/`, `tests/integration/`, `tests/llm/` placeholder directories (no action needed, remain as `.gitkeep` placeholders).
