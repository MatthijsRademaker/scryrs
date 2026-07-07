use std::fmt;
use std::path::Path;

use scryrs_types::{ProposalDocument, ProposalReviewDecision, ProposedContent, ReviewOutcome};

use super::inventory;

#[derive(Debug, Clone)]
pub struct ReviewWriteRequest {
    pub proposal_id: String,
    pub outcome: ReviewOutcome,
    pub reviewer: String,
    pub rationale: String,
    pub decided_at: String,
    pub override_content: Option<ProposedContent>,
}

#[derive(Debug)]
pub enum ReviewWriteError {
    Input(String),
    Conflict(String),
    Artifact(String),
    Failure(String),
}

impl fmt::Display for ReviewWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(message)
            | Self::Conflict(message)
            | Self::Artifact(message)
            | Self::Failure(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for ReviewWriteError {}

pub fn write_review_decision(
    repo_root: &Path,
    request: &ReviewWriteRequest,
) -> Result<(), ReviewWriteError> {
    inventory::validate_rfc3339(&request.decided_at)
        .map_err(|message| ReviewWriteError::Input(format!("invalid decidedAt: {message}")))?;

    let proposal_path = repo_root.join(format!(".scryrs/proposals/{}.json", request.proposal_id));
    if !proposal_path.is_file() {
        return Err(ReviewWriteError::Input(format!(
            "unknown proposal ID '{}'",
            request.proposal_id
        )));
    }

    let proposal_json = std::fs::read_to_string(&proposal_path).map_err(|error| {
        ReviewWriteError::Artifact(format!(
            "cannot read proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;
    let proposal: ProposalDocument = serde_json::from_str(&proposal_json).map_err(|error| {
        ReviewWriteError::Artifact(format!(
            "invalid proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;
    inventory::validate_proposal_document(&proposal_path, &proposal).map_err(|error| {
        ReviewWriteError::Artifact(format!(
            "invalid proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;

    if request.override_content.is_some()
        && !inventory::is_markdown_target_type(&proposal.target_type)
    {
        return Err(ReviewWriteError::Input(format!(
            "reviewed content is not supported for target type '{}'",
            serde_json::to_string(&proposal.target_type)
                .unwrap_or_else(|_| format!("{:?}", proposal.target_type))
        )));
    }

    let decision = build_review_decision(&proposal, request);
    decision
        .validate()
        .map_err(|error| ReviewWriteError::Input(format!("invalid review metadata: {error}")))?;
    inventory::validate_review_decision_matches_proposal(&decision, &proposal)
        .map_err(|error| ReviewWriteError::Input(format!("review validation failed: {error}")))?;

    let json = serde_json::to_string(&decision)
        .map_err(|error| ReviewWriteError::Failure(format!("serialization error: {error}")))?;

    let target_dir = repo_root.join(format!(
        ".scryrs/{}",
        inventory::review_dir_name(&request.outcome)
    ));
    let target_path = target_dir.join(format!("{}.json", request.proposal_id));
    let conflict_dir = repo_root.join(format!(
        ".scryrs/{}",
        inventory::review_dir_name(&opposite_outcome(&request.outcome))
    ));
    let conflict_path = conflict_dir.join(format!("{}.json", request.proposal_id));
    if conflict_path.exists() {
        return Err(ReviewWriteError::Conflict(format!(
            "conflicting terminal decision already exists at {}",
            conflict_path.display()
        )));
    }

    if target_path.exists() {
        let existing = std::fs::read_to_string(&target_path).map_err(|error| {
            ReviewWriteError::Artifact(format!(
                "cannot read existing review decision {}: {error}",
                target_path.display()
            ))
        })?;
        if existing == json {
            return Ok(());
        }
        return Err(ReviewWriteError::Conflict(format!(
            "existing review decision differs from requested bytes; refusing to overwrite {}",
            target_path.display()
        )));
    }

    std::fs::create_dir_all(&target_dir).map_err(|error| {
        ReviewWriteError::Failure(format!(
            "cannot create review directory {}: {error}",
            target_dir.display()
        ))
    })?;
    std::fs::write(&target_path, json).map_err(|error| {
        ReviewWriteError::Failure(format!(
            "cannot write review decision {}: {error}",
            target_path.display()
        ))
    })?;

    Ok(())
}

fn build_review_decision(
    proposal: &ProposalDocument,
    request: &ReviewWriteRequest,
) -> ProposalReviewDecision {
    let (target_type, accepted_content) = match request.outcome {
        ReviewOutcome::Accepted => {
            let content = match request.override_content.clone() {
                Some(overridden) if inventory::is_markdown_target_type(&proposal.target_type) => {
                    Some(overridden)
                }
                _ => Some(proposal.proposed_content.clone()),
            };
            (Some(proposal.target_type.clone()), content)
        }
        ReviewOutcome::Rejected => (None, None),
    };

    ProposalReviewDecision {
        schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: request.reviewer.clone(),
        decided_at: request.decided_at.clone(),
        rationale: request.rationale.clone(),
        source_evidence: proposal.evidence.clone(),
        outcome: request.outcome.clone(),
        target_type,
        accepted_content,
    }
}

fn opposite_outcome(outcome: &ReviewOutcome) -> ReviewOutcome {
    match outcome {
        ReviewOutcome::Accepted => ReviewOutcome::Rejected,
        ReviewOutcome::Rejected => ReviewOutcome::Accepted,
    }
}
