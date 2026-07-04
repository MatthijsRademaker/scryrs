use std::io::{self, Error, ErrorKind, Write};

#[cfg(feature = "core")]
use std::{path::Path, time::Duration};

#[cfg(feature = "core")]
use scryrs_core::{QueryError, TraceQuery, score_hotspots};
#[cfg(feature = "core")]
use scryrs_types::{
    HOTSPOT_SCHEMA_VERSION, HotspotsReport, LIVE_HOTSPOT_SCHEMA_VERSION, LiveHotspotsResponse,
    RunMetadata,
};

#[cfg(feature = "core")]
use crate::chrono::chrono_now;
#[cfg(feature = "core")]
use crate::remote_config::{RemoteConfigError, RemoteOverrides, resolve_remote_inputs};

pub(crate) const HOTSPOTS_USAGE: &str =
    "scryrs hotspots <PATH> [--mode <local|live>] [--server-url <URL>] [--repository-id <ID>]";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotspotsMode {
    Local,
    Live,
}

pub(crate) fn parse_hotspots_mode(value: &str) -> Option<HotspotsMode> {
    match value {
        "local" => Some(HotspotsMode::Local),
        "live" => Some(HotspotsMode::Live),
        _ => None,
    }
}

pub(crate) fn write_hotspots_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs hotspots <PATH>\n\
\n\
Materialize .scryrs/hotspots.json from either the local SQLite store\n\
(default) or the live hotspot server.\n\
\n\
FLAGS\n\
  --mode <local|live>\n\
      Source mode. local is the default when --mode is omitted.\n\
  --server-url <URL>\n\
      Live-mode server URL. Resolution precedence: flag, then\n\
      SCRYRS_REMOTE_INGEST_URL, then .scryrs/.env, then scryrs.json\n\
      remote.ingest_url.\n\
  --repository-id <ID>\n\
      Live-mode repository identity. Resolution precedence: flag, then\n\
      SCRYRS_REPOSITORY_ID, then .scryrs/.env, then scryrs.json\n\
      remote.repository_id.\n\
\n\
BEHAVIOR\n\
  local\n\
      Reads .scryrs/scryrs.db, scores local trace events, and writes the\n\
      versioned HotspotsReport artifact to .scryrs/hotspots.json.\n\
  live\n\
      Queries GET /v1/repositories/<ID>/hotspots?window=cumulative, writes the\n\
      same HotspotsReport artifact shape, and does not merge local SQLite data.\n\
\n\
OUTPUT\n\
  Success writes .scryrs/hotspots.json atomically in live mode, mirrors the\n\
  same JSON to stdout, and records the live source as\n\
  storePath = live:<query_url>."
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct HotspotsOptions<'a> {
    pub path: &'a str,
    pub mode: HotspotsMode,
    pub server_url: Option<&'a str>,
    pub repository_id: Option<&'a str>,
}

#[cfg(feature = "core")]
pub(crate) fn write_hotspots_json(
    out: &mut impl Write,
    err: &mut impl Write,
    options: HotspotsOptions<'_>,
) -> i32 {
    write_hotspots_json_with_fetcher(out, err, options, &UreqLiveHotspotsFetcher)
}

#[cfg(feature = "core")]
fn write_hotspots_json_with_fetcher(
    out: &mut impl Write,
    err: &mut impl Write,
    options: HotspotsOptions<'_>,
    fetcher: &dyn LiveHotspotsFetcher,
) -> i32 {
    let repo_root = match std::path::absolute(options.path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: cannot resolve path '{}': {e}",
                options.path
            );
            return 2;
        }
    };

    match options.mode {
        HotspotsMode::Local => write_local_hotspots_json(out, err, &repo_root),
        HotspotsMode::Live => {
            let live_target =
                match resolve_live_target(&repo_root, options.server_url, options.repository_id) {
                    Ok(target) => target,
                    Err(error) => {
                        let _ =
                            writeln!(err, "scryrs hotspots: {}", format_live_target_error(&error));
                        let _ = writeln!(err, "See `scryrs --help`");
                        return 2;
                    }
                };
            write_live_hotspots_json(out, err, &repo_root, &live_target, fetcher)
        }
    }
}

#[cfg(all(feature = "core", test))]
fn write_hotspots_json_for_tests(
    out: &mut impl Write,
    err: &mut impl Write,
    options: HotspotsOptions<'_>,
    fetcher: &dyn LiveHotspotsFetcher,
) -> i32 {
    write_hotspots_json_with_fetcher(out, err, options, fetcher)
}

