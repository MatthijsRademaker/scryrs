#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::run_with_writers;
use crate::test_support::verify_writes_confined;
use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
    ProposalReviewDecision, ProposalTargetType, ProposedContent, REVIEW_DECISION_SCHEMA_VERSION,
    ReviewOutcome,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn make_evidence(subject: &str) -> Vec<EvidenceLink> {
    vec![EvidenceLink {
        source_kind: EvidenceSourceKind::LocalTraceRow,
        subject: subject.to_string(),
        row_ids: vec![1],
        doc_ref: Some(".devagent/docs/docs/proposals.md".into()),
        description: Some(format!("evidence for {subject}")),
        score: Some(10),
        metadata: None,
    }]
}

fn make_markdown_proposal(
    target_type: ProposalTargetType,
    title: &str,
    body: &str,
) -> ProposalDocument {
    let proposed_content = ProposedContent::Markdown(body.to_string());
    let id = ProposalDocument::compute_id(&target_type, &proposed_content).expect("compute id");
    ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type,
        title: title.to_string(),
        rationale: format!("Capture knowledge for {title}"),
        proposed_content,
        evidence: make_evidence(title),
        created_at: "2026-07-01T08:00:00Z".into(),
    }
}

fn make_review_decision(
    proposal: &ProposalDocument,
    outcome: ReviewOutcome,
    reviewer: &str,
    rationale: &str,
) -> ProposalReviewDecision {
    ProposalReviewDecision {
        schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: reviewer.to_string(),
        decided_at: "2026-07-01T08:30:00Z".into(),
        rationale: rationale.to_string(),
        source_evidence: proposal.evidence.clone(),
        outcome: outcome.clone(),
        target_type: match outcome {
            ReviewOutcome::Accepted => Some(proposal.target_type.clone()),
            ReviewOutcome::Rejected => None,
        },
        accepted_content: match outcome {
            ReviewOutcome::Accepted => Some(proposal.proposed_content.clone()),
            ReviewOutcome::Rejected => None,
        },
    }
}

fn write_proposal(root: &Path, proposal: &ProposalDocument) {
    let proposals_dir = root.join(".scryrs/proposals");
    fs::create_dir_all(&proposals_dir).expect("create proposals dir");
    fs::write(
        proposals_dir.join(proposal.inbox_filename()),
        serde_json::to_string(proposal).expect("serialize proposal"),
    )
    .expect("write proposal");
}

fn write_review_decision(root: &Path, decision: &ProposalReviewDecision) {
    let dir_name = match decision.outcome {
        ReviewOutcome::Accepted => "accepted",
        ReviewOutcome::Rejected => "rejected",
    };
    let dir = root.join(format!(".scryrs/{dir_name}"));
    fs::create_dir_all(&dir).expect("create review dir");
    fs::write(
        dir.join(format!("{}.json", decision.proposal_id)),
        serde_json::to_string(decision).expect("serialize decision"),
    )
    .expect("write decision");
}

#[test]
fn publish_without_subcommand_exits_2_with_publish_specific_guidance() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["publish"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("scryrs publish: missing required subcommand"));
    assert!(stderr.contains("Usage: scryrs publish --help"));
}

#[test]
fn publish_unknown_subcommand_exits_2_without_writes() {
    let tmp = TempDir::new().expect("tempdir");
    let before = crate::test_support::compute_file_inventory(tmp.path());

    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit = run_with_writers(
        ["publish", "html", tmp.path().to_str().unwrap()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 2);
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("scryrs publish: unknown subcommand 'html'"));
    assert_eq!(
        before,
        crate::test_support::compute_file_inventory(tmp.path())
    );
}

#[test]
fn publish_markdown_writes_accepted_only_json_summary() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    let output_root = tmp.path().join("markdown-out");
    let accepted =
        make_markdown_proposal(ProposalTargetType::DocsNote, "accepted", "## Accepted\n");
    let pending = make_markdown_proposal(ProposalTargetType::Adr, "pending", "## Pending\n");
    let rejected = make_markdown_proposal(ProposalTargetType::Skill, "rejected", "## Rejected\n");
    write_proposal(&repo_root, &accepted);
    write_proposal(&repo_root, &pending);
    write_proposal(&repo_root, &rejected);
    write_review_decision(
        &repo_root,
        &make_review_decision(&accepted, ReviewOutcome::Accepted, "alice", "publish"),
    );
    write_review_decision(
        &repo_root,
        &make_review_decision(&rejected, ReviewOutcome::Rejected, "alice", "reject"),
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "publish",
                "markdown",
                repo_root.to_str().unwrap(),
                "--output",
                output_root.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        ),
        0
    );
    assert!(err.is_empty());

    let stdout: serde_json::Value = serde_json::from_slice(&out).expect("stdout json");
    assert_eq!(stdout["command"], "publish");
    assert_eq!(stdout["mode"], "markdown");
    assert_eq!(stdout["count"], 1);
    let paths = stdout["paths"].as_array().expect("paths array");
    assert_eq!(paths.len(), 1);
    let written = output_root
        .join("docs_note")
        .join(format!("{}.md", accepted.id));
    assert_eq!(paths[0].as_str(), Some(written.to_str().unwrap()));
    assert!(written.exists());
    assert!(!String::from_utf8_lossy(&out).contains(&pending.id));
    assert!(!String::from_utf8_lossy(&out).contains(&rejected.id));
    assert!(
        !output_root
            .join("adr")
            .join(format!("{}.md", pending.id))
            .exists()
    );
}

