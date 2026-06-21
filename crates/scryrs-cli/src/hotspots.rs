use std::io::Write;

#[cfg(feature = "core")]
use std::path::Path;

#[cfg(feature = "core")]
use scryrs_core::{QueryError, TraceQuery, score_hotspots};
#[cfg(feature = "core")]
use scryrs_types::HOTSPOT_SCHEMA_VERSION;
#[cfg(feature = "core")]
use scryrs_types::{HotspotsReport, RunMetadata};

#[cfg(feature = "core")]
use crate::chrono::chrono_now;

#[cfg(feature = "core")]
pub(crate) fn write_hotspots_json(out: &mut impl Write, err: &mut impl Write, path: &str) -> i32 {
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

    let first_event_id = subject_bearing.iter().map(|(id, _)| *id).min().unwrap_or(0);
    let last_event_id = subject_bearing.iter().map(|(id, _)| *id).max().unwrap_or(0);

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
        return 1;
    }

    0
}

/// Write success report for an empty (but valid) store.
#[cfg(feature = "core")]
pub(crate) fn write_empty_success_report(
    out: &mut impl Write,
    err: &mut impl Write,
    repo_root: &Path,
    store_path: &Path,
    store_schema_version: i64,
) -> i32 {
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
    if let Err(e) = std::fs::write(repo_root.join(".scryrs/hotspots.json"), &json) {
        let _ = writeln!(err, "scryrs hotspots: cannot write artifact file: {e}");
        return 1;
    }

    0
}

#[cfg(not(feature = "core"))]
pub(crate) fn write_hotspots_json(_out: &mut impl Write, err: &mut impl Write, _path: &str) -> i32 {
    let _ = writeln!(
        err,
        "scryrs hotspots: unavailable (core feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}
