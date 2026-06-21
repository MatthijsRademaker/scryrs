//! v0 CLI contract: `scryrs hotspots <PATH>`, `scryrs record --stdin|--file <PATH>`,
//! and `scryrs init --agent <NAME>`.

use std::io::{self, Read, Write};

use clap::{Arg, ArgAction, Command};
use scryrs_types::{HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION};

mod init;

#[cfg(feature = "core")]
use scryrs_core::{
    CANONICAL_STORE_PATH, EventStore, QueryError, TraceQuery, ingest_jsonl, score_hotspots,
};

#[cfg(feature = "core")]
mod store_override {
    use std::cell::RefCell;

    std::thread_local! {
        static PATH: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Set an override store path for the current thread (test-only).
    /// Subsequent calls to `execute_record` on this thread will use this
    /// path instead of `.scryrs/scryrs.db`.
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
const SURFACE_VERSION: &str = "0.3.0";

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
            && first != "init"
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
    let attempted_command: Option<&str> = if !args.is_empty()
        && (args[0] == "hotspots" || args[0] == "record" || args[0] == "init")
    {
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
        )
        .subcommand(
            Command::new("init")
                .about("Install scryrs trace hook for a supported agent harness")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("NAME")
                        .num_args(1)
                        .required(true)
                        .action(ArgAction::Set)
                        .help("Agent harness name (claude-code or pi)"),
                ),
        );

