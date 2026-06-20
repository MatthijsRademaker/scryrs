//! v0 CLI contract: `scryrs hotspots <PATH>` and `scryrs record --stdin|--file <PATH>`.

use std::io::{self, Read, Write};

use clap::{Arg, ArgAction, Command};
use scryrs_types::SCHEMA_VERSION;

#[cfg(feature = "core")]
use scryrs_core::{EventStore, ingest_jsonl};

#[cfg(feature = "core")]
mod store_override {
    use std::cell::RefCell;

    std::thread_local! {
        static PATH: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Set an override store path for the current thread (test-only).
    /// Subsequent calls to `execute_record` on this thread will use this
    /// path instead of `.scryrs/events.jsonl`.
    #[allow(dead_code)]
    pub fn set(path: String) {
        PATH.with(|p| *p.borrow_mut() = Some(path));
    }

    /// Get the override path, if set.
    pub fn get() -> Option<String> {
        PATH.with(|p| p.borrow().clone())
    }
}

/// Version of the `--help-json` surface document format, independent of
/// `SCHEMA_VERSION` which governs command output envelopes.
const SURFACE_VERSION: &str = "0.2.0";

pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    run_with_writers(args, io::stdout().lock(), io::stderr().lock())
}

pub fn run_with_writers<I, S, O, E>(args: I, out: O, err: E) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
{
    run_with_io(args, out, err, io::stdin().lock())
}

