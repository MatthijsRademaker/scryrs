//! Claude Code `PostToolUse` / `PostToolUseFailure` adapter.
//!
//! Claude Code spawns a `command` hook and pipes the event JSON on stdin. The
//! payload carries `session_id`, `cwd`, `hook_event_name`, `tool_name`
//! (PascalCase), and `tool_input`; `PostToolUse` additionally carries
//! `tool_response`.
//!
//! # Why two events
//!
//! `PostToolUse` fires **only after a tool call succeeds**. Failures arrive on a
//! separate `PostToolUseFailure` event. Registering on `PostToolUse` alone would
//! reproduce the `PreToolUse` defect this adapter replaced — every event would
//! carry [`Outcome::Success`] because failures would never reach the hook. So
//! the outcome is derived from `hook_event_name`, not inferred from response
//! contents.
//!
//! # Non-interference
//!
//! `PostToolUse` supports `decision: "block"` and `updatedToolOutput`, which
//! would replace the tool's result before Claude sees it. This adapter emits
//! neither: the hook command writes nothing to stdout and always exits 0, so
//! scryrs stays a pure observer.

use scryrs_types::{
    CommandExecutedPayload, DocRetrievedPayload, EditMadePayload, FailedLookupPayload,
    FileOpenedPayload, Outcome, SearchRunPayload, TraceEvent, TraceEventPayload, TraceEventType,
};
use serde_json::Value;

use crate::{
    AdapterError, HarnessAdapter, HookContext, build_event, key_field, normalize_path_subject,
};

/// Harness identifier used in missing-field diagnostics.
const HARNESS: &str = "claude-code";

/// The `hook_event_name` Claude Code sets when a tool call failed.
const FAILURE_EVENT: &str = "PostToolUseFailure";

/// Adapter for Claude Code `PostToolUse` and `PostToolUseFailure` events.
pub struct ClaudeCodeAdapter;