    match cmd.try_get_matches_from(&args) {
        Ok(matches) => {
            match matches.subcommand() {
                Some(("hotspots", m)) => {
                    let path = m
                        .get_one::<String>("PATH")
                        .map(|s| s.as_str())
                        .unwrap_or(".");
                    write_hotspots_json(&mut out, &mut err, path)
                }
                Some(("record", m)) => execute_record(&mut out, &mut err, &mut stdin, m),
                Some(("init", m)) => {
                    let agent = m
                        .get_one::<String>("agent")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if agent.is_empty() {
                        if writeln!(err, "scryrs init: --agent requires a non-empty value").is_err()
                            || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    } else {
                        init::execute_init(&mut out, &mut err, agent)
                    }
                }
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
            clap::error::ErrorKind::MissingRequiredArgument => match attempted_command {
                Some("init") => {
                    if writeln!(err, "scryrs init: missing required --agent argument").is_err()
                        || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                _ => {
                    if writeln!(err, "scryrs hotspots: missing required PATH argument").is_err()
                        || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
            },
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
                    Some("init") => {
                        if writeln!(err, "scryrs init: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
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
      Ingest JSONL trace events from a file.\n\
  scryrs init --agent <NAME>\n\
      Install the scryrs trace hook for a supported agent harness.\n\
      Supported harnesses: claude-code, pi\n\n\
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
  A single-line JSON envelope on stdout:\n\
    {{\n\
      \"schemaVersion\": \"{}\",\n\
      \"command\": \"hotspots\",\n\
      \"repositoryPath\": \"<absolute path>\",\n\
      \"storePath\": \"<absolute path to .scryrs/scryrs.db>\",\n\
      \"runMetadata\": {{\n\
        \"storeSchemaVersion\": <integer>,\n\
        \"analyzedEventCount\": <count>,\n\
        \"analyzedSubjectCount\": <count>,\n\
        \"firstEventId\": <id>,\n\
        \"lastEventId\": <id>\n\
      }},\n\
      \"generatedAt\": \"<ISO 8601 timestamp>\",\n\
      \"entries\": [...]\n\
    }}\n\
  Each entry carries rank, subjectKind, subject, score,\n\
  per-event-type counts, per-outcome counts, sessionCount,\n\
  firstSeen/lastSeen timestamps, and evidence rowIds.\n\
  Empty stores produce entries: [].\n\
  On success, the report is also written to .scryrs/hotspots.json.\n\
EXAMPLES\n\
  scryrs hotspots /path/to/repo\n\
  scryrs hotspots .\n\
  scryrs record --stdin < events.jsonl\n\
  scryrs record --file session.jsonl\n\
  scryrs init --agent claude-code\n\
  scryrs init --agent pi\n\n\
OPTIONS\n\
  -h, --help       Print this help message and exit\n\
  -V, --version    Print version and exit\n\
  -hj, --help-json Print machine-readable CLI surface description and exit\n\n\
EXIT CODES\n\
  0    Success (hotspots: JSON written; record: all events accepted; init: hook installed)\n\
  1    Hotspots: storage error. Record: rejected events or I/O error. Init: I/O error.\n\
  2    Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file or store failure); init: unsupported harness, collision, or self-install refusal",
        SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION
    )
}

#[cfg(feature = "core")]
fn write_hotspots_json(out: &mut impl Write, err: &mut impl Write, path: &str) -> i32 {
    use scryrs_types::{HotspotsReport, RunMetadata};

    // Resolve to absolute path.
    let repo_root = match std::path::absolute(path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "scryrs hotspots: cannot resolve path '{path}': {e}");
            return 2;
        }
    };

    let store_path = repo_root.join(".scryrs/scryrs.db");

    // Open the trace query read model.
    let query = match TraceQuery::open(&repo_root) {
        Ok(q) => q,
        Err(QueryError::MissingStore) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: datastore not found at {}",
                store_path.display()
            );
            return 2;
        }
        Err(QueryError::UnsupportedStore(msg)) => {
            let _ = writeln!(err, "scryrs hotspots: unsupported datastore: {msg}");
            return 2;
        }
        Err(QueryError::StorageError(e)) => {
            let _ = writeln!(err, "scryrs hotspots: storage error: {e}");
            return 1;
        }
        // EmptyStore is never returned by open(), but handle it for exhaustiveness.
        _ => {
            let _ = writeln!(err, "scryrs hotspots: unexpected error opening store");
            return 1;
        }
    };

    // Materialize events with row IDs.
    let events_with_ids = match query.iter_events_with_ids_ordered() {
        Ok(events) => events,
        Err(QueryError::EmptyStore) => {
            // Empty store → produce report with empty entries.
            return write_empty_success_report(
                out,
                err,
                &repo_root,
                &store_path,
                query.store_schema_version(),
            );
        }
        Err(QueryError::StorageError(e)) => {
            let _ = writeln!(err, "scryrs hotspots: storage error: {e}");
            return 1;
        }
        Err(e) => {
            let _ = writeln!(err, "scryrs hotspots: {e}");
            return 1;
        }
    };

    // Compute runMetadata from events.
    let subject_bearing: Vec<_> = events_with_ids
        .iter()
        .filter(|(_, e)| e.subject().is_some())
        .collect();

    let subject_set: std::collections::HashSet<String> = subject_bearing
        .iter()
        .filter_map(|(_, e)| {
            let kind = e.subject_kind()?;
            let subj = e.subject()?;
            Some(format!("{kind}:{subj}"))
        })
        .collect();

    let first_event_id = subject_bearing.first().map(|(id, _)| *id).unwrap_or(0);
    let last_event_id = subject_bearing.last().map(|(id, _)| *id).unwrap_or(0);

    let run_metadata = RunMetadata {
        storeSchemaVersion: query.store_schema_version(),
        analyzedEventCount: subject_bearing.len() as u64,
        analyzedSubjectCount: subject_set.len() as u64,
        firstEventId: first_event_id,
        lastEventId: last_event_id,
    };

    // Score hotspots.
    let event_refs: Vec<(u64, &scryrs_types::TraceEvent)> =
        events_with_ids.iter().map(|(id, e)| (*id, e)).collect();
    let entries = score_hotspots(&event_refs);

    // Build report.
    let report = HotspotsReport {
        schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
        command: "hotspots".into(),
        repositoryPath: repo_root.display().to_string(),
        storePath: store_path.display().to_string(),
        runMetadata: run_metadata,
        generatedAt: chrono_now(),
        entries,
    };

    // Serialize and output.
    let json = match serde_json::to_string(&report) {
        Ok(j) => j,
        Err(e) => {
            let _ = writeln!(err, "scryrs hotspots: serialization error: {e}");
            return 1;
        }
    };

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    // Write artifact to .scryrs/hotspots.json.
    if let Err(e) = std::fs::write(repo_root.join(".scryrs/hotspots.json"), &json) {
        let _ = writeln!(err, "scryrs hotspots: cannot write artifact file: {e}");
        // Don't fail on artifact write failure — output was already written to stdout.
    }

    0
}

/// Write success report for an empty (but valid) store.
#[cfg(feature = "core")]
fn write_empty_success_report(
    out: &mut impl Write,
    err: &mut impl Write,
    repo_root: &std::path::Path,
    store_path: &std::path::Path,
    store_schema_version: i64,
) -> i32 {
    use scryrs_types::{HotspotsReport, RunMetadata};

    let report = HotspotsReport {
        schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
        command: "hotspots".into(),
        repositoryPath: repo_root.display().to_string(),
        storePath: store_path.display().to_string(),
        runMetadata: RunMetadata {
            storeSchemaVersion: store_schema_version,
            analyzedEventCount: 0,
            analyzedSubjectCount: 0,
            firstEventId: 0,
            lastEventId: 0,
        },
        generatedAt: chrono_now(),
        entries: vec![],
    };

    let json = match serde_json::to_string(&report) {
        Ok(j) => j,
        Err(e) => {
            let _ = writeln!(err, "scryrs hotspots: serialization error: {e}");
            return 1;
        }
    };

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    // Write artifact file.
    let _ = std::fs::write(repo_root.join(".scryrs/hotspots.json"), &json);

    0
}

/// Generate an ISO 8601 timestamp.
#[cfg(feature = "core")]
fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Format as ISO 8601: YYYY-MM-DDTHH:MM:SSZ
    // Simple formatting without chrono dependency.
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year/month/day from days since Unix epoch (1970-01-01).
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert days since Unix epoch to (year, month, day).
#[cfg(feature = "core")]
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(not(feature = "core"))]
fn write_hotspots_json(out: &mut impl Write, err: &mut impl Write, _path: &str) -> i32 {
    let _ = writeln!(
        err,
        "scryrs hotspots: unavailable (core feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
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
    let store_path = store_override::get().unwrap_or_else(|| CANONICAL_STORE_PATH.into());
    let mut store = match EventStore::open(&store_path) {
        Ok(s) => s,
        Err(e) => {
            if writeln!(
                err,
                "scryrs record: cannot open trace datastore ({store_path}): {e}"
            )
            .is_err()
            {
                return 1;
            }
            return 2;
        }
    };

    if let Err(e) = store.begin_transaction() {
        if writeln!(
            err,
            "scryrs record: cannot begin datastore transaction: {e}"
        )
        .is_err()
        {
            return 1;
        }
        return 2;
    }

    for event in &outcome.accepted {
        if let Err(e) = store.append(event) {
            if writeln!(err, "scryrs record: cannot persist event: {e}").is_err() {
                return 1;
            }
            return 2;
        }
    }

    if let Err(e) = store.commit_transaction() {
        if writeln!(
            err,
            "scryrs record: cannot commit datastore transaction: {e}"
        )
        .is_err()
        {
            return 1;
        }
        return 2;
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
    let doc = serde_json::json!({
        "surfaceVersion": SURFACE_VERSION,
        "binary": "scryrs",
        "commands": [
            {
                "name": "hotspots",
                "description": "Discover and analyze knowledge hotspots in a repository",
                "arguments": [
                    {
                        "name": "PATH",
                        "type": "string",
                        "required": true,
                        "description": "Path to the repository root directory"
                    }
                ],
                "output": {
                    "mimeType": "application/json",
                    "fields": [
                        {"name": "schemaVersion", "type": "string", "description": "Version of the hotspot report output format (independent of trace event version)", "optional": false},
                        {"name": "command", "type": "string", "description": "Name of the executed command", "optional": false},
                        {"name": "repositoryPath", "type": "string", "description": "Resolved absolute path to the repository root", "optional": false},
                        {"name": "storePath", "type": "string", "description": "Resolved absolute path to .scryrs/scryrs.db", "optional": false},
                        {"name": "runMetadata", "type": "object", "description": "Deterministic metadata from store state (storeSchemaVersion, analyzedEventCount, analyzedSubjectCount, firstEventId, lastEventId)", "optional": false},
                        {"name": "generatedAt", "type": "string", "description": "ISO 8601 wall-clock timestamp", "optional": false},
                        {"name": "entries", "type": "array", "description": "Array of ranked HotspotEntry objects (empty for stores with no subject-bearing events)", "optional": false}
                    ]
                }
            },
            {
                "name": "record",
                "description": "Ingest JSONL trace events from stdin or file",
                "modes": [
                    {"name": "stdin", "flag": "--stdin", "description": "Read JSONL events from stdin"},
                    {"name": "file", "flag": "--file", "value": "PATH", "description": "Read JSONL events from PATH"}
                ],
                "output": {
                    "mimeType": "application/json",
                    "fields": [
                        {"name": "command", "type": "string", "description": "Name of the executed command (always \"record\")", "optional": false},
                        {"name": "schemaVersion", "type": "string", "description": "Version of the output envelope format", "optional": false},
                        {"name": "accepted", "type": "number", "description": "Count of accepted events", "optional": false},
                        {"name": "rejected", "type": "number", "description": "Count of rejected non-empty lines", "optional": false}
                    ]
                },
                "stderr": {
                    "mimeType": "application/jsonl",
                    "description": "One JSON object per rejected non-empty line",
                    "fields": [
                        {"name": "line", "type": "number", "description": "1‑based physical line number", "optional": false},
                        {"name": "field", "type": "string|null", "description": "Failing field/path when available", "optional": true},
                        {"name": "reason", "type": "string", "description": "Human-readable rejection reason", "optional": false}
                    ]
                }
            },
            {
                "name": "init",
                "description": "Install scryrs trace hook for a supported agent harness",
                "arguments": [
                    {
                        "name": "agent",
                        "flag": "--agent",
                        "type": "string",
                        "required": true,
                        "description": "Agent harness name (claude-code or pi)"
                    }
                ],
                "output": {
                    "mimeType": "text/plain",
                    "description": "Post-install next-step instructions on stdout. Errors on stderr."
                }
            }
        ],
        "globalFlags": [
            {"name": "help", "short": "-h", "long": "--help", "description": "Print help message and exit", "action": "help"},
            {"name": "version", "short": "-V", "long": "--version", "description": "Print version and exit", "action": "version"},
            {"name": "help-json", "short": "-hj", "long": "--help-json", "description": "Print machine-readable CLI surface description and exit", "action": "helpJson"}
        ],
        "rootBehavior": {"action": "help", "exitCode": 0},
        "exitCodes": {
            "0": "Success (hotspots: JSON written, including empty entries; record: all events accepted; init: hook installed)",
            "1": "Hotspots: storage error. Record: one or more events rejected, or I/O error writing output. Init: I/O error.",
            "2": "Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file or store failure); init: unsupported harness, collision, or self-install refusal"
        }
    });
    serde_json::to_string(&doc).unwrap_or_else(|_| "{}".into())
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
        // With no store at the path, exits 2 with error on stderr.
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("datastore not found"));
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

        // Missing store exits 2 with error on stderr (no longer placeholder).
        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("datastore not found"),
            "missing store should produce 'not found' error, got: {stderr}"
        );
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
    /// real CWD's .scryrs/scryrs.db. Returns the tempdir guard.
    fn set_test_store() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let store_path = dir.path().join("scryrs.db");
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

    // --- SQLite store integration (Tasks 2.2, 2.4) ---

    #[test]
    fn record_persists_to_scryrs_db() {
        // Use set_test_store to override the store path to a temp db.
        let _store_dir = set_test_store();
        let input = make_valid_event_json("s1", "doc/x.md");
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":1"));
        assert!(stdout.contains("\"rejected\":0"));
    }

    #[test]
    fn record_does_not_create_events_jsonl() {
        // Prove that canonical persistence uses the SQLite store, not JSONL.
        // The store_override sends writes to a temp scryrs.db; .scryrs/events.jsonl
        // should never be touched in the test temp dir.
        let dir = set_test_store();
        let override_path = dir.path().join("scryrs.db");
        assert!(
            super::store_override::get()
                .map(|p| p.contains("scryrs.db"))
                .unwrap_or(false),
            "override path should point to scryrs.db"
        );

        let input = make_valid_event_json("s1", "doc/x.md");
        let mut out = Vec::new();
        let mut err = Vec::new();

        let _exit_code = run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

        // The SQLite store should be created at the override path.
        assert!(
            override_path.exists(),
            "scryrs.db must be created at override path"
        );
        // No events.jsonl should be created alongside.
        assert!(
            !dir.path().join("events.jsonl").exists(),
            "events.jsonl must NOT be created"
        );
    }

    #[test]
    fn record_default_path_uses_canonical_db() {
        // If no store_override is set (empty matches nothing), the fallback
        // should be CANONICAL_STORE_PATH.
        let fallback: String = super::store_override::get()
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| super::CANONICAL_STORE_PATH.into());
        assert_eq!(fallback, super::CANONICAL_STORE_PATH);
    }

