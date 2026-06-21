## Why

Testing is uneven across the scryrs workspace. The Rust CLI and core crates have strong unit and integration coverage running in default CI (~150 tests across `scryrs-cli` and `scryrs-core`), and hook-level end-to-end verification fixtures exist for both Claude Code and Pi harnesses. However, three gaps put the project at risk:

1. **Hook verification runs entirely outside CI.** Both `scripts/hook-test` (fast, Node-only Claude Code hook contract checks) and `scripts/verify-trace-capture` (Docker-backed cross-harness e2e) exist but are not wired into `.github/workflows/ci.yml` or the default local `scripts/test` lane. A broken hook contract — non-interference, fail-open behavior, or JSON shaping — can merge undetected.
2. **No installed-hook end-to-end coverage.** Existing e2e fixtures (`claude-code-e2e.mjs`, `pi-hook-e2e.mjs`) import hook source directly from the repository rather than testing artifacts produced by `scryrs init --agent`. If `scryrs init` generates structurally correct but semantically broken hooks (wrong paths, missing dependencies), no test catches it.
3. **Scaffold crates have only sentinel tests.** Seven non-core crates (`scryrs-graph`, `scryrs-curator`, `scryrs-policy`, `scryrs-runtime`, `scryrs-sandbox`, `scryrs-telemetry`, `scryrs-llm`) each have exactly one `#[test]`, and repository-level `tests/golden/`, `tests/integration/`, `tests/llm/` directories are `.gitkeep` placeholders. While architecture docs describe these crates as scaffold-level, several already contain nontrivial logic that warrants targeted unit coverage.

This change inventories and addresses the highest-risk gaps with a small set of implementation tasks sized for hours, not a monolithic coverage drive.

## What Changes

- **CI lane for fast hook verification:** Wire `scripts/hook-test` (Node-only, no Docker or Rust build needed) into CI as a check that runs on pull requests touching `hooks/**` paths. Measure `scripts/verify-trace-capture` runtime and assign it to a nightly lane or keep it manual based on measured cost.
- **Installed-hook end-to-end validation:** Add a true installed-hook e2e test that runs `scryrs init --agent claude-code|pi` in a temporary consumer-style project, then loads and exercises the installed hook artifacts against a real or simulated harness, proving the init pipeline produces functional output.
- **Targeted unit tests for high-priority scaffold crates:** Add unit tests for `scryrs-curator` (`propose_from_hotspot` logic) and `scryrs-sandbox` (`ToolPolicy::can_write` path-matching) — the two scaffold crates closest to having nontrivial logic beyond constructors.
- **Developer-local test lane integration:** Update `scripts/test` to include a flag or second entrypoint that runs hook verification alongside `cargo test`, so developers can run the full suite without discovering separate entrypoints.

## Impact

- Pull requests touching hook code gain a fast (<5s) CI guard that catches hook contract regressions before merge.
- A silent correctness blind spot in the `scryrs init` output pipeline is closed with installed-hook e2e coverage.
- Two scaffold crates with existing nontrivial behavior gain basic unit coverage, reducing regression risk before further feature work lands.
- Blanket coverage targets for all remaining scaffold crates and the empty `tests/golden/`, `tests/integration/`, `tests/llm/` placeholder directories are explicitly deferred. No existing tests are refactored or rewritten.