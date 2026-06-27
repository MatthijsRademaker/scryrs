//! Native harness hook subcommand (`scryrs hook <harness>`).
//!
//! `hook` is the harness-facing integration entry point. It accepts a harness's
//! native tool event (Claude Code on stdin; Pi via `--file`), delegates
//! translation to the `scryrs-adapter-harness` crate, and persists any resulting
//! `TraceEvent` through the shared canonical `EventStore` (local mode) or remote
//! ingest submission (remote mode, when configured via `scryrs.json`).
//!
//! Its defining contract is **fail-open**: it NEVER blocks the harness. Any
//! error — malformed input, unknown harness, translation failure, store error,
//! remote submission failure — results in exit 0 with empty stdout, plus a
//! timestamped line in `.scryrs/hooks/<harness>-warnings.log`. This is the
//! inverse of `record`'s 1/2 exit policy, which is why `hook` is its own command.

use std::io::{Read, Write};

use clap::ArgMatches;

#[cfg(feature = "core")]
use {
    crate::remote_config,
    crate::remote_submit::{RemoteSubmitter, UreqSubmitter, build_envelope},
    scryrs_adapter_harness::{HookContext, adapter_for, debug_enabled, now_iso8601},
};

/// Execute `scryrs hook <harness>`.
///
/// Always returns exit code 0 (fail-open). Writes nothing to stdout.
#[cfg(feature = "core")]
pub(crate) fn execute_hook<R: Read>(
    _out: &mut impl Write,
    err: &mut impl Write,
    stdin: &mut R,
    harness: &str,
    m: &ArgMatches,
) -> i32 {
    use std::path::{Path, PathBuf};

    use scryrs_core::{CANONICAL_STORE_PATH, EventStore};
    use serde_json::Value;

    use crate::store_override;

    let debug = debug_enabled();

    // --- read input (stdin default, --file alternate; mutually exclusive) ---
    let raw = match read_hook_input(stdin, m) {
        Ok(raw) => raw,
        Err(reason) => {
            // No payload → cannot resolve cwd; warn relative to process cwd.
            warn(harness, Path::new("."), &reason, debug, err);
            return 0;
        }
    };

    // --- best-effort parse to resolve cwd, session_id, warning-log base ---
    let parsed: Option<Value> = serde_json::from_str(&raw).ok();

    let base_dir: PathBuf = parsed
        .as_ref()
        .and_then(|v| v.get("cwd"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    if debug {
        let _ = writeln!(
            err,
            "[scryrs-hook] stage=received harness={harness} bytes={} base={}",
            raw.len(),
            base_dir.display()
        );
    }

    if parsed.is_none() {
        warn(harness, &base_dir, "malformed JSON input", debug, err);
        return 0;
    }

    // --- select adapter (unknown harness fails open) ---
    let adapter = match adapter_for(harness) {
        Some(a) => a,
        None => {
            warn(
                harness,
                &base_dir,
                &format!("unknown harness '{harness}'"),
                debug,
                err,
            );
            return 0;
        }
    };

    // --- resolve identity + store location from the payload (D5) ---
    let session_id = parsed
        .as_ref()
        .and_then(|v| v.get("session_id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("scryrs-{}", std::process::id()));

    let store_path: PathBuf = store_override::get()
        .map(PathBuf::from)
        .unwrap_or_else(|| base_dir.join(CANONICAL_STORE_PATH));

    let ctx = HookContext {
        session_id,
        store_path: store_path.clone(),
        timestamp: now_iso8601(),
        bash_debug: debug,
    };

    // --- translate ---
    let event = match adapter.translate(&raw, &ctx) {
        Ok(Some(event)) => event,
        Ok(None) => {
            // Untracked tool / pass-through: persist nothing, succeed.
            if debug {
                let _ = writeln!(err, "[scryrs-hook] stage=passthrough harness={harness}");
            }
            return 0;
        }
        Err(e) => {
            warn(
                harness,
                &base_dir,
                &format!("translation error: {e}"),
                debug,
                err,
            );
            return 0;
        }
    };

    // --- persist through shared canonical store or remote submit ---
    let remote = remote_config::resolve_remote_config(Some(&base_dir))
        .ok()
        .flatten();

    if let Some(resolved) = remote {
        // Remote mode: submit one event as a batch of 1. Fail-open on errors.
        let accepted_pairs = vec![(1usize, event.clone())];
        let envelope = build_envelope(
            &accepted_pairs,
            &resolved.config.repository_id,
            &resolved.config.workspace_id,
            &resolved.config.agent_id,
        );
        let submitter = UreqSubmitter;
        match submitter.submit(
            &resolved.config.ingest_url,
            &envelope,
            resolved.config.timeout_ms,
        ) {
            Ok(_resp) => {
                if debug {
                    let _ = writeln!(
                        err,
                        "[scryrs-hook] stage=remote_persisted harness={harness} event_type={}",
                        event.event_type.payload_type_str()
                    );
                }
            }
            Err(submit_err) => {
                warn(
                    harness,
                    &base_dir,
                    &format!("remote ingest failed: {submit_err}"),
                    debug,
                    err,
                );
            }
        }
    } else {
        // Local mode: persist through the shared canonical store.
        let persist = (|| -> Result<(), String> {
            let mut store =
                EventStore::open(&store_path).map_err(|e| format!("cannot open store: {e}"))?;
            store
                .append(&event)
                .map_err(|e| format!("cannot append event: {e}"))?;
            Ok(())
        })();

        if let Err(reason) = persist {
            warn(harness, &base_dir, &reason, debug, err);
            return 0;
        }

        if debug {
            let _ = writeln!(
                err,
                "[scryrs-hook] stage=persisted harness={harness} event_type={} store={}",
                event.event_type.payload_type_str(),
                store_path.display()
            );
        }
    }

    0
}

/// Read the hook input from stdin (default) or `--file <PATH>`.
///
/// Unlike `record`, mismatched/missing modes are not a hard usage error — they
/// surface as an `Err(String)` the caller logs and fails open on.
#[cfg(feature = "core")]
fn read_hook_input<R: Read>(stdin: &mut R, m: &ArgMatches) -> Result<String, String> {
    use std::fs::File;

    let file_path: Option<&String> = m.get_one::<String>("file");
    match file_path {
        Some(path) => {
            let mut file = File::open(path).map_err(|e| format!("cannot read {path}: {e}"))?;
            let mut raw = String::new();
            file.read_to_string(&mut raw)
                .map_err(|e| format!("cannot read {path}: {e}"))?;
            Ok(raw)
        }
        None => {
            let mut raw = String::new();
            stdin
                .read_to_string(&mut raw)
                .map_err(|e| format!("stdin read error: {e}"))?;
            Ok(raw)
        }
    }
}

/// Append a timestamped warning line to `.scryrs/hooks/<harness>-warnings.log`
/// under `base_dir`. Best-effort: a failure to log must not change exit status.
#[cfg(feature = "core")]
fn warn(
    harness: &str,
    base_dir: &std::path::Path,
    message: &str,
    debug: bool,
    err: &mut impl Write,
) {
    use std::fs;

    if debug {
        let _ = writeln!(
            err,
            "[scryrs-hook] stage=warn harness={harness} reason={message}"
        );
    }

    let log_dir = base_dir.join(".scryrs").join("hooks");
    let _ = fs::create_dir_all(&log_dir);
    let log_path = log_dir.join(format!("{harness}-warnings.log"));
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        let _ = writeln!(file, "{} {}", now_iso8601(), message);
    }
}

#[cfg(not(feature = "core"))]
pub(crate) fn execute_hook<R: Read>(
    _out: &mut impl Write,
    _err: &mut impl Write,
    _stdin: &mut R,
    _harness: &str,
    _m: &ArgMatches,
) -> i32 {
    // Fail-open even when persistence is unavailable: never block the harness.
    0
}
