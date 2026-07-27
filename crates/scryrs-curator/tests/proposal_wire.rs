use scryrs_curator::proposals::review_write::ReviewWriteRequest;
use scryrs_curator::proposals::wire::{build_review_decision, serialize_proposal};
use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
    ProposalTargetType, ProposedContent, ReviewOutcome,
};

fn proposal() -> ProposalDocument {
    let content = ProposedContent::Markdown("# Knowledge".into());
    ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id: ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|error| panic!("id: {error}")),
        target_type: ProposalTargetType::DocsNote,
        title: "Knowledge".into(),
        rationale: "Evidence".into(),
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::ServerTraceRow,
            subject: "src/lib.rs".into(),
            row_ids: vec![1],
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        }],
        created_at: "2026-07-26T12:00:00Z".into(),
    }
}

#[test]
fn shared_wire_helpers_validate_and_serialize_proposals_and_reviews() {
    let proposal = proposal();
    let proposal_json =
        serialize_proposal(&proposal).unwrap_or_else(|error| panic!("serialize proposal: {error}"));
    assert_eq!(
        serde_json::from_str::<ProposalDocument>(&proposal_json)
            .unwrap_or_else(|error| panic!("parse proposal: {error}")),
        proposal
    );

    let decision = build_review_decision(
        &proposal,
        &ReviewWriteRequest {
            proposal_id: proposal.id.clone(),
            outcome: ReviewOutcome::Accepted,
            reviewer: "alice".into(),
            rationale: "approved".into(),
            decided_at: "2026-07-26T13:00:00Z".into(),
            override_content: Some(ProposedContent::Markdown("# Reviewed".into())),
        },
    )
    .unwrap_or_else(|error| panic!("build review: {error}"));
    assert_eq!(decision.reviewer, "alice");
    assert_eq!(
        decision.accepted_content,
        Some(ProposedContent::Markdown("# Reviewed".into()))
    );
}
