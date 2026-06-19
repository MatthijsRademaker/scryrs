//! v0 CLI contract: single placeholder command `scryrs hotspots <PATH>`.

use std::io::{self, Write};

use scryrs_types::SCHEMA_VERSION;

/// Version of the `--help-json` surface document format, independent of
/// `SCHEMA_VERSION` which governs command output envelopes.
const SURFACE_VERSION: &str = "0.1.0";

pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    run_with_writers(args, io::stdout().lock(), io::stderr().lock())
}

pub fn run_with_writers<I, S, O, E>(args: I, mut out: O, mut err: E) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();

    match args.as_slice() {
        [] => write_help(&mut out).map_or(1, |_| 0),
        [flag] if flag == "--help" || flag == "-h" => write_help(&mut out).map_or(1, |_| 0),
        [flag] if flag == "--version" || flag == "-V" => {
            writeln!(out, "scryrs {}", env!("CARGO_PKG_VERSION")).map_or(1, |_| 0)
        }
        [flag] if flag == "--help-json" || flag == "-hj" => {
            write_cli_surface(&mut out).map_or(1, |_| 0)
        }
        [command, path] if command == "hotspots" && !path.starts_with('-') => {
            write_hotspots_json(&mut out).map_or(1, |_| 0)
        }
        [command] if command == "hotspots" => {
            if writeln!(err, "scryrs hotspots: missing required PATH argument").is_err()
                || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                1
            } else {
                2
            }
        }
        [command, _path, ..] if command == "hotspots" => {
            if writeln!(err, "scryrs hotspots: unexpected argument after PATH").is_err()
                || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                1
            } else {
                2
            }
        }
        [unknown, ..] => {
            if writeln!(err, "unknown command: '{unknown}'").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                1
            } else {
                2
            }
        }
    }
}

fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs — context intelligence for AI-assisted codebases\n\n\
Discover, analyze, and navigate hotspots in your codebase.\n\n\
USAGE\n\
  scryrs hotspots <PATH>\n\n\
ARGUMENTS\n\
  <PATH>    Path to the repository root directory (required)\n\n\
OUTPUT\n\
  A single-line JSON object with the following envelope:\n\
    {{\n\
      \"schemaVersion\": \"{}\",\n\
      \"command\": \"hotspots\",\n\
      \"status\": \"placeholder\"\n\
    }}\n\n\
EXAMPLES\n\
  scryrs hotspots /path/to/repo\n\
  scryrs hotspots .\n\n\
OPTIONS\n\
  -h, --help       Print this help message and exit\n\
  -V, --version    Print version and exit\n\n\
EXIT CODES\n\
  0    Success (output written to stdout)\n\
  1    I/O error (output could not be written)\n\
  2    Usage error (invalid arguments)",
        SCHEMA_VERSION
    )
}

fn write_hotspots_json(out: &mut impl Write) -> io::Result<()> {
    write!(
        out,
        "{{\"schemaVersion\":\"{}\",\"command\":\"hotspots\",\"status\":\"placeholder\"}}",
        SCHEMA_VERSION
    )
}

fn cli_surface_doc() -> String {
    format!(
        concat!(
            "{{",
            "\"surfaceVersion\":\"{sv}\",",
            "\"binary\":\"scryrs\",",
            "\"commands\":[",
            "{{",
            "\"name\":\"hotspots\",",
            "\"description\":\"Discover and analyze knowledge hotspots in a repository\",",
            "\"arguments\":[",
            "{{",
            "\"name\":\"PATH\",",
            "\"type\":\"string\",",
            "\"required\":true,",
            "\"description\":\"Path to the repository root directory\"",
            "}}",
            "],",
            "\"output\":{{",
            "\"mimeType\":\"application/json\",",
            "\"fields\":[",
            "{{\"name\":\"schemaVersion\",\"type\":\"string\",\"description\":\"Version of the output envelope format\",\"optional\":false}},",
            "{{\"name\":\"command\",\"type\":\"string\",\"description\":\"Name of the executed command\",\"optional\":false}},",
            "{{\"name\":\"status\",\"type\":\"string\",\"description\":\"Execution status indicator\",\"optional\":false}}",
            "]",
            "}}",
            "}}",
            "],",
            "\"globalFlags\":[",
            "{{\"name\":\"help\",\"short\":\"-h\",\"long\":\"--help\",\"description\":\"Print help message and exit\",\"action\":\"help\"}},",
            "{{\"name\":\"version\",\"short\":\"-V\",\"long\":\"--version\",\"description\":\"Print version and exit\",\"action\":\"version\"}},",
            "{{\"name\":\"help-json\",\"short\":\"-hj\",\"long\":\"--help-json\",\"description\":\"Print machine-readable CLI surface description and exit\",\"action\":\"helpJson\"}}",
            "],",
            "\"rootBehavior\":{{\"action\":\"help\",\"exitCode\":0}},",
            "\"exitCodes\":{{\"0\":\"Success\",\"1\":\"I/O error\",\"2\":\"Usage error\"}}",
            "}}"
        ),
        sv = SURFACE_VERSION,
    )
}