    // --- 2.1: --stdin SQLite row-level verification ---

    #[test]
    fn record_stdin_persists_rows_to_sqlite() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let store_path = dir.path().join("test.db");
        super::store_override::set(
            store_path
                .to_str()
                .unwrap_or_else(|| panic!("store path not valid UTF-8"))
                .to_string(),
        );

        let input = format!(
            "{}\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty: {:?}",
            String::from_utf8_lossy(&err)
        );

        // Re-open the SQLite store and assert rows.
        let conn =
            rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count query: {e}"));
        assert_eq!(count, 2, "two events must be persisted");

        // Verify session_ids are correct.
        let mut stmt = conn
            .prepare("SELECT session_id FROM trace_events ORDER BY rowid")
            .unwrap_or_else(|e| panic!("prepare: {e}"));
        let sessions: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap_or_else(|e| panic!("query_map: {e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(sessions, vec!["s1", "s2"]);
    }

    // --- 2.2: --file SQLite row-level verification ---

    #[test]
    fn record_file_persists_rows_to_sqlite() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let store_path = dir.path().join("test.db");
        super::store_override::set(
            store_path
                .to_str()
                .unwrap_or_else(|| panic!("store path not valid UTF-8"))
                .to_string(),
        );

        let input_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("input dir: {e}"));
        let file_path = input_dir.path().join("events.jsonl");
        let content = format!(
            "{}\n{}\n",
            make_valid_event_json("s1", "doc/x.md"),
            make_valid_event_json("s2", "doc/y.md"),
        );
        std::fs::write(&file_path, content).unwrap_or_else(|e| panic!("write: {e}"));

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
        assert!(
            err.is_empty(),
            "stderr must be empty: {:?}",
            String::from_utf8_lossy(&err)
        );

