# cli-clap-migration Specification

## Purpose
TBD - created by archiving change task-29e9634d-3c50-4bb8-9f9a-4178b1f1aa31. Update Purpose after archive.
## Requirements
### Requirement: scryrs-cli parses arguments via clap builder API

The `run_with_writers` function SHALL use `clap::Command` built via the builder API and `try_get_matches_from` to parse arguments, replacing the current `match args.as_slice()` hand-written parser.

#### Scenario: clap Command is defined with builder API

- **GIVEN** the implementation at `crates/scryrs-cli/src/lib.rs`
- **WHEN** inspecting the argument parsing code
- **THEN** a `clap::Command` is constructed using the builder API (not derive macros)
- **AND** the Command uses `no_binary_name(true)` to match the existing calling convention where `main.rs` skips the binary name
- **AND** the Command has exactly one subcommand: `hotspots`

#### Scenario: try_get_matches_from is used exclusively

- **GIVEN** the argument parsing code
- **WHEN** clap is invoked to parse arguments
- **THEN** `try_get_matches_from` is called (never `get_matches_from`)
- **AND** the return value is matched as `Ok(ArgMatches)` or `Err(clap::error::Error)`
- **AND** `process::exit` is never called during parsing

#### Scenario: run_with_writers signature is unchanged

- **GIVEN** the public API in `crates/scryrs-cli/src/lib.rs`
- **WHEN** inspecting `run_with_writers`
- **THEN** the signature is `fn run_with_writers<I, S, O, E>(args: I, out: O, err: E) -> i32`
- **AND** all existing test call sites compile without modification

### Requirement: -hj alias is preserved via pre-clap normalization

The documented `-hj` short form for `--help-json` SHALL be handled by normalizing root-level `-hj` to `--help-json` before arguments reach clap.

#### Scenario: -hj at root level produces identical output to --help-json

- **WHEN** `run_with_writers(["-hj"], ...)` is invoked
- **THEN** the output SHALL be byte-identical to `run_with_writers(["--help-json"], ...)`
- **AND** the exit code SHALL be 0
- **AND** the existing test `short_hj_flag_works_identically` SHALL pass

#### Scenario: -hj after a subcommand is not normalized and exits 2

- **WHEN** `run_with_writers(["hotspots", "-hj"], ...)` is invoked
- **THEN** the exit code SHALL be 2
- **AND** stderr SHALL contain an error indicating invalid arguments
- **AND** `-hj` SHALL NOT be rewritten to `--help-json` (normalization only at root level)

#### Scenario: Normalization is a pure string replacement

- **GIVEN** the pre-clap normalization layer
- **WHEN** the argument list contains `-hj` at index 0 (the only argument)
- **THEN** `-hj` is replaced with `--help-json` before passing to clap
- **AND** no other arguments are modified

### Requirement: Help output preserves contract format

Help output from bare invocation and `--help`/`-h` SHALL match the existing hand-crafted format including all section headers.

#### Scenario: --help produces sectioned help text

- **WHEN** `run_with_writers(["--help"], ...)` is invoked
- **THEN** the output SHALL contain the sections USAGE, ARGUMENTS, OUTPUT, EXAMPLES, OPTIONS, EXIT CODES
- **AND** the output SHALL contain the `SCHEMA_VERSION` value
- **AND** the exit code SHALL be 0
- **AND** the existing test `help_flag_prints_help_and_exits_0` SHALL pass

#### Scenario: Bare invocation produces help text

- **WHEN** `run_with_writers` is invoked with an empty argument list
- **THEN** help text SHALL be written to stdout
- **AND** the exit code SHALL be 0
- **AND** the existing test `bare_invocation_prints_help_and_exits_0` SHALL pass

#### Scenario: clap's built-in help rendering is suppressed

