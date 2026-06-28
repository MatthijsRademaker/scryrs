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

fn make_evidence(subject: &str, row_id: u64) -> Vec<EvidenceLink> {
    vec![EvidenceLink {
        source_kind: EvidenceSourceKind::LocalTraceRow,
        subject: subject.to_string(),
        row_ids: vec![row_id],
        doc_ref: None,
        description: Some(format!("evidence for {subject}")),
        score: Some(10),
        metadata: None,
    }]
}

fn make_markdown_proposal(subject: &str, title: &str, created_at: &str) -> ProposalDocument {
    let target_type = ProposalTargetType::DocsNote;
    let proposed_content = ProposedContent::Markdown(format!("# Note for {subject}\n"));
    let id = ProposalDocument::compute_id(&target_type, &proposed_content).expect("compute id");
    ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type,
        title: title.to_string(),
        rationale: format!("Capture knowledge for {subject}"),
        proposed_content,
        evidence: make_evidence(subject, 1),
        created_at: created_at.to_string(),
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

fn make_review_decision(
    proposal: &ProposalDocument,
    outcome: ReviewOutcome,
    reviewer: &str,
    rationale: &str,
    decided_at: &str,
) -> ProposalReviewDecision {
    ProposalReviewDecision {
        schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: reviewer.to_string(),
        decided_at: decided_at.to_string(),
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

fn seed_protected_paths(root: &Path) {
    let docs_dir = root.join(".devagent/docs/docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(docs_dir.join("vision.md"), "# Vision\n").expect("write docs");

    let scryrs_dir = root.join(".scryrs");
    fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
    fs::write(scryrs_dir.join("graph.json"), "{}").expect("write graph");
    fs::write(scryrs_dir.join("routes.json"), "{}").expect("write routes");
}

#[test]
fn proposals_list_reports_pending_and_reviewed_states_in_sorted_json() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal_z = make_markdown_proposal("zeta", "Zeta", "2026-06-28T09:00:00Z");
    let proposal_a = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    let proposal_m = make_markdown_proposal("mu", "Mu", "2026-06-28T11:00:00Z");
    write_proposal(tmp.path(), &proposal_z);
    write_proposal(tmp.path(), &proposal_a);
    write_proposal(tmp.path(), &proposal_m);
    write_review_decision(
        tmp.path(),
        &make_review_decision(
            &proposal_z,
            ReviewOutcome::Accepted,
            "alice",
            "approved",
            "2026-06-28T12:00:00Z",
        ),
    );
    write_review_decision(
        tmp.path(),
        &make_review_decision(
            &proposal_m,
            ReviewOutcome::Rejected,
            "alice",
            "off-scope",
            "2026-06-28T12:30:00Z",
        ),
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            ["proposals", "list", tmp.path().to_str().unwrap()],
            &mut out,
            &mut err
        ),
        0
    );
    assert!(err.is_empty());
    let rows: serde_json::Value = serde_json::from_slice(&out).expect("stdout json");
    let rows = rows.as_array().expect("rows array");
    let ids: Vec<&str> = rows
        .iter()
        .map(|row| row["proposalId"].as_str().expect("proposalId"))
        .collect();
    let mut expected = vec![
        proposal_z.id.as_str(),
        proposal_a.id.as_str(),
        proposal_m.id.as_str(),
    ];
    expected.sort();
    assert_eq!(ids, expected);

    for row in rows {
        let proposal_id = row["proposalId"].as_str().unwrap();
        let state = row["state"].as_str().unwrap();
        if proposal_id == proposal_a.id {
            assert_eq!(state, "pending");
        } else if proposal_id == proposal_z.id {
            assert_eq!(state, "accepted");
        } else if proposal_id == proposal_m.id {
            assert_eq!(state, "rejected");
        } else {
            panic!("unexpected proposalId {proposal_id}");
        }
    }

    out.clear();
    err.clear();
    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "list",
                tmp.path().to_str().unwrap(),
                "--state",
                "accepted",
            ],
            &mut out,
            &mut err,
        ),
        0
    );
    assert!(err.is_empty());
    let accepted_rows: serde_json::Value = serde_json::from_slice(&out).expect("stdout json");
    let accepted_rows = accepted_rows.as_array().expect("accepted rows");
    assert_eq!(accepted_rows.len(), 1);
    assert_eq!(
        accepted_rows[0]["proposalId"].as_str(),
        Some(proposal_z.id.as_str())
    );
    assert_eq!(accepted_rows[0]["state"].as_str(), Some("accepted"));
}

