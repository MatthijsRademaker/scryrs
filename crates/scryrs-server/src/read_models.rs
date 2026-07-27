//! Repository-scoped read models exposed by live server APIs.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Numeric server event ID encoded as an opaque JSON string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventCursor(u64);

impl EventCursor {
    #[must_use]
    pub const fn new(event_id: u64) -> Self {
        Self(event_id)
    }

    #[must_use]
    pub const fn event_id(self) -> u64 {
        self.0
    }
}

impl fmt::Display for EventCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for EventCursor {
    type Err = CursorParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value
            .parse::<u64>()
            .map(Self)
            .map_err(|_| CursorParseError(value.to_owned()))
    }
}

impl Serialize for EventCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for EventCursor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Invalid event cursor supplied by a client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorParseError(String);

impl fmt::Display for CursorParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cursor must be an unsigned event ID: '{}'",
            self.0
        )
    }
}

impl std::error::Error for CursorParseError {}

/// Repository-scoped aggregate for one trace session.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub event_count: u64,
    pub source: String,
}

/// Cursor-paginated session summaries.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionsPage {
    pub sessions: Vec<SessionSummary>,
    pub next_cursor: Option<EventCursor>,
}

/// Trace event returned by live read APIs.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEventItem {
    pub event_id: u64,
    pub session_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub subject_kind: Option<String>,
    pub subject: Option<String>,
    pub payload: Value,
}

/// Cursor-paginated event response.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    pub events: Vec<TraceEventItem>,
    pub next_cursor: Option<EventCursor>,
}

/// Session aggregate plus events ordered by ascending server event ID.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session: SessionSummary,
    pub events: Vec<TraceEventItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_cursor_uses_string_wire_format() {
        let cursor = EventCursor::new(42);
        let json =
            serde_json::to_string(&cursor).unwrap_or_else(|error| panic!("serialize: {error}"));
        assert_eq!(json, "\"42\"");
        let decoded: EventCursor =
            serde_json::from_str(&json).unwrap_or_else(|error| panic!("deserialize: {error}"));
        assert_eq!(decoded, cursor);
    }
}