#[cfg(feature = "core")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedLiveTarget {
    server_url: String,
    repository_id: String,
    timeout_ms: u64,
}

#[cfg(feature = "core")]
fn resolve_live_target(
    repo_root: &Path,
    server_url_override: Option<&str>,
    repository_id_override: Option<&str>,
) -> Result<ResolvedLiveTarget, RemoteConfigError> {
    let inputs = resolve_remote_inputs(
        Some(repo_root),
        &RemoteOverrides {
            ingest_url: server_url_override.map(str::to_string),
            repository_id: repository_id_override.map(str::to_string),
            ..RemoteOverrides::default()
        },
    );

    let server_url = match inputs.ingest_url {
        Some(value) => value,
        None => {
            return Err(RemoteConfigError::MissingIdentity {
                field: "ingest_url",
            });
        }
    };
    let repository_id = match inputs.repository_id {
        Some(value) => value,
        None => {
            return Err(RemoteConfigError::MissingIdentity {
                field: "repository_id",
            });
        }
    };

    Ok(ResolvedLiveTarget {
        server_url,
        repository_id,
        timeout_ms: inputs.timeout_ms,
    })
}

#[cfg(feature = "core")]
fn format_live_target_error(error: &RemoteConfigError) -> String {
    match error.missing_field() {
        "ingest_url" => "live mode requires server_url — set --server-url, SCRYRS_REMOTE_INGEST_URL, .scryrs/.env, or scryrs.json remote.ingest_url".into(),
        "repository_id" => "live mode requires repository_id — set --repository-id, SCRYRS_REPOSITORY_ID, .scryrs/.env, or scryrs.json remote.repository_id".into(),
        _ => error.to_string(),
    }
}

#[cfg(feature = "core")]
trait LiveHotspotsFetcher {
    fn fetch(&self, query_url: &str, timeout_ms: u64) -> Result<String, LiveFetchError>;
}

#[cfg(feature = "core")]
struct UreqLiveHotspotsFetcher;

#[cfg(feature = "core")]
#[derive(Debug)]
enum LiveFetchError {
    Timeout,
    Connection(String),
    HttpStatus { status: u16, body: String },
    ResponseRead(String),
}

#[cfg(feature = "core")]
impl LiveHotspotsFetcher for UreqLiveHotspotsFetcher {
    fn fetch(&self, query_url: &str, timeout_ms: u64) -> Result<String, LiveFetchError> {
        let response = ureq::get(query_url)
            .set("Accept", "application/json")
            .set("User-Agent", "scryrs-cli/0.1.0")
            .timeout(Duration::from_millis(timeout_ms))
            .call();

        match response {
            Ok(resp) => resp
                .into_string()
                .map_err(|error| LiveFetchError::ResponseRead(error.to_string())),
            Err(ureq::Error::Status(status, resp)) => {
                let body = resp
                    .into_string()
                    .unwrap_or_else(|error| format!("<read error: {error}>"));
                Err(LiveFetchError::HttpStatus { status, body })
            }
            Err(ureq::Error::Transport(error)) => {
                let message = error.to_string();
                if message.contains("timed out") || message.contains("Timeout") {
                    Err(LiveFetchError::Timeout)
                } else {
                    Err(LiveFetchError::Connection(message))
                }
            }
        }
    }
}

