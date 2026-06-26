//! Dedicated server-owned SQLite store for central trace ingest.
//!
//! This store is intentionally separate from the local `trace_events` schema
//! in `scryrs-core`. It mirrors the normalized columns needed for future
//! scoring and adds identity/idempotency columns.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};
use scryrs_core::scoring::per_event_contribution;
use scryrs_types::{EnvelopeEvent, EventAck, EventAckStatus, ServerIngestEnvelope};

use crate::time::chrono_now;

/// Current server store schema version (independent of local datastore version).
const SERVER_STORE_SCHEMA_VERSION: i64 = 2;

/// Open a connection at `path`, creating parent directories.
fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            rusqlite::Error::InvalidParameterName(format!(
                "failed to create store parent directory '{}': {e}",
                parent.display()
            ))
        })?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

/// Ensure the server store schema exists, migrating from earlier versions.
fn ensure_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS server_schema_meta (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT NOT NULL
        );",
    )?;

    let existing_version: Option<i64> = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM server_schema_meta WHERE key = 'server_store_schema_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    match existing_version {
        Some(v) if v == SERVER_STORE_SCHEMA_VERSION => return Ok(()),
        Some(v) if v < SERVER_STORE_SCHEMA_VERSION => {
            // Migrate forward from v to current.
            migrate_from(conn, v)?;
        }
        Some(v) => {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "server store schema version mismatch: found {v}, expected {SERVER_STORE_SCHEMA_VERSION}"
            )));
        }
        None => {
            // Fresh database: create baseline v2 schema directly.
            conn.execute(
                "INSERT OR REPLACE INTO server_schema_meta (key, value) VALUES ('server_store_schema_version', ?1)",
                params![SERVER_STORE_SCHEMA_VERSION.to_string()],
            )?;

            create_v1_tables(conn)?;
            create_v2_tables(conn)?;
        }
    }

    Ok(())
}

/// Additive migration from an earlier schema version to current.
fn migrate_from(conn: &Connection, from_version: i64) -> rusqlite::Result<()> {
    if from_version < 2 {
        create_v2_tables(conn)?;
    }
    // Update version stamp after successful migration.
    conn.execute(
        "INSERT OR REPLACE INTO server_schema_meta (key, value) VALUES ('server_store_schema_version', ?1)",
        params![SERVER_STORE_SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

/// Baseline v1 tables.
fn create_v1_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS server_trace_events (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            repository_id     TEXT NOT NULL,
            workspace_id      TEXT NOT NULL,
            agent_id          TEXT NOT NULL,
            producer_event_id TEXT NOT NULL,
            client_timestamp  TEXT NOT NULL,
            received_at       TEXT NOT NULL,
            event_json        TEXT NOT NULL,
            schema_version    TEXT NOT NULL,
            timestamp         TEXT NOT NULL,
            session_id        TEXT NOT NULL,
            event_type        TEXT NOT NULL,
            tool_name         TEXT,
            subject_kind      TEXT,
            subject           TEXT,
            outcome           TEXT NOT NULL,
            failure_reason    TEXT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_server_events_dedup
            ON server_trace_events(repository_id, workspace_id, agent_id, producer_event_id);",
    )
}

/// V2 tables: cumulative hotspot accumulators and append-only signal history.
fn create_v2_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS hotspot_accumulators (
            repository_id     TEXT NOT NULL,
            window            TEXT NOT NULL,
            subject_kind      TEXT NOT NULL,
            subject           TEXT NOT NULL,
            score             INTEGER NOT NULL DEFAULT 0,
            event_type_counts TEXT NOT NULL DEFAULT '{}',
            outcome_counts    TEXT NOT NULL DEFAULT '{}',
            session_ids       TEXT NOT NULL DEFAULT '[]',
            first_seen        TEXT NOT NULL DEFAULT '',
            last_seen         TEXT NOT NULL DEFAULT '',
            evidence_row_ids  TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (repository_id, window, subject_kind, subject)
        );

        CREATE TABLE IF NOT EXISTS hotspot_signals (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            repository_id     TEXT NOT NULL,
            subject_kind      TEXT NOT NULL,
            subject           TEXT NOT NULL,
            score             INTEGER NOT NULL,
            delta             INTEGER NOT NULL,
            window            TEXT NOT NULL,
            threshold         INTEGER NOT NULL,
            evidence_row_ids  TEXT NOT NULL,
            created_at        TEXT NOT NULL
        );",
    )
}

/// Server-owned SQLite store for central trace ingest with idempotent inserts.
pub struct ServerStore {
    conn: Connection,
    signal_threshold: u32,
}

impl ServerStore {
    /// Open (or create) the server datastore at `path`.
    pub fn open(path: impl AsRef<Path>, signal_threshold: u32) -> Result<Self, rusqlite::Error> {
        let path_ref = path.as_ref();
        let conn = open_connection(path_ref)?;
        ensure_schema(&conn)?;
        Ok(Self {
            conn,
            signal_threshold,
        })
    }

    /// Process a batch ingest envelope, returning per-item acknowledgments.
    ///
    /// Each accepted event insert, accumulator update, and optional signal
    /// insert are committed in a single SQLite transaction so the stored event
    /// row and its live hotspot state cannot diverge.
    pub fn ingest_batch(&self, envelope: &ServerIngestEnvelope) -> Result<Vec<EventAck>, rusqlite::Error> {
        let received_at = chrono_now();
        let mut acks: Vec<EventAck> = Vec::with_capacity(envelope.events.len());

        // Wrap the entire batch in one explicit transaction so event inserts
        // and accumulator mutations commit together (or roll back together).
        self.conn.execute_batch("BEGIN TRANSACTION;")?;

        for (index, item) in envelope.events.iter().enumerate() {
            let (ack, row_id) = self.process_item(envelope, item, index, &received_at);
            if let Some(row_id) = row_id {
                self.apply_live_accumulator(envelope, index, row_id)?;
            }
            acks.push(ack);
        }

        self.conn.execute_batch("COMMIT;")?;
        Ok(acks)
    }

