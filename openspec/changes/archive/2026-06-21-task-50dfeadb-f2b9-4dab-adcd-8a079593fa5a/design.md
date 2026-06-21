## Context

The scryrs CLI crate (`crates/scryrs-cli`) is the primary user-facing entrypoint. Its `src/lib.rs` has accumulated ~3335 lines across multiple feature-delivery changes (cli-clap-migration, cli-discovery-ux, cli-foundation-closure, scryrs-record-endpoint, hotspot-report, hotspot-verification, init-installer, cli-machine-surface, phase-2-closure). The file now violates the repository's file-scope guidance (AGENTS.md rule 12) and mixes five distinct concerns.

The existing `src/init.rs` demonstrates the project's established pattern for splitting CLI command implementations into separate files. This change extends that pattern across the remaining command implementations and test modules.

All three refinement agents (architect, lead-dev, reviewer) independently confirmed the same scope, file structure, and risk profile with high confidence. The reviewer identified three non-blocking implementation concerns (snapshot migration, store_override visibility, fixture unification scope) that are addressed in the decisions below.

## Goals / Non-Goals

### Goals

1. Reduce `crates/scryrs-cli/src/lib.rs` from ~3335 lines to a thin entrypoint (~50-100 lines) that declares modules and re-exports the public API.
2. Separate production code into 7 responsibility-focused modules: `dispatch`, `help_text`, `help_json`, `hotspots`, `record`, `chrono`, `store_override`.
3. Extract 5 test modules into separate files, keeping them as `src/` submodules (not `tests/` integration tests) because they use `super::*` and internal items.
4. Eliminate duplicated CWD_GUARD/with_cwd by extracting to a single shared test-support module.
5. Preserve all externally observable behavior: `--help`, `--help-json`, `record`, `hotspots`, `init`, trace schemas, and hotspot output contracts.
6. Pass the Docker-backed test suite (`scripts/test`) with identical results.

### Non-Goals

- Do not add new CLI commands, new hotspot behavior, or new trace-event schema fields.
- Do not change scoring, storage, hook semantics, or user-facing contract text.
- Do not perform a broad style/refactor pass across every crate.
- Do not introduce speculative abstraction layers for single-use code.
- Do not split `crates/scryrs-types/src/lib.rs` (deferred to follow-up).
- Do not unify hotspot fixture builder families (they serve different pipeline stages).

## Decisions

### Decision 1: Test modules stay under `src/`, not `tests/`

The current test modules (`tests`, `record_tests`, `smoke`, `hotspot_integration_tests`, `init_tests`) use `super::*` to access internal items (`run_with_writers`, `run_with_io`, `run`) and `store_override`. If moved to `tests/` as integration tests, these would become inaccessible. The modules remain as `src/dispatch_tests.rs`, `src/record_tests.rs`, etc., declared with `#[cfg(test)]` or `#[cfg(all(test, feature = "core"))]` in `lib.rs`.

**Rationale**: Preserves existing test access patterns without requiring internal items to become `pub`. The `init.rs` pattern (production code only, no tests) doesn't apply here because these tests need internal visibility.

### Decision 2: Snapshot relocation to `tests/snapshots/`

Moving `tests` and `smoke` modules out of `lib.rs` changes the insta module path, which breaks `source:` metadata in `src/snapshots/scryrs_cli__tests__*.snap`. The snapshot files are relocated to `tests/snapshots/` alongside the existing e2e snapshots, and tests use `insta::with_settings!({snapshot_path => ...})` where needed. Alternatively, insta's default behavior of deriving snapshot paths from the test module path may naturally resolve after relocation — this is verified during implementation.

**Rationale**: Keeps all CLI snapshots in one directory for consistency and eliminates the `src/snapshots/` directory entirely.

### Decision 3: Hotspot fixture families remain separate

`make_file_opened`/`make_search_run`/etc. in `hotspot_integration_tests` construct typed `TraceEvent` structs for direct store population. `make_file_opened_json`/etc. in `hotspot_e2e.rs` produce raw JSONL strings for CLI stdin ingestion. These serve different pipeline stages and differ in kind (structs vs. strings). Unifying them would require an intermediate representation — a speculative abstraction disallowed by AGENTS.md Rule 8.

**Rationale**: The reviewer explicitly identified this as intentional structure, not duplication to eliminate. The CWD guard duplication is the only clear, actionable unification target.

### Decision 4: CWD_GUARD lives in `src/test_support.rs`

