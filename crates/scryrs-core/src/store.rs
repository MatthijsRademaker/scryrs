//! Canonical SQLite trace datastore owned by scryrs-core.
//!
//! The canonical accepted-event store is `.scryrs/scryrs.db` relative to the
//! current working directory. This module owns schema creation, version
//! validation, and event insertion. CLI and other consumers compose this API.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, params};
use scryrs_types::TraceEvent;

/// Current datastore schema version (independent of TraceEvent wire schema).
pub(crate) const DATASTORE_SCHEMA_VERSION: i64 = 1;

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

/// `schema_meta` key recording that historical subjects were normalized.
const SUBJECT_NORMALIZATION_KEY: &str = "subject_normalization_migrated";

/// Payload field holding the path subject, per event family.
fn path_field_for(event_type: &str) -> Option<&'static str> {
    match event_type {
        "FileOpened" => Some("path"),
        "EditMade" => Some("target"),
        "FailedLookup" => Some("subject"),
        _ => None,
    }
}

/// Derive the repository root from a canonical `<root>/.scryrs/scryrs.db` path.
///
/// The store is routinely opened through the *relative* [`CANONICAL_STORE_PATH`],
/// so the path is resolved to an absolute one first. Without that, the derived
/// root is the empty path, every absolute subject fails the prefix test, and the
/// migration would mark all of them `external_file` — the exact corruption the
/// caller's plausibility guard exists to prevent.
///
/// Returns `None` for any non-canonical layout (test stores, `SCRYRS_STORE`
/// overrides) or when the root cannot be resolved to a non-empty absolute path.
/// In those cases the root genuinely cannot be known and guessing would corrupt
/// subjects.
fn repo_root_of_store(store_path: &Path) -> Option<PathBuf> {
    let absolute = store_path
        .canonicalize()
        .unwrap_or_else(|_| store_path.to_path_buf());

    let scryrs_dir = absolute.parent()?;
    if scryrs_dir.file_name()? != ".scryrs" {
        return None;
    }

    let root = scryrs_dir.parent()?;
    if !root.is_absolute() || root.as_os_str().is_empty() {
        return None;
    }
    Some(root.to_path_buf())
}

/// Whether the derived repository root is a prefix of any recorded absolute
/// subject — the evidence that it is the root the events were recorded under.
fn root_matches_recorded_subjects(conn: &Connection, repo_root: &Path) -> rusqlite::Result<bool> {
    let Some(root) = repo_root.to_str() else {
        return Ok(false);
    };
    let prefix = format!("{}/", root.trim_end_matches('/'));
    let matching: i64 = conn.query_row(
        "SELECT COUNT(*) FROM trace_events
         WHERE subject LIKE ?1 || '%'
           AND event_type IN ('FileOpened', 'EditMade', 'FailedLookup')",
        params![prefix],
        |row| row.get(0),
    )?;
    Ok(matching > 0)
}

/// One-time normalization of historical path subjects recorded before the
/// adapter layer started normalizing them.
///
/// Rows written by older builds carry whatever path string the agent typed, so
/// the same file appears under both an absolute and a repository-relative
/// subject and its hotspot score is split across the two. This rewrites the
/// `subject` column, the derived `subject_kind`, and the matching path inside
/// `event_json` so historical evidence groups on the same key as new events.
///
/// Runs at most once per store, guarded by a `schema_meta` marker. It is a data
/// migration only — the table shape is unchanged, so `datastore_schema_version`
/// is untouched.
fn migrate_subject_normalization(conn: &Connection, store_path: &Path) -> rusqlite::Result<()> {
    let already: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = ?1",
            params![SUBJECT_NORMALIZATION_KEY],
            |row| row.get(0),
        )
        .optional()?;
    if already.is_some() {
        return Ok(());
    }

    // Without a canonical store path there is no knowable repository root. Leave
    // the store untouched and unmarked, so a later canonical open still migrates.
    let Some(repo_root) = repo_root_of_store(store_path) else {
        return Ok(());
    };

    // Guard against a root that does not match the recorded data. The store can
    // legitimately be opened from a different mount path than the one the events
    // were recorded under — a container bind-mounting the repository at
    // `/workspace` is the common case. Migrating with such a root would classify
    // every genuinely-internal absolute subject as `external_file` and then mark
    // the store done, making the damage permanent.
    //
    // A correct root is a prefix of at least one recorded absolute subject.
    // If none match, the root is wrong (or there is nothing to migrate): skip
    // without marking, so a later open under the right root still migrates.
    if !root_matches_recorded_subjects(conn, &repo_root)? {
        return Ok(());
    }

    let rows: Vec<(i64, String, String, Option<String>)> = {
        let mut stmt = conn.prepare(
            "SELECT id, event_type, event_json, subject_kind FROM trace_events
             WHERE subject IS NOT NULL AND event_type IN ('FileOpened', 'EditMade', 'FailedLookup')",
        )?;
        let mapped = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?;
        mapped.collect::<rusqlite::Result<Vec<_>>>()?
    };

    for (id, event_type, event_json, stored_kind) in rows {
        let Some(field) = path_field_for(&event_type) else {
            continue;
        };
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&event_json) else {
            continue;
        };
        let Some(raw) = value
            .get("payload")
            .and_then(|p| p.get(field))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };

        let normalized = scryrs_types::normalize_path_subject(raw, &repo_root).into_subject();
        let kind = scryrs_types::path_subject_kind(&normalized);

        // The kind must be re-derived even when the subject is unchanged: an
        // external path keeps its spelling but was stored as "file" by older
        // builds, and leaving that would group it with repository files —
        // exactly the split this migration exists to remove.
        if normalized == raw && stored_kind.as_deref() == Some(kind) {
            continue;
        }

        value["payload"][field] = serde_json::Value::String(normalized.clone());
        let Ok(rewritten) = serde_json::to_string(&value) else {
            continue;
        };

        conn.execute(
            "UPDATE trace_events SET subject = ?1, subject_kind = ?2, event_json = ?3 WHERE id = ?4",
            params![normalized, kind, rewritten, id],
        )?;
    }

    conn.execute(
        "INSERT OR REPLACE INTO schema_meta (key, value) VALUES (?1, '1')",
        params![SUBJECT_NORMALIZATION_KEY],
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
    in_transaction: bool,
}