#[test]
fn proposals_list_invalid_filter_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "list",
                tmp.path().to_str().unwrap(),
                "--state",
                "archived",
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("invalid --state value 'archived'"),
        "got: {stderr}"
    );
    assert!(
        stderr.contains(
            "Usage: scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"
        )
    );
}

#[test]
fn proposals_list_conflicting_terminal_state_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    write_review_decision(
        tmp.path(),
        &make_review_decision(
            &proposal,
            ReviewOutcome::Accepted,
            "alice",
            "approved",
            "2026-06-28T12:00:00Z",
        ),
    );
    write_review_decision(
        tmp.path(),
        &make_review_decision(
            &proposal,
            ReviewOutcome::Rejected,
            "alice",
            "off-scope",
            "2026-06-28T12:01:00Z",
        ),
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            ["proposals", "list", tmp.path().to_str().unwrap()],
            &mut out,
            &mut err
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("conflicting terminal state"));
}

#[test]
fn proposals_list_invalid_review_artifact_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);

    let mut decision = make_review_decision(
        &proposal,
        ReviewOutcome::Accepted,
        "alice",
        "approved",
        "2026-06-28T12:00:00Z",
    );
    decision.accepted_content = Some(ProposedContent::Markdown("different bytes".into()));
    write_review_decision(tmp.path(), &decision);

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            ["proposals", "list", tmp.path().to_str().unwrap()],
            &mut out,
            &mut err
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("does not preserve acceptedContent"));
}

#[test]
fn proposals_accept_writes_valid_decision_and_preserves_proposal_file() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    let proposal_path = tmp
        .path()
        .join(".scryrs/proposals")
        .join(proposal.inbox_filename());
    let proposal_before = fs::read(&proposal_path).expect("read proposal before");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
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
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        0
    );
    assert!(out.is_empty());
    assert!(err.is_empty());

    let decision_path = tmp
        .path()
        .join(".scryrs/accepted")
        .join(format!("{}.json", proposal.id));
    let decision_json = fs::read_to_string(&decision_path).expect("read decision");
    let decision: ProposalReviewDecision =
        serde_json::from_str(&decision_json).expect("decision json");
    assert_eq!(decision.outcome, ReviewOutcome::Accepted);
    assert_eq!(decision.target_type, Some(proposal.target_type.clone()));
    assert_eq!(
        decision.accepted_content,
        Some(proposal.proposed_content.clone())
    );
    assert_eq!(decision.source_evidence, proposal.evidence);
    assert_eq!(
        fs::read(&proposal_path).expect("read proposal after"),
        proposal_before
    );
}

#[test]
fn proposals_reject_writes_valid_decision_and_preserves_proposal_file() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    let proposal_path = tmp
        .path()
        .join(".scryrs/proposals")
        .join(proposal.inbox_filename());
    let proposal_before = fs::read(&proposal_path).expect("read proposal before");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "reject",
                tmp.path().to_str().unwrap(),
                proposal.id.as_str(),
                "--reviewer",
                "alice",
                "--rationale",
                "off-scope",
                "--decided-at",
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        0
    );
    assert!(out.is_empty());
    assert!(err.is_empty());

    let decision_path = tmp
        .path()
        .join(".scryrs/rejected")
        .join(format!("{}.json", proposal.id));
    let decision_json = fs::read_to_string(&decision_path).expect("read decision");
    let decision: ProposalReviewDecision =
        serde_json::from_str(&decision_json).expect("decision json");
    assert_eq!(decision.outcome, ReviewOutcome::Rejected);
    assert_eq!(decision.target_type, None);
    assert_eq!(decision.accepted_content, None);
    assert_eq!(decision.source_evidence, proposal.evidence);
    assert_eq!(
        fs::read(&proposal_path).expect("read proposal after"),
        proposal_before
    );
}

#[test]
fn proposals_accept_unknown_id_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "accept",
                tmp.path().to_str().unwrap(),
                "missing-id",
                "--reviewer",
                "alice",
                "--rationale",
                "approved",
                "--decided-at",
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("unknown proposal ID 'missing-id'"));
}

#[test]
fn proposals_accept_missing_required_metadata_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "accept",
                tmp.path().to_str().unwrap(),
                proposal.id.as_str()
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("missing required --reviewer argument"),
        "got: {stderr}"
    );
    assert!(stderr.contains("Usage: scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>"));
}

#[test]
fn proposals_accept_invalid_decided_at_exits_2() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
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
                "not-a-timestamp",
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("invalid --decided-at value"));
}