#[cfg(feature = "core")]
fn write_live_hotspots_json(
    out: &mut impl Write,
    err: &mut impl Write,
    repo_root: &Path,
    live_target: &ResolvedLiveTarget,
    fetcher: &dyn LiveHotspotsFetcher,
) -> i32 {
    let query_url = live_query_url(&live_target.server_url, &live_target.repository_id);
    let body = match fetcher.fetch(&query_url, live_target.timeout_ms) {
        Ok(body) => body,
        Err(LiveFetchError::Timeout) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: live hotspot export timed out after {} ms",
                live_target.timeout_ms
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
        Err(LiveFetchError::Connection(error)) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: cannot reach live hotspot server: {error}"
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
        Err(LiveFetchError::HttpStatus { status, body }) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: live hotspot server returned HTTP {status}: {}",
                body.lines().next().unwrap_or("(empty body)")
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
        Err(LiveFetchError::ResponseRead(error)) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: cannot read live hotspot response: {error}"
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
    };

    let response: LiveHotspotsResponse = match serde_json::from_str(&body) {
        Ok(response) => response,
        Err(error) => {
            let _ = writeln!(
                err,
                "scryrs hotspots: malformed live hotspot response: {error}"
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
    };

    if response.schemaVersion != LIVE_HOTSPOT_SCHEMA_VERSION {
        let _ = writeln!(
            err,
            "scryrs hotspots: live hotspot schema version mismatch: got '{}', expected '{}'",
            response.schemaVersion, LIVE_HOTSPOT_SCHEMA_VERSION
        );
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    if response.repositoryId != live_target.repository_id {
        let _ = writeln!(
            err,
            "scryrs hotspots: live hotspot response repository mismatch: requested '{}', got '{}'",
            live_target.repository_id, response.repositoryId
        );
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    let report = live_response_to_report(repo_root, &query_url, response);
    let json = match serde_json::to_string(&report) {
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "scryrs hotspots: serialization error: {error}");
            return 1;
        }
    };
    let json_with_newline = format!("{json}\n");

    let artifact_path = repo_root.join(".scryrs/hotspots.json");
    if let Err(error) = write_file_atomically(&artifact_path, json_with_newline.as_bytes()) {
        let _ = writeln!(err, "scryrs hotspots: cannot write artifact file: {error}");
        return 1;
    }

    if write!(out, "{json_with_newline}").is_err() {
        return 1;
    }
    if writeln!(
        err,
        "scryrs hotspots: exported live hotspots from {} for repository {}",
        live_target.server_url, live_target.repository_id
    )
    .is_err()
    {
        return 1;
    }

    0
}

#[cfg(feature = "core")]
fn live_query_url(server_url: &str, repository_id: &str) -> String {
    let base = server_url.trim_end_matches('/');
    let encoded_repository_id = percent_encode_path_segment(repository_id);
    format!("{base}/v1/repositories/{encoded_repository_id}/hotspots?window=cumulative")
}

#[cfg(feature = "core")]
fn percent_encode_path_segment(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }
    encoded
}

#[cfg(feature = "core")]
fn live_response_to_report(
    repo_root: &Path,
    query_url: &str,
    response: LiveHotspotsResponse,
) -> HotspotsReport {
    let analyzed_event_count = response
        .entries
        .iter()
        .map(|entry| entry.evidence.rowIds.len() as u64)
        .sum();
    let analyzed_subject_count = response.entries.len() as u64;

    HotspotsReport {
        schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
        command: "hotspots".into(),
        repositoryPath: repo_root.display().to_string(),
        storePath: format!("live:{query_url}"),
        runMetadata: RunMetadata {
            storeSchemaVersion: 0,
            analyzedEventCount: analyzed_event_count,
            analyzedSubjectCount: analyzed_subject_count,
            firstEventId: 0,
            lastEventId: 0,
        },
        generatedAt: response.generatedAt,
        entries: response.entries,
    }
}

#[cfg(feature = "core")]
fn write_file_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("artifact path has no parent: {}", path.display()),
        ));
    };
    std::fs::create_dir_all(parent)?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact.json");
    let temp_path = parent.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));

    std::fs::write(&temp_path, contents)?;
    if let Err(error) = std::fs::rename(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error);
    }

    Ok(())
}

#[cfg(feature = "core")]
fn write_local_hotspots_json(out: &mut impl Write, err: &mut impl Write, repo_root: &Path) -> i32 {
    let store_path = repo_root.join(".scryrs/scryrs.db");

    let query = match TraceQuery::open(repo_root) {
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
        Err(QueryError::StorageError(error)) => {
            let _ = writeln!(err, "scryrs hotspots: storage error: {error}");
            return 1;
        }
        _ => {
            let _ = writeln!(err, "scryrs hotspots: unexpected error opening store");
            return 1;
        }
    };

    let events_with_ids = match query.iter_events_with_ids_ordered() {
        Ok(events) => events,
        Err(QueryError::EmptyStore) => {
            return write_empty_success_report(
                out,
                err,
                repo_root,
                &store_path,
                query.store_schema_version(),
            );
        }
        Err(QueryError::StorageError(error)) => {
            let _ = writeln!(err, "scryrs hotspots: storage error: {error}");
            return 1;
        }
        Err(error) => {
            let _ = writeln!(err, "scryrs hotspots: {error}");
            return 1;
        }
    };

    let subject_bearing: Vec<_> = events_with_ids
        .iter()
        .filter(|(_, event)| event.subject().is_some())
        .collect();

    let subject_set: std::collections::HashSet<String> = subject_bearing
        .iter()
        .filter_map(|(_, event)| {
            let kind = event.subject_kind()?;
            let subject = event.subject()?;
            Some(format!("{kind}:{subject}"))
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

    let event_refs: Vec<(u64, &scryrs_types::TraceEvent)> = events_with_ids
        .iter()
        .map(|(id, event)| (*id, event))
        .collect();
    let entries = score_hotspots(&event_refs);

    let report = HotspotsReport {
        schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
        command: "hotspots".into(),
        repositoryPath: repo_root.display().to_string(),
        storePath: store_path.display().to_string(),
        runMetadata: run_metadata,
        generatedAt: chrono_now(),
        entries,
    };

    let json = match serde_json::to_string(&report) {
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "scryrs hotspots: serialization error: {error}");
            return 1;
        }
    };

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    if let Err(error) = std::fs::write(repo_root.join(".scryrs/hotspots.json"), &json) {
        let _ = writeln!(err, "scryrs hotspots: cannot write artifact file: {error}");
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
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "scryrs hotspots: serialization error: {error}");
            return 1;
        }
    };

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    if let Err(error) = std::fs::write(repo_root.join(".scryrs/hotspots.json"), &json) {
        let _ = writeln!(err, "scryrs hotspots: cannot write artifact file: {error}");
        return 1;
    }

    0
}

