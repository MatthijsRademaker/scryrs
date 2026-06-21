//! Canonical SQLite trace datastore owned by scryrs-core.
//!
//! The canonical accepted-event store is `.scryrs/scryrs.db` relative to the
//! current working directory. This module owns schema creation, version
//! validation, and event insertion. CLI and other consumers compose this API.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};
use scryrs_types::TraceEvent;

/// Current datastore schema version (independent of TraceEvent wire schema).
const DATASTORE_SCHEMA_VERSION: i64 = 1;

/// Canonical local datastore path relative to the current working directory.
pub const CANONICAL_STORE_PATH: &str = ".scryrs/scryrs.db";

/// Open a connection at `path`, creating parent directories and initializing
/// the schema if this is a new database.
fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

/// Ensure the schema exists: `schema_meta` version table and `trace_events`
/// table with required indexes. If the database already has a schema, validate
/// the stored version against the current one.
fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_meta (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );",
    )?;

    // Check if schema version row exists.
    let existing_version: Option<i64> = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM schema_meta WHERE key = 'datastore_schema_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    if let Some(v) = existing_version {
        if v != DATASTORE_SCHEMA_VERSION {
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "datastore schema version mismatch: found {v}, expected {DATASTORE_SCHEMA_VERSION}"
                    ),
                ),
            )));
        }
        // Schema already at correct version — nothing to create.
        return Ok(());
    }

    // New database: write version and create tables.
    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES ('datastore_schema_version', ?1)",
        params![DATASTORE_SCHEMA_VERSION.to_string()],
    )?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS trace_events (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            event_json      TEXT NOT NULL,
            schema_version  TEXT NOT NULL,
            timestamp       TEXT NOT NULL,
            session_id      TEXT NOT NULL,
            event_type      TEXT NOT NULL,
            tool_name       TEXT,
            subject_kind    TEXT,
            subject         TEXT,
            outcome         TEXT NOT NULL,
            failure_reason  TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_trace_events_subject
            ON trace_events(subject_kind, subject);
        CREATE INDEX IF NOT EXISTS idx_trace_events_event_type
            ON trace_events(event_type);
        CREATE INDEX IF NOT EXISTS idx_trace_events_session_ts
            ON trace_events(session_id, timestamp);
        CREATE INDEX IF NOT EXISTS idx_trace_events_outcome_reason
            ON trace_events(outcome, failure_reason);",
    )?;

    Ok(())
}

/// Outcome string for the `outcome` column.
fn outcome_str(event: &TraceEvent) -> &'static str {
    match &event.outcome {
        scryrs_types::Outcome::Success => "Success",
        scryrs_types::Outcome::Failure { .. } => "Failure",
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Canonical append-only SQLite trace datastore.
///
/// The store surface is intentionally narrow — it opens or creates the
/// datastore, inserts accepted events, and reports the stored count.
/// No query, delete, or analysis APIs.
pub struct EventStore {
    conn: Connection,
    stored_count: u64,
}

impl std::fmt::Debug for EventStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStore")
            .field("stored_count", &self.stored_count)
            .finish_non_exhaustive()
    }
}

