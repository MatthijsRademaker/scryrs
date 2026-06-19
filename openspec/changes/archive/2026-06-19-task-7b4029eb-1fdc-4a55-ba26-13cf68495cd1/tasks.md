## 1. Rewrite help text for standalone discovery

- [x] 1.1 Replace `write_help` function body with sectioned output: include USAGE, ARGUMENTS, OUTPUT (JSON envelope shape), EXAMPLES, OPTIONS, and EXIT CODES sections
- [x] 1.2 Remove "v0 placeholder contract" implementation-facing language from help text; replace with user-facing purpose description

## 2. Standardize error message format

- [x] 2.1 Update unknown command error to use consistent three-line pattern: `unknown command: '<name>'` + escalation to `--help`
- [x] 2.2 Update missing PATH error to include command context, usage line, and escalation: `scryrs hotspots: missing required PATH argument` + `Usage: scryrs hotspots <PATH>` + `See \`scryrs --help\``
- [x] 2.3 Update extra arguments error to include command context, usage line, and escalation: `scryrs hotspots: unexpected argument after PATH` + `Usage: scryrs hotspots <PATH>` + `See \`scryrs --help\``

## 3. Update tests for structural assertions

- [x] 3.1 Rewrite help-related tests (`help_flag_prints_help_and_exits_0`, `short_help_flag_prints_help_and_exits_0`, `bare_invocation_prints_help_and_exits_0`) to use structural assertions (check for section headers "USAGE", "EXAMPLES", "OPTIONS", "EXIT CODES")
- [x] 3.2 Update `hotspots_without_path_exits_2_with_error` test to match new error format (verify stderr contains command context, usage line, and escalation)
- [x] 3.3 Update `unknown_command_exits_2_with_error` test to match new error format (verify stderr contains command name and escalation)
- [x] 3.4 Update `hotspots_with_extra_args_exits_2_with_error` test to match new error format (verify stderr indicates extra args issue, contains usage line and escalation)
- [x] 3.5 Update `previously_stubbed_commands_exit_2` test to match new error format (verify each stubbed command produces formatted error with escalation)
- [x] 3.6 Verify `components_command_exits_2` test passes with new error format (already uses `contains("unknown command")` — confirm)

## 4. Validation

- [x] 4.1 Run `cargo test -p scryrs-cli` to confirm all updated tests pass
- [x] 4.2 Run `cargo check -p scryrs-cli` to confirm no dead code or warnings
- [x] 4.3 Verify `scryrs --help` output includes all required sections (USAGE, ARGUMENTS, OUTPUT, EXAMPLES, OPTIONS, EXIT CODES)
- [x] 4.4 Verify `scryrs hotspots /tmp` JSON output is unchanged
- [x] 4.5 Verify `scryrs hotspots` (no path) produces formatted error on stderr and exits 2
- [x] 4.6 Verify `scryrs unknowns` produces formatted error on stderr with escalation and exits 2
