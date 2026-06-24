//! Single source of truth for translating a harness's native tool event into a
//! canonical scryrs [`TraceEvent`].
//!
//! Each supported harness has one [`HarnessAdapter`] implementation. An adapter
//! parses the harness's native event JSON and returns zero or one canonical
//! [`TraceEvent`] (returning `None` for untracked tools — pass-through). Shared
//! envelope construction, newline collapsing, and the canonical schema version
//! live here so the translation logic is never duplicated across harnesses or
//! re-implemented in JavaScript.

use std::path::PathBuf;

use scryrs_types::{Outcome, SCHEMA_VERSION, TraceEvent, TraceEventPayload, TraceEventType};

mod claude_code;
mod pi;

pub use claude_code::ClaudeCodeAdapter;
pub use pi::PiAdapter;

/// Per-event context resolved by the hook command before translation.
///
/// `session_id` and `timestamp` are stamped onto the resulting envelope by the
/// adapter; `store_path` is the cwd-relative trace store the hook command will
/// persist into (carried here so callers resolve it once, from the payload).
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Session identifier taken from the harness payload (or a fallback).
    pub session_id: String,
    /// Resolved trace store path the hook command persists into.
    pub store_path: PathBuf,
    /// ISO 8601 timestamp stamped onto the event envelope.
    pub timestamp: String,
    /// Whether `Bash`/`bash` capture is enabled (`SCRYRS_DEBUG` non-empty).
    ///
    /// Resolved once by the hook command via [`debug_enabled`] and threaded in
    /// here so adapters stay pure and unit-testable without touching the
    /// process environment.
    pub bash_debug: bool,
}

/// Error returned when an adapter cannot translate a harness event.
///
/// The hook command treats every variant as fail-open: it logs a warning and
/// still exits 0, never blocking the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdapterError {
    /// The raw input was not valid JSON or did not match the expected shape.
    Parse(String),
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::Parse(reason) => write!(f, "parse error: {reason}"),
        }
    }
}

impl std::error::Error for AdapterError {}

/// Translate a harness's native event JSON into zero or one canonical
/// [`TraceEvent`].
pub trait HarnessAdapter {
    /// Parse `raw` and return `Some(event)` for a tracked tool, `None` for an
    /// untracked tool (pass-through), or `Err` when the input cannot be parsed.
    fn translate(&self, raw: &str, ctx: &HookContext) -> Result<Option<TraceEvent>, AdapterError>;
}

/// Return the adapter for a supported harness name, or `None` if unknown.
///
/// Unknown harnesses are part of the fail-open contract — the caller exits 0.
#[must_use]
pub fn adapter_for(harness: &str) -> Option<Box<dyn HarnessAdapter>> {
    match harness {
        "claude-code" => Some(Box::new(ClaudeCodeAdapter)),
        "pi" => Some(Box::new(PiAdapter)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Shared helpers (single source of truth, formerly duplicated in JS + record.rs)
// ---------------------------------------------------------------------------

/// Collapse embedded newlines so a serialized event occupies one physical line.
///
/// Kept identical to the historical JS/Rust implementations: CRLF and bare
/// CR/LF are all replaced with a visible `⏎` marker.
#[must_use]
pub fn collapse_newlines(value: &str) -> String {
    value.replace("\r\n", " ⏎ ").replace(['\n', '\r'], " ⏎ ")
}

/// Whether `SCRYRS_DEBUG` is set to a non-empty value (gates `Bash` capture).
#[must_use]
pub fn debug_enabled() -> bool {
    std::env::var("SCRYRS_DEBUG")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

/// Generate an ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`) timestamp for *now*.
#[must_use]
pub fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;
    let (year, month, day) = days_to_ymd((secs / 86400) as i64);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Convert days since the Unix epoch to `(year, month, day)`.
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Build a canonical [`TraceEvent`] envelope from a context, type, tool name,
/// payload, and outcome. The single envelope-construction site for all adapters.
pub(crate) fn build_event(
    ctx: &HookContext,
    event_type: TraceEventType,
    tool_name: &str,
    payload: TraceEventPayload,
    outcome: Outcome,
) -> TraceEvent {
    TraceEvent {
        schema_version: SCHEMA_VERSION.to_string(),
        timestamp: ctx.timestamp.clone(),
        session_id: ctx.session_id.clone(),
        event_type,
        tool_name: Some(tool_name.to_string()),
        payload,
        outcome,
    }
}

#[cfg(test)]
pub(crate) fn test_ctx(session_id: &str) -> HookContext {
    HookContext {
        session_id: session_id.to_string(),
        store_path: PathBuf::from(".scryrs/scryrs.db"),
        timestamp: "2026-06-24T00:00:00Z".to_string(),
        bash_debug: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_newlines_replaces_all_line_breaks() {
        assert_eq!(collapse_newlines("a\nb"), "a ⏎ b");
        assert_eq!(collapse_newlines("a\r\nb"), "a ⏎ b");
        assert_eq!(collapse_newlines("a\rb"), "a ⏎ b");
        assert_eq!(collapse_newlines("plain"), "plain");
    }

    #[test]
    fn now_iso8601_has_expected_shape() {
        let ts = now_iso8601();
        assert_eq!(ts.len(), 20, "expected YYYY-MM-DDTHH:MM:SSZ, got {ts}");
        assert!(ts.ends_with('Z'));
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn adapter_for_known_harnesses() {
        assert!(adapter_for("claude-code").is_some());
        assert!(adapter_for("pi").is_some());
    }

    #[test]
    fn adapter_for_unknown_harness_is_none() {
        assert!(adapter_for("bogus").is_none());
        assert!(adapter_for("").is_none());
    }
}