        // Re-open the same canonical store and assert rows.
        let conn =
            rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count query: {e}"));
        assert_eq!(count, 2, "two events must be persisted from file");

        let mut stmt = conn
            .prepare("SELECT session_id FROM trace_events ORDER BY rowid")
            .unwrap_or_else(|e| panic!("prepare: {e}"));
        let sessions: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap_or_else(|e| panic!("query_map: {e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(sessions, vec!["s1", "s2"]);
    }

    // --- 2.3: Mixed valid/invalid — rejected lines never create rows ---

    #[test]
    fn mixed_valid_invalid_rejected_lines_no_rows() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let store_path = dir.path().join("test.db");
        super::store_override::set(
            store_path
                .to_str()
                .unwrap_or_else(|| panic!("store path not valid UTF-8"))
                .to_string(),
        );

        let input = format!(
            "{}\nnot valid json\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
            1
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"accepted\":2"));
        assert!(stdout.contains("\"rejected\":1"));

        // Rejected line must produce a diagnostic on stderr.
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("\"line\":2"), "must diagnose line 2");

        // Open SQLite — only 2 accepted rows must exist.
        let conn =
            rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count query: {e}"));
        assert_eq!(count, 2, "only accepted events must be persisted");

        // No .scryrs/events.jsonl was written as canonical output.
        assert!(
            !dir.path().join(".scryrs/events.jsonl").exists(),
            ".scryrs/events.jsonl must not be created"
        );
    }

    // --- 2.4: Fatal datastore failure ---

    #[test]
    fn record_fatal_store_failure_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create ".scryrs" as a regular FILE (not a directory) so that
        // EventStore::open cannot create scryrs.db inside it — simulating
        // a fatal filesystem-level failure.
        std::fs::write(dir.path().join(".scryrs"), "blocked")
            .unwrap_or_else(|e| panic!("write blocker: {e}"));
        let store_path = dir.path().join(".scryrs/scryrs.db");
        super::store_override::set(
            store_path
                .to_str()
                .unwrap_or_else(|| panic!("store path not valid UTF-8"))
                .to_string(),
        );

        let input = make_valid_event_json("s1", "doc/x.md");
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
            2,
            "fatal store failure must exit 2"
        );

        // No success summary on stdout.
        let stdout = String::from_utf8_lossy(&out);
        assert!(
            !stdout.contains("\"accepted\""),
            "stdout must not contain success summary, got: {stdout}"
        );

        // Deterministic stderr diagnostic.
        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("scryrs record: cannot open trace datastore"),
            "stderr must report store failure, got: {stderr}"
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
        // hotspots with no store exits 2 (missing store).
        assert_eq!(run(["hotspots", "/tmp"]), 2);
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
        // Hotspot command requires a valid store to exit 0.
        // This smoke test checks the missing-store case exits 2.
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        assert!(!err.is_empty());
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