    fn process_item(
        &self,
        envelope: &ServerIngestEnvelope,
        item: &EnvelopeEvent,
        index: usize,
        received_at: &str,
    ) -> (EventAck, Option<i64>) {
        // Validate client_timestamp as RFC 3339.
        if !is_valid_rfc3339(&item.client_timestamp) {
            return (
                EventAck {
                    index,
                    producer_event_id: Some(item.producer_event_id.clone()),
                    status: EventAckStatus::Rejected,
                    server_event_id: None,
                    error_reason: Some(format!(
                        "invalid client_timestamp: '{}' is not valid RFC 3339",
                        item.client_timestamp
                    )),
                    received_at: received_at.to_string(),
                },
                None,
            );
        }

        // Validate inner TraceEvent.
        if let Err(reason) = item.event.validate() {
            return (
                EventAck {
                    index,
                    producer_event_id: Some(item.producer_event_id.clone()),
                    status: EventAckStatus::Rejected,
                    server_event_id: None,
                    error_reason: Some(format!("TraceEvent validation failed: {reason}")),
                    received_at: received_at.to_string(),
                },
                None,
            );
        }

        // Attempt insert with idempotency.
        match self.insert_event(envelope, item, received_at) {
            InsertResult::Accepted {
                server_event_id,
                stored_at,
                row_id,
            } => (
                EventAck {
                    index,
                    producer_event_id: Some(item.producer_event_id.clone()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some(server_event_id),
                    error_reason: None,
                    received_at: stored_at,
                },
                Some(row_id),
            ),
            InsertResult::Duplicate { stored_at } => (
                EventAck {
                    index,
                    producer_event_id: Some(item.producer_event_id.clone()),
                    status: EventAckStatus::Idempotent,
                    server_event_id: None,
                    error_reason: None,
                    received_at: stored_at,
                },
                None,
            ),
            InsertResult::SerializeError { error } | InsertResult::StorageError { error } => (
                EventAck {
                    index,
                    producer_event_id: Some(item.producer_event_id.clone()),
                    status: EventAckStatus::Rejected,
                    server_event_id: None,
                    error_reason: Some(error),
                    received_at: received_at.to_string(),
                },
                None,
            ),
        }
    }

    fn insert_event(
        &self,
        envelope: &ServerIngestEnvelope,
        item: &EnvelopeEvent,
        received_at: &str,
    ) -> InsertResult {
        let event_json = match serde_json::to_string(&item.event) {
            Ok(json) => json,
            Err(e) => {
                return InsertResult::SerializeError {
                    error: format!("TraceEvent serialization failed: {e}"),
                };
            }
        };

        let subject = item.event.subject().map(|s| s.to_string());
        let sk = item.event.subject_kind().map(|s| s.to_string());
        let fr = item.event.failure_reason().map(|s| s.to_string());

        let result = self.conn.execute(
            "INSERT OR IGNORE INTO server_trace_events
                (repository_id, workspace_id, agent_id, producer_event_id,
                 client_timestamp, received_at, event_json, schema_version,
                 timestamp, session_id, event_type, tool_name,
                 subject_kind, subject, outcome, failure_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                envelope.repository_id,
                envelope.workspace_id,
                envelope.agent_id,
                item.producer_event_id,
                item.client_timestamp,
                received_at,
                event_json,
                item.event.schema_version,
                item.event.timestamp,
                item.event.session_id,
                item.event.event_type.payload_type_str(),
                item.event.tool_name,
                sk,
                subject,
                outcome_str(&item.event),
                fr,
            ],
        );

        match result {
            Ok(1) => {
                // Row was inserted — new event.
                let rowid = self.conn.last_insert_rowid();
                InsertResult::Accepted {
                    server_event_id: format!("srv-{rowid}"),
                    stored_at: received_at.to_string(),
                    row_id: rowid,
                }
            }
            Ok(0) => {
                // INSERT OR IGNORE — duplicate key, read back original received_at.
                let stored_at: String = match self.conn.query_row(
                    "SELECT received_at FROM server_trace_events
                     WHERE repository_id = ?1 AND workspace_id = ?2
                       AND agent_id = ?3 AND producer_event_id = ?4",
                    params![
                        envelope.repository_id,
                        envelope.workspace_id,
                        envelope.agent_id,
                        item.producer_event_id,
                    ],
                    |row| row.get(0),
                ) {
                    Ok(ts) => ts,
                    Err(e) => {
                        return InsertResult::StorageError {
                            error: format!(
                                "failed to read back duplicate received_at for producer_event_id '{}' after INSERT OR IGNORE: {e}",
                                item.producer_event_id
                            ),
                        };
                    }
                };
                InsertResult::Duplicate { stored_at }
            }
            Err(e) => InsertResult::StorageError {
                error: format!(
                    "SQL error during INSERT OR IGNORE for producer_event_id '{}': {e}",
                    item.producer_event_id
                ),
            },
            Ok(_n) => InsertResult::StorageError {
                // Unreachable for SQLite INSERT — rows-changed is 0 or 1.
                error: format!(
                    "unexpected rows-changed ({_n}) from INSERT OR IGNORE for producer_event_id '{}'",
                    item.producer_event_id
                ),
            },
        }
    }

    /// Apply a newly-accepted event to the live hotspot accumulator for its
    /// subject. Lifecycle events (no subject) are silently skipped.
    ///
    /// Performs accumulator mutation, threshold-crossing check, and optional
    /// signal insert. Must be called within an explicit SQLite transaction.
    fn apply_live_accumulator(
        &self,
        envelope: &ServerIngestEnvelope,
        event_index: usize,
        row_id: i64,
    ) -> rusqlite::Result<()> {
        let Some(item) = envelope.events.get(event_index) else {
            return Ok(());
        };
        let event = &item.event;

        let Some(kind) = event.subject_kind() else {
            // Lifecycle event — skip.
            return Ok(());
        };
        let Some(subject) = event.subject() else {
            return Ok(());
        };

        let window = WINDOW_CUMULATIVE;
        let contribution = per_event_contribution(event);
        let repository_id = &envelope.repository_id;

        // Read existing accumulator or create a new one.
        let existing: Option<(u32, String, String, String, String, String, String)> = self
            .conn
            .query_row(
                "SELECT score, event_type_counts, outcome_counts, session_ids, evidence_row_ids,
                        first_seen, last_seen
             FROM hotspot_accumulators
             WHERE repository_id = ?1 AND window = ?2 AND subject_kind = ?3 AND subject = ?4",
                params![repository_id, window, kind, subject],
                |row| {
                    Ok((
                        row.get::<_, u32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .ok();

        let old_score: u32;
        let mut event_type_counts: serde_json::Value;
        let mut outcome_counts: serde_json::Value;
        let mut session_ids: serde_json::Value;
        let first_seen: String;
        let last_seen: String;
        let mut evidence_ids: serde_json::Value;

        if let Some((score, etc_json, oc_json, sess_json, ev_json, cur_first, cur_last)) = existing {
            old_score = score;
            event_type_counts = serde_json::from_str(&etc_json).unwrap_or(serde_json::json!({}));
            outcome_counts = serde_json::from_str(&oc_json).unwrap_or(serde_json::json!({}));
            session_ids = serde_json::from_str(&sess_json).unwrap_or(serde_json::json!([]));
            evidence_ids = serde_json::from_str(&ev_json).unwrap_or(serde_json::json!([]));

            first_seen = if event.timestamp < cur_first {
                event.timestamp.clone()
            } else {
                cur_first
            };

            last_seen = if event.timestamp > cur_last {
                event.timestamp.clone()
            } else {
                cur_last
            };
        } else {
            old_score = 0;
            event_type_counts = serde_json::json!({});
            outcome_counts = serde_json::json!({});
            session_ids = serde_json::json!([]);
            evidence_ids = serde_json::json!([]);
            first_seen = event.timestamp.clone();
            last_seen = event.timestamp.clone();
        }

        // Update per-event-type count.
        let type_name = event.event_type.payload_type_str();
        let type_count = event_type_counts[type_name].as_u64().unwrap_or(0) + 1;
        event_type_counts[type_name] = serde_json::json!(type_count);

        // Update per-outcome count.
        let outcome_key = match &event.outcome {
            scryrs_types::Outcome::Success => "success",
            scryrs_types::Outcome::Failure { .. } => "failure",
        };
        let oc_count = outcome_counts[outcome_key].as_u64().unwrap_or(0) + 1;
        outcome_counts[outcome_key] = serde_json::json!(oc_count);

        // Update session tracking.
        if let Some(sessions) = session_ids.as_array_mut() {
            let session_str = serde_json::json!(&event.session_id);
            if !sessions.contains(&session_str) {
                sessions.push(session_str);
            }
        }

        // Update evidence row IDs (stored sorted by timestamp ASC, id ASC).
        if let Some(ev_arr) = evidence_ids.as_array_mut() {
            ev_arr.push(serde_json::json!(row_id));
        }
        {
            // Re-sort evidence IDs so both accumulator and signal rows store
            // timestamp-ordered evidence matching batch hotspot semantics.
            let id_vec: Vec<i64> = evidence_ids
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_i64())
                        .collect()
                })
                .unwrap_or_default();
            if !id_vec.is_empty() {
                let sorted = self.sort_evidence_vec(&id_vec)?;
                evidence_ids = serde_json::json!(sorted);
            }
        }

        let new_score = old_score + contribution;

        let etc_json = serde_json::to_string(&event_type_counts)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let oc_json = serde_json::to_string(&outcome_counts)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let sess_json = serde_json::to_string(&session_ids)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let ev_json = serde_json::to_string(&evidence_ids)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        // Upsert accumulator row.
        self.conn.execute(
            "INSERT OR REPLACE INTO hotspot_accumulators
                (repository_id, window, subject_kind, subject,
                 score, event_type_counts, outcome_counts, session_ids,
                 first_seen, last_seen, evidence_row_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                repository_id,
                window,
                kind,
                subject,
                new_score,
                etc_json,
                oc_json,
                sess_json,
                first_seen,
                last_seen,
                ev_json,
            ],
        )?;

        // Check threshold crossing: old_score < threshold <= new_score.
        let threshold = self.signal_threshold;
        if old_score < threshold && new_score >= threshold {
            let created_at = chrono_now();

            self.conn.execute(
                "INSERT INTO hotspot_signals
                    (repository_id, subject_kind, subject, score, delta,
                     window, threshold, evidence_row_ids, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    repository_id,
                    kind,
                    subject,
                    new_score,
                    contribution,
                    window,
                    threshold,
                    ev_json,
                    created_at,
                ],
            )?;
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // Internal query helpers for tests
    // ------------------------------------------------------------------

    /// Return the raw accumulator row for a subject, or `None`.
    /// Evidence row IDs are returned in `timestamp ASC, id ASC` order
    /// matching batch hotspot semantics.
    pub fn get_accumulator_row(
        &self,
        repository_id: &str,
        window: &str,
        subject_kind: &str,
        subject: &str,
    ) -> rusqlite::Result<Option<AccumulatorRow>> {
        let row = self.conn.query_row(
            "SELECT score, event_type_counts, outcome_counts, session_ids,
                    first_seen, last_seen, evidence_row_ids
             FROM hotspot_accumulators
             WHERE repository_id = ?1 AND window = ?2
               AND subject_kind = ?3 AND subject = ?4",
            params![repository_id, window, subject_kind, subject],
            |r| {
                Ok(AccumulatorRow {
                    score: r.get(0)?,
                    event_type_counts: r.get(1)?,
                    outcome_counts: r.get(2)?,
                    session_ids: r.get(3)?,
                    first_seen: r.get(4)?,
                    last_seen: r.get(5)?,
                    evidence_row_ids: r.get(6)?,
                })
            },
        );

        match row {
            Ok(mut acc) => {
                acc.evidence_row_ids = self.sort_evidence_ids(&acc.evidence_row_ids)?;
                Ok(Some(acc))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Return all hotspot signals for a repository, ordered by creation time.
    pub fn get_signals(&self, repository_id: &str) -> rusqlite::Result<Vec<SignalRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, subject_kind, subject, score, delta, window,
                    threshold, evidence_row_ids, created_at
             FROM hotspot_signals
             WHERE repository_id = ?1
             ORDER BY created_at ASC",
        )?;

        let rows = stmt.query_map(params![repository_id], |r| {
            Ok(SignalRow {
                id: r.get(0)?,
                subject_kind: r.get(1)?,
                subject: r.get(2)?,
                score: r.get(3)?,
                delta: r.get(4)?,
                window: r.get(5)?,
                threshold: r.get(6)?,
                evidence_row_ids: r.get(7)?,
                created_at: r.get(8)?,
            })
        })?;

        rows.collect()
    }

    /// Return the total number of rows in hotspot_accumulators (diagnostic).
    pub fn accumulator_count(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM hotspot_accumulators", [], |row| {
                row.get(0)
            })
    }

    /// Return the total number of rows in hotspot_signals (diagnostic).
    pub fn signal_count(&self) -> rusqlite::Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM hotspot_signals", [], |row| row.get(0))
    }
}

/// Raw accumulator row exposed for tests.
#[derive(Debug, Clone)]
pub struct AccumulatorRow {
    pub score: u32,
    pub event_type_counts: String,
    pub outcome_counts: String,
    pub session_ids: String,
    pub first_seen: String,
    pub last_seen: String,
    pub evidence_row_ids: String,
}

/// Raw signal row exposed for tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalRow {
    pub id: i64,
    pub subject_kind: String,
    pub subject: String,
    pub score: u32,
    pub delta: u32,
    pub window: String,
    pub threshold: u32,
    pub evidence_row_ids: String,
    pub created_at: String,
}

/// Window tag for cumulative accumulator model.
pub const WINDOW_CUMULATIVE: &str = "cumulative";

impl ServerStore {
    /// Sort evidence row IDs by `timestamp ASC, id ASC` so they match
    /// batch hotspot semantics when materialized.
    fn sort_evidence_ids(&self, evidence_json: &str) -> rusqlite::Result<String> {
        let ids: Vec<i64> = serde_json::from_str(evidence_json).unwrap_or_default();

        if ids.is_empty() {
            return Ok(evidence_json.to_string());
        }

        let sorted = self.sort_evidence_vec(&ids)?;
        serde_json::to_string(&sorted)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    /// Sort a Vec of event row IDs by `timestamp ASC, id ASC`.
    fn sort_evidence_vec(&self, ids: &[i64]) -> rusqlite::Result<Vec<i64>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "SELECT id FROM server_trace_events WHERE id IN ({}) ORDER BY timestamp ASC, id ASC",
            placeholders.join(", ")
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<Box<dyn rusqlite::types::ToSql>> = ids
            .iter()
            .map(|id| Box::new(*id) as Box<dyn rusqlite::types::ToSql>)
            .collect();
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();
        stmt.query_map(param_refs.as_slice(), |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()
    }
}

enum InsertResult {
    Accepted {
        server_event_id: String,
        stored_at: String,
        row_id: i64,
    },
    Duplicate {
        stored_at: String,
    },
    SerializeError {
        error: String,
    },
    StorageError {
        error: String,
    },
}

fn outcome_str(event: &scryrs_types::TraceEvent) -> &'static str {
    match &event.outcome {
        scryrs_types::Outcome::Success => "Success",
        scryrs_types::Outcome::Failure { .. } => "Failure",
    }
}

/// Check whether `s` is valid RFC 3339.
fn is_valid_rfc3339(s: &str) -> bool {
    // RFC 3339 requires at minimum: YYYY-MM-DDTHH:MM:SS followed by Z or offset.
    // We do a structural check without depending on chrono.
    if s.len() < 20 {
        return false;
    }
    let bytes = s.as_bytes();
    // Check YYYY-MM-DDTHH:MM:SS prefix.
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    // After seconds, must have Z, fractional seconds + Z/offset, or offset.
    let rest = &s[19..];
    if rest.is_empty() {
        return false;
    }
    if rest == "Z" {
        return true;
    }
    if let Some(after_dot) = rest.strip_prefix('.') {
        // Fractional seconds: .digits followed by Z or [+-]HH:MM
        let digit_end = after_dot
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after_dot.len());
        if digit_end == 0 {
            return false; // no digits after dot
        }
        let suffix = &after_dot[digit_end..];
        return suffix == "Z" || is_offset(suffix);
    }
    is_offset(rest)
}

