//! Pi `tool_result` / `session_start` adapter.
//!
//! Pi loads an in-process extension and fires `tool_result` *after* execution,
//! so the adapter reflects the real outcome (`isError` → [`Outcome::Failure`]).
//! Pi tool names are matched as lowercase. The thin Pi shim forwards the raw
//! event JSON (with an injected `session_id`); a `tool_result` event carries a
//! `toolName`, while a `session_start` event does not.

use scryrs_types::{
    CommandExecutedPayload, EditMadePayload, FailedLookupPayload, FileOpenedPayload, Outcome,
    SearchRunPayload, SessionStartPayload, SymbolInspectedPayload, TraceEvent, TraceEventPayload,
    TraceEventType,
};
use serde_json::Value;

use crate::{AdapterError, HarnessAdapter, HookContext, build_event, collapse_newlines};

/// Adapter for Pi `tool_result` and `session_start` events.
pub struct PiAdapter;

/// Extract a string field from Pi's `input` object, returning `"unknown"` when
/// absent (matching the historical Pi shim's `mappedInputValue` fallback).
fn input_field(input: &Value, key: &str) -> String {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(collapse_newlines)
        .unwrap_or_else(|| "unknown".to_string())
}

impl HarnessAdapter for PiAdapter {
    fn translate(&self, raw: &str, ctx: &HookContext) -> Result<Option<TraceEvent>, AdapterError> {
        let root: Value = serde_json::from_str(raw)
            .map_err(|e| AdapterError::Parse(format!("invalid Pi event JSON: {e}")))?;

        // A tool_result event carries a `toolName`; a session_start does not.
        let tool_name = match root.get("toolName").and_then(Value::as_str) {
            Some(name) => name,
            None => {
                // Lifecycle event → SessionStart (no tool_name on the envelope).
                return Ok(Some(TraceEvent {
                    schema_version: scryrs_types::SCHEMA_VERSION.to_string(),
                    timestamp: ctx.timestamp.clone(),
                    session_id: ctx.session_id.clone(),
                    event_type: TraceEventType::SessionStart,
                    tool_name: None,
                    payload: TraceEventPayload::SessionStart(SessionStartPayload),
                    outcome: Outcome::Success,
                }));
            }
        };

        let is_error = root
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let input = root.get("input").cloned().unwrap_or(Value::Null);

        let (event_type, payload) = match tool_name {
            "read" => (
                TraceEventType::FileOpened,
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: input_field(&input, "path"),
                }),
            ),
            "ast_grep_search" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: input_field(&input, "query"),
                }),
            ),
            "edit" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: input_field(&input, "path"),
                }),
            ),
            "write" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: input_field(&input, "path"),
                }),
            ),
            "lsp_navigation" => {
                let symbol = input_field(&input, "symbol");
                if is_error {
                    (
                        TraceEventType::FailedLookup,
                        TraceEventPayload::FailedLookup(FailedLookupPayload { subject: symbol }),
                    )
                } else {
                    (
                        TraceEventType::SymbolInspected,
                        TraceEventPayload::SymbolInspected(SymbolInspectedPayload { name: symbol }),
                    )
                }
            }
            // Bash is captured only when SCRYRS_DEBUG is non-empty.
            "bash" => {
                if !ctx.bash_debug {
                    return Ok(None);
                }
                (
                    TraceEventType::CommandExecuted,
                    TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                        command: input_field(&input, "command"),
                    }),
                )
            }
            // Untracked tool → pass-through.
            _ => return Ok(None),
        };

        // tool_result fires post-execution: reflect the real outcome.
        let outcome = if is_error {
            Outcome::Failure {
                reason: Some("Tool execution error".to_string()),
            }
        } else {
            Outcome::Success
        };

        Ok(Some(build_event(
            ctx, event_type, tool_name, payload, outcome,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ctx;

    fn tool_result(tool: &str, input: serde_json::Value, is_error: bool) -> String {
        serde_json::json!({
            "session_id": "s1",
            "toolName": tool,
            "input": input,
            "isError": is_error,
        })
        .to_string()
    }

    fn translate(raw: &str) -> Option<TraceEvent> {
        PiAdapter
            .translate(raw, &test_ctx("s1"))
            .unwrap_or_else(|e| panic!("translate failed: {e}"))
    }

    #[test]
    fn read_maps_to_file_opened() {
        let ev = translate(&tool_result(
            "read",
            serde_json::json!({"path": "src/a.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FileOpened);
        assert_eq!(ev.tool_name.as_deref(), Some("read"));
        assert_eq!(ev.subject(), Some("src/a.rs"));
        assert_eq!(ev.outcome, Outcome::Success);
    }

    #[test]
    fn ast_grep_search_maps_to_search_run() {
        let ev = translate(&tool_result(
            "ast_grep_search",
            serde_json::json!({"query": "fn main"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::SearchRun);
        assert_eq!(ev.subject(), Some("fn main"));
    }

    #[test]
    fn edit_and_write_map_to_edit_made() {
        let edit = translate(&tool_result(
            "edit",
            serde_json::json!({"path": "a.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(edit.event_type, TraceEventType::EditMade);
        assert_eq!(edit.subject(), Some("a.rs"));
        let write = translate(&tool_result(
            "write",
            serde_json::json!({"path": "b.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(write.event_type, TraceEventType::EditMade);
        assert_eq!(write.subject(), Some("b.rs"));
    }

    #[test]
    fn lsp_navigation_success_maps_to_symbol_inspected() {
        let ev = translate(&tool_result(
            "lsp_navigation",
            serde_json::json!({"symbol": "Dispatcher"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::SymbolInspected);
        assert_eq!(ev.subject(), Some("Dispatcher"));
        assert_eq!(ev.outcome, Outcome::Success);
    }

    #[test]
    fn lsp_navigation_error_maps_to_failed_lookup() {
        let ev = translate(&tool_result(
            "lsp_navigation",
            serde_json::json!({"symbol": "Missing"}),
            true,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FailedLookup);
        assert_eq!(ev.subject(), Some("Missing"));
        assert!(matches!(ev.outcome, Outcome::Failure { .. }));
    }

    #[test]
    fn is_error_sets_failure_outcome() {
        let ev = translate(&tool_result(
            "read",
            serde_json::json!({"path": "a.rs"}),
            true,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FileOpened);
        assert!(matches!(ev.outcome, Outcome::Failure { .. }));
    }

    #[test]
    fn untracked_tool_passes_through() {
        assert!(translate(&tool_result("todo", serde_json::json!({}), false)).is_none());
    }

    #[test]
    fn session_start_event_maps_to_session_start() {
        let raw = serde_json::json!({"session_id": "s1", "reason": "startup"}).to_string();
        let ev = translate(&raw).unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::SessionStart);
        assert!(ev.tool_name.is_none());
        assert_eq!(ev.session_id, "s1");
        assert!(ev.subject().is_none());
    }

    #[test]
    fn bash_is_debug_gated() {
        // bash_debug=false (default) → dropped.
        assert!(
            translate(&tool_result(
                "bash",
                serde_json::json!({"command": "ls"}),
                false
            ))
            .is_none()
        );

        // bash_debug=true → CommandExecuted.
        let mut ctx = test_ctx("s1");
        ctx.bash_debug = true;
        let ev = PiAdapter
            .translate(
                &tool_result("bash", serde_json::json!({"command": "cargo test"}), false),
                &ctx,
            )
            .unwrap_or_else(|e| panic!("translate: {e}"))
            .unwrap_or_else(|| panic!("expected event when debug on"));
        assert_eq!(ev.event_type, TraceEventType::CommandExecuted);
        assert_eq!(ev.subject(), Some("cargo test"));
    }

    #[test]
    fn missing_input_field_yields_unknown() {
        let ev = translate(&tool_result("read", serde_json::json!({}), false))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("unknown"));
    }

    #[test]
    fn malformed_json_is_parse_error() {
        let err = PiAdapter
            .translate("{not json", &test_ctx("s1"))
            .err()
            .unwrap_or_else(|| panic!("must error"));
        assert!(matches!(err, AdapterError::Parse(_)));
    }
}
