## Why

The `scryrs` CLI and the `xtask` repository automation tool both use hand-written argument parsers (`match args.as_slice()` and `args().nth(1)` respectively). This is brittle, non-idiomatic Rust, and adds maintenance burden as the CLI surface grows. Adopting `clap` — the standard Rust CLI library — replaces ad-hoc parsing with a well-tested, documented framework while preserving the frozen v0 CLI contract exactly.

## What Changes

- **`crates/scryrs-cli/src/lib.rs`**: Replace the `match args.as_slice()` parser with a `clap::Command` built via the builder API. A pre-clap normalization layer maps the non-standard `-hj` alias to `--help-json` before clap processes arguments. `try_get_matches_from` is used exclusively (never `get_matches_from`), preserving the writer-injected testability pattern. Clap error variants (`DisplayHelp`, `DisplayVersion`, usage errors) are mapped back to the contract's hand-crafted `write_help` output, exact three-line error format, and exit-code mapping (0/1/2). The `run_with_writers` signature is unchanged.
- **`xtask/src/main.rs`**: Replace the hand-written `match command { ... }` parser with clap subcommands (`help`, `bootstrap`, `ci-fast`) using the same builder-API + `try_get_matches_from` pattern.
- **`Cargo.toml` (root)**: Add `clap` to `[workspace.dependencies]` with a pinned major version.
- **`crates/scryrs-cli/Cargo.toml`** and **`xtask/Cargo.toml`**: Reference `clap` from the workspace dependency.
- **Tests**: All 20+ existing tests continue to pass with structurally equivalent assertions. A subset of assertion targets are updated for clap's error formatting where exact byte matching is not preserved.
- **Docs/Specs**: Any intentional behavior drift is updated in `.devagent/docs/docs/cli-v0-contract.md` and the relevant OpenSpec specs in the same change.

## Impact

- **Public CLI contract preserved**: Every documented entrypoint (bare invocation, `-h`/`--help`, `-V`/`--version`, `-hj`/`--help-json`, `hotspots <PATH>`, missing PATH, extra args, unknown commands) routes to the expected output stream and exit code.
- **No new commands**: The frozen single-command v0 surface is not expanded.
- **No output schema changes**: The JSON envelope and `--help-json` surface document are byte-identical to the pre-migration output.
- **Testability intact**: `run_with_writers` signature and writer-injected testing (no process exits) are preserved.
- **Two crates, one clap version**: Workspace dependency prevents version drift between `scryrs-cli` and `xtask`.