/// Extract the first present required string field among `keys`.
///
/// Used where Claude Code names the same subject differently across tools
/// (`notebook_path` vs `file_path`). Absent from all candidates is a contract
/// violation, reported against the first (canonical) key.
fn first_key_field(
    tool_input: &Value,
    keys: &[&'static str],
    tool: &str,
) -> Result<String, AdapterError> {
    for key in keys {
        if let Ok(value) = key_field(tool_input, key, HARNESS, tool) {
            return Ok(value);
        }
    }
    Err(AdapterError::MissingField {
        harness: HARNESS,
        tool: tool.to_string(),
        key: keys[0],
    })
}

impl HarnessAdapter for ClaudeCodeAdapter {
    fn translate(&self, raw: &str, ctx: &HookContext) -> Result<Option<TraceEvent>, AdapterError> {
        let root: Value = serde_json::from_str(raw)
            .map_err(|e| AdapterError::Parse(format!("invalid PostToolUse JSON: {e}")))?;

        let tool_name = match root.get("tool_name").and_then(Value::as_str) {
            Some(name) => name,
            // No tool name → nothing to translate (pass-through).
            None => return Ok(None),
        };

        let tool_input = root.get("tool_input").cloned().unwrap_or(Value::Null);

        // The outcome comes from which event fired, never from response
        // contents: PostToolUse is success-only, PostToolUseFailure is failure.
        let is_error = root
            .get("hook_event_name")
            .and_then(Value::as_str)
            .is_some_and(|name| name == FAILURE_EVENT);

        // A path subject normalized against the repository root, applied
        // identically to successes and failures.
        let path_subject = |keys: &[&'static str]| -> Result<String, AdapterError> {
            let raw = first_key_field(&tool_input, keys, tool_name)?;
            Ok(normalize_path_subject(&raw, &ctx.repo_root).into_subject())
        };

        // Tool names are matched as documented PascalCase — never lowercased.
        let (event_type, payload) = match tool_name {
            // A failed read is a failed lookup, not an opened file — matching
            // the Pi adapter's behavior for the same situation.
            "Read" => {
                let path = path_subject(&["file_path"])?;
                if is_error {
                    (
                        TraceEventType::FailedLookup,
                        TraceEventPayload::FailedLookup(FailedLookupPayload { subject: path }),
                    )
                } else {
                    (
                        TraceEventType::FileOpened,
                        TraceEventPayload::FileOpened(FileOpenedPayload { path }),
                    )
                }
            }
            "Grep" | "Glob" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: key_field(&tool_input, "pattern", HARNESS, tool_name)?,
                }),
            ),
            "WebSearch" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: first_key_field(&tool_input, &["query", "searchTerm"], tool_name)?,
                }),
            ),
            "Edit" | "Write" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: path_subject(&["file_path"])?,
                }),
            ),
            "NotebookEdit" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: path_subject(&["notebook_path", "file_path"])?,
                }),
            ),
            "WebFetch" => (
                TraceEventType::DocRetrieved,
                TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: first_key_field(&tool_input, &["url", "website"], tool_name)?,
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
                        command: key_field(&tool_input, "command", HARNESS, tool_name)?,
                    }),
                )
            }
            // Untracked tool → pass-through.
            _ => return Ok(None),
        };

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

    /// A `PostToolUse` payload (success path).
    fn payload(tool_name: &str, input: serde_json::Value) -> String {
        hook_payload("PostToolUse", tool_name, input)
    }

    /// A `PostToolUseFailure` payload (failure path).
    fn failure_payload(tool_name: &str, input: serde_json::Value) -> String {
        hook_payload("PostToolUseFailure", tool_name, input)
    }

    fn hook_payload(event: &str, tool_name: &str, input: serde_json::Value) -> String {
        serde_json::json!({
            "session_id": "s1",
            "cwd": "/repo",
            "hook_event_name": event,
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

    /// Replaces `missing_field_yields_empty_subject`. An empty-string subject is
    /// no more groupable than a `"unknown"` placeholder — both are fabrications.
    #[test]
    fn missing_key_field_errors_instead_of_recording_an_empty_subject() {
        let err = ClaudeCodeAdapter
            .translate(&payload("Read", serde_json::json!({})), &test_ctx("s1"))
            .err()
            .unwrap_or_else(|| panic!("must error on missing file_path"));

        match err {
            AdapterError::MissingField { harness, tool, key } => {
                assert_eq!(harness, "claude-code");
                assert_eq!(tool, "Read");
                assert_eq!(key, "file_path");
            }
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    // --- PostToolUse vs PostToolUseFailure ---

    /// `PostToolUse` is success-only; `PostToolUseFailure` carries the failures.
    /// Registering on `PostToolUse` alone would leave every outcome `Success`,
    /// which is the `PreToolUse` defect this adapter replaced.
    #[test]
    fn outcome_is_derived_from_the_hook_event_name() {
        let ok = translate(&payload("Grep", serde_json::json!({"pattern": "foo"})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ok.outcome, Outcome::Success);

        let failed = translate(&failure_payload(
            "Grep",
            serde_json::json!({"pattern": "foo"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert!(matches!(failed.outcome, Outcome::Failure { .. }));
    }

    #[test]
    fn failed_read_maps_to_failed_lookup() {
        let ev = translate(&failure_payload(
            "Read",
            serde_json::json!({"file_path": "src/missing.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FailedLookup);
        assert_eq!(ev.subject(), Some("src/missing.rs"));
        assert!(matches!(ev.outcome, Outcome::Failure { .. }));
    }

    /// Matches the Pi adapter: one grouping key for a file's successes and
    /// failures, however the agent spelled the path.
    #[test]
    fn failed_and_successful_read_share_a_grouping_key() {
        let ok = translate(&payload(
            "Read",
            serde_json::json!({"file_path": "/repo/src/a.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        let failed = translate(&failure_payload(
            "Read",
            serde_json::json!({"file_path": "src/a.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));

        assert_eq!(ok.subject(), failed.subject());
        assert_eq!(ok.subject_kind(), failed.subject_kind());
        assert_eq!(ok.subject_kind(), Some("file"));
    }

    // --- Path normalization ---

    #[test]
    fn absolute_and_relative_paths_normalize_to_one_subject() {
        let absolute = translate(&payload(
            "Read",
            serde_json::json!({"file_path": "/repo/crates/a.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        let relative = translate(&payload(
            "Read",
            serde_json::json!({"file_path": "crates/a.rs"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));

        assert_eq!(absolute.subject(), Some("crates/a.rs"));
        assert_eq!(relative.subject(), Some("crates/a.rs"));
    }

    #[test]
    fn repository_external_path_is_preserved_and_marked() {
        let ev = translate(&payload(
            "Read",
            serde_json::json!({"file_path": "/home/user/.claude/CLAUDE.md"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("/home/user/.claude/CLAUDE.md"));
        assert_eq!(ev.subject_kind(), Some("external_file"));
    }

    /// After the reclassification, a failure never lands on `FileOpened`.
    #[test]
    fn file_opened_never_carries_a_failure_outcome() {
        for raw in [
            payload("Read", serde_json::json!({"file_path": "src/a.rs"})),
            failure_payload("Read", serde_json::json!({"file_path": "src/a.rs"})),
        ] {
            let ev = translate(&raw).unwrap_or_else(|| panic!("expected event"));
            if ev.event_type == TraceEventType::FileOpened {
                assert_eq!(
                    ev.outcome,
                    Outcome::Success,
                    "FileOpened must always be a success"
                );
            }
        }
    }

    /// Search patterns and URLs are not paths and must not be normalized.
    #[test]
    fn non_path_subjects_are_not_normalized() {
        let grep = translate(&payload("Grep", serde_json::json!({"pattern": "../src"})))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(grep.subject(), Some("../src"));

        let fetch = translate(&payload(
            "WebFetch",
            serde_json::json!({"url": "https://example.com/a/../b"}),
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(fetch.subject(), Some("https://example.com/a/../b"));
    }
}
