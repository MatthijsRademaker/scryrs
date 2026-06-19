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
            if writeln!(err, "error: missing required PATH argument").is_err()
                || writeln!(err, "usage: scryrs hotspots <PATH>").is_err()
            {
                1
            } else {
                2
            }
        }
        [unknown, ..] => {
            if writeln!(err, "unknown command: {unknown}").is_err()
                || writeln!(err, "run `scryrs --help`").is_err()
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
        "scryrs - context intelligence for AI-assisted codebases\n\n\
Usage:\n\
  scryrs hotspots <PATH>\n\n\
scryrs hotspots emits a versioned JSON summary for the given repository path.\n\
This is a v0 placeholder contract; only this command is defined."
    )
}

fn write_hotspots_json(out: &mut impl Write) -> io::Result<()> {
    writeln!(
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
        assert!(output.contains("Usage:"));
        assert!(output.contains("hotspots <PATH>"));
    }

    #[test]
    fn short_help_flag_prints_help_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["-h"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("hotspots <PATH>"));
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
        assert!(String::from_utf8_lossy(&out).contains("hotspots <PATH>"));
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
        assert!(String::from_utf8_lossy(&err).contains("missing required PATH argument"));
    }

    #[test]
    fn unknown_command_exits_2_with_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown command: unknown"));
    }

    #[test]
    fn components_command_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["components"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown command: components"));
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
            assert!(
                String::from_utf8_lossy(&err).contains("unknown command"),
                "command '{cmd}' should produce unknown command error on stderr"
            );
        }
    }
}