impl std::fmt::Debug for EventStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventStore")
            .field("stored_count", &self.stored_count)
            .finish_non_exhaustive()
    }
}

impl Drop for EventStore {
    fn drop(&mut self) {
        if self.in_transaction {
            let _ = self.conn.execute_batch("ROLLBACK;");
        }
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
        migrate_subject_normalization(&conn, path_ref)?;
        Ok(Self {
            conn,
            stored_count: 0,
            in_transaction: false,
        })
    }

    /// Open the default local datastore at [CANONICAL_STORE_PATH] relative to
    /// the current working directory.
    pub fn default_local() -> Result<Self, rusqlite::Error> {
        Self::open(CANONICAL_STORE_PATH)
    }

    /// Begin an explicit SQLite transaction.
    ///
    /// After this call, subsequent `append` calls are batched within the
    /// transaction. The caller must call `commit_transaction` (or drop the
    /// store, which will roll back) to finalize.
    pub fn begin_transaction(&mut self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch("BEGIN TRANSACTION;")?;
        self.in_transaction = true;
        Ok(())
    }

    /// Commit the active transaction.
    ///
    /// If no transaction is active, this is a no-op. After commit, the
    /// batch of accepted events is durable. The caller should only report
    /// success after this succeeds.
    pub fn commit_transaction(&mut self) -> Result<(), rusqlite::Error> {
        if self.in_transaction {
            self.conn.execute_batch("COMMIT;")?;
            self.in_transaction = false;
        }
        Ok(())
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

    // --- one-time historical subject normalization ---

    /// Build a store at the canonical `<root>/.scryrs/scryrs.db` layout and
    /// insert a raw row with an un-normalized subject, as an older build would.
    fn seed_legacy_row(repo_root: &std::path::Path, absolute_path: &str) -> std::path::PathBuf {
        let store_path = repo_root.join(CANONICAL_STORE_PATH);
        {
            let _store = open_ok(&store_path);
        }
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let event_json = format!(
            r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"s1","event_type":"FileOpened","tool_name":"read","payload":{{"type":"FileOpened","path":"{}"}},"outcome":{{"result":"Success"}}}}"#,
            SCHEMA_VERSION, absolute_path
        );
        conn.execute(
            "INSERT INTO trace_events
             (event_json, schema_version, timestamp, session_id, event_type, tool_name,
              subject_kind, subject, outcome, failure_reason)
             VALUES (?1, ?2, '2026-06-20T00:00:00Z', 's1', 'FileOpened', 'read',
                     'file', ?3, 'Success', NULL)",
            params![event_json, SCHEMA_VERSION, absolute_path],
        )
        .unwrap_or_else(|e| panic!("seed: {e}"));
        // Clear the marker the first open wrote, so the next open migrates.
        conn.execute(
            "DELETE FROM schema_meta WHERE key = ?1",
            params![SUBJECT_NORMALIZATION_KEY],
        )
        .unwrap_or_else(|e| panic!("clear marker: {e}"));
        store_path
    }

    /// Append one more un-normalized row to a store seeded by `seed_legacy_row`.
    fn append_legacy_row(store_path: &std::path::Path, absolute_path: &str) {
        let conn = Connection::open(store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let event_json = format!(
            r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:01Z","session_id":"s1","event_type":"FileOpened","tool_name":"read","payload":{{"type":"FileOpened","path":"{}"}},"outcome":{{"result":"Success"}}}}"#,
            SCHEMA_VERSION, absolute_path
        );
        conn.execute(
            "INSERT INTO trace_events
             (event_json, schema_version, timestamp, session_id, event_type, tool_name,
              subject_kind, subject, outcome, failure_reason)
             VALUES (?1, ?2, '2026-06-20T00:00:01Z', 's1', 'FileOpened', 'read',
                     'file', ?3, 'Success', NULL)",
            params![event_json, SCHEMA_VERSION, absolute_path],
        )
        .unwrap_or_else(|e| panic!("append: {e}"));
    }

    #[test]
    fn historical_absolute_subjects_are_normalized_on_open() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path();
        let absolute = format!("{}/crates/a.rs", root.display());
        let store_path = seed_legacy_row(root, &absolute);

        // Opening runs the migration.
        let _store = open_ok(&store_path);

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let (subject, kind, event_json): (String, String, String) = conn
            .query_row(
                "SELECT subject, subject_kind, event_json FROM trace_events LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap_or_else(|e| panic!("row: {e}"));

        assert_eq!(subject, "crates/a.rs", "subject column must be normalized");
        assert_eq!(kind, "file");
        assert!(
            event_json.contains(r#""path":"crates/a.rs""#),
            "event_json must be rewritten in step with the column, got: {event_json}"
        );
    }

    #[test]
    fn migration_runs_once_and_records_a_marker() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path();
        let absolute = format!("{}/crates/a.rs", root.display());
        let store_path = seed_legacy_row(root, &absolute);

        let _first = open_ok(&store_path);
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let marker: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = ?1",
                params![SUBJECT_NORMALIZATION_KEY],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("marker: {e}"));
        assert_eq!(marker, "1");

        // A second open is a no-op: the subject is already normalized and stays.
        drop(conn);
        let _second = open_ok(&store_path);
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let subject: String = conn
            .query_row("SELECT subject FROM trace_events LIMIT 1", [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("row: {e}"));
        assert_eq!(subject, "crates/a.rs");
    }

    #[test]
    fn migration_marks_repository_external_paths_without_rewriting_them() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path();
        // An internal absolute subject confirms the root; the external one is
        // what the assertion is about. A real store has both.
        let internal = format!("{}/crates/a.rs", root.display());
        let store_path = seed_legacy_row(root, &internal);
        append_legacy_row(&store_path, "/etc/hosts");

        let _store = open_ok(&store_path);

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let (subject, kind): (String, String) = conn
            .query_row(
                "SELECT subject, subject_kind FROM trace_events WHERE subject = '/etc/hosts'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|e| panic!("row: {e}"));

        assert_eq!(subject, "/etc/hosts", "external paths stay verbatim");
        assert_eq!(kind, "external_file");

        // And the internal one collapsed in the same pass.
        let internal_subject: String = conn
            .query_row(
                "SELECT subject FROM trace_events WHERE subject_kind = 'file'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("internal row: {e}"));
        assert_eq!(internal_subject, "crates/a.rs");
    }

    /// Regression: the store is routinely opened through the *relative*
    /// `CANONICAL_STORE_PATH`. If the root is derived without resolving that to
    /// an absolute path it comes out empty, every absolute subject looks
    /// external, and the migration corrupts the whole store and marks it done.
    #[test]
    fn migration_resolves_a_relative_canonical_store_path() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir
            .path()
            .canonicalize()
            .unwrap_or_else(|e| panic!("canonicalize: {e}"));
        let absolute = format!("{}/crates/a.rs", root.display());
        seed_legacy_row(&root, &absolute);

        // Open through the relative canonical path, as `record --mode local` does.
        let previous = std::env::current_dir().unwrap_or_else(|e| panic!("cwd: {e}"));
        std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("chdir: {e}"));
        let opened = EventStore::open(CANONICAL_STORE_PATH);
        std::env::set_current_dir(&previous).unwrap_or_else(|e| panic!("restore cwd: {e}"));
        opened.unwrap_or_else(|e| panic!("open relative: {e}"));

        let conn = Connection::open(root.join(CANONICAL_STORE_PATH))
            .unwrap_or_else(|e| panic!("open: {e}"));
        let (subject, kind): (String, String) = conn
            .query_row(
                "SELECT subject, subject_kind FROM trace_events LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|e| panic!("row: {e}"));
        assert_eq!(
            subject, "crates/a.rs",
            "relative store path must still resolve the root"
        );
        assert_eq!(kind, "file", "an internal path must not be marked external");
    }

    /// A store opened under a different mount path than the events were
    /// recorded under (a container bind-mount is the common case) must not
    /// migrate: it would mark every internal subject `external_file` and then
    /// record the marker, making the damage permanent.
    #[test]
    fn migration_skips_a_root_that_does_not_match_recorded_subjects() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path();
        // Recorded under a completely different root, as a bind-mounted or
        // relocated repository would be.
        let foreign = "/elsewhere/checkout/crates/a.rs";
        let store_path = seed_legacy_row(root, foreign);

        let _store = open_ok(&store_path);

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let (subject, kind): (String, String) = conn
            .query_row(
                "SELECT subject, subject_kind FROM trace_events LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap_or_else(|e| panic!("row: {e}"));
        assert_eq!(subject, foreign, "subject must be left untouched");
        assert_eq!(kind, "file", "kind must not be rewritten on a wrong root");

        let marker: Option<String> = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = ?1",
                params![SUBJECT_NORMALIZATION_KEY],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or_else(|e| panic!("marker query: {e}"));
        assert!(
            marker.is_none(),
            "a skipped migration must stay unmarked so a correct root can still migrate"
        );
    }

    /// A non-canonical store path has no knowable repository root, so the
    /// migration must leave it alone rather than guess and corrupt subjects.
    #[test]
    fn migration_skips_stores_outside_the_canonical_layout() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let store_path = dir.path().join("somewhere.db");
        {
            let _store = open_ok(&store_path);
        }
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
        let marker: Option<String> = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = ?1",
                params![SUBJECT_NORMALIZATION_KEY],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or_else(|e| panic!("marker query: {e}"));
        assert!(
            marker.is_none(),
            "a non-canonical store must not be marked migrated"
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
            // FailedLookup carries a path (an agent addressed a file that is
            // not there), so it groups as "file" alongside FileOpened/EditMade.
            (
                "file",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::FailedLookup,
                    tool_name: Some("read".into()),
                    payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
                        subject: "src/missing.rs".into(),
                    }),
                    outcome: Outcome::Failure { reason: None },
                },
            ),
            // An absolute subject is by construction outside the repository,
            // because adapters normalize internal paths to relative form.
            (
                "external_file",
                TraceEvent {
                    schema_version: SCHEMA_VERSION.into(),
                    timestamp: "t".into(),
                    session_id: "s".into(),
                    event_type: TraceEventType::FileOpened,
                    tool_name: Some("read".into()),
                    payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                        path: "/etc/hosts".into(),
                    }),
                    outcome: Outcome::Success,
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

    // --- Batch transaction ---

    #[test]
    fn batch_transaction_commits_all_or_nothing() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        {
            let mut store = open_ok(&store_path);
            store
                .begin_transaction()
                .unwrap_or_else(|e| panic!("begin: {e}"));
            store
                .append(&make_event("s1", "doc/a.md"))
                .unwrap_or_else(|e| panic!("append 1: {e}"));
            store
                .append(&make_event("s2", "doc/b.md"))
                .unwrap_or_else(|e| panic!("append 2: {e}"));
            store
                .commit_transaction()
                .unwrap_or_else(|e| panic!("commit: {e}"));
            assert_eq!(store.stored_count(), 2);
        }

        // Both rows must be visible after commit.
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(count, 2, "both rows must be persisted after commit");
    }

    #[test]
    fn uncommitted_transaction_rolls_back_on_drop() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        {
            let mut store = open_ok(&store_path);
            store
                .begin_transaction()
                .unwrap_or_else(|e| panic!("begin: {e}"));
            store
                .append(&make_event("s1", "doc/a.md"))
                .unwrap_or_else(|e| panic!("append: {e}"));
            assert_eq!(store.stored_count(), 1);
            // Drop without commit — transaction must roll back.
        }

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(
            count, 0,
            "uncommitted rows must not be persisted after drop"
        );
    }

    #[test]
    fn commit_transaction_without_begin_is_noop() {
        let dir = temp_dir();
        let store_path = dir.path().join("scryrs.db");

        let mut store = open_ok(&store_path);
        // commit without begin must not panic
        store
            .commit_transaction()
            .unwrap_or_else(|e| panic!("commit noop: {e}"));

        store
            .append(&make_event("s1", "doc/a.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));
        // autocommit still works after no-op commit
        drop(store);

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(count, 1, "auto-committed row must be persisted");
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
