//! Integration test harness for `scripts/verify-docs-publish`.
//!
//! Creates temporary fixtures (.scryrs/accepted/ + docs root), calls
//! `publish_accepted_rspress`, and verifies the output is correct.
//! The shell script uses this to ensure the adapter works before
//! running `bun run build` and checking llms surfaces.
//!
//! Controlled by environment variables:
//!   SCRYRS_VERIFY_SCRATCH_DIR — temp root for repo + docs fixtures

#![allow(clippy::unwrap_used, clippy::expect_used)]

use scryrs_adapter_rspress::publish_accepted_rspress;
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
        description: Some("verification fixture evidence".into()),
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
        title: title.into(),
        rationale: format!("verify {title}"),
        proposed_content,
        evidence: make_evidence(title),
        created_at: "2026-06-29T17:00:00Z".into(),
    }
}

fn make_accepted_decision(
    proposal: &ProposalDocument,
    reviewer: &str,
    rationale: &str,
) -> ProposalReviewDecision {
    ProposalReviewDecision {
        schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: reviewer.into(),
        decided_at: "2026-06-29T18:00:00Z".into(),
        rationale: rationale.into(),
        source_evidence: proposal.evidence.clone(),
        outcome: ReviewOutcome::Accepted,
        target_type: Some(proposal.target_type.clone()),
        accepted_content: Some(proposal.proposed_content.clone()),
    }
}

fn write_accepted_decision(repo_root: &Path, decision: &ProposalReviewDecision) {
    let dir = repo_root.join(".scryrs/accepted");
    fs::create_dir_all(&dir).expect("create accepted dir");
    fs::write(
        dir.join(format!("{}.json", decision.proposal_id)),
        serde_json::to_string(decision).expect("serialize"),
    )
    .expect("write decision");
}

/// Run the verification harness. The scratch directory holds:
///   <scratch>/repo  — repository root with .scryrs/accepted/
///   <scratch>/docs  — docs root where pages + _nav.json are written
#[test]
fn verify_publish_integration() {
    let scratch = TempDir::new().expect("create scratch dir");
    let repo_root = scratch.path().join("repo");
    let docs_root = scratch.path().join("docs");
    fs::create_dir_all(&repo_root).expect("create repo dir");
    fs::create_dir_all(&docs_root).expect("create docs dir");

    // Create two accepted proposals across different target types.
    let docs_proposal = make_markdown_proposal(
        ProposalTargetType::DocsNote,
        "verification-docs-note",
        "## Verification Docs Note\n\nPublished for verification.\n",
    );
    let adr_proposal = make_markdown_proposal(
        ProposalTargetType::Adr,
        "verification-adr",
        "## Verification ADR\n\nADR for verification.\n",
    );
    write_accepted_decision(
        &repo_root,
        &make_accepted_decision(&docs_proposal, "verifier", "publish"),
    );
    write_accepted_decision(
        &repo_root,
        &make_accepted_decision(&adr_proposal, "verifier", "publish"),
    );

    // Publish.
    let entries =
        publish_accepted_rspress(&repo_root, &docs_root).expect("publish_accepted_rspress failed");

    // Verify we got exactly two entries.
    assert_eq!(
        entries.len(),
        2,
        "expected 2 publish entries, got {}",
        entries.len()
    );

    // Sort expected IDs.
    let mut expected_ids = vec![docs_proposal.id.clone(), adr_proposal.id.clone()];
    expected_ids.sort();

    let actual_ids: Vec<String> = entries.iter().map(|e| e.proposal_id.clone()).collect();
    assert_eq!(actual_ids, expected_ids, "published entry ids mismatch");

    // Verify pages exist on disk.
    for entry in &entries {
        let page_path = docs_root.join(&entry.path);
        assert!(page_path.is_file(), "missing page: {page_path:?}");
        let content = fs::read_to_string(&page_path).expect("read page");
        assert!(
            content.contains("## Verification"),
            "page content missing '## Verification' heading"
        );
        assert!(content.starts_with("---"), "page missing frontmatter");
    }

    // Verify _nav.json exists and is parseable.
    let nav_path = docs_root.join("_nav.json");
    assert!(nav_path.is_file(), "missing _nav.json");
    let nav_raw = fs::read_to_string(&nav_path).expect("read _nav.json");
    let nav: serde_json::Value = serde_json::from_str(&nav_raw).expect("parse _nav.json");
    let sections = nav.as_array().expect("_nav.json is array");
    assert!(!sections.is_empty(), "_nav.json should have sections");
    let nav_str = serde_json::to_string(&nav).expect("serialize nav");
    for id in &expected_ids {
        assert!(nav_str.contains(id), "_nav.json missing proposal id: {id}");
    }
}
