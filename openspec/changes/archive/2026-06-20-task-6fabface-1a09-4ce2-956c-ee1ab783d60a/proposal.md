## Why

The repo now has two reference hook implementations (`hooks/claude-code/` and `hooks/pi/`), but installation is still manual, different per harness, and absent from the CLI surface. Harness integrators currently have to hand-copy files and hand-edit consumer config, which conflicts with the roadmap requirement for deterministic, scriptable hook setup and creates real risk of improvised installs or accidental writes into the reference-source repo itself. The Phase 1 roadmap explicitly requires `scryrs init --agent <name>` as the installer entrypoint that makes hook setup repeatable and scriptable.

## What Changes

1. **Add `scryrs init --agent <name>` subcommand** implemented in a separate `crates/scryrs-cli/src/init.rs` module backed by a typed harness registry for `claude-code` and `pi`. The installer embeds reference hook source artifacts at compile time via `include_str!()` so the binary is self-contained for distribution.

2. **Wire `init` into CLI dispatch** by adding `"init"` to the pre-clap command whitelist in `crates/scryrs-cli/src/lib.rs`, defining `init` as a clap subcommand with a required `--agent <NAME>` argument, and routing successful dispatch to `execute_init` in the installer module.

3. **Bump `SURFACE_VERSION` from `0.2.0` to `0.3.0`** and add the `init` command entry to `cli_surface_doc()` with argument metadata, output contract description, and exit code semantics.

4. **Update `write_help()`** to document the `init --agent <name>` subcommand, supported harnesses, and installation behavior.

5. **Update project documentation:** README.md ("The CLI ships two commands" → reflects three commands), `.devagent/docs/docs/cli-v0-contract.md` (unknown commands section + init contract), `.devagent/docs/docs/trace-hook-contract.md` (remove "manual process pending" language and "forthcoming Phase 1 deliverable" for Pi hook, since the installer now exists).

6. **Add tempdir-based tests** covering: supported harness installs, unsupported harness error, pre-existing file collision behavior, self-install boundary refusal, deterministic stdout/stderr, and source-repo boundary protection.

## Capabilities

### New Capabilities

- `init-installer`: Deterministic CLI entrypoint `scryrs init --agent <name>` that installs reference hook files from compiled-in assets into consumer-side project-local locations for supported harnesses (`claude-code`, `pi`), with loud refusal for unsupported harnesses, pre-existing target collisions, and self-install attempts.

### Modified Capabilities

- `cli-clap-migration`: The pre-clap command whitelist gains `"init"`; the clap `Command` definition gains an `init` subcommand.
- `cli-machine-surface`: `SURFACE_VERSION` bumps to `0.3.0`; the commands array gains an `init` entry.

## Impact

- **Code changes:** New file `crates/scryrs-cli/src/init.rs` (typed harness registry, `execute_init`, `include_str!` asset embedding). Modifications to `crates/scryrs-cli/src/lib.rs` (pre-clap whitelist, clap subcommand definition, `write_help()`, `cli_surface_doc()`, `SURFACE_VERSION`). Modifications to `crates/scryrs-cli/src/lib.rs` tests (add init to `previously_stubbed_commands_exit_2` exclusion, add installer tests, snapshot updates).
- **No contract changes to existing behavior:** All existing `record` and `hotspots` behavior, exit codes, JSON output format, error messages, and tests remain unchanged.
- **No changes to `scryrs-types`, `scryrs-core`, or any other workspace crate.**
- **No changes to hook business logic** — `hooks/claude-code/scryrs-hook.mjs` and `hooks/pi/index.ts` are source artifacts only; their contents are embedded but their logic is not modified.
- **Docs changes:** README.md, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/trace-hook-contract.md`.
- **No `scryrs.json` is created or required** — the installer does not create, read, or depend on the provisional manifest.