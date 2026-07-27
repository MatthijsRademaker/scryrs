use std::fmt;
use std::path::Path;

use scryrs_types::{ProposalDocument, ProposedContent, ReviewOutcome};

use super::{inventory, wire};

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

    let decision = wire::build_review_decision(&proposal, request)?;
    let json = wire::serialize_review_decision(&decision).map_err(ReviewWriteError::Failure)?;

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

fn opposite_outcome(outcome: &ReviewOutcome) -> ReviewOutcome {
    match outcome {
        ReviewOutcome::Accepted => ReviewOutcome::Rejected,
        ReviewOutcome::Rejected => ReviewOutcome::Accepted,
    }
}