- **GIVEN** the clap Command definition
- **WHEN** `--help` is parsed by clap
- **THEN** clap's `ErrorKind::DisplayHelp` is intercepted
- **AND** the existing `write_help` function is called (not clap's default help template)

### Requirement: Error messages follow contract's three-line format

Usage error messages SHALL follow the existing three-line format: a problem description line, a `Usage: scryrs hotspots <PATH>` line, and a `See \`scryrs --help\`` line.

#### Scenario: Unknown command error format

- **WHEN** `run_with_writers(["unknown"], ...)` is invoked
- **THEN** the exit code SHALL be 2
- **AND** stderr SHALL contain `unknown command: 'unknown'`
- **AND** stderr SHALL contain `See \`scryrs --help\``
- **AND** the existing test `unknown_command_exits_2_with_error` SHALL pass

#### Scenario: Missing PATH error format

- **WHEN** `run_with_writers(["hotspots"], ...)` is invoked
- **THEN** the exit code SHALL be 2
- **AND** stderr SHALL contain `scryrs hotspots:`
- **AND** stderr SHALL contain `missing required PATH argument`
- **AND** stderr SHALL contain `Usage: scryrs hotspots <PATH>`
- **AND** stderr SHALL contain `See \`scryrs --help\``
- **AND** the existing test `hotspots_without_path_exits_2_with_error` SHALL pass

#### Scenario: Extra arguments error format

- **WHEN** `run_with_writers(["hotspots", "/tmp", "extra"], ...)` is invoked
- **THEN** the exit code SHALL be 2
- **AND** stderr SHALL contain `unexpected argument after PATH`
- **AND** stderr SHALL contain `Usage: scryrs hotspots <PATH>`
- **AND** stderr SHALL contain `See \`scryrs --help\``
- **AND** the existing test `hotspots_with_extra_args_exits_2_with_error` SHALL pass

### Requirement: Exit code mapping matches documented policy

Clap error variants SHALL be mapped to exit codes matching the documented 0/1/2 policy.

#### Scenario: DisplayHelp and DisplayVersion exit 0

- **WHEN** clap returns `ErrorKind::DisplayHelp` or `ErrorKind::DisplayVersion`
- **THEN** the corresponding output SHALL be written to stdout
- **AND** the exit code SHALL be 0

#### Scenario: Usage errors exit 2

- **WHEN** clap returns any usage error (`InvalidSubcommand`, `MissingRequiredArgument`, `TooManyValues`, `UnknownArgument`, or any unrecognized clap error)
- **THEN** the error SHALL be formatted and written to stderr
- **AND** the exit code SHALL be 2

#### Scenario: I/O errors from writer operations exit 1

- **WHEN** any writer operation (stdout or stderr) fails during output
- **THEN** the exit code SHALL be 1

### Requirement: hotspots --help-json exits 2

The behavior where `scryrs hotspots --help-json` exits 2 with a usage error SHALL be preserved.

#### Scenario: --help-json after hotspots is rejected

- **WHEN** `run_with_writers(["hotspots", "--help-json"], ...)` is invoked
- **THEN** the exit code SHALL be 2
- **AND** stdout SHALL be empty
- **AND** stderr SHALL contain `unexpected argument after PATH`
- **AND** the existing test `help_json_after_command_exits_2` SHALL pass

#### Scenario: --help-json is not a clap global flag

- **GIVEN** the clap Command definition
- **WHEN** inspecting the registered flags
- **THEN** `--help-json` SHALL NOT be registered as a clap flag (global or subcommand-level)
- **AND** `--help-json` handling SHALL occur via a check before clap subcommand dispatch

### Requirement: xtask migrates to clap subcommands

The `xtask` binary SHALL use clap subcommands instead of hand-parsing `std::env::args().nth(1)`.

#### Scenario: xtask defines three clap subcommands

- **GIVEN** `xtask/src/main.rs`
- **WHEN** inspecting the argument parsing code
- **THEN** a `clap::Command` is defined with subcommands `help`, `bootstrap`, and `ci-fast`
- **AND** `try_get_matches_from` is used to parse arguments
- **AND** the old `match command { ... }` parser is removed

#### Scenario: xtask unknown command preserves error format

- **WHEN** an unknown subcommand is passed to xtask
- **THEN** the exit code SHALL be 2
- **AND** stderr SHALL contain `unknown xtask command:`
- **AND** the existing test `unknown_command_exits_with_usage_error` SHALL pass

#### Scenario: xtask help produces expected output

- **WHEN** `xtask help` (or bare `xtask`) is invoked
- **THEN** the output SHALL list `bootstrap` and `ci-fast` commands
- **AND** the exit code SHALL be 0

### Requirement: clap is declared as a workspace dependency

clap SHALL be declared in `[workspace.dependencies]` of the root `Cargo.toml` and referenced by both `scryrs-cli` and `xtask`.

#### Scenario: Workspace dependency entry exists

- **GIVEN** the root `Cargo.toml`
- **WHEN** inspecting `[workspace.dependencies]`
- **THEN** an entry `clap = { version = "4", features = [...] }` exists

#### Scenario: Both crates reference workspace clap

- **GIVEN** `crates/scryrs-cli/Cargo.toml` and `xtask/Cargo.toml`
- **WHEN** inspecting their `[dependencies]` sections
- **THEN** both contain `clap = { workspace = true }`

### Requirement: No new public subcommands are introduced

The migration SHALL NOT add any new public `scryrs` subcommands beyond the existing `hotspots` command.

#### Scenario: Only hotspots subcommand exists

- **GIVEN** the clap Command definition
- **WHEN** inspecting subcommands
- **THEN** exactly one subcommand exists: `hotspots`
- **AND** no `trace`, `graph`, `route`, `adapters`, `report`, `suggest-docs`, or `components` subcommands are defined

#### Scenario: Future-vision commands still exit 2

- **WHEN** any command from the future vision vocabulary (`trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`) is invoked
- **THEN** the exit code SHALL be 2
- **AND** the existing test `previously_stubbed_commands_exit_2` SHALL pass

### Requirement: --help-json surface document is byte-identical

The `--help-json` surface document SHALL remain byte-identical to its pre-migration output.

#### Scenario: Surface document is unchanged

- **WHEN** `run_with_writers(["--help-json"], ...)` is invoked
- **THEN** the output SHALL be byte-identical to the pre-migration `cli_surface_doc()` output
- **AND** the existing tests for `--help-json` surface content SHALL pass without modification

#### Scenario: cli_surface_doc remains the source of truth

- **GIVEN** the implementation
- **WHEN** `--help-json` or `-hj` (normalized) is requested
- **THEN** the existing `cli_surface_doc()` function SHALL be called to produce the output
- **AND** clap's command introspection SHALL NOT be used to derive the surface document

### Requirement: Writer-injected testing is preserved

All tests SHALL continue to use writer injection (`Vec<u8>` for stdout/stderr) without spawning subprocesses.

#### Scenario: run_with_writers accepts writer arguments

- **GIVEN** the test suite in `crates/scryrs-cli/src/lib.rs`
- **WHEN** any test constructs `Vec::new()` writers and calls `run_with_writers(args, &mut out, &mut err)`
- **THEN** the test compiles and runs
- **AND** assertions check captured `out` and `err` bytes
- **AND** no test calls `std::process::Command` or spawns a subprocess

#### Scenario: All existing tests pass

- **GIVEN** the test suite
- **WHEN** `cargo test -p scryrs-cli` is executed
- **THEN** all 20+ tests SHALL pass
- **AND** no test SHALL be removed unless its contract is deliberately modified and the corresponding spec is updated

