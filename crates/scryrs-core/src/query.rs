//! Read-only trace query surface for the canonical scryrs SQLite datastore.
//!
//! `TraceQuery` opens a `.scryrs/scryrs.db` database in guaranteed read-only,
//! non-creating mode. It provides deterministic, indexed query methods over the
//! normalized `trace_events` columns. All query methods return materialized
//! `Vec<TraceEvent>` ordered by `timestamp ASC, id ASC`.
//!
//! # Error model
//!
//! - `MissingStore` — the database file does not exist at the given path.
//! - `EmptyStore` — the database is valid but `trace_events` has zero rows.
//! - `UnsupportedStore(String)` — schema version mismatch or missing tables.
//! - `StorageError(rusqlite::Error)` — corrupt file, permissions, or other
//!   SQLite-level failure.
//!
//! # No mutation guarantee
//!
//! `TraceQuery::open` uses `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE` and
//! performs only read-only `SELECT` queries. It never creates directories,
//! executes DDL, or applies `PRAGMA journal_mode`.

use std::path::Path;

use rusqlite::{Connection, OpenFlags, params};
use scryrs_types::TraceEvent;

use crate::store::DATASTORE_SCHEMA_VERSION;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Errors returned by the read-only trace query surface.
#[derive(Debug)]
pub enum QueryError {
    /// `.scryrs/scryrs.db` does not exist at the given repository root.
    MissingStore,
    /// The database is valid but `trace_events` contains zero rows.
    EmptyStore,
    /// Schema version does not match or required tables are missing.
    UnsupportedStore(String),
    /// Corrupt file, I/O failure, permissions, or other SQLite-level error.
    StorageError(rusqlite::Error),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::MissingStore => write!(
                f,
                "scryrs datastore not found: no .scryrs/scryrs.db exists at the given path"
            ),
            QueryError::EmptyStore => write!(f, "trace_events table is empty"),
            QueryError::UnsupportedStore(msg) => write!(f, "unsupported datastore: {msg}"),
            QueryError::StorageError(e) => write!(f, "storage error: {e}"),
        }
    }
}

impl std::error::Error for QueryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QueryError::StorageError(e) => Some(e),
            _ => None,
        }
    }
}

/// Read-only query handle over a scryrs trace datastore.
///
/// Created via [`TraceQuery::open`] with a repository root path. The handle
/// validates the datastore schema version on open and exposes deterministic,
/// indexed query methods that materialize `Vec<TraceEvent>` from the
/// `trace_events` table.
#[derive(Debug)]
pub struct TraceQuery {
    conn: Connection,
}

impl TraceQuery {
    /// Open a read-only connection to `<repo_root>/.scryrs/scryrs.db`.
    ///
    /// Uses `SQLITE_OPEN_READ_ONLY` (without `SQLITE_OPEN_CREATE`) — the file
    /// is never created, never written to, and WAL/journal PRAGMAs are never
    /// applied.
    /// Schema version is validated via a read-only `SELECT` from `schema_meta`.
    pub fn open(repo_root: impl AsRef<Path>) -> Result<Self, QueryError> {
        let db_path = repo_root.as_ref().join(".scryrs/scryrs.db");

        let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e| match &e {
                rusqlite::Error::SqliteFailure(ffi_err, _)
                    if ffi_err.code == rusqlite::ffi::ErrorCode::CannotOpen =>
                {
                    QueryError::MissingStore
                }
                _ => QueryError::StorageError(e),
            })?;

        // Validate schema version via read-only SELECT.
        match conn.query_row(
            "SELECT CAST(value AS INTEGER) FROM schema_meta \
             WHERE key = 'datastore_schema_version'",
            [],
            |row| row.get::<_, i64>(0),
        ) {
            Ok(v) if v == DATASTORE_SCHEMA_VERSION => {} // Version matches.
            Ok(v) => {
                return Err(QueryError::UnsupportedStore(format!(
                    "datastore schema version mismatch: found {v}, expected {DATASTORE_SCHEMA_VERSION}"
                )));
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(QueryError::UnsupportedStore(
                    "'datastore_schema_version' key not found in schema_meta".into(),
                ));
            }
            Err(e) => {
                // Distinguish "not a database" from missing tables.
                if let rusqlite::Error::SqliteFailure(ref ffi_err, _) = e {
                    if ffi_err.code == rusqlite::ffi::ErrorCode::NotADatabase {
                        return Err(QueryError::StorageError(e));
                    }
                }
                return Err(QueryError::UnsupportedStore(format!(
                    "schema_meta query failed: {e}"
                )));
            }
        }