pub fn run_with_io<I, S, O, E, R>(args: I, mut out: O, mut err: E, mut stdin: R) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
    R: Read,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();

    // D1: Pre-clap normalization: root-level -hj -> --help-json
    if args.len() == 1 && args[0] == "-hj" {
        args[0] = "--help-json".to_string();
    }

    // D5: Pre-clap --help-json handling (not a clap flag)
    if args.len() == 1 && args[0] == "--help-json" {
        return write_cli_surface(&mut out).map_or(1, |_| 0);
    }

    // Unknown command check before clap dispatch.
    // Only known root-level entrypoints pass through to clap.
    // Everything else produces the contract error: "unknown command: 'X'".
    if !args.is_empty() {
        let first = &args[0];
        if first != "hotspots"
            && first != "record"
            && first != "--help"
            && first != "-h"
            && first != "--version"
            && first != "-V"
        {
            if writeln!(err, "unknown command: '{first}'").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                return 1;
            }
            return 2;
        }
    }

    // Capture the attempted subcommand before clap consumes args, so
    // error handlers can emit subcommand-specific messages.
    let attempted_command: Option<&str> =
        if !args.is_empty() && (args[0] == "hotspots" || args[0] == "record") {
            Some(args[0].as_str())
        } else {
            None
        };

    // D2: Clap builder API with try_get_matches_from (never get_matches_from).
    // Help/version flags enabled on root (so clap triggers DisplayHelp/DisplayVersion).
    // Disabled on the hotspots subcommand so --help/--version after hotspots are
    // rejected as unknown arguments (preserving the v0 contract).
    let cmd = Command::new("scryrs")
        .no_binary_name(true)
        .subcommand_required(false)
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(
            Command::new("hotspots")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(Arg::new("PATH").required(true).value_name("PATH")),
        )
        .subcommand(
            Command::new("record")
                .about("Ingest JSONL trace events from stdin or file")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("stdin")
                        .long("stdin")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Read JSONL events from stdin"),
                )
                .arg(
                    Arg::new("file")
                        .long("file")
                        .value_name("PATH")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Read JSONL events from PATH"),
                ),
        );

    match cmd.try_get_matches_from(&args) {
        Ok(matches) => {
            match matches.subcommand() {
                Some(("hotspots", _)) => write_hotspots_json(&mut out).map_or(1, |_| 0),
                Some(("record", m)) => execute_record(&mut out, &mut err, &mut stdin, m),
                // Bare invocation (no subcommand matched).
                _ => write_help(&mut out).map_or(1, |_| 0),
            }
        }
        Err(e) => match e.kind() {
            // D3/D4: Help and version — route to existing contract functions.
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::MissingSubcommand => {
                write_help(&mut out).map_or(1, |_| 0)
            }
            clap::error::ErrorKind::DisplayVersion => {
                writeln!(out, "scryrs {}", env!("CARGO_PKG_VERSION")).map_or(1, |_| 0)
            }
            // D4: Usage errors -> exit 2 with contract three-line format.
            clap::error::ErrorKind::MissingRequiredArgument => {
                if writeln!(err, "scryrs hotspots: missing required PATH argument").is_err()
                    || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                    || writeln!(err, "See `scryrs --help`").is_err()
                {
                    1
                } else {
                    2
                }
            }
            clap::error::ErrorKind::TooManyValues | clap::error::ErrorKind::UnknownArgument => {
                match attempted_command {
                    Some("record") => {
                        if writeln!(err, "scryrs record: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs record --stdin").is_err()
                            || writeln!(err, "Usage: scryrs record --file <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    _ => {
                        if writeln!(err, "scryrs hotspots: unexpected argument after PATH").is_err()
                            || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                }
            }
            // Unrecognized clap error -> exit 1.
            _ => 1,
        },
    }
}

fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs — context intelligence for AI-assisted codebases\n\n\
Discover, analyze, and navigate hotspots in your codebase.\n\n\
COMMANDS\n\
  scryrs hotspots <PATH>\n\
      Emit a versioned JSON placeholder for repository hotspots.\n\
  scryrs record --stdin\n\
      Ingest JSONL trace events from stdin.\n\
  scryrs record --file <PATH>\n\
      Ingest JSONL trace events from a file.\n\n\
RECORD OUTPUT\n\
  A single-line JSON summary on stdout:\n\
    {{\n\
      \"command\": \"record\",\n\
      \"schemaVersion\": \"{}\",\n\
      \"accepted\": <count>,\n\
      \"rejected\": <count>\n\
    }}\n\
  Rejection diagnostics are written as JSON objects to stderr,\n\
  one per rejected non-empty line.\n\n\
HOTSPOTS OUTPUT\n\
  A single-line JSON placeholder on stdout:\n\
    {{\n\
      \"schemaVersion\": \"{}\",\n\
      \"command\": \"hotspots\",\n\
      \"status\": \"placeholder\"\n\
    }}\n\n\
EXAMPLES\n\
  scryrs hotspots /path/to/repo\n\
  scryrs hotspots .\n\
  scryrs record --stdin < events.jsonl\n\
  scryrs record --file session.jsonl\n\n\
OPTIONS\n\
  -h, --help       Print this help message and exit\n\
  -V, --version    Print version and exit\n\
  -hj, --help-json Print machine-readable CLI surface description and exit\n\n\
EXIT CODES\n\
  0    Success (all events accepted for record; output written for all commands)\n\
  1    Record: one or more events rejected; I/O error (output could not be written)\n\
  2    Usage error (invalid arguments) or fatal I/O error (unreadable file)",
        SCHEMA_VERSION, SCHEMA_VERSION
    )
}

fn write_hotspots_json(out: &mut impl Write) -> io::Result<()> {
    write!(
        out,
        "{{\"schemaVersion\":\"{}\",\"command\":\"hotspots\",\"status\":\"placeholder\"}}",
        SCHEMA_VERSION
    )
}

#[cfg(feature = "core")]
fn execute_record<R: Read>(
    out: &mut impl Write,
    err: &mut impl Write,
    stdin: &mut R,
    m: &clap::ArgMatches,
) -> i32 {
    use std::fs::File;
    use std::io::BufReader;

    let use_stdin = m.get_flag("stdin");
    let file_path: Option<&String> = m.get_one::<String>("file");

    // Validate: exactly one of --stdin or --file must be specified.
    match (use_stdin, file_path) {
        (true, None) => { /* stdin mode */ }
        (false, Some(_)) => { /* file mode */ }
        (true, Some(_)) => {
            if writeln!(
                err,
                "scryrs record: --stdin and --file are mutually exclusive"
            )
            .is_err()
                || writeln!(err, "Usage: scryrs record --stdin").is_err()
                || writeln!(err, "Usage: scryrs record --file <PATH>").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                return 1;
            }
            return 2;
        }
        (false, None) => {
            if writeln!(
                err,
                "scryrs record: must specify one of --stdin or --file <PATH>"
            )
            .is_err()
                || writeln!(
                    err,
                    "Usage: scryrs record --stdin | scryrs record --file <PATH>"
                )
                .is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                return 1;
            }
            return 2;
        }
    }

    // Set up the input reader.
    let reader: Box<dyn std::io::BufRead> = if use_stdin {
        Box::new(BufReader::new(stdin))
    } else {
        let path = match file_path {
            Some(p) => p,
            None => {
                if writeln!(err, "scryrs record: internal error").is_err() {
                    return 1;
                }
                return 2;
            }
        };
        match File::open(path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                if writeln!(err, "scryrs record: cannot read {path}: {e}").is_err()
                    || writeln!(err, "See `scryrs --help`").is_err()
                {
                    return 1;
                }
                return 2;
            }
        }
    };

    // Ingest.
    let outcome = match ingest_jsonl(reader) {
        Ok(o) => o,
        Err(e) => {
            if writeln!(err, "scryrs record: I/O error while reading input: {e}").is_err() {
                return 1;
            }
            return 2;
        }
    };

    // Persist accepted events.
    let store_path = store_override::get().unwrap_or_else(|| ".scryrs/events.jsonl".into());
    let mut store = match EventStore::open(&store_path) {
        Ok(s) => s,
        Err(e) => {
            if writeln!(
                err,
                "scryrs record: cannot open event store ({store_path}): {e}"
            )
            .is_err()
            {
                return 1;
            }
            return 2;
        }
    };

    for event in &outcome.accepted {
        if let Err(e) = store.append(event) {
            if writeln!(err, "scryrs record: cannot persist event: {e}").is_err() {
                return 1;
            }
            return 2;
        }
    }

    let accepted = outcome.accepted.len();
    let rejected = outcome.rejected.len();

    // Summary to stdout.
    let summary = format!(
        r#"{{"command":"record","schemaVersion":"{}","accepted":{},"rejected":{}}}"#,
        SCHEMA_VERSION, accepted, rejected,
    );
    if writeln!(out, "{summary}").is_err() {
        return 1;
    }

    // Rejection diagnostics to stderr.
    for rejection in &outcome.rejected {
        let field_json = match &rejection.field {
            Some(f) => serde_json::to_string(f).unwrap_or_else(|_| "null".into()),
            None => "null".to_string(),
        };
        let reason_json = serde_json::to_string(&rejection.reason)
            .unwrap_or_else(|_| "\"<serialization error>\"".into());
        let diag = format!(
            r#"{{"line":{},"field":{},"reason":{}}}"#,
            rejection.line, field_json, reason_json,
        );
        if writeln!(err, "{diag}").is_err() {
            return 1;
        }
    }

    if rejected > 0 { 1 } else { 0 }
}

