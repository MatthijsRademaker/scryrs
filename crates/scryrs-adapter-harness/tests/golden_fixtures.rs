//! Golden-fixture tests over captured harness payloads.
//!
//! Each fixture is a real-shaped payload for one supported tool. Their purpose
//! is drift detection: if a harness renames a key input field, the fixture stops
//! producing the expected subject and this test fails — instead of the adapter
//! silently dropping the event and the corpus quietly losing a whole event
//! family, which is exactly how `ast_grep_search` recorded `"unknown"` for a
//! month while `grep` and `find` were discarded entirely.
//!
//! Pi shapes are verified against the typed schemas in Pi 0.81.1
//! (`dist/core/tools/*.d.ts`) plus 450 real tool-call payloads observed in
//! session transcripts. Claude Code shapes are verified against the documented
//! hook payload contract.

use std::path::{Path, PathBuf};

use scryrs_adapter_harness::{ClaudeCodeAdapter, HarnessAdapter, HookContext, PiAdapter};

fn ctx() -> HookContext {
    HookContext {
        session_id: "fixture".into(),
        store_path: PathBuf::from(".scryrs/scryrs.db"),
        timestamp: "2026-07-26T00:00:00Z".into(),
        bash_debug: true,
        repo_root: PathBuf::from("/repo"),
    }
}

fn load(harness: &str, name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(harness)
        .join(format!("{name}.json"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

/// `(fixture, expected event type, expected subject)`.
const PI_CASES: &[(&str, &str, &str)] = &[
    ("read", "FileOpened", "crates/scryrs-cli/src/init.rs"),
    (
        "read-failure",
        "FailedLookup",
        "crates/scryrs-cli/src/missing.rs",
    ),
    ("grep", "SearchRun", "fn translate"),
    ("find", "SearchRun", "*.rs"),
    ("ast-grep-search", "SearchRun", "console.log($MSG)"),
    ("edit", "EditMade", "crates/scryrs-cli/src/init.rs"),
    ("write", "EditMade", "crates/scryrs-cli/src/new.rs"),
    ("bash", "CommandExecuted", "cargo test --workspace"),
];

const CLAUDE_CASES: &[(&str, &str, &str)] = &[
    ("read", "FileOpened", "crates/scryrs-cli/src/init.rs"),
    (
        "read-failure",
        "FailedLookup",
        "crates/scryrs-cli/src/missing.rs",
    ),
    ("grep", "SearchRun", "fn translate"),
    ("glob", "SearchRun", "**/*.rs"),
    ("websearch", "SearchRun", "rust trait objects"),
    ("edit", "EditMade", "crates/scryrs-cli/src/init.rs"),
    ("write", "EditMade", "crates/scryrs-cli/src/new.rs"),
    ("notebookedit", "EditMade", "analysis.ipynb"),
    (
        "webfetch",
        "DocRetrieved",
        "https://example.com/docs/a/../b",
    ),
    ("bash", "CommandExecuted", "cargo test --workspace"),
];

#[test]
fn pi_fixtures_translate_to_expected_subjects() {
    for (fixture, expected_type, expected_subject) in PI_CASES {
        let raw = load("pi", fixture);
        let event = PiAdapter
            .translate(&raw, &ctx())
            .unwrap_or_else(|e| panic!("pi/{fixture}: translate failed: {e}"))
            .unwrap_or_else(|| {
                panic!("pi/{fixture}: dropped — a key input field name likely changed")
            });

        assert_eq!(
            event.event_type.payload_type_str(),
            *expected_type,
            "pi/{fixture}: event type"
        );
        assert_eq!(
            event.subject(),
            Some(*expected_subject),
            "pi/{fixture}: subject"
        );
    }
}

#[test]
fn claude_code_fixtures_translate_to_expected_subjects() {
    for (fixture, expected_type, expected_subject) in CLAUDE_CASES {
        let raw = load("claude-code", fixture);
        let event = ClaudeCodeAdapter
            .translate(&raw, &ctx())
            .unwrap_or_else(|e| panic!("claude-code/{fixture}: translate failed: {e}"))
            .unwrap_or_else(|| {
                panic!("claude-code/{fixture}: dropped — a key input field name likely changed")
            });

        assert_eq!(
            event.event_type.payload_type_str(),
            *expected_type,
            "claude-code/{fixture}: event type"
        );
        assert_eq!(
            event.subject(),
            Some(*expected_subject),
            "claude-code/{fixture}: subject"
        );
    }
}

/// No fixture may produce a placeholder subject.
#[test]
fn no_fixture_produces_a_placeholder_subject() {
    for (harness, cases) in [("pi", PI_CASES), ("claude-code", CLAUDE_CASES)] {
        for (fixture, _, _) in cases {
            let raw = load(harness, fixture);
            let event = if harness == "pi" {
                PiAdapter.translate(&raw, &ctx())
            } else {
                ClaudeCodeAdapter.translate(&raw, &ctx())
            }
            .unwrap_or_else(|e| panic!("{harness}/{fixture}: {e}"))
            .unwrap_or_else(|| panic!("{harness}/{fixture}: dropped"));

            let subject = event.subject().unwrap_or("");
            assert_ne!(
                subject, "unknown",
                "{harness}/{fixture}: placeholder subject"
            );
            assert!(
                !subject.trim().is_empty(),
                "{harness}/{fixture}: empty subject"
            );
        }
    }
}

/// Outcomes must come from the payload, not from a default.
#[test]
fn failure_fixtures_carry_failure_outcomes() {
    for harness in ["pi", "claude-code"] {
        let raw = load(harness, "read-failure");
        let event = if harness == "pi" {
            PiAdapter.translate(&raw, &ctx())
        } else {
            ClaudeCodeAdapter.translate(&raw, &ctx())
        }
        .unwrap_or_else(|e| panic!("{harness}/read-failure: {e}"))
        .unwrap_or_else(|| panic!("{harness}/read-failure: dropped"));

        assert!(
            event.failure_reason().is_some(),
            "{harness}/read-failure: must carry a failure outcome"
        );
    }
}

/// Path subjects arrive absolute in the Claude Code fixtures and relative in the
/// Pi ones; both must normalize to the same repository-relative subject.
#[test]
fn both_harnesses_normalize_the_same_file_to_one_subject() {
    let pi = PiAdapter
        .translate(&load("pi", "read"), &ctx())
        .unwrap_or_else(|e| panic!("pi: {e}"))
        .unwrap_or_else(|| panic!("pi: dropped"));
    let cc = ClaudeCodeAdapter
        .translate(&load("claude-code", "read"), &ctx())
        .unwrap_or_else(|e| panic!("claude-code: {e}"))
        .unwrap_or_else(|| panic!("claude-code: dropped"));

    assert_eq!(pi.subject(), cc.subject());
    assert_eq!(pi.subject_kind(), cc.subject_kind());
    assert_eq!(pi.subject_kind(), Some("file"));
}
