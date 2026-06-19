## Context

The repository has two hand-written Rust CLIs:

1. **`crates/scryrs-cli`** — the public `scryrs` binary. Parses arguments via `match args.as_slice()` with 9 branches covering bare invocation, `--help`/`-h`, `--version`/`-V`, `--help-json`/`-hj`, `hotspots <PATH>`, `hotspots` (missing PATH), `hotspots <PATH> <extra>`, and unknown commands. Emits hand-crafted help text (`write_help`), version string, JSON envelope (`write_hotspots_json`), and a machine-readable surface document (`cli_surface_doc`). Supports writer-injected testing via `run_with_writers<I, S, O, E>`.

2. **`xtask`** — repository automation binary. Parses `std::env::args().nth(1)` and matches on `"help"`, `"bootstrap"`, `"ci-fast"`, and unknown. Exposed via `.cargo/config.toml` aliases (`xtask` and `ci-fast`).

The frozen v0 CLI contract is documented across three OpenSpec specs (`cli-foundation-closure`, `cli-discovery-ux`, `cli-machine-surface`) and `.devagent/docs/docs/cli-v0-contract.md`. These specs codify non-default behaviors that diverge from clap's defaults: the `-hj` multi-character short flag, bare-invocation help rendering, custom help/error formatting, writer-injected testing without `process::exit`, and flags-after-subcommand rejection.

## Goals

- Replace the `match args.as_slice()` parser in `crates/scryrs-cli/src/lib.rs` with a clap-based parser using the builder API.
- Migrate `xtask/src/main.rs` from hand-parsed `args().nth(1)` to clap subcommands.
- Preserve the exact v0 CLI contract: all documented entrypoints, exit codes (0/1/2), output streams, error message format, `-hj` alias, and `--help-json` behavior.
- Keep `run_with_writers` signature unchanged; all existing tests must compile and pass (assertion targets may shift for structurally equivalent clap-generated error text).
- Add clap as a workspace dependency; both `scryrs-cli` and `xtask` reference it.

## Non-Goals

- Do not add new public `scryrs` commands beyond `hotspots`.
- Do not change the JSON envelope or `--help-json` surface document output.
- Do not add shell completions, man pages, or broader CLI redesign.
- Do not fold unrelated doc drift cleanup (e.g., stale `components` examples in `architecture.mdx`) into this migration unless parser changes force it.
- Do not create a shared CLI abstraction crate between `scryrs-cli` and `xtask`.

## Decisions

### D1: Pre-clap normalization layer for -hj

**Choice**: Add a normalization pass at the top of `run_with_writers` that rewrites root-level `-hj` to `--help-json` before handing arguments to clap's `try_get_matches_from`.

**Rationale**: clap enforces single-character short flags and cannot natively express `-hj`. The documented contract in `cli-v0-contract.md` and the `cli-machine-surface` spec requires `-hj` parity with `--help-json`. Normalization is a trivial string replacement that preserves the contract without fighting clap's API. Only `-hj` at the first position (root-level) is normalized — flags after a subcommand position are not rewritten, matching current behavior where `hotspots -hj` falls through to the argument parser and fails.

**Alternatives considered**: (a) Deliberate contract break removing `-hj` — rejected because it requires coordinated spec/doc updates and the team preferred preservation. (b) Clap custom flag parser — rejected as more complex than the normalization layer for a single alias.

### D2: Clap builder API with try_get_matches_from

**Choice**: Use clap's builder API (not derive macros), exclusively call `try_get_matches_from`, and manually handle `clap::error::Error` variants.

**Rationale**: The builder API keeps command definition explicit and does not pull in proc-macro dependencies. `try_get_matches_from` returns structured outcomes (`Ok(ArgMatches)` or `Err(Error)`) without calling `process::exit`, which maps cleanly to the `run_with_writers` return-value contract. The alternative `get_matches_from` would call `process::exit` on help/version/error, breaking writer-injected testing.

### D3: Custom help and error rendering (not clap defaults)

**Choice**: Clap's built-in help rendering is suppressed. `ErrorKind::DisplayHelp` and `ErrorKind::DisplayVersion` route to the existing `write_help` and version output functions. Usage errors are intercepted via `e.kind()` matching and re-formatted to the contract's three-line pattern (`problem line` / `Usage: scryrs hotspots <PATH>` / `See \`scryrs --help\``).