#[test]
fn proposals_accept_is_idempotent_for_identical_bytes_only() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);

    let args = [
        "proposals",
        "accept",
        tmp.path().to_str().unwrap(),
        proposal.id.as_str(),
        "--reviewer",
        "alice",
        "--rationale",
        "approved",
        "--decided-at",
        "2026-06-28T12:00:00Z",
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(args, &mut out, &mut err), 0);
    out.clear();
    err.clear();
    assert_eq!(run_with_writers(args, &mut out, &mut err), 0);
    assert!(out.is_empty());
    assert!(err.is_empty());

    out.clear();
    err.clear();
    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "accept",
                tmp.path().to_str().unwrap(),
                proposal.id.as_str(),
                "--reviewer",
                "alice",
                "--rationale",
                "changed rationale",
                "--decided-at",
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("refusing to overwrite"));
}

#[test]
fn proposals_reject_conflicts_with_existing_accepted_decision() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    write_review_decision(
        tmp.path(),
        &make_review_decision(
            &proposal,
            ReviewOutcome::Accepted,
            "alice",
            "approved",
            "2026-06-28T12:00:00Z",
        ),
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(
            [
                "proposals",
                "reject",
                tmp.path().to_str().unwrap(),
                proposal.id.as_str(),
                "--reviewer",
                "alice",
                "--rationale",
                "off-scope",
                "--decided-at",
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        2
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("conflicting terminal decision"));
    assert!(
        !tmp.path()
            .join(".scryrs/rejected")
            .join(format!("{}.json", proposal.id))
            .exists()
    );
}

#[test]
fn proposals_accept_write_failure_exits_1() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    let scryrs_dir = tmp.path().join(".scryrs");
    fs::write(scryrs_dir.join("accepted"), "not a directory").expect("write collision file");

    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
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
                "2026-06-28T12:00:00Z",
            ],
            &mut out,
            &mut err,
        ),
        1
    );
    assert!(out.is_empty());
    assert!(String::from_utf8_lossy(&err).contains("cannot create review directory"));
}

#[test]
fn proposals_commands_do_not_mutate_protected_paths() {
    let tmp = TempDir::new().expect("tempdir");
    let proposal = make_markdown_proposal("alpha", "Alpha", "2026-06-28T10:00:00Z");
    write_proposal(tmp.path(), &proposal);
    seed_protected_paths(tmp.path());

    verify_writes_confined(
        tmp.path(),
        &[
            ".devagent/docs/",
            ".scryrs/graph.json",
            ".scryrs/routes.json",
            ".scryrs/proposals/",
        ],
        &[],
        || {
            let mut out = Vec::new();
            let mut err = Vec::new();
            run_with_writers(
                ["proposals", "list", tmp.path().to_str().unwrap()],
                &mut out,
                &mut err,
            )
        },
    );

    let tmp_accept = TempDir::new().expect("tempdir");
    let accept_proposal = make_markdown_proposal("beta", "Beta", "2026-06-28T10:00:00Z");
    write_proposal(tmp_accept.path(), &accept_proposal);
    seed_protected_paths(tmp_accept.path());
    verify_writes_confined(
        tmp_accept.path(),
        &[
            ".devagent/docs/",
            ".scryrs/graph.json",
            ".scryrs/routes.json",
            ".scryrs/proposals/",
        ],
        &[".scryrs/accepted/"],
        || {
            let mut out = Vec::new();
            let mut err = Vec::new();
            run_with_writers(
                [
                    "proposals",
                    "accept",
                    tmp_accept.path().to_str().unwrap(),
                    accept_proposal.id.as_str(),
                    "--reviewer",
                    "alice",
                    "--rationale",
                    "approved",
                    "--decided-at",
                    "2026-06-28T12:00:00Z",
                ],
                &mut out,
                &mut err,
            )
        },
    );

    let tmp_reject = TempDir::new().expect("tempdir");
    let reject_proposal = make_markdown_proposal("gamma", "Gamma", "2026-06-28T10:00:00Z");
    write_proposal(tmp_reject.path(), &reject_proposal);
    seed_protected_paths(tmp_reject.path());
    verify_writes_confined(
        tmp_reject.path(),
        &[
            ".devagent/docs/",
            ".scryrs/graph.json",
            ".scryrs/routes.json",
            ".scryrs/proposals/",
        ],
        &[".scryrs/rejected/"],
        || {
            let mut out = Vec::new();
            let mut err = Vec::new();
            run_with_writers(
                [
                    "proposals",
                    "reject",
                    tmp_reject.path().to_str().unwrap(),
                    reject_proposal.id.as_str(),
                    "--reviewer",
                    "alice",
                    "--rationale",
                    "off-scope",
                    "--decided-at",
                    "2026-06-28T12:00:00Z",
                ],
                &mut out,
                &mut err,
            )
        },
    );
}
