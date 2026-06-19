//! v0 CLI contract: single placeholder command `scryrs hotspots <PATH>`.

use std::io::{self, Write};

use scryrs_types::SCHEMA_VERSION;

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
        [command, _path] if command == "hotspots" => write_hotspots_json(&mut out).map_or(1, |_| 0),
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