#[cfg(not(feature = "core"))]
fn execute_record<R: Read>(
    _out: &mut impl Write,
    err: &mut impl Write,
    _stdin: &mut R,
    _m: &clap::ArgMatches,
) -> i32 {
    let _ = writeln!(err, "scryrs record: unavailable (core feature not enabled)");
    2
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
            "}},",
            "{{",
            "\"name\":\"record\",",
            "\"description\":\"Ingest JSONL trace events from stdin or file\",",
            "\"modes\":[",
            "{{\"name\":\"stdin\",\"flag\":\"--stdin\",\"description\":\"Read JSONL events from stdin\"}},",
            "{{\"name\":\"file\",\"flag\":\"--file\",\"value\":\"PATH\",\"description\":\"Read JSONL events from PATH\"}}",
            "],",
            "\"output\":{{",
            "\"mimeType\":\"application/json\",",
            "\"fields\":[",
            "{{\"name\":\"command\",\"type\":\"string\",\"description\":\"Name of the executed command (always \\\"record\\\")\",\"optional\":false}},",
            "{{\"name\":\"schemaVersion\",\"type\":\"string\",\"description\":\"Version of the output envelope format\",\"optional\":false}},",
            "{{\"name\":\"accepted\",\"type\":\"number\",\"description\":\"Count of accepted events\",\"optional\":false}},",
            "{{\"name\":\"rejected\",\"type\":\"number\",\"description\":\"Count of rejected non-empty lines\",\"optional\":false}}",
            "]",
            "}},",
            "\"stderr\":{{",
            "\"mimeType\":\"application/jsonl\",",
            "\"description\":\"One JSON object per rejected non-empty line\",",
            "\"fields\":[",
            "{{\"name\":\"line\",\"type\":\"number\",\"description\":\"1‑based physical line number\"}},",
            "{{\"name\":\"field\",\"type\":\"string|null\",\"description\":\"Failing field/path when available\"}},",
            "{{\"name\":\"reason\",\"type\":\"string\",\"description\":\"Human-readable rejection reason\"}}",
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
            "\"exitCodes\":{{\"0\":\"Success (hotspots: JSON written; record: all events accepted)\",\"1\":\"Output error (hotspots); rejected events or output error (record)\",\"2\":\"Usage error (both commands) or fatal I/O error (record)\"}}",
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
        insta::assert_snapshot!(String::from_utf8_lossy(&out));
    }

    #[test]
    fn short_help_flag_produces_identical_output_to_long_help() {
        let mut out_long = Vec::new();
        let mut out_short = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out_long, &mut err), 0);
        assert!(err.is_empty());
        assert_eq!(run_with_writers(["-h"], &mut out_short, &mut err), 0);
        assert!(err.is_empty());
        assert_eq!(
            out_short, out_long,
            "-h must produce identical output to --help"
        );
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
    fn bare_invocation_produces_identical_output_to_help() {
        let mut out_help = Vec::new();
        let mut out_bare = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out_help, &mut err), 0);
        assert!(err.is_empty());
        assert_eq!(
            run_with_writers(Vec::<&str>::new(), &mut out_bare, &mut err),
            0
        );
        assert!(err.is_empty());
        assert_eq!(
            out_bare, out_help,
            "bare invocation must produce identical output to --help"
        );
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
        insta::assert_snapshot!(
            String::from_utf8_lossy(&out),
            @r#"{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}"#
        );
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
    fn record_with_help_flag_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["record", "--help"], &mut out, &mut err),
            2,
            "record --help must exit 2"
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
        assert!(
            err_str.contains("unexpected argument"),
            "must report unexpected argument, got: {err_str}"
        );
    }

    #[test]
    fn record_with_version_flag_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["record", "--version"], &mut out, &mut err),
            2,
            "record --version must exit 2"
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
    }

    // --- --help-json surface tests (CLI Foundation 04) ---

    #[test]
    fn help_json_flag_outputs_valid_json_and_exits_0() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        insta::assert_snapshot!(String::from_utf8_lossy(&out));
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
    fn help_json_does_not_interfere_with_existing_behavior() {
        // All existing commands and flags must still produce their expected output.
        // This test re-runs a representative subset to catch regressions.

        // --help still produces help text
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("COMMANDS"));

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
        assert!(String::from_utf8_lossy(&out).contains("COMMANDS"));

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
    fn hotspots_short_hj_exits_2() {
        // -hj after a subcommand is not normalized (normalization only at root level)
        // and is rejected as an invalid argument.
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "-hj"], &mut out, &mut err),
            2,
            "-hj after hotspots must exit 2 (normalization only at root level)"
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

