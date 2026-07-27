//! Pi `tool_result` / `session_start` adapter.
//!
//! Pi loads an in-process extension and fires `tool_result` *after* execution,
//! so the adapter reflects the real outcome (`isError` → [`Outcome::Failure`]).
//! Pi tool names are matched as lowercase. The thin Pi shim forwards the raw
//! event JSON (with an injected `session_id` and `cwd`); a `tool_result` event
//! carries a `toolName`, while a `session_start` event does not.
//!
//! # Tool coverage
//!
//! Pi's core tool set is the closed union `read | bash | edit | write | grep |
//! find | ls` (verified against `dist/core/tools/index.d.ts` in Pi 0.81.1).
//! `grep` and `find` both take a required `pattern` and are the source of
//! [`TraceEventType::SearchRun`] evidence.
//!
//! `ast_grep_search` is **not** a Pi core tool — it comes from the third-party
//! `pi-lens` extension. It is mapped because it is a real search surface in
//! practice, and its key field is `pattern` (present in 324/324 observed
//! payloads; `query`, which this adapter previously read, never appears).
//!
//! `lsp_navigation` (also `pi-lens`) is deliberately **not** mapped: it is an
//! operation dispatcher with no single key field. Across 126 observed payloads
//! `operation` was always present but `symbol` appeared in only 5%, and a
//! symbol name (`query`) in 50%. There is no honest single subject to record.
//!
//! `ls` is not mapped: its `path` is optional and a directory listing is not a
//! hotspot subject in any of the canonical event families.

use scryrs_types::{
    CommandExecutedPayload, EditMadePayload, FailedLookupPayload, FileOpenedPayload, Outcome,
    SearchRunPayload, SessionStartPayload, TraceEvent, TraceEventPayload, TraceEventType,
};
use serde_json::Value;

use crate::{
    AdapterError, HarnessAdapter, HookContext, build_event, key_field, normalize_path_subject,
};

/// Harness identifier used in missing-field diagnostics.
const HARNESS: &str = "pi";