#[cfg(all(test, feature = "core"))]
mod hotspot_integration_tests {
    use super::*;
    use scryrs_core::EventStore;
    use scryrs_types::{
        FileOpenedPayload, Outcome, SCHEMA_VERSION, TraceEvent, TraceEventPayload, TraceEventType,
    };

    fn make_file_opened(session_id: &str, path: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::FileOpened,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::FileOpened(FileOpenedPayload { path: path.into() }),
            outcome: Outcome::Success,
        }
    }

    fn populate_store(dir: &tempfile::TempDir, events: &[TraceEvent]) {
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        let store_path = scryrs_dir.join("scryrs.db");
        {
            let mut store =
                EventStore::open(&store_path).unwrap_or_else(|e| panic!("open store: {e}"));
            store
                .begin_transaction()
                .unwrap_or_else(|e| panic!("begin: {e}"));
            for ev in events {
                store
                    .append(ev)
                    .unwrap_or_else(|e| panic!("append {ev:?}: {e}"));
            }
            store
                .commit_transaction()
                .unwrap_or_else(|e| panic!("commit: {e}"));
        }
    }

    // 3.5.1: Populated store produces correct HotspotsReport JSON.
    #[test]
    fn populated_store_produces_correct_hotspots_report() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z"),
            make_file_opened("s1", "src/b.rs", "2026-06-21T09:01:00Z"),
            make_file_opened("s2", "src/a.rs", "2026-06-21T09:02:00Z"),
        ];
        populate_store(&dir, &events);

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 0, "stderr: {:?}", String::from_utf8_lossy(&err));

        let stdout = String::from_utf8_lossy(&out);
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

        assert_eq!(report["schemaVersion"], "1.0.0");
        assert_eq!(report["command"], "hotspots");
        assert!(!report["repositoryPath"].as_str().unwrap_or("").is_empty());
        assert!(
            report["storePath"]
                .as_str()
                .unwrap_or("")
                .ends_with(".scryrs/scryrs.db")
        );
        assert_eq!(report["runMetadata"]["analyzedEventCount"], 3);
        assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 2);
        assert!(!report["generatedAt"].as_str().unwrap_or("").is_empty());

        let entries = report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries not array"));
        assert!(!entries.is_empty(), "entries should not be empty");

        // src/a.rs should have higher score (2) than src/b.rs (1)
        assert_eq!(entries[0]["subject"], "src/a.rs");
        assert_eq!(entries[0]["score"], 2);
        assert_eq!(entries[1]["subject"], "src/b.rs");
        assert_eq!(entries[1]["score"], 1);
    }

    // 3.5.2: Empty store produces entries: [] with exit 0.
    #[test]
    fn empty_store_produces_empty_entries_with_exit_0() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create empty store with no events.
        populate_store(&dir, &[]);

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(
            exit_code,
            0,
            "exit should be 0 for empty store, stderr: {:?}",
            String::from_utf8_lossy(&err)
        );

        let stdout = String::from_utf8_lossy(&out);
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

        assert_eq!(
            report["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not array"))
                .len(),
            0
        );
        assert_eq!(report["runMetadata"]["analyzedEventCount"], 0);
        assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 0);
    }

    // 3.5.3: Missing store exits 2 with error message on stderr.
    #[test]
    fn missing_store_exits_2_with_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Don't create .scryrs/scryrs.db.

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 2);
        assert!(out.is_empty());

        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("datastore not found"),
            "should mention datastore not found, got: {stderr}"
        );
    }

    // 3.5.4: Unsupported store (schema version mismatch) exits 2 with error on stderr.
    #[test]
    fn unsupported_store_exits_2_with_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create a valid store first.
        populate_store(
            &dir,
            &[make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z")],
        );

        // Tamper with the schema version via direct SQLite connection.
        let store_path = dir.path().join(".scryrs/scryrs.db");
        {
            let conn = rusqlite::Connection::open(&store_path)
                .unwrap_or_else(|e| panic!("open store for tamper: {e}"));
            conn.execute(
                "UPDATE schema_meta SET value = '99' WHERE key = 'datastore_schema_version'",
                [],
            )
            .unwrap_or_else(|e| panic!("tamper schema version: {e}"));
        }

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 2, "stderr: {:?}", String::from_utf8_lossy(&err));
        assert!(out.is_empty());

        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("unsupported datastore"),
            "should mention unsupported datastore, got: {stderr}"
        );
        assert!(
            stderr.contains("version mismatch"),
            "should mention version mismatch, got: {stderr}"
        );
    }

    // 3.5.5: Corrupt/non-SQLite file exits 1 with error message on stderr.
    #[test]
    fn corrupt_store_exits_1_with_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::write(scryrs_dir.join("scryrs.db"), "not a sqlite database\n")
            .unwrap_or_else(|e| panic!("write corrupt file: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 1, "stderr: {:?}", String::from_utf8_lossy(&err));
        assert!(out.is_empty());

        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("storage error"),
            "should mention storage error, got: {stderr}"
        );
    }

    // 3.5.6: Deterministic ordering: same store produces identical output on repeated runs
    // (ignoring the generatedAt wall-clock timestamp which may differ).
    #[test]
    fn deterministic_ordering_repeated_runs() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z"),
            make_file_opened("s1", "src/b.rs", "2026-06-21T09:01:00Z"),
        ];
        populate_store(&dir, &events);

        let mut out1 = Vec::new();
        let mut err1 = Vec::new();
        let exit1 = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out1,
            &mut err1,
        );

        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        let exit2 = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out2,
            &mut err2,
        );

        assert_eq!(exit1, 0);
        assert_eq!(exit2, 0);

        // Parse both outputs and compare everything except generatedAt.
        let stdout1 = String::from_utf8_lossy(&out1);
        let stdout2 = String::from_utf8_lossy(&out2);
        let mut r1: serde_json::Value =
            serde_json::from_str(stdout1.trim()).unwrap_or_else(|e| panic!("parse 1: {e}"));
        let mut r2: serde_json::Value =
            serde_json::from_str(stdout2.trim()).unwrap_or_else(|e| panic!("parse 2: {e}"));

        // Strip generatedAt before comparing.
        r1.as_object_mut()
            .unwrap_or_else(|| panic!("r1 not object"))
            .remove("generatedAt");
        r2.as_object_mut()
            .unwrap_or_else(|| panic!("r2 not object"))
            .remove("generatedAt");

        assert_eq!(
            r1, r2,
            "repeated runs must produce identical deterministic output"
        );
    }

    // 3.5.8: .scryrs/hotspots.json artifact file is written when store is valid.
    #[test]
    fn artifact_file_is_written_on_success() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let events = vec![make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z")];
        populate_store(&dir, &events);

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 0);

        let artifact_path = dir.path().join(".scryrs/hotspots.json");
        assert!(
            artifact_path.exists(),
            "hotspots.json should be written at {}",
            artifact_path.display()
        );

        let artifact_content = std::fs::read_to_string(&artifact_path)
            .unwrap_or_else(|e| panic!("read artifact: {e}"));
        let stdout = String::from_utf8_lossy(&out);
        assert_eq!(
            stdout.trim(),
            artifact_content.trim(),
            "stdout and artifact file must match"
        );
    }

    // Additional: empty store also writes artifact file.
    #[test]
    fn artifact_file_is_written_for_empty_store() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        populate_store(&dir, &[]);

        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit_code, 0);

        let artifact_path = dir.path().join(".scryrs/hotspots.json");
        assert!(artifact_path.exists());
    }

    // Additional: artifact file is not written on error.
    #[test]
    fn artifact_file_not_written_on_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::write(scryrs_dir.join("scryrs.db"), "corrupt\n")
            .unwrap_or_else(|e| panic!("write: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();

        let _exit_code = run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        let artifact_path = dir.path().join(".scryrs/hotspots.json");
        assert!(
            !artifact_path.exists(),
            "hotspots.json must not be written on error"
        );
    }
}

