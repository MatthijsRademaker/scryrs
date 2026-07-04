---
name: add-adapter
description: "Recipe for adding a publishing adapter crate (accepted knowledge → a docs surface) to the scryrs workspace. Use when: the task mentions crates/scryrs-adapter-markdown, crates/scryrs-adapter-rspress, a new publishing target, or publish_accepted_*. Do not use for: harness trace-event translation in crates/scryrs-adapter-harness (that is a different 'adapter' — see crate-map) or CLI command wiring (add-cli-command)."
metadata:
  curated: true
  sources:
    - crates/scryrs-adapter-markdown/**
    - crates/scryrs-adapter-rspress/**
    - crates/scryrs-cli/Cargo.toml
    - Cargo.toml
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Add a Publishing Adapter

Two facts that break assumptions: **there is no adapter trait** — publishing adapters are plain-function crates following a copy-the-exemplar convention, and nothing at compile time enforces the shape. And **`scryrs-adapter-harness` is unrelated** — despite the name it translates agent-harness tool events, not publishing.

## Read first

1. `crates/scryrs-adapter-markdown/src/lib.rs` — the base exemplar: `descriptor() -> FeatureDescriptor`, `publish_accepted_markdown(repository_root, output_root) -> Result<Vec<_>, PublishError>`, crate-local `PublishError`, inline tests with byte-stability assertions.
2. `crates/scryrs-adapter-rspress/src/lib.rs` — the layered exemplar (calls markdown into a tempdir, then transforms); mirror this if your adapter builds on markdown output.

## Steps

1. Scaffold `crates/scryrs-adapter-<name>/` copying markdown's shape: `Cargo.toml` (deps: `scryrs-types.workspace = true`, `serde_json.workspace = true`; `[lints] workspace = true`) and `crates/scryrs-adapter-<name>/src/lib.rs` with `descriptor()`, `publish_accepted_<name>(...)`, and a `PublishError` type.
   Verify: `grep -n "fn descriptor\|fn publish_accepted\|PublishError" crates/scryrs-adapter-<name>/src/lib.rs` — all three present.
2. Register in the root workspace: add to `members` AND to `[workspace.dependencies]` in `Cargo.toml` (both required; forgetting the second breaks `workspace = true` resolution in consumers).
   Verify: `grep -cn "scryrs-adapter-<name>" Cargo.toml` — expect 2 hits.
3. Wire the CLI feature flag in `crates/scryrs-cli/Cargo.toml`: an optional dep line and a `<name> = ["dep:scryrs-adapter-<name>"]` feature. Decide default vs full-only (markdown is default; rspress is `full`-only). **Forgetting this compiles fine — the adapter just never links into the binary.**
   Verify: `grep -n "scryrs-adapter-<name>\|<name> = \[" crates/scryrs-cli/Cargo.toml` — dep + feature both present.
4. Add feature reporting to `available_features()` in `crates/scryrs-cli/src/doctor.rs` (hand-maintained; no compile error when forgotten).
   Verify: `grep -n 'feature = "<name>"' crates/scryrs-cli/src/doctor.rs` — one hit.
5. Write tests following the established assertions: byte-stable reruns (publish twice, compare dirs), pending-vs-accepted filtering, non-Markdown target types skipped, malformed artifact fails loudly with no partial output, missing accepted dir is a no-op success. Fixtures are built in-test with `tempfile::TempDir` (dev-dep `tempfile = "3"`) — there are no checked-in fixture files.
   Verify: `grep -cn "#\[test\]" crates/scryrs-adapter-<name>/src/lib.rs` — several hits.
6. Update the architecture model and docs (none of this fails the build): a crate node + edges in `.devagent/architecture/model.c4`, the crate tables in `.devagent/docs/docs/architecture.mdx`, and the suite row in `.devagent/docs/docs/production-suite.md`.
   Verify: `grep -n "adapter-<name>\|adapter_<name>" .devagent/architecture/model.c4 .devagent/docs/docs/architecture.mdx` — hits in both.
7. Run `scripts/precommit-run` (never host cargo). CI's real gate is `--all-features`, which your feature-gated code must pass.

## Easy-to-miss details

- **Workspace lints are strict deny**: `unwrap_used`, `expect_used`, `print_stdout/stderr`. Test modules opt out with `#![allow(clippy::unwrap_used, clippy::expect_used)]` at the top of `mod tests`; example binaries need `#![allow(clippy::print_stderr)]` (see `crates/scryrs-adapter-rspress/examples/verify-publish.rs`).
- **There is no runtime registry and no `scryrs publish` command today.** Adapters are invoked via example binaries and verify scripts (rspress via `scripts/verify-docs-publish`). Don't invent CLI wiring the task didn't ask for; if the task DOES ask for a command, switch to `add-cli-command`.
- **Determinism is a contract**: publishing the same input twice must produce byte-identical output. Sort everything you iterate.
- **`doctor.rs` feature list rots silently** — same failure mode as its command list.

## Self-check before reporting done

- [ ] Root `Cargo.toml` has both the member AND the workspace-dependency entry?
- [ ] CLI optional dep + feature flag exist, and the default/full decision is deliberate?
- [ ] `doctor.rs` `available_features()` reports the feature?
- [ ] Tests cover byte-stable rerun + malformed-input fail-loud?
- [ ] `scripts/precommit-run` green?