/// Check whether `s` is a valid RFC 3339 time-zone offset: `[+-]HH:MM`.
fn is_offset(s: &str) -> bool {
    s.len() == 6
        && (s.as_bytes()[0] == b'+' || s.as_bytes()[0] == b'-')
        && s.as_bytes()[1].is_ascii_digit()
        && s.as_bytes()[2].is_ascii_digit()
        && s.as_bytes()[3] == b':'
        && s.as_bytes()[4].is_ascii_digit()
        && s.as_bytes()[5].is_ascii_digit()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use scryrs_types::{
        DocRetrievedPayload, EnvelopeEvent, EventAckStatus, Outcome, SCHEMA_VERSION,
        ServerIngestEnvelope, TraceEvent, TraceEventPayload, TraceEventType,
    };

    fn make_event(session_id: &str, doc_ref: &str) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-24T10:00:00Z".into(),
            session_id: session_id.into(),
            event_type: TraceEventType::DocRetrieved,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: doc_ref.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    fn make_envelope(
        repo: &str,
        ws: &str,
        agent: &str,
        events: Vec<EnvelopeEvent>,
    ) -> ServerIngestEnvelope {
        ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: repo.into(),
            workspace_id: ws.into(),
            agent_id: agent.into(),
            events,
        }
    }

    fn env_event(id: &str, event: TraceEvent) -> EnvelopeEvent {
        EnvelopeEvent {
            producer_event_id: id.into(),
            client_timestamp: "2026-06-24T10:00:05Z".into(),
            event,
        }
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"))
    }

    #[test]
    fn store_creates_schema_tables_and_indexes() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");

        let _store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));
        assert!(store_path.exists());

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));

        // server_schema_meta exists with correct version.
        let version: String = conn
            .query_row(
                "SELECT value FROM server_schema_meta WHERE key = 'server_store_schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("version query: {e}"));
        assert_eq!(version, SERVER_STORE_SCHEMA_VERSION.to_string());

        // server_trace_events table exists.
        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='server_trace_events'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("table check: {e}"));
        assert_eq!(table_count, 1);

        // Unique dedup index exists.
        let idx_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_server_events_dedup'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("index check: {e}"));
        assert_eq!(idx_count, 1);
    }

    #[test]
    fn first_insert_is_accepted() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let acks = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].index, 0);
        assert_eq!(acks[0].status, EventAckStatus::Accepted);
        assert!(acks[0].server_event_id.is_some());
        assert!(acks[0].error_reason.is_none());
    }

    #[test]
    fn duplicate_submission_is_idempotent() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let acks1 = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks1[0].status, EventAckStatus::Accepted);
        let original_received_at = acks1[0].received_at.clone();

        let acks2 = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks2[0].status, EventAckStatus::Idempotent);
        assert_eq!(acks2[0].received_at, original_received_at);
    }

    #[test]
    fn ingest_batch_propagates_accumulator_errors() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        // Drop the accumulator table to force an error during apply_live_accumulator.
        store
            .conn
            .execute_batch("DROP TABLE hotspot_accumulators;")
            .unwrap();

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let result = store.ingest_batch(&envelope);
        assert!(result.is_err(), "expected error when accumulator table is missing");
    }

    #[test]
    fn different_keys_produce_different_rows() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let env1 = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![env_event("evt-001", make_event("s1", "doc/a.md"))],
        );
        let env2 = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![env_event("evt-002", make_event("s2", "doc/b.md"))],
        );

        let acks1 = store.ingest_batch(&env1).unwrap();
        assert_eq!(acks1[0].status, EventAckStatus::Accepted);

        let acks2 = store.ingest_batch(&env2).unwrap();
        assert_eq!(acks2[0].status, EventAckStatus::Accepted);

        // Verify two rows in the store.
        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM server_trace_events", [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(count, 2);
    }

    #[test]
    fn invalid_client_timestamp_is_rejected() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "not-a-timestamp".into(),
                event,
            }],
        );

        let acks = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks.len(), 1);
        assert_eq!(acks[0].status, EventAckStatus::Rejected);
        assert!(
            acks[0]
                .error_reason
                .as_deref()
                .unwrap_or("")
                .contains("invalid client_timestamp")
        );
    }

    #[test]
    fn mixed_batch_accepts_valid_and_rejects_invalid() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let mut invalid_event = make_event("s1", "doc/a.md");
        invalid_event.schema_version = "0.9.9".into(); // will fail validate()

        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![
                env_event("evt-001", make_event("s1", "doc/a.md")),
                env_event("evt-002", invalid_event),
                env_event("evt-003", make_event("s2", "doc/b.md")),
            ],
        );

        let acks = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks.len(), 3);

        assert_eq!(acks[0].index, 0);
        assert_eq!(acks[0].status, EventAckStatus::Accepted);

        assert_eq!(acks[1].index, 1);
        assert_eq!(acks[1].status, EventAckStatus::Rejected);
        assert!(
            acks[1]
                .error_reason
                .as_deref()
                .unwrap_or("")
                .contains("validation failed")
        );

        assert_eq!(acks[2].index, 2);
        assert_eq!(acks[2].status, EventAckStatus::Accepted);
    }

    #[test]
    fn empty_events_array_returns_empty_acks() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![]);
        let acks = store.ingest_batch(&envelope).unwrap();
        assert!(acks.is_empty());
    }

    // --- Schema migration v1 -> v2 ---

    #[test]
    fn v2_tables_created_on_fresh_store() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let _store = ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open: {e}"));

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));

        // Version is now 2.
        let version: String = conn
            .query_row(
                "SELECT value FROM server_schema_meta WHERE key = 'server_store_schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("version query: {e}"));
        assert_eq!(version, "2");

        // hotspot_accumulators table exists.
        let acc_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hotspot_accumulators'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("accumulators check: {e}"));
        assert_eq!(acc_count, 1);

        // hotspot_signals table exists.
        let sig_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hotspot_signals'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("signals check: {e}"));
        assert_eq!(sig_count, 1);
    }

    #[test]
    fn existing_v1_store_migrates_to_v2_additively() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");

        // Create a v1 database with an event already in the table.
        {
            let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("create v1: {e}"));
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS server_schema_meta (key TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO server_schema_meta VALUES ('server_store_schema_version', '1');
                 CREATE TABLE IF NOT EXISTS server_trace_events (
                     id                INTEGER PRIMARY KEY AUTOINCREMENT,
                     repository_id     TEXT NOT NULL,
                     workspace_id      TEXT NOT NULL,
                     agent_id          TEXT NOT NULL,
                     producer_event_id TEXT NOT NULL,
                     client_timestamp  TEXT NOT NULL,
                     received_at       TEXT NOT NULL,
                     event_json        TEXT NOT NULL,
                     schema_version    TEXT NOT NULL,
                     timestamp         TEXT NOT NULL,
                     session_id        TEXT NOT NULL,
                     event_type        TEXT NOT NULL,
                     tool_name         TEXT,
                     subject_kind      TEXT,
                     subject           TEXT,
                     outcome           TEXT NOT NULL,
                     failure_reason    TEXT
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_server_events_dedup
                     ON server_trace_events(repository_id, workspace_id, agent_id, producer_event_id);
                 INSERT INTO server_trace_events
                     (repository_id, workspace_id, agent_id, producer_event_id,
                      client_timestamp, received_at, event_json, schema_version,
                      timestamp, session_id, event_type, tool_name,
                      subject_kind, subject, outcome, failure_reason)
                 VALUES ('repo-a', 'ws-1', 'pi', 'evt-pre-upgrade',
                         '2026-06-24T10:00:05Z', '2026-06-24T10:00:07Z', '{}', '0.1.0',
                         '2026-06-24T10:00:00Z', 's1', 'DocRetrieved', 'read',
                         'document', 'doc/old.md', 'Success', NULL);",
            )
            .unwrap_or_else(|e| panic!("setup v1: {e}"));
        }

        // Open with the upgraded store — migration should succeed.
        let store =
            ServerStore::open(&store_path, 10).unwrap_or_else(|e| panic!("open v1→v2: {e}"));

        let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen: {e}"));

        // Version is now 2.
        let version: String = conn
            .query_row(
                "SELECT value FROM server_schema_meta WHERE key = 'server_store_schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("version query: {e}"));
        assert_eq!(version, "2");

        // New tables exist.
        let acc_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hotspot_accumulators'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("accumulators check: {e}"));
        assert_eq!(acc_count, 1);

        let sig_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='hotspot_signals'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("signals check: {e}"));
        assert_eq!(sig_count, 1);

        // Pre-existing event row still there.
        let old_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM server_trace_events WHERE producer_event_id = 'evt-pre-upgrade'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("old event check: {e}"));
        assert_eq!(old_count, 1);

        // No accumulator rows were backfilled from pre-upgrade events.
        let acc_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM hotspot_accumulators", [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("acc count: {e}"));
        assert_eq!(acc_rows, 0, "no backfill from pre-upgrade events");

        // Verify the store opened successfully after migration.
        let _store = store;
    }

    #[test]
    fn unknown_schema_version_fails_fast() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");

        {
            let conn = Connection::open(&store_path).unwrap_or_else(|e| panic!("create: {e}"));
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS server_schema_meta (key TEXT PRIMARY KEY, value TEXT);
                 INSERT INTO server_schema_meta VALUES ('server_store_schema_version', '99');",
            )
            .unwrap_or_else(|e| panic!("write version: {e}"));
        }

        let result = ServerStore::open(&store_path, 10);
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
    }

    // ------------------------------------------------------------------
    // Live hotspot accumulator tests (4.3, 4.4)
    // ------------------------------------------------------------------

    use scryrs_core::scoring::WEIGHT_FILE_OPENED;
    use scryrs_types::{
        CommandExecutedPayload, EditMadePayload, FileOpenedPayload, SessionStartPayload,
    };

    fn subject_event(
        event_type: TraceEventType,
        subject: &str,
        session_id: &str,
        timestamp: &str,
        outcome: Outcome,
    ) -> TraceEvent {
        let payload = match event_type {
            TraceEventType::FileOpened => TraceEventPayload::FileOpened(FileOpenedPayload {
                path: subject.to_string(),
            }),
            TraceEventType::EditMade => TraceEventPayload::EditMade(EditMadePayload {
                target: subject.to_string(),
            }),
            TraceEventType::CommandExecuted => {
                TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: subject.to_string(),
                })
            }
            _ => TraceEventPayload::FileOpened(FileOpenedPayload {
                path: subject.to_string(),
            }),
        };
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type,
            tool_name: Some("tool".into()),
            payload,
            outcome,
        }
    }

    // 4.3: First accepted subject-bearing event creates accumulator.
    #[test]
    fn first_accepted_event_creates_accumulator() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let event = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let _acks = store.ingest_batch(&envelope).unwrap();

        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/main.rs")
            .unwrap()
            .unwrap_or_else(|| panic!("accumulator must exist"));
        assert_eq!(acc.score, WEIGHT_FILE_OPENED); // 1
        assert_eq!(acc.first_seen, "2026-06-25T10:00:00Z");
        assert_eq!(acc.last_seen, "2026-06-25T10:00:00Z");

        let sessions: serde_json::Value = serde_json::from_str(&acc.session_ids).unwrap();
        assert_eq!(sessions.as_array().unwrap().len(), 1);

        let evidence: serde_json::Value = serde_json::from_str(&acc.evidence_row_ids).unwrap();
        assert_eq!(evidence.as_array().unwrap().len(), 1);
    }

    // 4.3: Two events for same subject accumulate.
    #[test]
    fn two_events_same_subject_accumulate() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let e1 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        let e2 = subject_event(
            TraceEventType::EditMade,
            "src/main.rs",
            "s1",
            "2026-06-25T10:01:00Z",
            Outcome::Success,
        );

        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![env_event("evt-001", e1), env_event("evt-002", e2)],
        );
        let _acks = store.ingest_batch(&envelope).unwrap();

        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/main.rs")
            .unwrap()
            .unwrap();
        // FileOpened(1) + EditMade(3) = 4
        assert_eq!(acc.score, 4);
        assert_eq!(acc.first_seen, "2026-06-25T10:00:00Z");
        assert_eq!(acc.last_seen, "2026-06-25T10:01:00Z");
    }

    // 4.3: Failure bonus applied.
    #[test]
    fn failure_bonus_applied_in_accumulator() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let event = subject_event(
            TraceEventType::EditMade,
            "src/x.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Failure {
                reason: Some("err".into()),
            },
        );
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);
        let _acks = store.ingest_batch(&envelope).unwrap();

        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/x.rs")
            .unwrap()
            .unwrap();
        // EditMade base 3 + failure bonus 2 = 5
        assert_eq!(acc.score, 5);

        let outcome_counts: serde_json::Value = serde_json::from_str(&acc.outcome_counts).unwrap();
        assert_eq!(outcome_counts["failure"], 1);
        assert_eq!(outcome_counts.get("success").and_then(|v| v.as_u64()), None);
    }

    // 4.3: Duplicate replay does not change accumulator.
    #[test]
    fn duplicate_replay_does_not_change_accumulator() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let event = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let _acks1 = store.ingest_batch(&envelope).unwrap();
        let _acks2 = store.ingest_batch(&envelope).unwrap();

        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/main.rs")
            .unwrap()
            .unwrap();
        // Still 1 — duplicate does not double-count.
        assert_eq!(acc.score, 1);

        let evidence: serde_json::Value = serde_json::from_str(&acc.evidence_row_ids).unwrap();
        assert_eq!(evidence.as_array().unwrap().len(), 1);

        // No duplicate signal.
        assert_eq!(store.signal_count().unwrap(), 0);
    }

    // 4.3: Lifecycle events do not create accumulators.
    #[test]
    fn lifecycle_events_do_not_create_accumulators() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let event = make_event("s1", "doc/a.md"); // DocRetrieved is subject-bearing
        let session_start = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-25T10:00:00Z".into(),
            session_id: "s1".into(),
            event_type: TraceEventType::SessionStart,
            tool_name: None,
            payload: TraceEventPayload::SessionStart(SessionStartPayload),
            outcome: Outcome::Success,
        };

        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![
                env_event("evt-001", session_start),
                env_event("evt-002", event),
            ],
        );
        let _acks = store.ingest_batch(&envelope).unwrap();

        // Only one accumulator (for doc/a.md), not for the lifecycle event.
        assert_eq!(store.accumulator_count().unwrap(), 1);
    }

    // 4.3: Rejected events do not create accumulators.
    #[test]
    fn rejected_events_do_not_create_accumulators() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let mut bad_event = subject_event(
            TraceEventType::FileOpened,
            "src/x.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        bad_event.schema_version = "0.9.9".into(); // will fail validate()

        let good_event = subject_event(
            TraceEventType::FileOpened,
            "src/y.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );

        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![
                env_event("evt-001", bad_event),
                env_event("evt-002", good_event),
            ],
        );
        let acks = store.ingest_batch(&envelope).unwrap();
        assert_eq!(acks[0].status, EventAckStatus::Rejected);
        assert_eq!(acks[1].status, EventAckStatus::Accepted);

        // Only one accumulator: only the accepted subject-bearing event.
        assert_eq!(store.accumulator_count().unwrap(), 1);
        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/y.rs")
            .unwrap()
            .unwrap();
        assert_eq!(acc.score, 1);
        // src/x.rs must not have an accumulator.
        assert!(
            store
                .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/x.rs")
                .unwrap()
                .is_none()
        );
    }

    // 4.3: Threshold crossing emits a signal.
    #[test]
    fn threshold_crossing_emits_signal() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        // 5 FileOpened events = score 5 (below threshold 10).
        for i in 0..5 {
            let event = subject_event(
                TraceEventType::FileOpened,
                "src/main.rs",
                &format!("s{i}"),
                &format!("2026-06-25T10:0{i}:00Z"),
                Outcome::Success,
            );
            let envelope = make_envelope(
                "repo-a",
                "ws-1",
                "pi",
                vec![env_event(&format!("evt-00{i}"), event)],
            );
            let _acks = store.ingest_batch(&envelope).unwrap();
        }

        assert_eq!(store.signal_count().unwrap(), 0);

        // 6th FileOpened = score 6 (still below).
        let e6 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s6",
            "2026-06-25T10:06:00Z",
            Outcome::Success,
        );
        let env6 = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-006", e6)]);
        let _acks6 = store.ingest_batch(&env6).unwrap();
        assert_eq!(store.signal_count().unwrap(), 0);

        // 7..9 more FileOpened = total score 9 (still below 10).
        for i in 7..=9 {
            let event = subject_event(
                TraceEventType::FileOpened,
                "src/main.rs",
                &format!("s{i}"),
                &format!("2026-06-25T10:0{i}:00Z"),
                Outcome::Success,
            );
            let envelope = make_envelope(
                "repo-a",
                "ws-1",
                "pi",
                vec![env_event(&format!("evt-00{i}"), event)],
            );
            let _acks = store.ingest_batch(&envelope).unwrap();
        }
        assert_eq!(store.signal_count().unwrap(), 0);

        // 10th FileOpened crosses threshold (9 -> 10).
        let e10 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s10",
            "2026-06-25T10:10:00Z",
            Outcome::Success,
        );
        let env10 = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-010", e10)]);
        let _acks10 = store.ingest_batch(&env10).unwrap();

        assert_eq!(store.signal_count().unwrap(), 1);
        let signals = store.get_signals("repo-a").unwrap();
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].subject_kind, "file");
        assert_eq!(signals[0].subject, "src/main.rs");
        assert_eq!(signals[0].score, 10);
        assert_eq!(signals[0].delta, 1);
        assert_eq!(signals[0].window, WINDOW_CUMULATIVE);
        assert_eq!(signals[0].threshold, 10);
    }

    // 4.3: No duplicate signal when already above threshold.
    #[test]
    fn no_duplicate_signal_when_already_above_threshold() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        // 10 FileOpened crosses threshold and emits one signal.
        for i in 0..10 {
            let event = subject_event(
                TraceEventType::FileOpened,
                "src/main.rs",
                &format!("s{i}"),
                &format!("2026-06-25T10:{i:02}:00Z"),
                Outcome::Success,
            );
            let envelope = make_envelope(
                "repo-a",
                "ws-1",
                "pi",
                vec![env_event(&format!("evt-{i:03}"), event)],
            );
            let _acks = store.ingest_batch(&envelope).unwrap();
        }

        assert_eq!(store.signal_count().unwrap(), 1);

        // 11th event — still above threshold, no new signal.
        let e11 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s11",
            "2026-06-25T10:11:00Z",
            Outcome::Success,
        );
        let env11 = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-011", e11)]);
        let _acks11 = store.ingest_batch(&env11).unwrap();

        assert_eq!(store.signal_count().unwrap(), 1);
        let acc = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/main.rs")
            .unwrap()
            .unwrap();
        assert_eq!(acc.score, 11);
    }

    // 4.3: Duplicate replay does not emit a duplicate signal.
    #[test]
    fn duplicate_replay_does_not_emit_duplicate_signal() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        // Cross threshold with 10 events.
        for i in 0..10 {
            let event = subject_event(
                TraceEventType::FileOpened,
                "src/main.rs",
                &format!("s{i}"),
                &format!("2026-06-25T10:{i:02}:00Z"),
                Outcome::Success,
            );
            let envelope = make_envelope(
                "repo-a",
                "ws-1",
                "pi",
                vec![env_event(&format!("evt-{i:03}"), event)],
            );
            let _acks = store.ingest_batch(&envelope).unwrap();
        }

        assert_eq!(store.signal_count().unwrap(), 1);

        // Replay the 10th event.
        let e10 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s9",
            "2026-06-25T10:09:00Z",
            Outcome::Success,
        );
        let env10_replay = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-009", e10)]);
        let acks = store.ingest_batch(&env10_replay).unwrap();
        assert_eq!(acks[0].status, EventAckStatus::Idempotent);

        // Still just one signal.
        assert_eq!(store.signal_count().unwrap(), 1);
    }

    // 4.4: Cumulative live scores match batch scores.
    #[test]
    fn cumulative_live_scores_match_batch_scores() {
        use scryrs_core::scoring::score_hotspots;

        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        let event1 = subject_event(
            TraceEventType::FileOpened,
            "src/a.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        let event2 = subject_event(
            TraceEventType::EditMade,
            "src/a.rs",
            "s1",
            "2026-06-25T10:01:00Z",
            Outcome::Failure {
                reason: Some("err".into()),
            },
        );
        let event3 = subject_event(
            TraceEventType::FileOpened,
            "src/b.rs",
            "s2",
            "2026-06-25T10:02:00Z",
            Outcome::Success,
        );

        // Ingest events into the live store.
        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![
                env_event("evt-001", event1.clone()),
                env_event("evt-002", event2.clone()),
                env_event("evt-003", event3.clone()),
            ],
        );
        let _acks = store.ingest_batch(&envelope).unwrap();

        // Run batch scoring on the same events.
        let events_ref: Vec<(u64, &TraceEvent)> =
            vec![(1u64, &event1), (2u64, &event2), (3u64, &event3)];
        let batch_entries = score_hotspots(&events_ref);

        // Compare accumulator scores with batch scores.
        // src/a.rs: FileOpened(1) + EditMade(3+2) = 6
        let acc_a = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/a.rs")
            .unwrap()
            .unwrap();
        // src/b.rs: FileOpened(1) = 1
        let acc_b = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/b.rs")
            .unwrap()
            .unwrap();

        assert_eq!(acc_a.score, 6);
        assert_eq!(acc_b.score, 1);

        // Batch should produce same scores and order.
        assert_eq!(batch_entries.len(), 2);
        // Higher score first.
        assert_eq!(batch_entries[0].subject, "src/a.rs");
        assert_eq!(batch_entries[0].score, 6);
        assert_eq!(batch_entries[1].subject, "src/b.rs");
        assert_eq!(batch_entries[1].score, 1);
    }

    // 4.4: Subject-bearing event families all contribute to live state.
    #[test]
    fn all_subject_bearing_families_contribute_to_live_state() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path, 10).unwrap();

        // FileOpened (file) - weight 1
        let e1 = subject_event(
            TraceEventType::FileOpened,
            "src/main.rs",
            "s1",
            "2026-06-25T10:00:00Z",
            Outcome::Success,
        );
        // EditMade (file) - weight 3
        let e2 = subject_event(
            TraceEventType::EditMade,
            "src/main.rs",
            "s1",
            "2026-06-25T10:01:00Z",
            Outcome::Success,
        );
        // CommandExecuted (command) - weight 1
        let e3 = subject_event(
            TraceEventType::CommandExecuted,
            "cargo build",
            "s1",
            "2026-06-25T10:02:00Z",
            Outcome::Success,
        );

        let envelope = make_envelope(
            "repo-a",
            "ws-1",
            "pi",
            vec![
                env_event("evt-001", e1),
                env_event("evt-002", e2),
                env_event("evt-003", e3),
            ],
        );
        let _acks = store.ingest_batch(&envelope).unwrap();

        // file accumulator for src/main.rs: 1 + 3 = 4
        let acc_file = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "file", "src/main.rs")
            .unwrap()
            .unwrap();
        assert_eq!(acc_file.score, 4);

        // command accumulator for cargo build: 1
        let acc_cmd = store
            .get_accumulator_row("repo-a", WINDOW_CUMULATIVE, "command", "cargo build")
            .unwrap()
            .unwrap();
        assert_eq!(acc_cmd.score, 1);

        assert_eq!(store.accumulator_count().unwrap(), 2);
    }
}