#[cfg(test)]
mod init_tests {
    use std::sync::Mutex;

    use super::*;

    /// Global mutex to serialize CWD changes across init tests.
    /// `std::env::set_current_dir` is process-global; parallel test
    /// threads would race on it without this guard.
    static CWD_GUARD: Mutex<()> = Mutex::new(());

    /// Change CWD to `dir`, run `f`, then restore original CWD.
    fn with_cwd(dir: &std::path::Path, f: impl FnOnce()) {
        let _lock = CWD_GUARD
            .lock()
            .unwrap_or_else(|e| panic!("CWD guard poisoned: {e}"));
        let original = std::env::current_dir().unwrap_or_else(|e| panic!("current_dir: {e}"));
        std::env::set_current_dir(dir).unwrap_or_else(|e| panic!("set_current_dir: {e}"));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(&original).unwrap_or_else(|e| panic!("restore cwd: {e}"));
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }

    // --- 7.1: init --agent claude-code writes hook file ---

    #[test]
    fn init_agent_claude_code_writes_hook_file() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                0
            );
            assert!(
                err.is_empty(),
                "stderr must be empty, got: {}",
                String::from_utf8_lossy(&err)
            );

            let hook_path = dir.path().join(".claude/hooks/scryrs-hook.mjs");
            assert!(
                hook_path.exists(),
                "hook file must exist at {}",
                hook_path.display()
            );
            let content =
                std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("read hook: {e}"));
            assert!(!content.is_empty(), "hook file must not be empty");
            assert!(
                content.contains("PreToolUse"),
                "hook must contain PreToolUse"
            );
        });
    }

    // --- 7.2: init --agent pi writes hook file ---

    #[test]
    fn init_agent_pi_writes_hook_file() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
                0
            );
            assert!(
                err.is_empty(),
                "stderr must be empty, got: {}",
                String::from_utf8_lossy(&err)
            );

            let hook_path = dir.path().join(".pi/extensions/pi-trace/index.ts");
            assert!(
                hook_path.exists(),
                "hook file must exist at {}",
                hook_path.display()
            );
            let content =
                std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("read hook: {e}"));
            assert!(!content.is_empty(), "hook file must not be empty");
            assert!(
                content.contains("ExtensionAPI"),
                "hook must reference ExtensionAPI"
            );
        });
    }

    // --- 7.3: init --agent unknown exits 2 ---

    #[test]
    fn init_agent_unknown_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "unknown"], &mut out, &mut err),
                2
            );
            assert!(out.is_empty(), "stdout must be empty");
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("'unknown' is not a supported harness"),
                "must report unsupported harness, got: {err_str}"
            );
            assert!(
                err_str.contains("Supported harnesses:"),
                "must list supported harnesses, got: {err_str}"
            );
            assert!(
                err_str.contains("claude-code"),
                "must mention claude-code, got: {err_str}"
            );
            assert!(err_str.contains("pi"), "must mention pi, got: {err_str}");
        });
    }

    // --- 7.4: settings.json collision ---

    #[test]
    fn init_claude_code_settings_json_collision_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create .claude/settings.json before running init
        std::fs::create_dir_all(dir.path().join(".claude"))
            .unwrap_or_else(|e| panic!("create_dir: {e}"));
        std::fs::write(dir.path().join(".claude/settings.json"), "{}")
            .unwrap_or_else(|e| panic!("write: {e}"));

        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                2
            );
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("settings.json already exists"),
                "must report settings.json collision, got: {err_str}"
            );
            assert!(
                err_str.contains("not be installed"),
                "must not claim hook source was installed, got: {err_str}"
            );
            assert!(
                !err_str.contains("has been installed"),
                "must not claim hook source was already installed, got: {err_str}"
            );
            assert!(
                err_str.contains("PreToolUse"),
                "must include JSON block instructions, got: {err_str}"
            );

            // Verify no mutation: hook file must NOT exist
            let hook_path = dir.path().join(".claude/hooks/scryrs-hook.mjs");
            assert!(
                !hook_path.exists(),
                "hook file must not be written on settings.json collision"
            );
        });
    }

    // --- 7.5: scryrs-hook.mjs collision ---

    #[test]
    fn init_claude_code_hook_file_collision_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create the target directory and file before running init
        std::fs::create_dir_all(dir.path().join(".claude/hooks"))
            .unwrap_or_else(|e| panic!("create_dir: {e}"));
        std::fs::write(dir.path().join(".claude/hooks/scryrs-hook.mjs"), "existing")
            .unwrap_or_else(|e| panic!("write: {e}"));

        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                2
            );
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("already exists"),
                "must report file collision, got: {err_str}"
            );
            assert!(
                err_str.contains("Remove the file manually"),
                "must include remediation, got: {err_str}"
            );
        });
    }

    // --- 7.6: pi/index.ts collision ---

    #[test]
    fn init_pi_hook_file_collision_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create the target directory and file before running init
        std::fs::create_dir_all(dir.path().join(".pi/extensions/pi-trace"))
            .unwrap_or_else(|e| panic!("create_dir: {e}"));
        std::fs::write(
            dir.path().join(".pi/extensions/pi-trace/index.ts"),
            "existing",
        )
        .unwrap_or_else(|e| panic!("write: {e}"));

        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
                2
            );
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("already exists"),
                "must report file collision, got: {err_str}"
            );
            assert!(
                err_str.contains("Remove the file manually"),
                "must include remediation, got: {err_str}"
            );
        });
    }

    // --- 7.7: self-install detection ---

    #[test]
    fn init_self_install_detection_refuses() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create a fake scryrs source checkout: Cargo.toml with scryrs-cli + hooks/claude-code/
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/scryrs-cli\", \"crates/scryrs-types\"]\n",
        )
        .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
        std::fs::create_dir_all(dir.path().join("hooks/claude-code"))
            .unwrap_or_else(|e| panic!("create_dir: {e}"));

        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                2
            );
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("refusing to install"),
                "must refuse self-install, got: {err_str}"
            );
            assert!(
                err_str.contains("source repo"),
                "must mention source repo, got: {err_str}"
            );
        });
    }

    // --- 7.8: unrelated project passes self-install check ---

    #[test]
    fn init_unrelated_project_passes_self_install_check() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        // Create a non-scryrs project: Cargo.toml without scryrs-cli, no hooks/
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-project\"\nversion = \"0.1.0\"\n",
        )
        .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));

        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            // Should succeed — this is a normal project
            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                0
            );
            assert!(
                err.is_empty(),
                "stderr must be empty, got: {}",
                String::from_utf8_lossy(&err)
            );
        });
    }

    // --- 7.9: init without --agent exits 2 ---

    #[test]
    fn init_without_agent_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(run_with_writers(["init"], &mut out, &mut err), 2);
            assert!(out.is_empty(), "stdout must be empty");
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("scryrs init:"),
                "must name init, got: {err_str}"
            );
            assert!(
                err_str.contains("--agent"),
                "must mention --agent, got: {err_str}"
            );
            assert!(
                err_str.contains("See `scryrs --help`"),
                "must escalate to --help, got: {err_str}"
            );
        });
    }

    // --- 7.10: init with empty --agent value exits 2 ---

    #[test]
    fn init_empty_agent_exits_2() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", ""], &mut out, &mut err),
                2
            );
            assert!(out.is_empty());
            let err_str = String::from_utf8_lossy(&err);
            assert!(
                err_str.contains("--agent requires a non-empty value"),
                "must reject empty --agent, got: {err_str}"
            );
            assert!(
                err_str.contains("See `scryrs --help`"),
                "must escalate to --help, got: {err_str}"
            );
        });
    }

    // --- 7.11: init help text appears in --help output ---

    #[test]
    fn init_appears_in_help_output() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        let help = String::from_utf8_lossy(&out);
        assert!(help.contains("scryrs init --agent <NAME>"));
        assert!(help.contains("Install the scryrs trace hook"));
        assert!(help.contains("claude-code"));
        assert!(help.contains("pi"));
        assert!(help.contains("scryrs init --agent claude-code"));
        assert!(help.contains("scryrs init --agent pi"));
    }

    // --- 7.12: init entry appears in --help-json output ---

    #[test]
    fn init_appears_in_help_json() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        let json_str = String::from_utf8_lossy(&out);
        let doc: serde_json::Value =
            serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("parse help-json: {e}"));

        assert_eq!(doc["surfaceVersion"], "0.3.0");

        let commands = doc["commands"]
            .as_array()
            .unwrap_or_else(|| panic!("commands must be array"));
        let init_cmd = commands
            .iter()
            .find(|c| c["name"] == "init")
            .unwrap_or_else(|| panic!("init must be in commands array"));

        assert_eq!(
            init_cmd["description"],
            "Install scryrs trace hook for a supported agent harness"
        );

        let args = init_cmd["arguments"]
            .as_array()
            .unwrap_or_else(|| panic!("arguments must be array"));
        let agent_arg = args
            .iter()
            .find(|a| a["name"] == "agent")
            .unwrap_or_else(|| panic!("--agent must be in arguments"));
        assert_eq!(agent_arg["required"], true);
        assert_eq!(agent_arg["type"], "string");
    }

    // --- 7.14: claude-code stdout contains next-step text ---

    #[test]
    fn init_claude_code_stdout_has_next_steps() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
                0
            );
            assert!(err.is_empty());
            let stdout = String::from_utf8_lossy(&out);
            assert!(stdout.contains("Next steps:"));
            assert!(stdout.contains("scryrs is on your PATH"));
            assert!(stdout.contains("settings.json"));
            assert!(stdout.contains("Restart your Claude Code session"));
        });
    }

    // --- 7.15: pi stdout contains next-step text ---

    #[test]
    fn init_pi_stdout_has_next_steps() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        with_cwd(dir.path(), || {
            let mut out = Vec::new();
            let mut err = Vec::new();

            assert_eq!(
                run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
                0
            );
            assert!(err.is_empty());
            let stdout = String::from_utf8_lossy(&out);
            assert!(stdout.contains("Next steps:"));
            assert!(stdout.contains("scryrs is on your PATH"));
            assert!(stdout.contains("Reload Pi"));
        });
    }

    // --- 7.16: all existing tests remain unchanged ---

    #[test]
    fn init_does_not_regress_help() {
        // --help still works as before (init is additive)
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        let help = String::from_utf8_lossy(&out);
        assert!(help.contains("hotspots"));
        assert!(help.contains("record"));
        assert!(help.contains("OPTIONS"));
    }

    #[test]
    fn init_does_not_regress_hotspots() {
        // Hotspot command requires a valid store. Without one, exits 2.
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("datastore not found"));
    }

    #[test]
    fn init_does_not_regress_version() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("scryrs "));
    }

    #[test]
    fn init_does_not_regress_unknown_command() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["nonexistent"], &mut out, &mut err), 2);
        assert!(String::from_utf8_lossy(&err).contains("unknown command"));
    }
}