#[cfg(all(test, feature = "core"))]
mod record_tests {
    use scryrs_types::SCHEMA_VERSION;

    use super::run_with_io;

    /// Set a thread-local store path override so tests don't pollute the
    /// real CWD's .scryrs/events.jsonl. Returns the tempdir guard.
    fn set_test_store() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let store_path = dir.path().join("events.jsonl");
        super::store_override::set(
            store_path
                .to_str()
                .unwrap_or_else(|| panic!("store path not valid UTF-8"))
                .to_string(),
        );
        dir
    }

    fn make_valid_event_json(session_id: &str, doc_ref: &str) -> String {
        format!(
            r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"{}","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"{}"}},"outcome":{{"result":"Success"}}}}"#,
            SCHEMA_VERSION, session_id, doc_ref
        )
    }

    // --- stdin ingestion ---

    #[test]
    fn record_stdin_all_valid_exits_0() {
        let _store_dir = set_test_store();
        let input = format!(
            "{}\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"command\":\"record\""));
        assert!(stdout.contains("\"accepted\":2"));
        assert!(stdout.contains("\"rejected\":0"));
        assert!(err.is_empty());
    }

    #[test]
    fn record_stdin_some_invalid_exits_1() {
        let _store_dir = set_test_store();
        let input = format!(
            "{}\nnot valid json\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
            1
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":2"));
        assert!(stdout.contains("\"rejected\":1"));

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("\"line\":2"));
        assert!(stderr.contains("\"reason\":"));
    }

    #[test]
    fn record_stdin_blank_lines_are_skipped() {
        let _store_dir = set_test_store();
        let input = format!(
            "\n{}\n\n{}\n  \n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":2"));
        assert!(stdout.contains("\"rejected\":0"));
        assert!(err.is_empty());
    }

    // --- file ingestion ---

    #[test]
    fn record_file_all_valid_exits_0() {
        let _store_dir = set_test_store();
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let file_path = dir.path().join("events.jsonl");
        let content = format!(
            "{}\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        if let Err(e) = std::fs::write(&file_path, content) {
            panic!("write test file: {e}");
        }

        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(
                ["record", "--file", &file_path.display().to_string()],
                &mut out,
                &mut err,
                stdin,
            ),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":2"));
        assert!(err.is_empty());
    }

    #[test]
    fn record_file_some_invalid_exits_1() {
        let _store_dir = set_test_store();
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let file_path = dir.path().join("events.jsonl");
        let content = format!(
            "{}\nbad line\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        if let Err(e) = std::fs::write(&file_path, content) {
            panic!("write test file: {e}");
        }

        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(
                ["record", "--file", &file_path.display().to_string()],
                &mut out,
                &mut err,
                stdin,
            ),
            1
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":2"));
        assert!(stdout.contains("\"rejected\":1"));

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("\"line\":2"));
    }

    // --- mutually exclusive input modes ---

    #[test]
    fn record_both_modes_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(
                ["record", "--stdin", "--file", "some.jsonl"],
                &mut out,
                &mut err,
                stdin,
            ),
            2
        );

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("mutually exclusive"));
        assert!(stderr.contains("See `scryrs --help`"));
    }

    #[test]
    fn record_neither_mode_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(run_with_io(["record"], &mut out, &mut err, stdin,), 2);

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("must specify one of --stdin or --file"));
        assert!(stderr.contains("See `scryrs --help`"));
    }

    // --- unreadable file ---

    #[test]
    fn record_unreadable_file_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(
                ["record", "--file", "/nonexistent/path/events.jsonl"],
                &mut out,
                &mut err,
                stdin,
            ),
            2
        );

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("cannot read"));
        assert!(stderr.contains("See `scryrs --help`"));
    }

    // --- deterministic output ---

    #[test]
    fn record_output_is_valid_json() {
        let _store_dir = set_test_store();
        let input = make_valid_event_json("s1", "doc/x.md");
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

        let out_str = String::from_utf8_lossy(&out);
        let summary: serde_json::Value = match serde_json::from_str(out_str.trim()) {
            Ok(v) => v,
            Err(e) => panic!("stdout must be valid JSON: {e}"),
        };
        assert_eq!(summary["command"], "record");
        assert_eq!(summary["accepted"], 1);
        assert_eq!(summary["rejected"], 0);
    }

    #[test]
    fn record_rejection_diagnostics_are_valid_json() {
        let _store_dir = set_test_store();
        let input = "not valid json\n";
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

        let stderr = String::from_utf8_lossy(&err);
        let diag: serde_json::Value = match serde_json::from_str(stderr.trim()) {
            Ok(v) => v,
            Err(e) => panic!("stderr must be valid JSON: {e}"),
        };
        assert_eq!(diag["line"], 1);
        assert!(diag["field"].is_null());
        assert!(diag["reason"].is_string());
    }

    #[test]
    fn record_multiple_rejections_all_on_stderr() {
        let _store_dir = set_test_store();
        let input = "bad1\nbad2\nbad3\n";
        let mut out = Vec::new();
        let mut err = Vec::new();

        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":0"));
        assert!(stdout.contains("\"rejected\":3"));

        let stderr = String::from_utf8_lossy(&err);
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(lines.len(), 3, "must have 3 rejection diagnostics");
        for (i, line) in lines.iter().enumerate() {
            let diag: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => panic!("each stderr line must be valid JSON: {e}"),
            };
            assert_eq!(diag["line"], (i + 1) as u64);
        }
    }

    // --- invalid arguments to record (error handler disambiguates subcommands) ---

    #[test]
    fn record_help_flag_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(["record", "--help"], &mut out, &mut err, stdin),
            2,
            "record --help must exit 2 (help flag disabled on subcommand)"
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
        assert!(
            err_str.contains("unexpected argument"),
            "must report unexpected argument, got: {err_str}"
        );
        assert!(
            err_str.contains("See `scryrs --help`"),
            "must escalate to --help, got: {err_str}"
        );
    }

    #[test]
    fn record_version_flag_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(["record", "--version"], &mut out, &mut err, stdin),
            2,
            "record --version must exit 2 (version flag disabled on subcommand)"
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
    }

    #[test]
    fn record_unknown_flag_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(["record", "--unknown-flag"], &mut out, &mut err, stdin),
            2
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
        assert!(
            err_str.contains("unexpected argument"),
            "must report unexpected argument, got: {err_str}"
        );
    }

    #[test]
    fn record_extra_args_exits_2() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let stdin = &[] as &[u8];

        assert_eq!(
            run_with_io(["record", "--stdin", "extra"], &mut out, &mut err, stdin,),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs record:"),
            "must name record, not hotspots, got: {err_str}"
        );
    }
}

