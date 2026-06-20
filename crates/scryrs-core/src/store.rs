//! Minimal append-only event store backed by a local JSONL file.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use scryrs_types::TraceEvent;

/// Append-only store that persists accepted [`TraceEvent`]s as JSONL.
///
/// The store surface is intentionally narrow — it only appends events
/// and reports the stored count. No query, delete, or analysis APIs.
pub struct EventStore {
    file: fs::File,
    stored_count: u64,
}

impl EventStore {
    /// Open (or create) the store at `path`, creating parent directories
    /// as needed.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path_ref = path.as_ref();
        if let Some(parent) = path_ref.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path_ref)?;
        Ok(Self {
            file,
            stored_count: 0,
        })
    }

    /// Open the default local store at `.scryrs/events.jsonl` relative
    /// to the current working directory.
    pub fn default_local() -> io::Result<Self> {
        Self::open(".scryrs/events.jsonl")
    }

    /// Append a single accepted event to the store.
    ///
    /// The event is serialized as a single JSON line and flushed
    /// immediately so downstream readers see durable writes.
    pub fn append(&mut self, event: &TraceEvent) -> io::Result<()> {
        let line =
            serde_json::to_string(event).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        writeln!(self.file, "{}", line)?;
        self.file.flush()?;
        self.stored_count += 1;
        Ok(())
    }

    /// Number of events appended to this store instance so far.
    #[must_use]
    pub fn stored_count(&self) -> u64 {
        self.stored_count
    }
}

#[cfg(test)]
mod tests {
    use scryrs_types::{
        DocRetrievedPayload, Outcome, SCHEMA_VERSION, TraceEvent, TraceEventPayload, TraceEventType,
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

    fn read_to_string(path: &std::path::Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|e| panic!("read: {e}"))
    }

    fn parse_event(json: &str) -> TraceEvent {
        serde_json::from_str(json).unwrap_or_else(|e| panic!("deserialize: {e}"))
    }

    #[test]
    fn store_creates_and_appends() {
        let dir = temp_dir();
        let store_path = dir.path().join("events.jsonl");

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

        let contents = read_to_string(&store_path);
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let _event = parse_event(line);
        }
    }

    #[test]
    fn default_local_creates_dot_scryrs_dir() {
        let dir = temp_dir();
        let cwd = std::env::current_dir().unwrap_or_else(|e| panic!("current dir: {e}"));
        std::env::set_current_dir(dir.path()).unwrap_or_else(|e| panic!("chdir: {e}"));

        let result = EventStore::default_local();

        std::env::set_current_dir(&cwd).unwrap_or_else(|e| panic!("restore cwd: {e}"));

        let mut store = result.unwrap_or_else(|e| panic!("default_local should succeed: {e}"));
        store
            .append(&make_event("s1", "doc/x.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));
        assert_eq!(store.stored_count(), 1);
        assert!(dir.path().join(".scryrs/events.jsonl").exists());
    }

    #[test]
    fn open_creates_parent_directories() {
        let dir = temp_dir();
        let nested = dir.path().join("sub1/sub2/events.jsonl");

        let mut store = open_ok(&nested);
        store
            .append(&make_event("s1", "doc/z.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));

        assert!(nested.exists());
        let contents = read_to_string(&nested);
        assert!(!contents.is_empty());
    }

    #[test]
    fn store_count_is_zero_initially() {
        let dir = temp_dir();
        let store_path = dir.path().join("fresh.jsonl");

        let store = open_ok(&store_path);
        assert_eq!(store.stored_count(), 0);
    }

    #[test]
    fn appended_events_are_valid_jsonl() {
        let dir = temp_dir();
        let store_path = dir.path().join("events.jsonl");

        let mut store = open_ok(&store_path);
        store
            .append(&make_event("s1", "doc/a.md"))
            .unwrap_or_else(|e| panic!("append: {e}"));

        let contents = read_to_string(&store_path);
        let lines: Vec<&str> = contents.trim().lines().collect();
        assert_eq!(lines.len(), 1);

        let event = parse_event(lines[0]);
        assert_eq!(event.session_id, "s1");
    }
}