#[test]
fn publish_markdown_missing_output_flag_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    fs::create_dir_all(repo_root.join(".scryrs/accepted")).expect("create accepted dir");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            ["publish", "markdown", repo_root.to_str().unwrap()],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("missing required --output argument"));
}

#[test]
fn publish_markdown_malformed_accepted_artifact_exits_2_without_partial_output() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    let output_root = tmp.path().join("markdown-out");
    let accepted =
        make_markdown_proposal(ProposalTargetType::DocsNote, "accepted", "## Accepted\n");
    write_review_decision(
        &repo_root,
        &make_review_decision(&accepted, ReviewOutcome::Accepted, "alice", "publish"),
    );
    let accepted_dir = repo_root.join(".scryrs/accepted");
    fs::write(accepted_dir.join("broken.json"), "{not valid json").expect("write invalid json");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "publish",
                "markdown",
                repo_root.to_str().unwrap(),
                "--output",
                output_root.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("invalid accepted artifact"));
    assert!(!output_root.exists());
}

#[test]
fn publish_rspress_writes_entries_and_nav() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    let docs_root = tmp.path().join("docs");
    let accepted = make_markdown_proposal(ProposalTargetType::Adr, "accepted-adr", "## ADR\n");
    let pending = make_markdown_proposal(ProposalTargetType::DocsNote, "pending", "## Pending\n");
    write_proposal(&repo_root, &accepted);
    write_proposal(&repo_root, &pending);
    write_review_decision(
        &repo_root,
        &make_review_decision(&accepted, ReviewOutcome::Accepted, "alice", "publish"),
    );
    fs::create_dir_all(&docs_root).expect("create docs root");
    fs::write(docs_root.join("_nav.json"), "[]\n").expect("write nav");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "publish",
                "rspress",
                repo_root.to_str().unwrap(),
                "--docs-root",
                docs_root.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        ),
        0
    );
    assert!(err.is_empty());

    let stdout: serde_json::Value = serde_json::from_slice(&out).expect("stdout json");
    assert_eq!(stdout["command"], "publish");
    assert_eq!(stdout["mode"], "rspress");
    assert_eq!(stdout["count"], 1);
    let entries = stdout["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["proposalId"], accepted.id);
    assert_eq!(entries[0]["targetType"], "adr");
    assert!(
        docs_root
            .join("accepted-knowledge/adr")
            .join(format!("{}.md", accepted.id))
            .exists()
    );
    let nav = fs::read_to_string(docs_root.join("_nav.json")).expect("read nav");
    assert!(nav.contains("Accepted Knowledge"));
    assert!(nav.contains(&accepted.id));
    assert!(!nav.contains(&pending.id));
}

#[test]
fn publish_rspress_malformed_nav_exits_2_without_partial_writes() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    let docs_root = tmp.path().join("docs");
    let accepted =
        make_markdown_proposal(ProposalTargetType::DocsNote, "accepted", "## Accepted\n");
    write_review_decision(
        &repo_root,
        &make_review_decision(&accepted, ReviewOutcome::Accepted, "alice", "publish"),
    );
    fs::create_dir_all(docs_root.join("accepted-knowledge/docs_note"))
        .expect("create existing docs");
    let existing_page = docs_root.join("accepted-knowledge/docs_note/existing.md");
    fs::write(&existing_page, "existing bytes\n").expect("write existing page");
    fs::write(docs_root.join("_nav.json"), "not json").expect("write malformed nav");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "publish",
                "rspress",
                repo_root.to_str().unwrap(),
                "--docs-root",
                docs_root.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("malformed _nav.json"));
    assert_eq!(
        fs::read_to_string(&existing_page).unwrap(),
        "existing bytes\n"
    );
}

#[test]
fn publish_runtime_write_failure_exits_1() {
    let tmp = TempDir::new().expect("tempdir");
    let repo_root = tmp.path().join("repo");
    let output_root = tmp.path().join("not-a-directory");
    let accepted =
        make_markdown_proposal(ProposalTargetType::DocsNote, "accepted", "## Accepted\n");
    write_review_decision(
        &repo_root,
        &make_review_decision(&accepted, ReviewOutcome::Accepted, "alice", "publish"),
    );
    fs::write(&output_root, "file collision").expect("write collision file");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "publish",
                "markdown",
                repo_root.to_str().unwrap(),
                "--output",
                output_root.to_str().unwrap(),
            ],
            &mut out,
            &mut err,
        ),
        1
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("cannot create output directory"));
}

#[test]
fn proposals_accept_remains_ledger_only_without_publish_side_effects() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "alpha", "## Accepted\n");
    write_proposal(tmp.path(), &proposal);
    fs::create_dir_all(tmp.path().join(".devagent/docs/docs")).expect("create docs tree");
    fs::write(tmp.path().join(".devagent/docs/docs/_nav.json"), "[]\n").expect("write nav");

    verify_writes_confined(
        tmp.path(),
        &[".devagent/docs/", ".scryrs/proposals/"],
        &[".scryrs/accepted/"],
        || {
            let mut out = Vec::new();
            let mut err = Vec::new();
            run_with_writers(
                [
                    "proposals",
                    "accept",
                    tmp.path().to_str().unwrap(),
                    proposal.id.as_str(),
                    "--reviewer",
                    "alice",
                    "--rationale",
                    "approved",
                    "--decided-at",
                    "2026-07-01T09:00:00Z",
                ],
                &mut out,
                &mut err,
            )
        },
    );

    assert!(
        !tmp.path()
            .join(".devagent/docs/docs/accepted-knowledge")
            .exists()
    );
}
