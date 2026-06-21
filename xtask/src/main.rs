use std::io::{self, Write};

use clap::Command;

fn main() {
    let exit_code = run(
        std::env::args().skip(1),
        io::stdout().lock(),
        io::stderr().lock(),
    );
    std::process::exit(exit_code);
}

fn run<I, S>(args: I, mut out: impl Write, mut err: impl Write) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();

    // Unknown command check before clap dispatch.
    // Only known subcommands pass through to clap.
    // "help" is handled directly (clap reserves it as a built-in subcommand).
    if !args.is_empty() {
        let first = &args[0];
        if first == "help" {
            return write_help(&mut out).map_or(1, |_| 0);
        }
        if first != "bootstrap" && first != "ci-fast" {
            if writeln!(err, "unknown xtask command: {first}").is_err() {
                return 1;
            }
            return 2;
        }
    }

    let cmd = Command::new("xtask")
        .no_binary_name(true)
        .subcommand_required(true)
        .subcommand(Command::new("bootstrap"))
        .subcommand(Command::new("ci-fast"));

    match cmd.try_get_matches_from(&args) {
        Ok(matches) => match matches.subcommand() {
            Some(("bootstrap", _)) => writeln!(
                out,
                "xtask bootstrap: scaffold only; implementation pending"
            )
            .map_or(1, |_| 0),
            Some(("ci-fast", _)) => {
                writeln!(out, "xtask ci-fast: scaffold only; implementation pending")
                    .map_or(1, |_| 0)
            }
            _ => write_help(&mut out).map_or(1, |_| 0),
        },
        Err(e) => match e.kind() {
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::MissingSubcommand => {
                write_help(&mut out).map_or(1, |_| 0)
            }
            _ => 1,
        },
    }
}

fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "xtask commands:\n\
  bootstrap  prepare local developer environment (pending)\n\
  ci-fast    run fast local checks (pending)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_exits_with_usage_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run(["wat"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown xtask command: wat"));
    }

    #[test]
    fn help_subcommand_prints_help_and_exits_zero() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run(["help"], &mut out, &mut err), 0);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("xtask commands:"));
        assert!(output.contains("bootstrap"));
        assert!(output.contains("ci-fast"));
    }

    #[test]
    fn bootstrap_subcommand_exits_zero() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run(["bootstrap"], &mut out, &mut err), 0);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("scaffold only"));
        assert!(err.is_empty());
    }

    #[test]
    fn ci_fast_subcommand_exits_zero() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run(["ci-fast"], &mut out, &mut err), 0);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("scaffold only"));
        assert!(err.is_empty());
    }

    #[test]
    fn no_args_prints_help_and_exits_zero() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let args: [&str; 0] = [];
        assert_eq!(run(args, &mut out, &mut err), 0);
        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("xtask commands:"));
    }

    #[test]
    fn empty_string_arg_is_unknown_command() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run([""], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown xtask command:"));
    }
}
