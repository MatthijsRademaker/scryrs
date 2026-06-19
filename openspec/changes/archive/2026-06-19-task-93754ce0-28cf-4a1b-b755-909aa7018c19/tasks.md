## 1. Verify and document closure

- [x] 1.1 Confirm binary target `scryrs` exists at `crates/scryrs-cli/Cargo.toml` with `[[bin]] name = "scryrs"`.
- [x] 1.2 Confirm `crates/scryrs-cli/src/main.rs` delegates to `scryrs_cli::run()`.
- [x] 1.3 Confirm `crates/scryrs-cli/src/lib.rs` argument parser accepts only `hotspots <PATH>`, `-h`/`--help`, `-V`/`--version`, and bare invocation.
- [x] 1.4 Confirm `write_hotspots_json()` emits deterministic versioned JSON with no backend calls.
- [x] 1.5 Confirm unknown commands, missing PATH, and extra arguments all exit 2 with stderr diagnostics.
- [x] 1.6 Confirm 11 unit tests pass and cover all acceptance scenarios.
- [x] 1.7 Confirm `README.md` and `.devagent/docs/docs/cli-v0-contract.md` reflect the single-command contract.

## 2. Track residual drift

- [x] 2.1 Acknowledge stale `cargo run -p scryrs-cli -- components` examples in `.devagent/docs/docs/architecture.mdx` (lines 99-101) as known deferred drift from prior change (task-9b98b3fd risk R4).
- [x] 2.2 Create or reference a separate follow-up task for architecture.mdx example cleanup (not part of this change). → Created task `8cb9aa32`.

## 3. Publish closure artifacts

- [x] 3.1 Publish completed `proposal.md`, `design.md`, `tasks.md`, and `specs/cli-foundation-closure/spec.md` through the Swarm Refinement Room.