/// Adapter for Pi `tool_result` and `session_start` events.
pub struct PiAdapter;

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

        // A path subject normalized against the repository root. Applied
        // identically to successes and failures so a file has one subject
        // spelling regardless of how the agent addressed it.
        let path_subject = |key: &'static str| -> Result<String, AdapterError> {
            let raw = key_field(&input, key, HARNESS, tool_name)?;
            Ok(normalize_path_subject(&raw, &ctx.repo_root).into_subject())
        };

        let (event_type, payload) = match tool_name {
            // A failed read is a failed lookup, not an opened file: the agent
            // addressed a path that does not exist, which is the single most
            // routing-relevant signal available.
            "read" => {
                let path = path_subject("path")?;
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
            // Pi core search tools. Both take a required `pattern`.
            "grep" | "find" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: key_field(&input, "pattern", HARNESS, tool_name)?,
                }),
            ),
            // pi-lens extension search tool. Key field is `pattern`, not `query`.
            "ast_grep_search" => (
                TraceEventType::SearchRun,
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: key_field(&input, "pattern", HARNESS, tool_name)?,
                }),
            ),
            "edit" | "write" => (
                TraceEventType::EditMade,
                TraceEventPayload::EditMade(EditMadePayload {
                    target: path_subject("path")?,
                }),
            ),
            // Bash is captured only when SCRYRS_DEBUG is non-empty.
            "bash" => {
                if !ctx.bash_debug {
                    return Ok(None);
                }
                (
                    TraceEventType::CommandExecuted,
                    TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                        command: key_field(&input, "command", HARNESS, tool_name)?,
                    }),
                )
            }
            // Untracked tool → pass-through. Covers `ls` and `lsp_navigation`.
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
    fn grep_and_find_map_to_search_run_on_pattern() {
        for tool in ["grep", "find"] {
            let ev = translate(&tool_result(
                tool,
                serde_json::json!({"pattern": "fn main", "path": "crates/"}),
                false,
            ))
            .unwrap_or_else(|| panic!("expected event for {tool}"));
            assert_eq!(ev.event_type, TraceEventType::SearchRun);
            assert_eq!(ev.tool_name.as_deref(), Some(tool));
            assert_eq!(ev.subject(), Some("fn main"));
            assert_eq!(ev.subject_kind(), Some("search"));
        }
    }

    /// Regression: the adapter previously read `query`, which never appears in a
    /// real `ast_grep_search` payload (0/324 observed). `pattern` is the key.
    #[test]
    fn ast_grep_search_reads_pattern_not_query() {
        let ev = translate(&tool_result(
            "ast_grep_search",
            serde_json::json!({"pattern": "console.log($MSG)", "lang": "typescript"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::SearchRun);
        assert_eq!(ev.subject(), Some("console.log($MSG)"));

        // A payload carrying only the old key is a contract violation now.
        let err = PiAdapter
            .translate(
                &tool_result("ast_grep_search", serde_json::json!({"query": "x"}), false),
                &test_ctx("s1"),
            )
            .err()
            .unwrap_or_else(|| panic!("must error on missing pattern"));
        assert!(matches!(
            err,
            AdapterError::MissingField { key: "pattern", .. }
        ));
    }

    /// `lsp_navigation` is an operation dispatcher with no single key field
    /// (`symbol` present in 7/126 real payloads), so it is not mapped.
    #[test]
    fn lsp_navigation_is_not_mapped() {
        assert!(
            translate(&tool_result(
                "lsp_navigation",
                serde_json::json!({"operation": "workspaceSymbol", "query": "Dispatcher"}),
                false,
            ))
            .is_none()
        );
    }

    /// `ls` has an optional `path` and is not a canonical hotspot subject.
    #[test]
    fn ls_is_not_mapped() {
        assert!(translate(&tool_result("ls", serde_json::json!({}), false)).is_none());
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

    /// A failed read is a failed lookup, not an opened file with a sad outcome.
    #[test]
    fn failed_read_maps_to_failed_lookup() {
        let ev = translate(&tool_result(
            "read",
            serde_json::json!({"path": "src/missing.rs"}),
            true,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.event_type, TraceEventType::FailedLookup);
        assert_eq!(ev.subject(), Some("src/missing.rs"));
        assert!(matches!(ev.outcome, Outcome::Failure { .. }));
    }

    /// The whole point of the reclassification: a file's successful and failed
    /// accesses share one `(subject_kind, subject)` grouping key.
    #[test]
    fn failed_and_successful_read_share_a_grouping_key() {
        let ok = translate(&tool_result(
            "read",
            serde_json::json!({"path": "/repo/src/a.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        let failed = translate(&tool_result(
            "read",
            serde_json::json!({"path": "src/a.rs"}),
            true,
        ))
        .unwrap_or_else(|| panic!("expected event"));

        assert_eq!(ok.subject(), failed.subject());
        assert_eq!(ok.subject_kind(), failed.subject_kind());
        assert_eq!(ok.subject_kind(), Some("file"));
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

    /// Replaces `missing_input_field_yields_unknown`, which enshrined the
    /// placeholder that hid a 100%-failure-rate field-mapping bug.
    #[test]
    fn missing_key_field_errors_instead_of_recording_a_placeholder() {
        let err = PiAdapter
            .translate(
                &tool_result("read", serde_json::json!({}), false),
                &test_ctx("s1"),
            )
            .err()
            .unwrap_or_else(|| panic!("must error on missing path"));

        match err {
            AdapterError::MissingField { harness, tool, key } => {
                assert_eq!(harness, "pi");
                assert_eq!(tool, "read");
                assert_eq!(key, "path");
            }
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_whitespace_key_fields_are_treated_as_absent() {
        for value in ["", "   "] {
            assert!(
                PiAdapter
                    .translate(
                        &tool_result("read", serde_json::json!({"path": value}), false),
                        &test_ctx("s1"),
                    )
                    .is_err(),
                "empty subject must not be recorded (value {value:?})"
            );
        }
    }

    #[test]
    fn absolute_and_relative_paths_normalize_to_one_subject() {
        let absolute = translate(&tool_result(
            "read",
            serde_json::json!({"path": "/repo/crates/a.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        let relative = translate(&tool_result(
            "read",
            serde_json::json!({"path": "crates/a.rs"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));

        assert_eq!(absolute.subject(), Some("crates/a.rs"));
        assert_eq!(relative.subject(), Some("crates/a.rs"));
    }

    #[test]
    fn redundant_path_segments_are_normalized_away() {
        for raw in [
            "./crates/a.rs",
            "crates/../crates/a.rs",
            "/repo/./crates/a.rs",
        ] {
            let ev = translate(&tool_result(
                "read",
                serde_json::json!({"path": raw}),
                false,
            ))
            .unwrap_or_else(|| panic!("expected event for {raw}"));
            assert_eq!(ev.subject(), Some("crates/a.rs"), "raw input {raw}");
        }
    }

    #[test]
    fn repository_external_path_is_preserved_and_marked() {
        let ev = translate(&tool_result(
            "read",
            serde_json::json!({"path": "/home/user/.claude/CLAUDE.md"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("/home/user/.claude/CLAUDE.md"));
        assert_eq!(ev.subject_kind(), Some("external_file"));
    }

    /// Search queries are not paths and must never be path-normalized.
    #[test]
    fn search_queries_are_not_path_normalized() {
        let ev = translate(&tool_result(
            "grep",
            serde_json::json!({"pattern": "../src"}),
            false,
        ))
        .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("../src"));
    }

    /// Bash commands are not paths either.
    #[test]
    fn commands_are_not_path_normalized() {
        let mut ctx = test_ctx("s1");
        ctx.bash_debug = true;
        let ev = PiAdapter
            .translate(
                &tool_result("bash", serde_json::json!({"command": "cd .. && ls"}), false),
                &ctx,
            )
            .unwrap_or_else(|e| panic!("translate: {e}"))
            .unwrap_or_else(|| panic!("expected event"));
        assert_eq!(ev.subject(), Some("cd .. && ls"));
        assert_eq!(ev.subject_kind(), Some("command"));
    }

    /// After the reclassification, a failure never lands on `FileOpened` — it
    /// becomes `FailedLookup`. Nothing downstream may rely on that combination.
    #[test]
    fn file_opened_never_carries_a_failure_outcome() {
        for is_error in [false, true] {
            let ev = translate(&tool_result(
                "read",
                serde_json::json!({"path": "src/a.rs"}),
                is_error,
            ))
            .unwrap_or_else(|| panic!("expected event"));
            if ev.event_type == TraceEventType::FileOpened {
                assert_eq!(
                    ev.outcome,
                    Outcome::Success,
                    "FileOpened must always be a success"
                );
            }
        }
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
