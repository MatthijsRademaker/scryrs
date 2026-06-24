//! Claude Code `PreToolUse` adapter.
//!
//! Claude Code spawns a `command` hook and pipes the `PreToolUse` event JSON on
//! stdin. The payload carries `session_id`, `cwd`, `tool_name` (PascalCase), and
//! `tool_input`. Because the hook fires *before* execution, the outcome of every
//! emitted event is unconditionally [`Outcome::Success`].

use scryrs_types::{
    CommandExecutedPayload, DocRetrievedPayload, EditMadePayload, FileOpenedPayload, Outcome,
    SearchRunPayload, TraceEvent, TraceEventPayload, TraceEventType,
};
use serde_json::Value;

use crate::{AdapterError, HarnessAdapter, HookContext, build_event, collapse_newlines};

/// Adapter for Claude Code `PreToolUse` events.
pub struct ClaudeCodeAdapter;

/// Extract a string field from `tool_input`, returning `""` when absent.
fn field(tool_input: &Value, key: &str) -> String {
    tool_input
        .get(key)
        .and_then(Value::as_str)
        .map(collapse_newlines)
        .unwrap_or_default()
}

/// Extract the first present string field among `keys`, returning `""` when none.
fn first_field(tool_input: &Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(v) = tool_input.get(*key).and_then(Value::as_str) {
            return collapse_newlines(v);
        }
    }
    String::new()
}

