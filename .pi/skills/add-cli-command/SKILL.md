---
name: add-cli-command
description: "Recipe for adding or modifying a subcommand of the scryrs CLI. Use when: the task mentions crates/scryrs-cli, a new `scryrs <command>`, a CLI flag, help text, --help-json, or dispatch.rs. Do not use for: general testing workflow (testing-this-repo) or graph/type contract changes (graph-core-changes)."
metadata:
  curated: true
  sources:
    - crates/scryrs-cli/**
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Add a CLI Command

The CLI uses clap v4 **builder API** (no derive, no `#[derive(Parser)]` enum) with hand-rolled dispatch. A new command must be registered in **several disconnected places** — most are NOT compiler-enforced, and skipping one produces wrong behavior, not a compile error. Follow every step.

## Read first

1. `crates/scryrs-cli/src/dispatch.rs` — the root `Command::new("scryrs")` builder, the known-command allow-list, and all dispatch/error match arms. Everything wires through here.
2. `crates/scryrs-cli/src/graph.rs` — the exemplar to mirror: `write_graph_json(out, err, path) -> i32` signature, feature-gated impl + feature-off stub, inline tests.

## Steps

1. Create `crates/scryrs-cli/src/<cmd>.rs` mirroring `graph.rs`: a `pub(crate) fn write_<cmd>(out: &mut impl Write, err: &mut impl Write, ...) -> i32` returning exit codes 0/1/2. If it depends on an optional crate, gate with `#[cfg(feature = "...")]` and add a `#[cfg(not(feature = "..."))]` stub returning exit 2 (mirror `graph.rs`).
   Verify: `ls crates/scryrs-cli/src/<cmd>.rs`
2. Declare the module in `crates/scryrs-cli/src/lib.rs` (the `mod ...;` list near the top).
   Verify: `grep -n "mod <cmd>" crates/scryrs-cli/src/lib.rs` — one hit.
3. In `dispatch.rs`, wire **all** of: the `use crate::<cmd>::...` import; the unknown-command allow-list chain (`&& first != "<cmd>"`); the `attempted_command` capture (`|| args[0] == "<cmd>"`); the `.subcommand(Command::new("<cmd>")...)` builder block; the success match arm `Some(("<cmd>", m)) => ...`; and both error arms (`MissingRequiredArgument` and `TooManyValues | UnknownArgument`).
   Verify: `grep -cn '"<cmd>"' crates/scryrs-cli/src/dispatch.rs` — expect ≥6 hits. If you skipped the allow-list, clap never sees your command: it's rejected as `unknown command` even though the builder registered it.
4. Update **both** hand-maintained help surfaces: add a COMMANDS + EXAMPLES entry in `crates/scryrs-cli/src/help_text.rs`, and a JSON object in the `"commands"` array in `crates/scryrs-cli/src/help_json.rs`. Bump `SURFACE_VERSION` in `help_json.rs` — a test pins the exact value.
   Verify: `grep -n "<cmd>" crates/scryrs-cli/src/help_text.rs crates/scryrs-cli/src/help_json.rs` — hits in both files. Then `grep -n "surfaceVersion" crates/scryrs-cli/src/dispatch_tests.rs` and update the pinned value to match.
5. Add the command to `available_commands()` in `crates/scryrs-cli/src/doctor.rs` (a hand-maintained vec that `scryrs doctor` reports; it does not fail compile when forgotten).
   Verify: `grep -n '"<cmd>"' crates/scryrs-cli/src/doctor.rs` — one hit.
6. Add dispatch tests in `crates/scryrs-cli/src/dispatch_tests.rs` (call `run_with_writers([...], &mut out, &mut err)`, assert exit code + output; mirror the existing `graph` test block). Deeper behavior tests go in an inline `#[cfg(test)] mod tests` in your command file.
   Verify: `grep -n "<cmd>" crates/scryrs-cli/src/dispatch_tests.rs` — hits present.
7. Regenerate the two golden help snapshots — any help change breaks them byte-for-byte. Through Docker: `source scripts/lib/docker-verification.sh && run_rust env INSTA_UPDATE=always cargo test -p scryrs-cli --locked`, then re-run plain to confirm green.
   Verify: `git status --short crates/scryrs-cli/src/snapshots/` — both `.snap` files modified.
8. Document the command in `.devagent/docs/docs/cli-v0-contract.md`.
   Verify: `grep -n "<cmd>" .devagent/docs/docs/cli-v0-contract.md` — hit present.
9. Run full verification: `scripts/precommit-run` (never host cargo).

## Easy-to-miss details

- **The allow-list and builder are separate registries.** Registering the `.subcommand(...)` without the allow-list guard means exit 2 `unknown command`. Both must change (step 3).
- **`doctor.rs` `available_commands()` rots silently** — it was already missing `setup` and `up` when this skill was written. Your command makes it worse unless you add it (step 5).
- **`SURFACE_VERSION` is pinned in a test.** Bumping it in `help_json.rs` without updating the assertion in `dispatch_tests.rs` (or vice versa) fails CI.
- **Some commands are intercepted pre-clap** by raw string matching near the top of `dispatch.rs` (`proposals`, `doctor`, `route explain`). If your command needs nested subcommands or custom help, mirror `route`, not `graph`.
- **Feature-gate both arms**: CI runs `--all-features`, but the default build must still link — the feature-off stub (step 1) is mandatory for gated commands.

## Self-check before reporting done

- [ ] `grep -c '"<cmd>"' crates/scryrs-cli/src/dispatch.rs` ≥ 6?
- [ ] Both help surfaces mention the command, and `SURFACE_VERSION` + its pinned test assertion match?
- [ ] `doctor.rs` `available_commands()` includes it?
- [ ] Both golden `.snap` files under `crates/scryrs-cli/src/snapshots/` regenerated?
- [ ] `scripts/precommit-run` green?