#[cfg(test)]
mod smoke {
    use super::{run, run_with_writers};

    // Basic entrypoint smoke: verifies run() arg-collection wiring does not panic.
    #[test]
    fn public_run_entrypoint_no_panic() {
        assert_eq!(run(["--help"]), 0);
        assert_eq!(run(["--version"]), 0);
        assert_eq!(run(["--help-json"]), 0);
        assert_eq!(run(["hotspots", "/tmp"]), 0);
        assert_eq!(run(["record", "--file", "/nonexistent"]), 2);
        assert_eq!(run(Vec::<&str>::new()), 0);
        assert_eq!(run(["unknown"]), 2);
        assert_eq!(run(["hotspots"]), 2);
    }

    #[test]
    fn help_exits_0_stdout_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(!out.is_empty());
    }

    #[test]
    fn version_exits_0_stdout_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(!out.is_empty());
    }

    #[test]
    fn hotspots_path_exits_0_stdout_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());
        assert!(!out.is_empty());
    }

    #[test]
    fn bare_invocation_exits_0_stdout_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(!out.is_empty());
    }

    #[test]
    fn unknown_command_exits_2_stderr_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(!err.is_empty());
    }

    #[test]
    fn hotspots_without_path_exits_2_stderr_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(!err.is_empty());
    }

    #[test]
    fn help_json_exits_0_stdout_nonempty() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(!out.is_empty());
    }
}