#[cfg(not(feature = "core"))]
pub(crate) fn write_hotspots_json(
    _out: &mut impl Write,
    err: &mut impl Write,
    _options: HotspotsOptions<'_>,
) -> i32 {
    let _ = writeln!(
        err,
        "scryrs hotspots: unavailable (core feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

#[cfg(all(test, feature = "core"))]
mod tests {
    use super::*;
    use crate::run_with_writers;
    use crate::test_support::with_cwd;
    use scryrs_core::EventStore;
    use scryrs_types::{
        FileOpenedPayload, HotspotCounts, HotspotEntry, HotspotEvidence, LiveHotspotsResponse,
        Outcome, TraceEvent, TraceEventPayload, TraceEventType,
    };
    use std::collections::HashMap;

    struct StubFetcher {
        response: Result<String, LiveFetchError>,
    }

    impl LiveHotspotsFetcher for StubFetcher {
        fn fetch(&self, _query_url: &str, _timeout_ms: u64) -> Result<String, LiveFetchError> {
            match &self.response {
                Ok(body) => Ok(body.clone()),
                Err(LiveFetchError::Timeout) => Err(LiveFetchError::Timeout),
                Err(LiveFetchError::Connection(error)) => {
                    Err(LiveFetchError::Connection(error.clone()))
                }
                Err(LiveFetchError::HttpStatus { status, body }) => {
                    Err(LiveFetchError::HttpStatus {
                        status: *status,
                        body: body.clone(),
                    })
                }
                Err(LiveFetchError::ResponseRead(error)) => {
                    Err(LiveFetchError::ResponseRead(error.clone()))
                }
            }
        }
    }

    fn make_file_opened(session_id: &str, path: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
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
        let mut store = EventStore::open(&store_path).unwrap_or_else(|e| panic!("open store: {e}"));
        store
            .begin_transaction()
            .unwrap_or_else(|e| panic!("begin: {e}"));
        for event in events {
            store
                .append(event)
                .unwrap_or_else(|e| panic!("append {event:?}: {e}"));
        }
        store
            .commit_transaction()
            .unwrap_or_else(|e| panic!("commit: {e}"));
    }

    fn make_live_entry(subject: &str, row_ids: Vec<u64>) -> HotspotEntry {
        HotspotEntry {
            rank: 1,
            subjectKind: "file".into(),
            subject: subject.into(),
            score: row_ids.len() as u32,
            counts: HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 1,
            firstSeen: "2026-07-01T10:00:00Z".into(),
            lastSeen: "2026-07-01T10:00:00Z".into(),
            evidence: HotspotEvidence { rowIds: row_ids },
        }
    }

    fn live_response_json(repository_id: &str, entries: Vec<HotspotEntry>) -> String {
        serde_json::to_string(&LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: repository_id.into(),
            cursor: String::new(),
            generatedAt: "2026-07-01T12:00:00Z".into(),
            entries,
        })
        .unwrap_or_else(|e| panic!("serialize live response: {e}"))
    }

    fn repo_path(dir: &tempfile::TempDir) -> String {
        dir.path().to_string_lossy().into_owned()
    }

    #[test]
    fn live_export_writes_hotspots_artifact_without_opening_local_sqlite() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::write(scryrs_dir.join("scryrs.db"), "not sqlite\n")
            .unwrap_or_else(|e| panic!("write corrupt db: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(live_response_json(
                "repo-a",
                vec![make_live_entry("src/live.rs", vec![11, 12])],
            )),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 0, "stderr: {}", String::from_utf8_lossy(&err));
        let stdout = String::from_utf8_lossy(&out);
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim_end()).unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(report["generatedAt"], "2026-07-01T12:00:00Z");
        assert_eq!(
            report["storePath"],
            "live:http://live.example/v1/repositories/repo-a/hotspots?window=cumulative"
        );
        assert_eq!(report["runMetadata"]["storeSchemaVersion"], 0);
        assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 1);
        assert_eq!(report["runMetadata"]["analyzedEventCount"], 2);
        assert_eq!(report["entries"][0]["subject"], "src/live.rs");

        let artifact = std::fs::read_to_string(dir.path().join(".scryrs/hotspots.json"))
            .unwrap_or_else(|e| panic!("read artifact: {e}"));
        assert_eq!(
            artifact, stdout,
            "live artifact must match stdout byte-for-byte"
        );
        assert!(
            String::from_utf8_lossy(&err)
                .contains("exported live hotspots from http://live.example for repository repo-a")
        );
    }

    #[test]
    fn live_export_uses_only_live_entries_and_does_not_merge_local_subjects() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        populate_store(
            &dir,
            &[make_file_opened(
                "s1",
                "src/local-only.rs",
                "2026-07-01T09:00:00Z",
            )],
        );

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(live_response_json(
                "repo-a",
                vec![make_live_entry("src/live-only.rs", vec![99])],
            )),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 0, "stderr: {}", String::from_utf8_lossy(&err));
        let report: serde_json::Value =
            serde_json::from_slice(&out).unwrap_or_else(|e| panic!("parse: {e}"));
        let entries = report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["subject"], "src/live-only.rs");
    }

    #[test]
    fn live_export_validation_failures_leave_existing_artifact_unchanged() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        let artifact_path = scryrs_dir.join("hotspots.json");
        std::fs::write(&artifact_path, "original-artifact")
            .unwrap_or_else(|e| panic!("seed artifact: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok("{not-json".into()),
        };
        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8_lossy(&err).contains("malformed live hotspot response"),
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        let artifact =
            std::fs::read_to_string(&artifact_path).unwrap_or_else(|e| panic!("read: {e}"));
        assert_eq!(artifact, "original-artifact");
    }

    #[test]
    fn live_export_non_2xx_failure_leaves_existing_artifact_unchanged() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        let artifact_path = scryrs_dir.join("hotspots.json");
        std::fs::write(&artifact_path, "original-artifact")
            .unwrap_or_else(|e| panic!("seed artifact: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Err(LiveFetchError::HttpStatus {
                status: 503,
                body: "server overloaded".into(),
            }),
        };
        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("returned HTTP 503"));
        let artifact =
            std::fs::read_to_string(&artifact_path).unwrap_or_else(|e| panic!("read: {e}"));
        assert_eq!(artifact, "original-artifact");
    }

    #[test]
    fn live_export_timeout_failure_reports_clear_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Err(LiveFetchError::Timeout),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8_lossy(&err).contains("live hotspot export timed out after 3000 ms"),
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
    }

    #[test]
    fn live_export_connection_failure_reports_clear_error() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Err(LiveFetchError::Connection("connection refused".into())),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8_lossy(&err)
                .contains("cannot reach live hotspot server: connection refused"),
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
    }

    #[test]
    fn live_export_schema_mismatch_fails_before_write() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(serde_json::json!({
                "schemaVersion": "99.0.0",
                "repositoryId": "repo-a",
                "cursor": "",
                "generatedAt": "2026-07-01T12:00:00Z",
                "entries": [make_live_entry("src/live.rs", vec![1])],
            })
            .to_string()),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8_lossy(&err).contains("live hotspot schema version mismatch"),
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(!dir.path().join(".scryrs/hotspots.json").exists());
    }

    #[test]
    fn live_export_reports_repository_identity_mismatch() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(live_response_json(
                "repo-b",
                vec![make_live_entry("src/live.rs", vec![1])],
            )),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty());
        assert!(
            String::from_utf8_lossy(&err).contains("requested 'repo-a', got 'repo-b'"),
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(!dir.path().join(".scryrs/hotspots.json").exists());
    }

    #[test]
    fn live_export_cleans_up_temp_file_on_atomic_write_failure() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::create_dir(scryrs_dir.join("hotspots.json"))
            .unwrap_or_else(|e| panic!("create artifact dir: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(live_response_json(
                "repo-a",
                vec![make_live_entry("src/live.rs", vec![1])],
            )),
        };

        let exit = write_hotspots_json_for_tests(
            &mut out,
            &mut err,
            HotspotsOptions {
                path: &repo_path(&dir),
                mode: HotspotsMode::Live,
                server_url: Some("http://live.example"),
                repository_id: Some("repo-a"),
            },
            &fetcher,
        );

        assert_eq!(exit, 1);
        let leftover_tmp_files: Vec<_> = std::fs::read_dir(&scryrs_dir)
            .unwrap_or_else(|e| panic!("read .scryrs: {e}"))
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains(".tmp-"))
            .collect();
        assert!(
            leftover_tmp_files.is_empty(),
            "leftover tmp files: {leftover_tmp_files:?}"
        );
        assert!(String::from_utf8_lossy(&err).contains("cannot write artifact file"));
    }

    #[test]
    fn live_query_url_percent_encodes_repository_id_path_segment() {
        assert_eq!(
            live_query_url("http://live.example/", "https://github.com/acme/widgets",),
            "http://live.example/v1/repositories/https%3A%2F%2Fgithub.com%2Facme%2Fwidgets/hotspots?window=cumulative"
        );
    }

    #[test]
    fn resolve_live_target_uses_flag_then_dotenv_then_manifest_precedence() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        std::fs::write(
            dir.path().join("scryrs.json"),
            r#"{"remote":{"ingest_url":"http://manifest.example","repository_id":"manifest-repo"}}"#,
        )
        .unwrap_or_else(|e| panic!("write manifest: {e}"));
        std::fs::create_dir_all(dir.path().join(".scryrs"))
            .unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::write(
            dir.path().join(".scryrs/.env"),
            "SCRYRS_REMOTE_INGEST_URL=http://dotenv.example\nSCRYRS_REPOSITORY_ID=dotenv-repo\n",
        )
        .unwrap_or_else(|e| panic!("write dotenv: {e}"));

        with_cwd(dir.path(), || {
            let resolved = resolve_live_target(dir.path(), None, None)
                .unwrap_or_else(|e| panic!("resolve from dotenv: {e}"));
            assert_eq!(resolved.server_url, "http://dotenv.example");
            assert_eq!(resolved.repository_id, "dotenv-repo");

            let resolved =
                resolve_live_target(dir.path(), Some("http://flag.example"), Some("flag-repo"))
                    .unwrap_or_else(|e| panic!("resolve from flags: {e}"));
            assert_eq!(resolved.server_url, "http://flag.example");
            assert_eq!(resolved.repository_id, "flag-repo");
        });
    }

    #[test]
    fn live_exported_artifact_runs_through_graph_route_and_propose() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
        let docs_dir = dir.path().join(".devagent/docs/docs");
        std::fs::create_dir_all(&docs_dir).unwrap_or_else(|e| panic!("create docs: {e}"));
        std::fs::write(docs_dir.join("_nav.json"), "[]")
            .unwrap_or_else(|e| panic!("write nav: {e}"));
        std::fs::write(docs_dir.join("graph.md"), "# Graph\n")
            .unwrap_or_else(|e| panic!("write doc: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();
        let fetcher = StubFetcher {
            response: Ok(live_response_json(
                "repo-a",
                vec![make_live_entry("src/live.rs", vec![1, 2])],
            )),
        };
        assert_eq!(
            write_hotspots_json_for_tests(
                &mut out,
                &mut err,
                HotspotsOptions {
                    path: &repo_path(&dir),
                    mode: HotspotsMode::Live,
                    server_url: Some("http://live.example"),
                    repository_id: Some("repo-a"),
                },
                &fetcher,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["graph", &repo_path(&dir)], &mut out, &mut err),
            0,
            "graph stderr: {}",
            String::from_utf8_lossy(&err)
        );

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["route", &repo_path(&dir)], &mut out, &mut err),
            0,
            "route stderr: {}",
            String::from_utf8_lossy(&err)
        );

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["propose", &repo_path(&dir)], &mut out, &mut err),
            0,
            "propose stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(dir.path().join(".scryrs/graph.json").exists());
        assert!(dir.path().join(".scryrs/routes.json").exists());
        assert!(dir.path().join(".scryrs/proposals").exists());
    }
}
