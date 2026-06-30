use std::io::{Read, Write};

use clap::ArgMatches;

use scryrs_types::SCHEMA_VERSION;

#[cfg(feature = "core")]
use {
    crate::remote_config,
    crate::remote_submit::{RemoteSubmitter, SubmitError, UreqSubmitter, build_envelope},
    crate::store_override,
};

#[cfg(feature = "core")]
const RECORD_DEBUG_PREFIX: &str = "[scryrs-record]";

#[cfg(feature = "core")]
const RECORD_DEBUG_PREVIEW_LIMIT: usize = 160;

#[cfg(feature = "core")]
fn record_debug_enabled() -> bool {
    std::env::var("SCRYRS_DEBUG")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(feature = "core")]
use scryrs_adapter_harness::collapse_newlines;

#[cfg(feature = "core")]
fn truncate_debug(value: &str) -> String {
    let mut chars = value.chars();
    let preview: String = chars.by_ref().take(RECORD_DEBUG_PREVIEW_LIMIT).collect();

    if chars.next().is_some() {
        format!("{preview}…({} bytes)", value.len())
    } else {
        preview
    }
}

#[cfg(feature = "core")]
fn preview_debug(value: &str) -> String {
    truncate_debug(&collapse_newlines(value))
}

#[cfg(feature = "core")]
fn write_debug(err: &mut impl Write, stage: &str, fields: &[(&str, String)]) {
    let mut line = format!("{RECORD_DEBUG_PREFIX} stage={stage}");
    for (key, value) in fields {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
    let _ = writeln!(err, "{line}");
}

#[cfg(feature = "core")]
pub(crate) fn execute_record<R: Read>(
    out: &mut impl Write,
    err: &mut impl Write,
    stdin: &mut R,
    m: &ArgMatches,
) -> i32 {
    use std::fs::File;
    use std::io::Cursor;

    use scryrs_core::{CANONICAL_STORE_PATH, EventStore, ingest_jsonl_detailed};

    let use_stdin = m.get_flag("stdin");
    let file_path: Option<&String> = m.get_one::<String>("file");
    let debug_enabled = record_debug_enabled();

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

    // --- Resolve transport mode before input/store I/O ---
    // Live (remote) is the default; local SQLite is an explicit opt-in. When the
    // default is live but required config is unresolved, fail fast with guidance
    // rather than silently degrading to local mode.
    let mode_str = m
        .get_one::<String>("mode")
        .map(|s| s.as_str())
        .unwrap_or("live");
    if mode_str != "local" && mode_str != "live" {
        let _ = writeln!(err, "scryrs record: --mode must be local or live");
        if writeln!(err, "See `scryrs --help`").is_err() {
            return 1;
        }
        return 2;
    }

    let remote = if mode_str == "local" {
        None
    } else {
        match remote_config::resolve_remote_config(None) {
            Ok(Some(resolved)) => Some(resolved),
            Ok(None) => {
                remote_config::write_missing_config_guidance(err, "record", "ingest_url");
                return 2;
            }
            Err(e) => {
                remote_config::write_missing_config_guidance(err, "record", e.missing_field());
                return 2;
            }
        }
    };

    let (raw_input, input_source) = if use_stdin {
        let mut raw_input = String::new();
        if let Err(e) = stdin.read_to_string(&mut raw_input) {
            if writeln!(err, "scryrs record: I/O error while reading input: {e}").is_err() {
                return 1;
            }
            return 2;
        }
        (raw_input, "stdin".to_string())
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
            Ok(mut file) => {
                let mut raw_input = String::new();
                if let Err(e) = file.read_to_string(&mut raw_input) {
                    if writeln!(err, "scryrs record: cannot read {path}: {e}").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        return 1;
                    }
                    return 2;
                }
                (raw_input, format!("file:{}", preview_debug(path)))
            }
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

    if debug_enabled {
        for (zero_based, line) in raw_input.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }

            write_debug(
                err,
                "received",
                &[
                    ("line", (zero_based + 1).to_string()),
                    ("source", input_source.clone()),
                    ("bytes", line.len().to_string()),
                    ("preview", preview_debug(line)),
                ],
            );
        }
    }

    // Ingest.
    let outcome = match ingest_jsonl_detailed(Cursor::new(raw_input.as_bytes())) {
        Ok(o) => o,
        Err(e) => {
            if writeln!(err, "scryrs record: I/O error while reading input: {e}").is_err() {
                return 1;
            }
            return 2;
        }
    };

    if debug_enabled {
        for accepted in &outcome.accepted {
            write_debug(
                err,
                "accepted",
                &[
                    ("line", accepted.line.to_string()),
                    (
                        "event_type",
                        accepted.event.event_type.payload_type_str().to_string(),
                    ),
                    ("session_id", preview_debug(&accepted.event.session_id)),
                    (
                        "tool_name",
                        accepted
                            .event
                            .tool_name
                            .as_deref()
                            .unwrap_or("none")
                            .to_string(),
                    ),
                ],
            );
        }

        for rejection in &outcome.rejected {
            write_debug(
                err,
                "rejected",
                &[
                    ("line", rejection.line.to_string()),
                    (
                        "field",
                        rejection.field.as_deref().unwrap_or("none").to_string(),
                    ),
                    ("reason", preview_debug(&rejection.reason)),
                ],
            );
        }
    }

    // --- Branch: remote mode or local mode ---
    let local_rejected_count = outcome.rejected.len();
    if let Some(resolved) = remote {
        return execute_remote_path(
            out,
            err,
            &outcome,
            local_rejected_count,
            &resolved.config,
            &UreqSubmitter,
            debug_enabled,
        );
    }

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

    if debug_enabled {
        write_debug(
            err,
            "datastore_open",
            &[("path", preview_debug(&store_path))],
        );
    }

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

    for (index, accepted) in outcome.accepted.iter().enumerate() {
        if let Err(e) = store.append(&accepted.event) {
            if writeln!(err, "scryrs record: cannot persist event: {e}").is_err() {
                return 1;
            }
            return 2;
        }

        if debug_enabled {
            write_debug(
                err,
                "inserted",
                &[
                    ("index", (index + 1).to_string()),
                    (
                        "event_type",
                        accepted.event.event_type.payload_type_str().to_string(),
                    ),
                    ("session_id", preview_debug(&accepted.event.session_id)),
                    (
                        "tool_name",
                        accepted
                            .event
                            .tool_name
                            .as_deref()
                            .unwrap_or("none")
                            .to_string(),
                    ),
                ],
            );
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

    if debug_enabled {
        write_debug(
            err,
            "transaction_commit",
            &[
                ("accepted", accepted.to_string()),
                ("rejected", rejected.to_string()),
            ],
        );
        write_debug(
            err,
            "summary",
            &[
                ("accepted", accepted.to_string()),
                ("rejected", rejected.to_string()),
                (
                    "exit",
                    if rejected > 0 {
                        "rejections"
                    } else {
                        "success"
                    }
                    .to_string(),
                ),
            ],
        );
    }

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

/// Execute the remote submission path: build envelope, submit, and map response.
#[cfg(feature = "core")]
fn execute_remote_path(
    out: &mut impl Write,
    err: &mut impl Write,
    outcome: &scryrs_core::DetailedIngestionOutcome,
    local_rejected: usize,
    config: &remote_config::RemoteConfig,
    submitter: &dyn RemoteSubmitter,
    debug_enabled: bool,
) -> i32 {
    // Build envelope from accepted events only.
    let accepted_pairs: Vec<(usize, scryrs_types::TraceEvent)> = outcome
        .accepted
        .iter()
        .map(|a| (a.line, a.event.clone()))
        .collect();

    let envelope = build_envelope(
        &accepted_pairs,
        &config.repository_id,
        &config.workspace_id,
        &config.agent_id,
    );

    if debug_enabled {
        write_debug(
            err,
            "remote_submit",
            &[
                ("url", config.ingest_url.clone()),
                ("events", accepted_pairs.len().to_string()),
                ("timeout_ms", config.timeout_ms.to_string()),
            ],
        );
    }

    // Submit.
    let response = submitter.submit(&config.ingest_url, &envelope, config.timeout_ms);

    match response {
        Ok(resp) => {
            // Map BatchIngestResponse to deterministic counts.
            let mut accepted = 0u64;
            let mut duplicate = 0u64;
            let mut failed = 0u64;

            for ack in &resp.events {
                match ack.status {
                    scryrs_types::EventAckStatus::Accepted => accepted += 1,
                    scryrs_types::EventAckStatus::Idempotent => duplicate += 1,
                    scryrs_types::EventAckStatus::Rejected => {
                        // Per-item server rejection.
                        failed += 1;
                        // Emit a diagnostic for server-rejected items.
                        let field = ack
                            .producer_event_id
                            .clone()
                            .unwrap_or_else(|| "unknown".into());
                        let reason = ack
                            .error_reason
                            .clone()
                            .unwrap_or_else(|| "server rejected".into());
                        let field_json =
                            serde_json::to_string(&field).unwrap_or_else(|_| "null".into());
                        let reason_json = serde_json::to_string(&reason)
                            .unwrap_or_else(|_| "\"<serialization error>\"".into());
                        let diag = format!(
                            r#"{{"line":-1,"field":{},"reason":{}}}"#,
                            field_json, reason_json,
                        );
                        if writeln!(err, "{diag}").is_err() {
                            return 1;
                        }
                    }
                }
            }

            let total_rejected = local_rejected as u64 + failed;

            if debug_enabled {
                write_debug(
                    err,
                    "remote_response",
                    &[
                        ("accepted", accepted.to_string()),
                        ("duplicate", duplicate.to_string()),
                        ("rejected", total_rejected.to_string()),
                        ("failed", failed.to_string()),
                    ],
                );
            }

            // Remote summary to stdout.
            let summary = format!(
                r#"{{"command":"record","schemaVersion":"{}","transport":"remote","accepted":{},"duplicate":{},"rejected":{},"failed":{}}}"#,
                SCHEMA_VERSION, accepted, duplicate, total_rejected, failed,
            );
            if writeln!(out, "{summary}").is_err() {
                return 1;
            }

            if total_rejected > 0 { 1 } else { 0 }
        }
        Err(submit_err) => {
            // Fatal transport/server failure → exit 2.
            let diagnostic = match &submit_err {
                SubmitError::Timeout => {
                    format!(
                        "scryrs record: remote ingest timed out after {} ms",
                        config.timeout_ms
                    )
                }
                SubmitError::Connection(e) => {
                    format!("scryrs record: cannot connect to ingest server: {e}")
                }
                SubmitError::HttpStatus { status, body } => {
                    format!(
                        "scryrs record: ingest server returned HTTP {status}: {}",
                        body.lines().next().unwrap_or("(empty body)")
                    )
                }
                SubmitError::MalformedResponse(e) => {
                    format!("scryrs record: malformed response from ingest server: {e}")
                }
                SubmitError::Serialization(e) => {
                    format!("scryrs record: cannot serialize request: {e}")
                }
            };

            if writeln!(err, "{diagnostic}").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                return 1;
            }
            2
        }
    }
}

/// Test-only entry point that accepts a pre-resolved `RemoteConfig`.
/// Bypasses environment-based config resolution for deterministic tests.
#[cfg(all(feature = "core", test))]
pub(crate) fn execute_record_with_config<R: Read>(
    out: &mut impl Write,
    err: &mut impl Write,
    stdin: &mut R,
    m: &ArgMatches,
    remote: Option<crate::remote_config::ResolvedRemote>,
    submitter: &dyn RemoteSubmitter,
) -> i32 {
    use std::fs::File;
    use std::io::Cursor;

    use scryrs_core::{CANONICAL_STORE_PATH, EventStore, ingest_jsonl_detailed};

    let use_stdin = m.get_flag("stdin");
    let file_path: Option<&String> = m.try_get_one::<String>("file").ok().flatten();

    // Simplified mode validation for tests.
    match (use_stdin, file_path) {
        (true, None) => {}
        (false, Some(_)) => {}
        _ => return 2,
    }

    let (raw_input, _input_source) = if use_stdin {
        let mut raw_input = String::new();
        if stdin.read_to_string(&mut raw_input).is_err() {
            return 2;
        }
        (raw_input, "stdin".to_string())
    } else {
        let path = match file_path {
            Some(p) => p.as_str(),
            None => "unknown",
        };
        match File::open(path) {
            Ok(mut file) => {
                let mut raw_input = String::new();
                if file.read_to_string(&mut raw_input).is_err() {
                    return 2;
                }
                (raw_input, format!("file:{path}"))
            }
            Err(_) => return 2,
        }
    };

    let outcome = match ingest_jsonl_detailed(Cursor::new(raw_input.as_bytes())) {
        Ok(o) => o,
        Err(_) => return 2,
    };

    let local_rejected = outcome.rejected.len();

    // Emit rejection diagnostics.
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

    // Branch: remote or local.
    if let Some(resolved) = remote {
        return execute_remote_path(
            out,
            err,
            &outcome,
            local_rejected,
            &resolved.config,
            submitter,
            false, // debug disabled in tests
        );
    }

    // Local mode.
    let store_path = store_override::get().unwrap_or_else(|| CANONICAL_STORE_PATH.into());
    let mut store = match EventStore::open(&store_path) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(
                err,
                "scryrs record: cannot open trace datastore ({store_path}): {e}"
            );
            return 2;
        }
    };

    if store.begin_transaction().is_err() {
        let _ = writeln!(err, "scryrs record: cannot begin datastore transaction");
        return 2;
    }

    for accepted in &outcome.accepted {
        if store.append(&accepted.event).is_err() {
            let _ = writeln!(err, "scryrs record: cannot persist event");
            return 2;
        }
    }

    if store.commit_transaction().is_err() {
        let _ = writeln!(err, "scryrs record: cannot commit datastore transaction");
        return 2;
    }

    let accepted = outcome.accepted.len();
    let summary = format!(
        r#"{{"command":"record","schemaVersion":"{}","accepted":{},"rejected":{}}}"#,
        SCHEMA_VERSION, accepted, local_rejected,
    );
    if writeln!(out, "{summary}").is_err() {
        return 1;
    }

    if local_rejected > 0 { 1 } else { 0 }
}

#[cfg(not(feature = "core"))]
pub(crate) fn execute_record<R: Read>(
    _out: &mut impl Write,
    err: &mut impl Write,
    _stdin: &mut R,
    _m: &ArgMatches,
) -> i32 {
    let _ = writeln!(err, "scryrs record: unavailable (core feature not enabled)");
    2
}
