## Context

- `scryrs-cli` (~55 tests) and `scryrs-core` (~64 tests) have strong Rust unit/integration coverage that runs in default CI via `cargo test --workspace`.
- `scryrs-types` (24 tests) has solid type-level coverage.
- Seven scaffold crates (`scryrs-graph`, `scryrs-curator`, `scryrs-policy`, `scryrs-runtime`, `scryrs-sandbox`, `scryrs-telemetry`, `scryrs-llm`) each have exactly one sentinel `#[test]`.
- Hook verification scripts exist (`scripts/hook-test` for fast Claude Code JSON shaping and fail-open checks, `scripts/verify-trace-capture` for Docker-backed cross-harness e2e) but are completely absent from `.github/workflows/ci.yml` and `scripts/test`.
- Existing e2e fixtures (`scripts/verification/claude-code-e2e.mjs`, `scripts/verification/pi-hook-e2e.mjs`) import hook source directly from the repository tree rather than testing artifacts produced by `scryrs init --agent`.
- `scryrs-cli` init tests only verify file creation and collision handling — they never execute installed hooks through a harness.
- `tests/golden/`, `tests/integration/`, `tests/llm/` are `.gitkeep` placeholders with no active test suites.
- The project uses OpenSpec with scenario-based specs (GIVEN/WHEN/THEN).

## Goals

1. Wire `scripts/hook-test` into CI as a path-triggered check on `hooks/**` changes, with `scripts/verify-trace-capture` gated by measured runtime (fast → PR gate, medium → nightly, slow → investigate first).
2. Add one installed-hook end-to-end test that proves `scryrs init --agent claude-code|pi` produces loadable, functional hook artifacts in a consumer-style temp project.
3. Add targeted unit tests for `scryrs-curator` (`propose_from_hotspot`) and `scryrs-sandbox` (`ToolPolicy::can_write`, `ToolPolicy::read_only`) before those crates gain further behavior changes.
4. Update `scripts/test` so developers can run the full test suite (Rust tests + hook verification) from a single documented entrypoint.

## Non-Goals

- Do not refactor the existing test framework or rewrite existing passing tests.
- Do not add blanket coverage targets for scaffold crates beyond `scryrs-curator` and `scryrs-sandbox` in this change.
- Do not populate the empty `tests/golden/`, `tests/integration/`, or `tests/llm/` directories until a real suite is designed to live there.
- Do not require `scripts/verify-trace-capture` (Docker-backed, full release build) as a per-PR CI gate without first measuring and documenting its runtime.
- Do not add an RTK dependency, change the `CommandExecutedPayload` schema, or alter hook execution semantics.

## Decisions

1. **CI lane for hook verification is path-triggered, not blanket.**
   - `scripts/hook-test` (fast, Node-only, no Rust build) runs on PRs touching `hooks/**` paths.
   - `scripts/verify-trace-capture` (Docker-backed, multi-minute) is assigned based on measured runtime: <3 min → PR gate candidate, 3–10 min → nightly, >10 min → investigate optimization first.
   - Rationale: Per-commit Docker tax is unwarranted for pure Rust changes; path-triggering balances protection with CI latency.

2. **Installed-hook e2e validates the init pipeline end-to-end.**
   - The test creates a temp consumer project, runs `scryrs init --agent claude-code|pi`, loads the installed hook file from its consumer location, and exercises record/forwarding behavior.
   - Existing source-import e2e fixtures remain as complementary checks but no longer serve as sole hook validation.
   - Rationale: `init` tests currently stop at file existence checks — semantic breakage in installed hook output is invisible.

3. **Scaffold crate test targets are `scryrs-curator` and `scryrs-sandbox`.**
   - `scryrs-curator` has `propose_from_hotspot` with `KnowledgeProposal` construction logic.
   - `scryrs-sandbox` has `ToolPolicy` with `can_write` path-matching logic and `read_only` constructor logic.
   - Other scaffold crates (graph, policy, runtime, telemetry, llm) remain at sentinel-only coverage until they gain nontrivial logic.
   - Rationale: These two crates already have testable behavior beyond simple struct descriptors — curator was also flagged by the reviewer as closest to pragmatically demanding more than a sentinel test.

4. **Developer-local test lane gets a discoverable hook verification entrypoint.**
   - `scripts/test` remains the default Rust test lane.
   - A new `scripts/test --full` flag or `scripts/test-all` entrypoint runs both `cargo test` and `scripts/hook-test` so developers can easily run the complete suite.
   - Rationale: The current `scripts/test` silently skips hook verification, creating a discoverability gap where a developer believes they have run all tests.

5. **Existing placeholder test directories are preserved as-is.**
   - `tests/golden/.gitkeep`, `tests/integration/.gitkeep`, `tests/llm/.gitkeep` remain until a real suite is designed to occupy them.
   - Rationale: Removing `.gitkeep` files would lose the directory structure; populating them prematurely would create untethered suites. All three agents agreed to defer this.

## Conflict Resolution

- **Task ordering:** The architect and lead-dev prioritized CI lane integration as task #1 (fast, catches regressions on every PR, requires no new infrastructure). The reviewer recommended installed-hook e2e as task #1 (most dangerous blind spot — silent user-facing breakage). Both gaps are real. The synthesized tasks list treats them as independently implementable (task #1 and task #2), with no hard dependency between them. The CI lane task is listed first because `scripts/hook-test` is the lowest-effort, highest-ROI change.
- **Scaffold crate naming:** The architect named `scryrs-llm` and `scryrs-graph` as targets. The reviewer named `scryrs-curator` and `scryrs-sandbox` as targets with concrete justification (existing nontrivial logic). The reviewer's specificity is adopted because the evidence shows `propose_from_hotspot` and `ToolPolicy::can_write` are real testable logic, while `scryrs-graph` and `scryrs-llm` remain closer to pure scaffold (simple struct + descriptor only).
- **Developer-local lane:** The reviewer's recommendation to update `scripts/test` is adopted as a concrete task (task #4) because it addresses a discoverability gap that all three agents implicitly acknowledged — the default test lane skips hook verification silently.

## Risks

- **CI runner availability:** If `scripts/hook-test` is added as a path-triggered CI check, it consumes a CI runner slot. Mitigation: `hook-test` runs in <5s on Node.js only, so runner cost is negligible compared to `cargo test`.
- **Docker backend availability in CI:** `scripts/verify-trace-capture` requires Docker (DinD or host Docker). Mitigation: measure runtime first; if runtime is acceptable, verify Docker availability in the CI environment before gating; otherwise assign to nightly.
- **Installed-hook e2e binary dependency:** The installed-hook e2e test requires the `scryrs` binary. Mitigation: share the binary build step with `verify-trace-capture` or build it as a CI artifact.
- **Scaffold crate scope creep:** Adding unit tests to curator and sandbox could trigger a chain of "why not also test X" for other scaffold crates. Mitigation: the non-goals section explicitly defers all other scaffold crates.
- **Pi hook SessionEnd limitation:** The Pi hook README documents `SessionEnd` as deferred. The installed-hook e2e test should not assert SessionEnd behavior unless that limitation is lifted first.

## Traceability

- `task:16b7f8f4-3261-4fd1-ad0e-882a91e71439`
- `dossier:2026-06-21T15:06:20.136Z`
- `decision:1-swarm-architect-recommendation`
- `decision:1-swarm-lead-dev-recommendation`
- `decision:1-swarm-reviewer-recommendation`
- `round:1:agent:swarm-architect`
- `round:1:agent:swarm-lead-dev`
- `round:1:agent:swarm-reviewer`