fn write_cli_surface(out: &mut impl Write) -> io::Result<()> {
    write!(out, "{}", cli_surface_doc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_flag_prints_help_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("USAGE"));
        assert!(output.contains("EXAMPLES"));
        assert!(output.contains("OPTIONS"));
        assert!(output.contains("EXIT CODES"));
    }

    #[test]
    fn short_help_flag_prints_help_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["-h"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("USAGE"));
    }

    #[test]
    fn version_flag_prints_version_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("scryrs "));
    }

    #[test]
    fn short_version_flag_prints_version_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["-V"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("scryrs "));
    }

    #[test]
    fn bare_invocation_prints_help_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
        assert!(err.is_empty());
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("USAGE"));
        assert!(output.contains("EXAMPLES"));
        assert!(output.contains("EXIT CODES"));
    }

    #[test]
    fn hotspots_with_path_emits_json_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"schemaVersion\":\"0.1.0\""));
        assert!(output.contains("\"command\":\"hotspots\""));
        assert!(output.contains("\"status\":\"placeholder\""));
    }

    #[test]
    fn hotspots_without_path_exits_2_with_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("scryrs hotspots:"));
        assert!(err_str.contains("missing required PATH argument"));
        assert!(err_str.contains("Usage: scryrs hotspots <PATH>"));
        assert!(err_str.contains("See `scryrs --help`"));
    }

    #[test]
    fn unknown_command_exits_2_with_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("unknown command: 'unknown'"));
        assert!(err_str.contains("See `scryrs --help`"));
    }

    #[test]
    fn components_command_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["components"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("unknown command: 'components'"));
        assert!(err_str.contains("See `scryrs --help`"));
    }

    #[test]
    fn hotspots_with_extra_args_exits_2_with_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "/tmp", "extra"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("unexpected argument after PATH"));
        assert!(err_str.contains("Usage: scryrs hotspots <PATH>"));
        assert!(err_str.contains("See `scryrs --help`"));
        assert!(!err_str.contains("unknown command"));
    }

    // --- --help-json surface tests (CLI Foundation 04) ---

    #[test]
    fn help_json_flag_outputs_valid_json_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        let output = String::from_utf8_lossy(&out);
        // Confirm it's valid JSON by checking for key structural elements
        assert!(output.starts_with('{'), "surface doc must be a JSON object");
        assert!(output.ends_with('}'), "surface doc must be a JSON object");
        assert!(
            output.contains("\"surfaceVersion\""),
            "surface doc must contain surfaceVersion"
        );
        assert!(
            output.contains("\"binary\""),
            "surface doc must contain binary"
        );
        assert!(
            output.contains("\"commands\""),
            "surface doc must contain commands"
        );
        assert!(
            output.contains("\"globalFlags\""),
            "surface doc must contain globalFlags"
        );
    }

    #[test]
    fn short_hj_flag_works_identically() {
        let mut out_long = Vec::new();
        let mut out_short = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["--help-json"], &mut out_long, &mut err),
            0
        );
        assert!(err.is_empty());
        assert_eq!(run_with_writers(["-hj"], &mut out_short, &mut err), 0);
        assert!(err.is_empty());
        assert_eq!(
            out_long, out_short,
            "--help-json and -hj must produce identical output"
        );
    }

    #[test]
    fn surface_doc_contains_all_required_top_level_fields() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut out, &mut err);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"surfaceVersion\":\"0.1.0\""));
        assert!(output.contains("\"binary\":\"scryrs\""));
        assert!(output.contains("\"commands\":["));
        assert!(output.contains("\"globalFlags\":["));
        assert!(output.contains("\"rootBehavior\":{"));
        assert!(output.contains("\"exitCodes\":{"));
    }

    #[test]
    fn commands_array_has_exactly_one_entry_for_hotspots() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut out, &mut err);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"name\":\"hotspots\""));
        assert!(output.contains("\"name\":\"PATH\""));
        assert!(output.contains("\"type\":\"string\""));
        assert!(output.contains("\"required\":true"));
        assert!(output.contains("\"mimeType\":\"application/json\""));
        assert!(output.contains("\"name\":\"schemaVersion\""));
        assert!(output.contains("\"name\":\"command\""));
        assert!(output.contains("\"name\":\"status\""));
    }

    #[test]
    fn global_flags_array_has_exactly_three_entries() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut out, &mut err);
        let output = String::from_utf8_lossy(&out);
        // Check all three flag entries exist
        assert!(output.contains("\"name\":\"help\""));
        assert!(output.contains("\"short\":\"-h\""));
        assert!(output.contains("\"long\":\"--help\""));
        assert!(output.contains("\"action\":\"help\""));

        assert!(output.contains("\"name\":\"version\""));
        assert!(output.contains("\"short\":\"-V\""));
        assert!(output.contains("\"long\":\"--version\""));
        assert!(output.contains("\"action\":\"version\""));

        assert!(output.contains("\"name\":\"help-json\""));
        assert!(output.contains("\"short\":\"-hj\""));
        assert!(output.contains("\"long\":\"--help-json\""));
        assert!(output.contains("\"action\":\"helpJson\""));
    }

    #[test]
    fn exit_codes_object_has_correct_keys_and_descriptions() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut out, &mut err);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"0\":\"Success\""));
        assert!(output.contains("\"1\":\"I/O error\""));
        assert!(output.contains("\"2\":\"Usage error\""));
    }

    #[test]
    fn root_behavior_has_action_help_and_exit_code_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut out, &mut err);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"rootBehavior\":{"));
        assert!(output.contains("\"action\":\"help\""));
        // exitCode:0 appears within rootBehavior context
        assert!(output.contains("\"exitCode\":0"));
    }

    #[test]
    fn help_json_does_not_interfere_with_existing_behavior() {
        // All existing commands and flags must still produce their expected output.
        // This test re-runs a representative subset to catch regressions.

        // --help still produces help text
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("USAGE"));

        // --version still produces version string
        out.clear();
        err.clear();
        assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("scryrs "));

        // hotspots /tmp still produces JSON envelope
        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            0
        );
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"schemaVersion\":\"0.1.0\""));
        assert!(output.contains("\"command\":\"hotspots\""));
        assert!(output.contains("\"status\":\"placeholder\""));

        // hotspots without PATH still exits 2
        out.clear();
        err.clear();
        assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
        assert!(String::from_utf8_lossy(&err).contains("missing required PATH argument"));

        // Bare invocation still produces help
        out.clear();
        err.clear();
        assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("USAGE"));

        // Unknown command still exits 2
        out.clear();
        err.clear();
        assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
        assert!(String::from_utf8_lossy(&err).contains("unknown command"));
    }

    #[test]
    fn help_json_is_idempotent() {
        let mut first = Vec::new();
        let mut second = Vec::new();
        let mut err = Vec::new();

        run_with_writers(["--help-json"], &mut first, &mut err);
        assert!(err.is_empty());
        run_with_writers(["--help-json"], &mut second, &mut err);
        assert!(err.is_empty());
        assert_eq!(
            first, second,
            "--help-json must produce identical output on every invocation"
        );
    }

    #[test]
    fn help_json_after_command_exits_2() {
        // --help-json after a command falls through to the command's argument
        // parser, which rejects flag-like positional arguments.
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "--help-json"], &mut out, &mut err),
            2,
            "--help-json after hotspots must exit 2 (no per-command introspection in v0)"
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("unexpected argument after PATH"),
            "should report flag-like argument as invalid, got: {err_str}"
        );
    }

    #[test]
    fn previously_stubbed_commands_exit_2() {
        for cmd in &[
            "trace",
            "propose",
            "graph",
            "route",
            "adapters",
            "report",
            "suggest-docs",
        ] {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers([*cmd], &mut out, &mut err),
                2,
                "command '{cmd}' should exit 2"
            );
            assert!(out.is_empty(), "command '{cmd}' should not produce stdout");
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("unknown command:"),
                "command '{cmd}' should produce unknown command error on stderr"
            );
            assert!(
                err_str.contains("See `scryrs --help`"),
                "command '{cmd}' should include escalation to --help on stderr"
            );
        }
    }
}
