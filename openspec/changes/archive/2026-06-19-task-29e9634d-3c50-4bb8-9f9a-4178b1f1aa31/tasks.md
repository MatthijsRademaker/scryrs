## 1. Workspace Dependency

- [x] 1.1 Add `clap = { version = "4" }` to `[workspace.dependencies]` in root `Cargo.toml`
- [x] 1.2 Add `clap = { workspace = true }` to `crates/scryrs-cli/Cargo.toml` `[dependencies]`
- [x] 1.3 Add `clap = { workspace = true }` to `xtask/Cargo.toml` `[dependencies]`

## 2. scryrs-cli clap migration

- [x] 2.1 Add pre-clap normalization at top of `run_with_writers`: map root-level `-hj` to `--help-json` before constructing clap args vec
- [x] 2.2 Define `clap::Command` with builder API: root command with `no_binary_name(true)`, `subcommand_required(false)`, `disable_help_flag(true)`, `disable_version_flag(true)`; one `hotspots` subcommand with a required positional `PATH` argument
- [x] 2.3 Implement `try_get_matches_from` dispatch: on `Ok(matches)` with `hotspots` subcommand matched, call `write_hotspots_json`; on `Err(e)`, match `e.kind()`
- [x] 2.4 Handle `ErrorKind::DisplayHelp` and `ErrorKind::DisplayVersion` by routing to existing `write_help` and version output; exit 0 (or 1 on I/O error)
- [x] 2.5 Map clap usage errors (`InvalidSubcommand`, `MissingRequiredArgument`, `TooManyValues`, `UnknownArgument`) to contract's three-line error format on stderr; exit 2 (or 1 on I/O error)
- [x] 2.6 Handle bare invocation (empty args): before clap dispatch or via `DisplayHelp` on empty matches, route to `write_help`
- [x] 2.7 Handle `--help-json` as a standalone flag: check for `--help-json` (post-normalization) before clap dispatch when no subcommand matched; emit `cli_surface_doc()` and exit 0
- [x] 2.8 Ensure `hotspots --help-json` still exits 2: `--help-json` is not a clap global flag; it reaches the `hotspots` subcommand as an unrecognized positional argument and is rejected
- [x] 2.9 Ensure `hotspots -hj` exits 2 (same as above — `-hj` was normalized to `--help-json` only at root, not after a subcommand)
- [x] 2.10 Remove the old `match args.as_slice()` parser

## 3. xtask clap migration

- [x] 3.1 Define `clap::Command` with three subcommands: `help`, `bootstrap`, `ci-fast`
- [x] 3.2 Use `try_get_matches_from` and dispatch to existing `write_help` and stub output functions
- [x] 3.3 Preserve `unknown xtask command:` error format for invalid subcommands
- [x] 3.4 Remove old `match command { ... }` parser and `args().nth(1)` call
- [x] 3.5 Ensure existing test `unknown_command_exits_with_usage_error` passes

## 4. Test updates

- [x] 4.1 Update test assertions that assert exact error message strings if clap error formatting differs (e.g., `unknown command: 'X'` vs clap's default `error: unrecognized subcommand 'X'`)
- [x] 4.2 Ensure all 20+ existing tests in `crates/scryrs-cli/src/lib.rs` pass:
  - [x] `help_flag_prints_help_and_exits_0`
  - [x] `short_help_flag_prints_help_and_exits_0`
  - [x] `version_flag_prints_version_and_exits_0`
  - [x] `short_version_flag_prints_version_and_exits_0`
  - [x] `bare_invocation_prints_help_and_exits_0`
  - [x] `hotspots_with_path_emits_json_and_exits_0`
  - [x] `hotspots_without_path_exits_2_with_error`
  - [x] `unknown_command_exits_2_with_error`
  - [x] `components_command_exits_2`
  - [x] `hotspots_with_extra_args_exits_2_with_error`
  - [x] `help_json_flag_outputs_valid_json_and_exits_0`
  - [x] `short_hj_flag_works_identically`
  - [x] `surface_doc_contains_all_required_top_level_fields`
  - [x] `commands_array_has_exactly_one_entry_for_hotspots`
  - [x] `global_flags_array_has_exactly_three_entries`
  - [x] `exit_codes_object_has_correct_keys_and_descriptions`
  - [x] `root_behavior_has_action_help_and_exit_code_0`
  - [x] `help_json_does_not_interfere_with_existing_behavior`
  - [x] `help_json_is_idempotent`
  - [x] `help_json_after_command_exits_2`
  - [x] `previously_stubbed_commands_exit_2`
- [x] 4.3 Add a regression test for `hotspots -hj` exits 2 (normalization only applies at root level)
- [x] 4.4 Ensure xtask test `unknown_command_exits_with_usage_error` passes
- [x] 4.5 Run `cargo test -p scryrs-cli -p xtask` and confirm all tests pass
- [x] 4.6 Run `cargo clippy --workspace` and confirm no new warnings (use `#[allow]` for any clap-sourced unwrap patterns where safe)

## 5. Documentation and spec updates

- [x] 5.1 If error message format changes (structural equivalence, not byte-identical), update `openspec/specs/cli-discovery-ux/spec.md` to reflect new phrasing
- [x] 5.2 If `-hj` handling is preserved via normalization, no change to `cli-v0-contract.md`; if a deliberate contract break is chosen, update both the doc and `cli-machine-surface` spec
- [x] 5.3 Verify `openspec validate --strict` passes for the change
