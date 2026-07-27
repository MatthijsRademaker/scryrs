use std::path::PathBuf;

use scryrs_types::{ProposalDocument, ProposalReviewDecision, ReviewOutcome};

use super::inventory;
use super::review_write::{ReviewWriteError, ReviewWriteRequest};

/// Validate and serialize a proposal using the same invariants as local inbox files.
pub fn serialize_proposal(proposal: &ProposalDocument) -> Result<String, String> {
    let path = PathBuf::from(proposal.inbox_filename());
    inventory::validate_proposal_document(&path, proposal).map_err(|error| error.to_string())?;
    serde_json::to_string(proposal).map_err(|error| format!("serialization error: {error}"))
}

/// Build a validated review decision without performing filesystem I/O.
pub fn build_review_decision(
    proposal: &ProposalDocument,
    request: &ReviewWriteRequest,
) -> Result<ProposalReviewDecision, ReviewWriteError> {
    if request.proposal_id != proposal.id {
        return Err(ReviewWriteError::Input(format!(
            "review proposal ID '{}' does not match proposal '{}'",
            request.proposal_id, proposal.id
        )));
    }
    inventory::validate_rfc3339(&request.decided_at)
        .map_err(|message| ReviewWriteError::Input(format!("invalid decidedAt: {message}")))?;
    if request.override_content.is_some()
        && !inventory::is_markdown_target_type(&proposal.target_type)
    {
        return Err(ReviewWriteError::Input(format!(
            "reviewed content is not supported for target type '{}'",
            serde_json::to_string(&proposal.target_type)
                .unwrap_or_else(|_| format!("{:?}", proposal.target_type))
        )));
    }

    let (target_type, accepted_content) = match request.outcome {
        ReviewOutcome::Accepted => {
            let content = match request.override_content.clone() {
                Some(overridden) if inventory::is_markdown_target_type(&proposal.target_type) => {
                    overridden
                }
                _ => proposal.proposed_content.clone(),
            };
            (Some(proposal.target_type.clone()), Some(content))
        }
        ReviewOutcome::Rejected => (None, None),
    };
    let decision = ProposalReviewDecision {
        schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: request.reviewer.clone(),
        decided_at: request.decided_at.clone(),
        rationale: request.rationale.clone(),
        source_evidence: proposal.evidence.clone(),
        outcome: request.outcome.clone(),
        target_type,
        accepted_content,
    };
    decision
        .validate()
        .map_err(|error| ReviewWriteError::Input(format!("invalid review metadata: {error}")))?;
    inventory::validate_review_decision_matches_proposal(&decision, proposal)
        .map_err(|error| ReviewWriteError::Input(format!("review validation failed: {error}")))?;
    Ok(decision)
}

/// Serialize an already validated review decision to canonical compact JSON.
pub fn serialize_review_decision(decision: &ProposalReviewDecision) -> Result<String, String> {
    decision.validate()?;
    inventory::validate_rfc3339(&decision.decided_at)
        .map_err(|message| format!("invalid decidedAt: {message}"))?;
    serde_json::to_string(decision).map_err(|error| format!("serialization error: {error}"))
}