`CWD_GUARD` and `with_cwd` are extracted to `src/test_support.rs` as a `pub(crate)` module gated by `#[cfg(test)]`. Both extracted `init_tests.rs` and the existing `tests/hotspot_e2e.rs` import from this single location. `lib.rs` declares `#[cfg(test)] mod test_support;`.

**Rationale**: The `test_support.rs` under `src/` approach matches how `store_override` is structured and avoids creating a `tests/support/` directory for a single helper. The `pub(crate)` visibility allows both unit-test submodules and the integration-test binary to access it.

### Decision 5: store_override extracted to own file with `pub(crate)` visibility

`store_override` currently lives as an inline module in `lib.rs:17-37`. When extracted to `src/store_override.rs`, it must be `pub(crate)` so `record_tests` can access `crate::store_override::set()`. The `super::store_override` path used by `record_tests` is updated to `crate::store_override`.

**Rationale**: `super::` paths won't resolve when `record_tests` moves to a separate file. Making `store_override` a `pub(crate)` module at crate root keeps it accessible while maintaining test-only intent (the module itself remains `#[cfg(feature = "core")]`).

### Decision 6: CLI-only scope; types deferred

The `scryrs-types` crate at ~1000 lines is at the soft limit but not grossly exceeding it, and its internal boundaries (trace contracts, hotspot reports, placeholder types) are already clear with section comments. Splitting it carries cross-crate coordination risk because `GraphNode`, `KnowledgeProposal`, `RouteHint` are re-exported and consumed by `scryrs-curator` and `scryrs-graph`.

**Rationale**: All three refinement agents agreed the CLI split is the highest-value first pass. The reviewer explicitly recommended deferring types. The architect's original two-phase plan was revised by reviewer to commit to CLI-only scope upfront, eliminating uncertainty.

## Risks

| Risk | Mitigation |
|------|------------|
| Insta snapshot source paths break after test relocation | Relocate snapshots to `tests/snapshots/`; verify with `cargo insta test --review`; update `source:` metadata in moved snapshots |
| `super::store_override` path breaks when `record_tests` moves | Extract `store_override` to `src/store_override.rs` with `pub(crate)`; update `record_tests` to use `crate::store_override::set()` |
| Feature-gate (`#[cfg(feature = "core")]`) propagation missed on extracted module | Each extracted file carries its own `#[cfg(feature = "core")]` attribute; `lib.rs` module declarations carry `#[cfg(all(test, feature = "core"))]` for test modules that need core |
| `smoke` module uses `run()` which calls `process::exit()` in default IO path — moving tests that exercise `run()` to a separate file might trigger process exit during test execution | The smoke tests use specific args that don't trigger exit (help/version exit 0 via the normal return path); verify all smoke tests still pass without process termination |
| Namespacing collision between extracted modules and existing `init.rs` | New modules use disambiguating names: `dispatch.rs`, `help_text.rs`, `help_json.rs`, etc. No `init.rs`-style module collision risk since `init.rs` is already a separate file |
| Deferred types split creates incomplete feeling | Explicitly documented as deferred in proposal and tasks; creates a follow-up task on the swarm board |

## Traceability

| Source | Link |
|--------|------|
| Task prompt | task:50dfeadb-f2b9-4dab-adcd-8a079593fa5a — 'Code quality pass' |
| Exploration dossier | dossier:2026-06-21T20:42:39.522Z — identifies CLI lib.rs and types lib.rs as maintainability hotspots |
| Architect recommendation | round:1:agent:swarm-architect — 'Split crates/scryrs-cli/src/lib.rs (~3335 lines) by responsibility into focused modules' |
| Lead-dev recommendation | round:1:agent:swarm-lead-dev — 'Split into 6-8 small responsibility-focused modules, extract shared test support, defer scryrs-types' |
| Reviewer recommendation | round:1:agent:swarm-reviewer — 'Approve with targeted modifications: snapshot path migration, store_override visibility, keep fixture families separate' |
| AGENTS.md rule 12 | 'Prefer small files with one clear responsibility. Split files before they grow past ~1000 lines' |
| AGENTS.md rule 8 | 'Do not create abstractions merely because code looks similar. Duplication is better than the wrong abstraction.' |
| AGENTS.md rule 2 | 'Minimum code that solves the problem. Nothing speculative.' |
| Frozen CLI contract | `project-docs/cli-v0-contract.md` — defines the immutable contract this refactor must preserve |
| Existing `src/init.rs` | Establishes the project pattern for splitting CLI commands into separate files |