**Rationale**: The contract's help text has hand-crafted sections (USAGE, ARGUMENTS, OUTPUT, EXAMPLES, OPTIONS, EXIT CODES) with `SCHEMA_VERSION` interpolation that clap's default help template cannot reproduce without extensive customization. The bespoke error format (`unknown command: 'X'`, `missing required PATH argument`, `unexpected argument after PATH`) is tested by exact substring assertions. Routing through the existing functions avoids duplication and keeps the output source of truth in one place.

### D4: Exit code mapping

**Choice**: `DisplayHelp`/`DisplayVersion` → 0; clap usage errors (`ErrorKind::InvalidSubcommand`, `ErrorKind::MissingRequiredArgument`, `ErrorKind::TooManyValues`, `ErrorKind::UnknownArgument`) → 2; I/O errors from writer operations and any unrecognized clap error → 1.

**Rationale**: Matches the documented exit-code policy: 0 for success/help/version, 2 for usage errors, 1 for unexpected failures.

### D5: hotspots --help-json exits 2

**Choice**: `--help-json` is NOT registered as a clap global flag on the root `Command`. Instead, it is handled by the pre-clap normalization pass (which maps `-hj` to `--help-json`) and a manual check before subcommand dispatch: if `--help-json` is the only argument, emit the surface doc; if a subcommand is matched, `--help-json` as a subsequent positional argument is rejected by the subcommand's argument parser as an unexpected value after PATH.

**Rationale**: If `--help-json` were registered as a clap global flag, clap could intercept it before the `hotspots` subcommand validates its positional argument, breaking the `hotspots --help-json → exit 2` contract. Treating `--help-json` as a standalone check before clap dispatch preserves exact current behavior.

### D6: Clap as workspace dependency

**Choice**: Add clap (major version 4.x) to `[workspace.dependencies]` in root `Cargo.toml`. Both `crates/scryrs-cli/Cargo.toml` and `xtask/Cargo.toml` reference it via `clap = { workspace = true }`.

**Rationale**: Prevents version drift between the two crates. Follows standard Rust workspace practice. Does not require a shared abstraction crate — each binary defines its own `Command` independently.

### D7: xtask migration scope

**Choice**: Migrate xtask to clap in the same change. Define three clap subcommands (`help`, `bootstrap`, `ci-fast`) using the same `try_get_matches_from` pattern.

**Rationale**: The task explicitly says "Adjust any existing cli tooling or helpers to clap." xtask has only 3 commands and 1 test — migration is trivial. Both architect and lead-dev decisions support inclusion. The reviewer's preference to defer is noted and overridden because the scope is small and the task prompt requires it.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Clap error format diverges from contract on new clap version | Low | Medium | Pin clap major version in workspace dependency; tests assert exact error format |
| Pre-clap -hj normalization matches a path starting with `-hj` | Low | Low | Status-quo risk: current parser also treats `-hj` as a flag unconditionally at root level |
| Workspace lint rules (`deny unwrap_used`, `deny expect_used`) block clap error-handling code | Medium | High | Use proper `match` patterns or explicit `#[allow]` annotations on clap result handling |
| Migration inflates lib.rs beyond its current 369 lines | Low | Low | Builder API produces ~50 extra lines for Command definition and error mapping; acceptable |
| `hotspots --help-json` handling breaks if clap normalizes `--help-json` before subcommand dispatch | Medium | High | Prevented by D5: `--help-json` is not a clap global flag, handled by pre-clap check |

## Traceability

- **Task**: `task:29e9634d-3c50-4bb8-9f9a-4178b1f1aa31`
- **Dossier**: `dossier:2026-06-19T22:10:48.321Z`
- **Decisions**: `decision:1-swarm-architect-recommendation`, `decision:1-swarm-lead-dev-recommendation`, `decision:1-swarm-reviewer-recommendation`
- **Round outputs**: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- **Evidence files**: `crates/scryrs-cli/src/lib.rs`, `crates/scryrs-cli/src/main.rs`, `xtask/src/main.rs`, `Cargo.toml`, `.cargo/config.toml`
- **Contract sources**: `openspec/specs/cli-foundation-closure/spec.md`, `openspec/specs/cli-discovery-ux/spec.md`, `openspec/changes/task-cc52db89-f03d-4461-9b94-d93e237a8f99/specs/cli-machine-surface/spec.md`, `.devagent/docs/docs/cli-v0-contract.md`