impl EventStore {
    /// Open (or create) the datastore at `path`, initializing the schema and
    /// validating the datastore version.
    ///
    /// Returns an error if the datastore exists with an unsupported schema
    /// version.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let path_ref = path.as_ref();
        let conn = open_connection(path_ref)?;
        ensure_schema(&conn)?;
        Ok(Self {
            conn,
            stored_count: 0,
        })
    }

    /// Open the default local datastore at [CANONICAL_STORE_PATH] relative to
    /// the current working directory.
    pub fn default_local() -> Result<Self, rusqlite::Error> {
        Self::open(CANONICAL_STORE_PATH)
    }

    /// Insert a single accepted event into the datastore.
    ///
    /// The event is stored as canonical `serde_json` serialization of the
    /// validated `TraceEvent` plus normalized query columns.
    pub fn append(&mut self, event: &TraceEvent) -> Result<(), rusqlite::Error> {
        let event_json = serde_json::to_string(event)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        let subject = event.subject().map(|s| s.to_string());
        let sk = event.subject_kind().map(|s| s.to_string());
        let fr = event.failure_reason().map(|s| s.to_string());

        self.conn.execute(
            "INSERT INTO trace_events
                (event_json, schema_version, timestamp, session_id, event_type,
                 tool_name, subject_kind, subject, outcome, failure_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                event_json,
                event.schema_version,
                event.timestamp,
                event.session_id,
                event.event_type.payload_type_str(),
                event.tool_name,
                sk,
                subject,
                outcome_str(event),
                fr,
            ],
        )?;

        self.stored_count += 1;
        Ok(())
    }

    /// Number of events inserted into this store instance so far.
    #[must_use]
    pub fn stored_count(&self) -> u64 {
        self.stored_count
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use scryrs_types::{
        DocRetrievedPayload, Outcome, SCHEMA_VERSION, SessionStartPayload, TraceEvent,
        TraceEventPayload, TraceEventType,
    };

    use super::*;

    fn make_event(session_id: &str, doc_ref: &str) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: session_id.into(),
            event_type: TraceEventType::DocRetrieved,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: doc_ref.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"))
    }

    fn open_ok(path: &std::path::Path) -> EventStore {
        EventStore::open(path).unwrap_or_else(|e| panic!("open store: {e}"))
    }

    fn open_default_in(dir: &std::path::Path) -> EventStore {
        let cwd = std::env::current_dir().unwrap_or_else(|e| panic!("current dir: {e}"));
        std::env::set_current_dir(dir).unwrap_or_else(|e| panic!("chdir: {e}"));

        let result = EventStore::default_local();

        std::env::set_current_dir(&cwd).unwrap_or_else(|e| panic!("restore cwd: {e}"));

        result.unwrap_or_else(|e| panic!("default_local should succeed: {e}"))
    }

    // --- Schema creation ---

    #[test]
    fn schema_creates_tables_and_indexes() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        let _store = open_ok(&store_path);
        assert!(store_path.exists());

        // Re-open and check structure via PRAGMA.
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));

        // schema_meta exists with correct version
        let version: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'datastore_schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("version query: {e}"));
        assert_eq!(version, "1");

        // trace_events table exists
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='trace_events'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("table check: {e}"));
        assert_eq!(table_count, 1);

        // Indexes exist
        let expected_indexes = [
            "idx_trace_events_subject",
            "idx_trace_events_event_type",
            "idx_trace_events_session_ts",
            "idx_trace_events_outcome_reason",
        ];
        for idx_name in &expected_indexes {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
                    params![idx_name],
                    |row| row.get(0),
                )
                .unwrap_or_else(|e| panic!("index check {idx_name}: {e}"));
            assert_eq!(count, 1, "index {idx_name} must exist");
        }
    }

    // --- Canonical path ---

    #[test]
    fn default_local_creates_dot_scryrs_dir_and_db() {
        let dir = temp_dir();
        let mut store = open_default_in(dir.path());
        store
            .append(&make_event("s1", "doc/x.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));
        assert_eq!(store.stored_count(), 1);
        assert!(dir.path().join(".scryrs/scryrs.db").exists());
        // Old JSONL path must NOT be created.
        assert!(
            !dir.path().join(".scryrs/events.jsonl").exists(),
            ".scryrs/events.jsonl must not be created"
        );
    }

    #[test]
    fn open_creates_parent_directories() {
        let dir = temp_dir();
        let nested = dir.path().join("sub1/sub2/scryrs.db");

        let mut store = open_ok(&nested);
        store
            .append(&make_event("s1", "doc/z.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));

        assert!(nested.exists());
    }

    // --- Row insertion and normalized field extraction ---

    #[test]
    fn store_creates_and_inserts() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        {
            let mut store = open_ok(&store_path);
            store
                .append(&make_event("s1", "doc/a.md"))
                .unwrap_or_else(|e| panic!("append 1: {e}"));
            store
                .append(&make_event("s2", "doc/b.md"))
                .unwrap_or_else(|e| panic!("append 2: {e}"));
            assert_eq!(store.stored_count(), 2);
        }

        // Verify rows via direct SQLite query.
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(count, 2);

        // Check event_json is valid canonical JSON
        let mut stmt = conn
            .prepare("SELECT event_json, subject, subject_kind, outcome, failure_reason, event_type FROM trace_events ORDER BY rowid")
            .unwrap_or_else(|e| panic!("prepare: {e}"));

        let rows: Vec<_> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .unwrap_or_else(|e| panic!("query_map: {e}"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| panic!("collect: {e}"));

        assert_eq!(rows.len(), 2);

        // First row
        let (json1, subject1, kind1, outcome1, reason1, event_type1) = &rows[0];
        let parsed: TraceEvent =
            serde_json::from_str(json1).unwrap_or_else(|e| panic!("deserialize event_json: {e}"));
        assert_eq!(parsed.session_id, "s1");
        assert_eq!(subject1.as_deref(), Some("doc/a.md"));
        assert_eq!(kind1.as_deref(), Some("document"));
        assert_eq!(outcome1.as_str(), "Success");
        assert_eq!(reason1.as_ref(), None);
        assert_eq!(event_type1.as_str(), "DocRetrieved");

        // Second row
        let (_json2, subject2, kind2, outcome2, reason2, event_type2) = &rows[1];
        assert_eq!(subject2.as_deref(), Some("doc/b.md"));
        assert_eq!(kind2.as_deref(), Some("document"));
        assert_eq!(outcome2.as_str(), "Success");
        assert_eq!(reason2.as_ref(), None);
        assert_eq!(event_type2.as_str(), "DocRetrieved");
    }

    #[test]
    fn store_count_is_zero_initially() {
        let dir = temp_dir();
        let store_path = dir.path().join("fresh.db");

        let store = open_ok(&store_path);
        assert_eq!(store.stored_count(), 0);
    }

    // --- Normalized field extraction: lifecycle events ---

    #[test]
    fn lifecycle_event_has_null_subject_and_kind() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        let event = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s-lifecycle".into(),
            event_type: TraceEventType::SessionStart,
            tool_name: None,
            payload: TraceEventPayload::SessionStart(SessionStartPayload),
            outcome: Outcome::Success,
        };

        {
            let mut store = open_ok(&store_path);
            store
                .append(&event)
                .unwrap_or_else(|e| panic!("append lifecycle: {e}"));
        }

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let (sk, subj): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT subject_kind, subject FROM trace_events WHERE event_type='SessionStart'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|e| panic!("query lifecycle: {e}"));
        assert!(
            sk.is_none(),
            "subject_kind must be NULL for lifecycle events"
        );
        assert!(subj.is_none(), "subject must be NULL for lifecycle events");
    }

    // --- Failure reason extraction ---

    #[test]
    fn failure_reason_is_persisted() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        let event = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s-fail".into(),
            event_type: TraceEventType::CommandExecuted,
            tool_name: Some("bash".into()),
            payload: TraceEventPayload::CommandExecuted(scryrs_types::CommandExecutedPayload {
                command: "bad-command".into(),
            }),
            outcome: Outcome::Failure {
                reason: Some("exit code 1".into()),
            },
        };

        {
            let mut store = open_ok(&store_path);
            store
                .append(&event)
                .unwrap_or_else(|e| panic!("append failure: {e}"));
        }

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let (outcome_col, reason_col): (String, Option<String>) = conn
            .query_row(
                "SELECT outcome, failure_reason FROM trace_events WHERE session_id='s-fail'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|e| panic!("query failure: {e}"));
        assert_eq!(outcome_col, "Failure");
        assert_eq!(reason_col.as_deref(), Some("exit code 1"));
    }

    // --- subject_kind mapping coverage ---

    #[test]
    fn subject_kind_covers_all_subject_bearing_families() {
        use scryrs_types::{
            CommandExecutedPayload, EditMadePayload, FailedLookupPayload, FileOpenedPayload,
            SearchRunPayload, SymbolInspectedPayload,
        };

        let cases: Vec<(&str, TraceEvent)> = vec![
            (
                "file",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::FileOpened,
                    tool_name: Some("read".into()),
                    payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                        path: "a.rs".into(),
                    }),
                    outcome: Outcome::Success,
                },
            ),
            (
                "file",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::EditMade,
                    tool_name: Some("edit".into()),
                    payload: TraceEventPayload::EditMade(EditMadePayload {
                        target: "b.rs".into(),
                    }),
                    outcome: Outcome::Success,
                },
            ),
            (
                "document",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::DocRetrieved,
                    tool_name: Some("read".into()),
                    payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                        doc_ref: "api.md".into(),
                    }),
                    outcome: Outcome::Success,
                },
            ),
            (
                "search",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::SearchRun,
                    tool_name: Some("grep".into()),
                    payload: TraceEventPayload::SearchRun(SearchRunPayload { query: "fn".into() }),
                    outcome: Outcome::Success,
                },
            ),
            (
                "symbol",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::SymbolInspected,
                    tool_name: Some("lsp".into()),
                    payload: TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                        name: "Foo".into(),
                    }),
                    outcome: Outcome::Success,
                },
            ),
            (
                "symbol",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::FailedLookup,
                    tool_name: Some("lsp".into()),
                    payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
                        subject: "Bar".into(),
                    }),
                    outcome: Outcome::Failure { reason: None },
                },
            ),
            (
                "command",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::CommandExecuted,
                    tool_name: Some("bash".into()),
                    payload: TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                        command: "cargo build".into(),
                    }),
                    outcome: Outcome::Success,
                },
            ),
        ];

        for (expected_kind, event) in &cases {
            let actual = event.subject_kind();
            assert_eq!(
                actual,
                Some(*expected_kind),
                "subject_kind for {:?} should be {expected_kind}",
                event.event_type,
            );
        }

        // Lifecycle events return None
        for event_type in [TraceEventType::SessionStart, TraceEventType::SessionEnd] {
            let lifecycle = TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "t".into(),
                session_id: "s".into(),
                event_type,
                tool_name: None,
                payload: match event_type {
                    TraceEventType::SessionStart => {
                        TraceEventPayload::SessionStart(SessionStartPayload)
                    }
                    _ => TraceEventPayload::SessionEnd(scryrs_types::SessionEndPayload),
                },
                outcome: Outcome::Success,
            };
            assert!(
                lifecycle.subject_kind().is_none(),
                "subject_kind for {event_type:?} must be None"
            );
        }
    }

    // --- Unknown schema version fails fast ---

    #[test]
    fn unknown_schema_version_fails_fast() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        // Create a database with a future schema version.
        {
            let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("create: {e}"));
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO schema_meta VALUES ('datastore_schema_version', '99');",
            )
            .unwrap_or_else(|e| panic!("write version: {e}"));
        }

        let result = EventStore::open(&store_path);
        assert!(
            result.is_err(),
            "opening with unknown schema version must fail"
        );
        let err = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error, got Ok"),
        };
        assert!(
            err.contains("schema version mismatch"),
            "error must mention version mismatch, got: {err}"
        );
        assert!(err.contains("99"), "error must mention found version 99");
        assert!(
            err.contains(&DATASTORE_SCHEMA_VERSION.to_string()),
            "error must mention expected version"
        );
    }
}