        Ok(Self { conn })
    }

    /// Return all events in deterministic order: `timestamp ASC, id ASC`.
    ///
    /// Returns `QueryError::EmptyStore` when `trace_events` has zero rows.
    pub fn iter_events_ordered(&self) -> Result<Vec<TraceEvent>, QueryError> {
        let sql = "SELECT event_json FROM trace_events ORDER BY timestamp ASC, id ASC";
        self.query_events(sql, [])
    }

    /// Return events where `subject_kind` matches the given kind.
    ///
    /// Uses the existing `idx_trace_events_subject` index. Returns
    /// `QueryError::EmptyStore` when no matching events exist.
    pub fn query_by_subject_kind(&self, kind: &str) -> Result<Vec<TraceEvent>, QueryError> {
        let sql = "SELECT event_json FROM trace_events \
                   WHERE subject_kind = ?1 \
                   ORDER BY timestamp ASC, id ASC";
        self.query_events(sql, params![kind])
    }

    /// Return events where `event_type` matches the given type string.
    ///
    /// Uses the existing `idx_trace_events_event_type` index. Returns
    /// `QueryError::EmptyStore` when no matching events exist.
    pub fn query_by_event_type(&self, event_type: &str) -> Result<Vec<TraceEvent>, QueryError> {
        let sql = "SELECT event_json FROM trace_events \
                   WHERE event_type = ?1 \
                   ORDER BY timestamp ASC, id ASC";
        self.query_events(sql, params![event_type])
    }

    /// Return events where `outcome = 'Failure'`.
    ///
    /// Uses the existing `idx_trace_events_outcome_reason` index. Returns
    /// `QueryError::EmptyStore` when no failure events exist.
    pub fn query_failures(&self) -> Result<Vec<TraceEvent>, QueryError> {
        let sql = "SELECT event_json FROM trace_events \
                   WHERE outcome = 'Failure' \
                   ORDER BY timestamp ASC, id ASC";
        self.query_events(sql, [])
    }

    // --- private helpers ---

    /// Execute the given SQL with params, deserializing each `event_json` row
    /// into a `TraceEvent`. Returns `QueryError::EmptyStore` when zero rows are
    /// returned and `QueryError::StorageError` on any row deserialization or
    /// SQLite failure.
    fn query_events(
        &self,
        sql: &str,
        params: impl rusqlite::Params,
    ) -> Result<Vec<TraceEvent>, QueryError> {
        let mut stmt = self.conn.prepare(sql).map_err(QueryError::StorageError)?;

        let rows: Vec<TraceEvent> = stmt
            .query_map(params, |row| {
                let json: String = row.get(0)?;
                serde_json::from_str(&json)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })
            .map_err(QueryError::StorageError)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(QueryError::StorageError)?;

        if rows.is_empty() {
            return Err(QueryError::EmptyStore);
        }

        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use scryrs_types::{
        CommandExecutedPayload, DocRetrievedPayload, EditMadePayload, FailedLookupPayload,
        FileOpenedPayload, Outcome, SearchRunPayload, SessionEndPayload, SessionStartPayload,
        SymbolInspectedPayload, TraceEvent, TraceEventPayload, TraceEventType,
    };

    use std::error::Error as StdError;

    use crate::store::EventStore;

    use super::*;

    // ------------------------------------------------------------------
    // Test helpers
    // ------------------------------------------------------------------

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"))
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

    fn make_search_run(session_id: &str, query: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::SearchRun,
            tool_name: Some("grep".into()),
            payload: TraceEventPayload::SearchRun(SearchRunPayload {
                query: query.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_command_executed(session_id: &str, command: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::CommandExecuted,
            tool_name: Some("bash".into()),
            payload: TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: command.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_doc_retrieved(session_id: &str, doc_ref: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::DocRetrieved,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: doc_ref.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_symbol_inspected(session_id: &str, name: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::SymbolInspected,
            tool_name: Some("lsp".into()),
            payload: TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                name: name.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_edit_made(session_id: &str, target: &str, timestamp: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::EditMade,
            tool_name: Some("edit".into()),
            payload: TraceEventPayload::EditMade(EditMadePayload {
                target: target.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_failed_lookup(
        session_id: &str,
        subject: &str,
        reason: Option<&str>,
        timestamp: &str,
    ) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type: TraceEventType::FailedLookup,
            tool_name: Some("lsp".into()),
            payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
                subject: subject.into(),
            }),
            outcome: Outcome::Failure {
                reason: reason.map(Into::into),
            },
        }
    }

    /// Build a store with the given events in `<dir>/.scryrs/scryrs.db`,
    /// commit, and return.
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

    /// Create a `.scryrs/scryrs.db` directory structure under `dir`.
    fn mk_scryrs_db_at(dir: &tempfile::TempDir) -> std::path::PathBuf {
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs dir: {e}"));
        scryrs_dir.join("scryrs.db")
    }

    // ------------------------------------------------------------------
    // 5.1: Open from EventStore — all rows visible
    // ------------------------------------------------------------------

    #[test]
    fn open_store_created_by_eventstore_all_rows_visible() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_search_run("s1", "routing", "2026-06-21T10:00:01Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let rows = query
            .iter_events_ordered()
            .unwrap_or_else(|e| panic!("iter: {e}"));

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].session_id, "s1");
        assert_eq!(rows[1].session_id, "s1");

        // Verify normalized columns survived round-trip through event_json.
        let first: &TraceEvent = &rows[0];
        assert_eq!(first.event_type, TraceEventType::FileOpened);
        match &first.payload {
            TraceEventPayload::FileOpened(p) => assert_eq!(p.path, "src/a.rs"),
            _ => panic!("expected FileOpened payload"),
        }

        let second: &TraceEvent = &rows[1];
        assert_eq!(second.event_type, TraceEventType::SearchRun);
        match &second.payload {
            TraceEventPayload::SearchRun(p) => assert_eq!(p.query, "routing"),
            _ => panic!("expected SearchRun payload"),
        }
    }

    // ------------------------------------------------------------------
    // 5.2: Deterministic ordering — same timestamp, different id
    // ------------------------------------------------------------------

    #[test]
    fn deterministic_ordering_same_timestamp_different_id() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/c.rs", "2026-06-21T12:00:00Z"),
            make_file_opened("s1", "src/a.rs", "2026-06-21T12:00:00Z"),
            make_file_opened("s1", "src/b.rs", "2026-06-21T12:00:00Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let rows = query
            .iter_events_ordered()
            .unwrap_or_else(|e| panic!("iter: {e}"));

        assert_eq!(rows.len(), 3);
        // id tie-breaker: lower id first
        let subjects: Vec<&str> = rows.iter().filter_map(|e| e.subject()).collect();
        // Insertion order was c, a, b so ids are 1, 2, 3.
        // Timestamp ascending all equal → tie-break by id ASC → c, a, b
        assert_eq!(subjects, vec!["src/c.rs", "src/a.rs", "src/b.rs"]);
    }

    #[test]
    fn deterministic_ordering_mixed_timestamps() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "third", "2026-06-21T12:00:03Z"),
            make_file_opened("s1", "first", "2026-06-21T12:00:01Z"),
            make_file_opened("s1", "second", "2026-06-21T12:00:02Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let rows = query
            .iter_events_ordered()
            .unwrap_or_else(|e| panic!("iter: {e}"));

        let subjects: Vec<&str> = rows.iter().filter_map(|e| e.subject()).collect();
        assert_eq!(subjects, vec!["first", "second", "third"]);
    }

    // ------------------------------------------------------------------
    // 5.3: query_by_subject_kind across all seven families
    // ------------------------------------------------------------------

    #[test]
    fn query_by_subject_kind_filters_correctly() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_symbol_inspected("s1", "MyStruct", "2026-06-21T10:00:01Z"),
            make_search_run("s1", "routing", "2026-06-21T10:00:02Z"),
            make_command_executed("s1", "cargo build", "2026-06-21T10:00:03Z"),
            make_doc_retrieved("s1", "api.md", "2026-06-21T10:00:04Z"),
            make_edit_made("s1", "src/b.rs", "2026-06-21T10:00:05Z"),
            make_failed_lookup(
                "s1",
                "missing_fn",
                Some("not found"),
                "2026-06-21T10:00:06Z",
            ),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        // File kind: FileOpened + EditMade
        let files = query
            .query_by_subject_kind("file")
            .unwrap_or_else(|e| panic!("file: {e}"));
        let file_subjects: Vec<&str> = files.iter().filter_map(|e| e.subject()).collect();
        assert_eq!(file_subjects, vec!["src/a.rs", "src/b.rs"]);

        // Symbol kind: SymbolInspected + FailedLookup
        let symbols = query
            .query_by_subject_kind("symbol")
            .unwrap_or_else(|e| panic!("symbol: {e}"));
        let sym_subjects: Vec<&str> = symbols.iter().filter_map(|e| e.subject()).collect();
        assert_eq!(sym_subjects, vec!["MyStruct", "missing_fn"]);

        // Search kind
        let searches = query
            .query_by_subject_kind("search")
            .unwrap_or_else(|e| panic!("search: {e}"));
        assert_eq!(searches.len(), 1);
        assert_eq!(searches[0].subject(), Some("routing"));

        // Command kind
        let commands = query
            .query_by_subject_kind("command")
            .unwrap_or_else(|e| panic!("command: {e}"));
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].subject(), Some("cargo build"));

        // Document kind
        let docs = query
            .query_by_subject_kind("document")
            .unwrap_or_else(|e| panic!("document: {e}"));
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].subject(), Some("api.md"));

        // Kind with no matching events returns EmptyStore.
        let result = query.query_by_subject_kind("nonexistent");
        assert!(matches!(result, Err(QueryError::EmptyStore)));
    }

    // ------------------------------------------------------------------
    // 5.4: query_by_event_type filtering
    // ------------------------------------------------------------------

    #[test]
    fn query_by_event_type_filters_correctly() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_search_run("s1", "foo", "2026-06-21T10:00:01Z"),
            make_file_opened("s1", "src/b.rs", "2026-06-21T10:00:02Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let file_events = query
            .query_by_event_type("FileOpened")
            .unwrap_or_else(|e| panic!("FileOpened: {e}"));
        assert_eq!(file_events.len(), 2);
        for ev in &file_events {
            assert_eq!(ev.event_type, TraceEventType::FileOpened);
        }

        let search_events = query
            .query_by_event_type("SearchRun")
            .unwrap_or_else(|e| panic!("SearchRun: {e}"));
        assert_eq!(search_events.len(), 1);
        assert_eq!(search_events[0].event_type, TraceEventType::SearchRun);

        // Type with no matches returns EmptyStore.
        let result = query.query_by_event_type("SessionStart");
        assert!(matches!(result, Err(QueryError::EmptyStore)));
    }

    // ------------------------------------------------------------------
    // 5.5: query_failures filtering
    // ------------------------------------------------------------------

    #[test]
    fn query_failures_returns_only_failure_events() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_failed_lookup("s1", "bad_fn", Some("not found"), "2026-06-21T10:00:01Z"),
            make_search_run("s1", "q", "2026-06-21T10:00:02Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let failures = query
            .query_failures()
            .unwrap_or_else(|e| panic!("failures: {e}"));
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].event_type, TraceEventType::FailedLookup);
        assert!(matches!(failures[0].outcome, Outcome::Failure { .. }));
    }

    #[test]
    fn query_failures_with_no_failures_returns_empty_store() {
        let dir = temp_dir();
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_search_run("s1", "q", "2026-06-21T10:00:01Z"),
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let result = query.query_failures();
        assert!(matches!(result, Err(QueryError::EmptyStore)));
    }

    // ------------------------------------------------------------------
    // 5.6: MissingStore
    // ------------------------------------------------------------------

    #[test]
    fn missing_store_returned_when_scryrs_db_does_not_exist() {
        let dir = temp_dir();
        // No file at all — not even .scryrs dir.
        let result = TraceQuery::open(dir.path());
        assert!(matches!(result, Err(QueryError::MissingStore)));
    }

    #[test]
    fn missing_store_when_scryrs_dir_exists_but_no_db() {
        let dir = temp_dir();
        // Create .scryrs directory but no scryrs.db inside.
        let scryrs_dir = dir.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));

        let result = TraceQuery::open(dir.path());
        assert!(
            matches!(result, Err(QueryError::MissingStore)),
            "expected MissingStore when .scryrs exists but scryrs.db is absent"
        );
    }

    // ------------------------------------------------------------------
    // 5.7: EmptyStore from query methods
    // ------------------------------------------------------------------

    #[test]
    fn empty_store_returned_when_trace_events_has_zero_rows() {
        let dir = temp_dir();
        let _db_path = mk_scryrs_db_at(&dir);

        // Create a valid database with schema but zero events.
        {
            EventStore::open(&_db_path).unwrap_or_else(|e| panic!("create store: {e}"));
            // Don't append any events — close immediately via drop.
        }

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let result = query.iter_events_ordered();
        assert!(
            matches!(result, Err(QueryError::EmptyStore)),
            "iter_events_ordered on empty store must return EmptyStore"
        );

        let result = query.query_by_subject_kind("file");
        assert!(matches!(result, Err(QueryError::EmptyStore)));

        let result = query.query_by_event_type("FileOpened");
        assert!(matches!(result, Err(QueryError::EmptyStore)));

        let result = query.query_failures();
        assert!(matches!(result, Err(QueryError::EmptyStore)));
    }

    // ------------------------------------------------------------------
    // 5.8: UnsupportedStore — schema version mismatch
    // ------------------------------------------------------------------

    #[test]
    fn unsupported_store_when_schema_version_does_not_match() {
        let dir = temp_dir();
        let db_path = mk_scryrs_db_at(&dir);

        // Create a valid schema first.
        {
            let _store = EventStore::open(&db_path).unwrap_or_else(|e| panic!("create store: {e}"));
        }

        // Tamper with the schema version.
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap_or_else(|e| panic!("open: {e}"));
            conn.execute(
                "UPDATE schema_meta SET value = '99' WHERE key = 'datastore_schema_version'",
                [],
            )
            .unwrap_or_else(|e| panic!("update version: {e}"));
        }

        let result = TraceQuery::open(dir.path());
        match result {
            Err(QueryError::UnsupportedStore(msg)) => {
                assert!(
                    msg.contains("version mismatch"),
                    "message must mention version mismatch, got: {msg}"
                );
                assert!(msg.contains("99"), "message must include found version 99");
                assert!(
                    msg.contains(&DATASTORE_SCHEMA_VERSION.to_string()),
                    "message must include expected version"
                );
            }
            other => panic!("expected UnsupportedStore, got: {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 5.9: StorageError — non-SQLite file
    // ------------------------------------------------------------------

    #[test]
    fn storage_error_for_non_sqlite_file() {
        let dir = temp_dir();
        let db_path = mk_scryrs_db_at(&dir);

        // Write a plain-text file at the scryrs.db path.
        std::fs::write(&db_path, "this is not a sqlite database\n")
            .unwrap_or_else(|e| panic!("write text file: {e}"));

        let result = TraceQuery::open(dir.path());
        assert!(
            matches!(result, Err(QueryError::StorageError(_))),
            "expected StorageError for non-SQLite file, got: {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // 5.10: WAL-mode database readable
    // ------------------------------------------------------------------

    #[test]
    fn read_only_connection_reads_wal_mode_database() {
        let dir = temp_dir();
        let _db_path = mk_scryrs_db_at(&dir);

        // Write path: EventStore enables WAL mode.
        let events = vec![
            make_file_opened("s1", "src/a.rs", "2026-06-21T10:00:00Z"),
            make_search_run("s1", "wal-test", "2026-06-21T10:00:01Z"),
        ];
        populate_store(&dir, &events);

        // Read path: opens read-only, all committed rows visible.
        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let rows = query
            .iter_events_ordered()
            .unwrap_or_else(|e| panic!("iter: {e}"));

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].subject(), Some("src/a.rs"));
        assert_eq!(rows[1].subject(), Some("wal-test"));
    }

    // ------------------------------------------------------------------
    // 5.x: Lifecycle events have NULL subject_kind and are queryable
    // ------------------------------------------------------------------

    #[test]
    fn lifecycle_events_have_no_subject_and_are_returned_by_iter() {
        let dir = temp_dir();
        let events: Vec<TraceEvent> = vec![
            TraceEvent {
                schema_version: scryrs_types::SCHEMA_VERSION.into(),
                timestamp: "2026-06-21T09:00:00Z".into(),
                session_id: "s-lifecycle".into(),
                event_type: TraceEventType::SessionStart,
                tool_name: None,
                payload: TraceEventPayload::SessionStart(SessionStartPayload),
                outcome: Outcome::Success,
            },
            make_file_opened("s-lifecycle", "src/x.rs", "2026-06-21T09:00:01Z"),
            TraceEvent {
                schema_version: scryrs_types::SCHEMA_VERSION.into(),
                timestamp: "2026-06-21T09:00:02Z".into(),
                session_id: "s-lifecycle".into(),
                event_type: TraceEventType::SessionEnd,
                tool_name: None,
                payload: TraceEventPayload::SessionEnd(SessionEndPayload),
                outcome: Outcome::Success,
            },
        ];
        populate_store(&dir, &events);

        let query = TraceQuery::open(dir.path()).unwrap_or_else(|e| panic!("open: {e}"));

        let rows = query
            .iter_events_ordered()
            .unwrap_or_else(|e| panic!("iter: {e}"));

        assert_eq!(rows.len(), 3);
        // Lifecycle events are included in iter, but have no subject_kind.
        assert_eq!(rows[0].event_type, TraceEventType::SessionStart);
        assert!(rows[0].subject().is_none());
        assert!(rows[0].subject_kind().is_none());
        assert_eq!(rows[1].event_type, TraceEventType::FileOpened);
        assert_eq!(rows[2].event_type, TraceEventType::SessionEnd);
        assert!(rows[2].subject().is_none());
    }

    // ------------------------------------------------------------------
    // 5.x: Error Display and Error trait impls
    // ------------------------------------------------------------------

    #[test]
    fn query_error_display_and_error_trait() {
        // MissingStore
        let e = QueryError::MissingStore;
        let msg = e.to_string();
        assert!(msg.contains("not found"), "MissingStore display: {msg}");

        // EmptyStore
        let e = QueryError::EmptyStore;
        let msg = e.to_string();
        assert!(msg.contains("empty"), "EmptyStore display: {msg}");

        // UnsupportedStore
        let e = QueryError::UnsupportedStore("version 99".into());
        let msg = e.to_string();
        assert!(msg.contains("version 99"));

        // StorageError has a source
        let fake_err = match rusqlite::Connection::open_with_flags(
            "/nonexistent/path/to/file.db",
            OpenFlags::default(),
        ) {
            Err(e) => e,
            Ok(_) => panic!("expected error from nonexistent path"),
        };
        let e = QueryError::StorageError(fake_err);
        assert!(
            StdError::source(&e).is_some(),
            "StorageError must have source"
        );
        let msg = e.to_string();
        assert!(msg.contains("storage error"), "StorageError display: {msg}");
    }
}
