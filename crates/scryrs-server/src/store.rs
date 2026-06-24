//! Dedicated server-owned SQLite store for central trace ingest.
//!
//! This store is intentionally separate from the local `trace_events` schema
//! in `scryrs-core`. It mirrors the normalized columns needed for future
//! scoring and adds identity/idempotency columns.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, params};
use scryrs_types::{EnvelopeEvent, EventAck, EventAckStatus, ServerIngestEnvelope};

use crate::time::chrono_now;

/// Current server store schema version (independent of local datastore version).
const SERVER_STORE_SCHEMA_VERSION: i64 = 1;

/// Open a connection at `path`, creating parent directories.
fn open_connection(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    }

    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    Ok(conn)
}

/// Ensure the server store schema exists.
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

    if let Some(v) = existing_version {
        if v != SERVER_STORE_SCHEMA_VERSION {
            return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "server store schema version mismatch: found {v}, expected {SERVER_STORE_SCHEMA_VERSION}"
                    ),
                ),
            )));
        }
        return Ok(());
    }

    conn.execute(
        "INSERT OR REPLACE INTO server_schema_meta (key, value) VALUES ('server_store_schema_version', ?1)",
        params![SERVER_STORE_SCHEMA_VERSION.to_string()],
    )?;

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
    )?;

    Ok(())
}

/// Server-owned SQLite store for central trace ingest with idempotent inserts.
pub struct ServerStore {
    conn: Connection,
}

impl ServerStore {
    /// Open (or create) the server datastore at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let path_ref = path.as_ref();
        let conn = open_connection(path_ref)?;
        ensure_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Process a batch ingest envelope, returning per-item acknowledgments.
    ///
    /// Each `EnvelopeEvent` in the request is processed independently:
    ///
    /// - Valid events are accepted (first-writer-wins) or acknowledged as idempotent.
    /// - Invalid events are rejected with per-item diagnostics.
    ///
    /// Returns acknowledgments in request order (matching `events[].index`).
    pub fn ingest_batch(&self, envelope: &ServerIngestEnvelope) -> Vec<EventAck> {
        let received_at = chrono_now();

        envelope
            .events
            .iter()
            .enumerate()
            .map(|(index, item)| self.process_item(envelope, item, index, &received_at))
            .collect()
    }

    fn process_item(
        &self,
        envelope: &ServerIngestEnvelope,
        item: &EnvelopeEvent,
        index: usize,
        received_at: &str,
    ) -> EventAck {
        // Validate client_timestamp as RFC 3339.
        if !is_valid_rfc3339(&item.client_timestamp) {
            return EventAck {
                index,
                producer_event_id: Some(item.producer_event_id.clone()),
                status: EventAckStatus::Rejected,
                server_event_id: None,
                error_reason: Some(format!(
                    "invalid client_timestamp: '{}' is not valid RFC 3339",
                    item.client_timestamp
                )),
                received_at: received_at.to_string(),
            };
        }

        // Validate inner TraceEvent.
        if let Err(reason) = item.event.validate() {
            return EventAck {
                index,
                producer_event_id: Some(item.producer_event_id.clone()),
                status: EventAckStatus::Rejected,
                server_event_id: None,
                error_reason: Some(format!("TraceEvent validation failed: {reason}")),
                received_at: received_at.to_string(),
            };
        }

        // Attempt insert with idempotency.
        match self.insert_event(envelope, item, received_at) {
            InsertResult::Accepted {
                server_event_id,
                stored_at,
            } => EventAck {
                index,
                producer_event_id: Some(item.producer_event_id.clone()),
                status: EventAckStatus::Accepted,
                server_event_id: Some(server_event_id),
                error_reason: None,
                received_at: stored_at,
            },
            InsertResult::Duplicate { stored_at } => EventAck {
                index,
                producer_event_id: Some(item.producer_event_id.clone()),
                status: EventAckStatus::Idempotent,
                server_event_id: None,
                error_reason: None,
                received_at: stored_at,
            },
            InsertResult::SerializeError { error } => EventAck {
                index,
                producer_event_id: Some(item.producer_event_id.clone()),
                status: EventAckStatus::Rejected,
                server_event_id: None,
                error_reason: Some(error),
                received_at: received_at.to_string(),
            },
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
                }
            }
            Ok(0) => {
                // INSERT OR IGNORE — duplicate key, read back original received_at.
                let stored_at: String = self
                    .conn
                    .query_row(
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
                    )
                    .unwrap_or_else(|_| received_at.to_string());
                InsertResult::Duplicate { stored_at }
            }
            _ => {
                // Unexpected — treat as duplicate for safety.
                InsertResult::Duplicate {
                    stored_at: received_at.to_string(),
                }
            }
        }
    }
}

enum InsertResult {
    Accepted {
        server_event_id: String,
        stored_at: String,
    },
    Duplicate {
        stored_at: String,
    },
    SerializeError {
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

        let _store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));
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
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let acks = store.ingest_batch(&envelope);
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
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

        let event = make_event("s1", "doc/a.md");
        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![env_event("evt-001", event)]);

        let acks1 = store.ingest_batch(&envelope);
        assert_eq!(acks1[0].status, EventAckStatus::Accepted);
        let original_received_at = acks1[0].received_at.clone();

        let acks2 = store.ingest_batch(&envelope);
        assert_eq!(acks2[0].status, EventAckStatus::Idempotent);
        assert_eq!(acks2[0].received_at, original_received_at);
    }

    #[test]
    fn different_keys_produce_different_rows() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

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

        let acks1 = store.ingest_batch(&env1);
        assert_eq!(acks1[0].status, EventAckStatus::Accepted);

        let acks2 = store.ingest_batch(&env2);
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
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

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

        let acks = store.ingest_batch(&envelope);
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
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

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

        let acks = store.ingest_batch(&envelope);
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
        let store = ServerStore::open(&store_path).unwrap_or_else(|e| panic!("open: {e}"));

        let envelope = make_envelope("repo-a", "ws-1", "pi", vec![]);
        let acks = store.ingest_batch(&envelope);
        assert!(acks.is_empty());
    }
}