impl HarnessAdapter for ClaudeCodeAdapter {
    fn translate(&self, raw: &str, ctx: &HookContext) -> Result<Option<TraceEvent>, AdapterError> {
        let root: Value = serde_json::from_str(raw)
            .map_err(|e| AdapterError::Parse(format!("invalid PreToolUse JSON: {e}")))?;

        let tool_name = match root.get("tool_name").and_then(Value::as_str) {
            Some(name) => name,
            // No tool name → nothing to translate (pass-through).
            None => return Ok(None),
        };

        let tool_input = root.get("tool_input").cloned().unwrap_or(Value::Null);

        // Tool names are matched as documented PascalCase — never lowercased.
        let (event_type, payload) = match tool_name {
            "Read" => (
                TraceEventType::FileOpened,
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: field(&tool_input, "file_path"),
                }),
            ),
            "Grep" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: field(&tool_input, "pattern"),
                }),
            ),
            "Glob" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: field(&tool_input, "pattern"),
                }),
            ),
            "WebSearch" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: first_field(&tool_input, &["query", "searchTerm"]),
                }),
            ),
            "Edit" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: field(&tool_input, "file_path"),
                }),
            ),
            "Write" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: field(&tool_input, "file_path"),
                }),
            ),
            "NotebookEdit" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: first_field(&tool_input, &["notebook_path", "file_path"]),
                }),
            ),
            "WebFetch" => (
                TraceEventType::DocRetrieved,
                TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: first_field(&tool_input, &["url", "website"]),
                }),
            ),
            // Bash is captured only when SCRYRS_DEBUG is non-empty.
            "Bash" => {
                if !ctx.bash_debug {
                    return Ok(None);
                }
                (
                    TraceEventType::CommandExecuted,
                    TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                        command: field(&tool_input, "command"),
                    }),
                )
            }
            // Untracked tool → pass-through.
            _ => return Ok(None),
        };

        Ok(Some(build_event(
            ctx,
            event_type,
            tool_name,
            payload,
            // PreToolUse fires pre-execution: outcome is always Success.
            Outcome::Success,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ctx;

    fn payload(tool_name: &str, input: serde_json::Value) -> String {
        serde_json::json!({
            "session_id": "s1",
            "cwd": "/proj",
            "tool_name": tool_name,
            "tool_input": input,
        })
        .to_string()
    }

    fn translate(raw: &str) -> Option<TraceEvent> {
        ClaudeCodeAdapter
            .translate(raw, &test_ctx("s1"))
            .unwrap_or_else(|e| panic!("translate failed: {e}"))
    }

    #[test]
    fn read_maps_to_file_opened() {
        let ev = translate(&payload(
            "Read",
            serde_json::json!({"file_path": "src/a.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FileOpened);
        assert_eq!(ev.tool_name.as_deref(), Some("Read"));
        assert_eq!(ev.subject(), Some("src/a.rs"));
        assert_eq!(ev.outcome, Outcome::Success);
        assert_eq!(ev.session_id, "s1");
    }

    #[test]
    fn grep_and_glob_map_to_search_run() {
        let grep = translate(&payload("Grep", serde_json::json!({"pattern": "foo"})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(grep.event_type, TraceEventType::SearchRun);
        assert_eq!(grep.subject(), Some("foo"));
        let glob = translate(&payload("Glob", serde_json::json!({"pattern": "*.rs"})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(glob.event_type, TraceEventType::SearchRun);
        assert_eq!(glob.subject(), Some("*.rs"));
    }

    #[test]
    fn websearch_maps_to_search_run_pascalcase() {
        // The name is matched as PascalCase, not lowercased.
        let ev = translate(&payload(
            "WebSearch",
            serde_json::json!({"query": "rust traits"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::SearchRun);
        assert_eq!(ev.tool_name.as_deref(), Some("WebSearch"));
        assert_eq!(ev.subject(), Some("rust traits"));
    }

    #[test]
    fn edit_write_notebookedit_map_to_edit_made() {
        for (tool, input, expected) in [
            ("Edit", serde_json::json!({"file_path": "a.rs"}), "a.rs"),
            ("Write", serde_json::json!({"file_path": "b.rs"}), "b.rs"),
            (
                "NotebookEdit",
                serde_json::json!({"notebook_path": "n.ipynb"}),
                "n.ipynb",
            ),
        ] {
            let ev = translate(&payload(tool, input)).unwrap_or_else(|| panic!("expected event"));
            assert_eq!(ev.event_type, TraceEventType::EditMade);
            assert_eq!(ev.subject(), Some(expected));
        }
    }

    #[test]
    fn webfetch_maps_to_doc_retrieved() {
        let ev = translate(&payload(
            "WebFetch",
            serde_json::json!({"url": "https://example.com/api"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::DocRetrieved);
        assert_eq!(ev.subject(), Some("https://example.com/api"));
    }

    #[test]
    fn untracked_tool_passes_through() {
        assert!(translate(&payload("TodoWrite", serde_json::json!({}))).is_none());
        assert!(translate(&payload("Task", serde_json::json!({}))).is_none());
    }

    #[test]
    fn bash_is_debug_gated() {
        // bash_debug=false (default) → dropped.
        assert!(
            translate(&payload("Bash", serde_json::json!({"command": "ls"}))).is_none(),
            "Bash must be dropped when bash_debug is false"
        );

        // bash_debug=true → CommandExecuted.
        let mut ctx = test_ctx("s1");
        ctx.bash_debug = true;
        let ev = ClaudeCodeAdapter
            .translate(
                &payload("Bash", serde_json::json!({"command": "cargo build"})),
                &ctx,
            )
            .unwrap_or_else(|e| panic!("translate: {e}"))
            .unwrap_or_else(|| panic!("expected event when debug on"));
        assert_eq!(ev.event_type, TraceEventType::CommandExecuted);
        assert_eq!(ev.subject(), Some("cargo build"));
    }

    #[test]
    fn missing_tool_name_passes_through() {
        let raw = serde_json::json!({"tool_input": {"file_path": "x"}}).to_string();
        assert!(translate(&raw).is_none());
    }

    #[test]
    fn malformed_json_is_parse_error() {
        let err = ClaudeCodeAdapter
            .translate("not json", &test_ctx("s1"))
            .err()
            .unwrap_or_else(|| panic!("must error"));
        assert!(matches!(err, AdapterError::Parse(_)));
    }

    #[test]
    fn newlines_in_payload_are_collapsed() {
        let ev = translate(&payload("Read", serde_json::json!({"file_path": "a\nb"})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("a ⏎ b"));
    }

    #[test]
    fn missing_field_yields_empty_subject() {
        let ev = translate(&payload("Read", serde_json::json!({})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some(""));
    }
}
