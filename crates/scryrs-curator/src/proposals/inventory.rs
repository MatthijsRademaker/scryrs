//! Proposal inventory loading and validation shared between CLI and dashboard.
//!
//! Extracted from `scryrs-cli/src/proposals.rs` so that `scryrs-dashboard`
//! can reuse identical deterministic listing and validation semantics.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

use scryrs_types::{ProposalDocument, ProposalReviewDecision, ProposalTargetType, ReviewOutcome};

// ---------------------------------------------------------------------------
// InventoryError
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum InventoryError {
    /// Filesystem-level failure (directory read error, path resolution).
    Io { path: PathBuf, message: String },
    /// JSON parse failure for a specific file.
    Parse { path: PathBuf, message: String },
    /// Semantic validation failure for a specific file.
    Validation { path: PathBuf, message: String },
    /// Duplicate proposal ID detected (id, path1, path2).
    DuplicateId {
        proposal_id: String,
        path_a: PathBuf,
        path_b: PathBuf,
    },
    /// Conflicting accepted and rejected artifacts for the same proposal ID.
    ConflictingState {
        proposal_id: String,
        accepted_path: PathBuf,
        rejected_path: PathBuf,
    },
    /// A reviewed artifact has no matching proposal inbox document.
    OrphanReview { path: PathBuf, proposal_id: String },
    /// The proposals directory is missing entirely.
    MissingDirectory { path: PathBuf },
}

impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => {
                write!(f, "cannot read path {}: {message}", path.display())
            }
            Self::Parse { path, message } => {
                write!(f, "invalid JSON in {}: {message}", path.display())
            }
            Self::Validation { path, message } => {
                write!(f, "invalid artifact {}: {message}", path.display())
            }
            Self::DuplicateId {
                proposal_id,
                path_a,
                path_b,
            } => {
                write!(
                    f,
                    "duplicate proposal ID '{proposal_id}' in {} and {}",
                    path_a.display(),
                    path_b.display()
                )
            }
            Self::ConflictingState {
                proposal_id,
                accepted_path,
                rejected_path,
            } => {
                write!(
                    f,
                    "conflicting terminal state for proposal ID '{proposal_id}': accepted at {} and rejected at {}",
                    accepted_path.display(),
                    rejected_path.display()
                )
            }
            Self::OrphanReview { path, proposal_id } => {
                write!(
                    f,
                    "reviewed artifact {} has no matching proposal inbox document for proposal ID '{proposal_id}'",
                    path.display()
                )
            }
            Self::MissingDirectory { path } => {
                write!(f, "proposals directory not found: {}", path.display())
            }
        }
    }
}

impl std::error::Error for InventoryError {}

// ---------------------------------------------------------------------------
// ProposalListRow and ProposalState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalState {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStateFilter {
    Pending,
    Accepted,
    Rejected,
    All,
}

impl ProposalStateFilter {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "all" => Ok(Self::All),
            other => Err(format!(
                "invalid --state value '{other}' (expected pending, accepted, rejected, or all)"
            )),
        }
    }

    pub fn matches(self, state: ProposalState) -> bool {
        match self {
            Self::All => true,
            Self::Pending => state == ProposalState::Pending,
            Self::Accepted => state == ProposalState::Accepted,
            Self::Rejected => state == ProposalState::Rejected,
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalListRow {
    pub proposal_id: String,
    pub title: String,
    pub target_type: ProposalTargetType,
    pub created_at: String,
    pub state: ProposalState,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load proposal documents from `.scryrs/proposals/`, validated.
pub fn load_proposals(
    repo_root: &Path,
) -> Result<BTreeMap<String, ProposalDocument>, InventoryError> {
    let proposals_dir = repo_root.join(".scryrs/proposals");
    let mut proposals = BTreeMap::new();

    for path in json_files_in_dir(&proposals_dir)? {
        let json = std::fs::read_to_string(&path).map_err(|error| InventoryError::Io {
            path: path.clone(),
            message: format!("cannot read proposal document: {error}"),
        })?;
        let proposal: ProposalDocument =
            serde_json::from_str(&json).map_err(|error| InventoryError::Parse {
                path: path.clone(),
                message: format!("invalid proposal document: {error}"),
            })?;
        validate_proposal_document(&path, &proposal)?;
        if let Some(existing) = proposals.insert(proposal.id.clone(), proposal) {
            return Err(InventoryError::DuplicateId {
                proposal_id: existing.id.clone(),
                path_a: path,
                path_b: {
                    // Reconstruct the path for the earlier entry.
                    // validate_proposal_document (called above) guarantees
                    // that every proposal file is named `{id}.json`, so this
                    // reconstruction yields the exact filesystem path of the
                    // earlier entry.
                    let mut pb = proposals_dir.clone();
                    pb.push(inbox_json_filename(&existing));
                    pb
                },
            });
        }
    }

    Ok(proposals)
}

/// Load review decision artifacts from `.scryrs/accepted/` or `.scryrs/rejected/`.
pub fn load_review_decisions(
    repo_root: &Path,
    expected_outcome: ReviewOutcome,
) -> Result<BTreeMap<String, ProposalReviewDecision>, InventoryError> {
    let review_dir = repo_root.join(format!(".scryrs/{}", review_dir_name(&expected_outcome)));
    let proposals = load_proposals(repo_root)?;
    let mut decisions = BTreeMap::new();

    for path in json_files_in_dir(&review_dir)? {
        let json = std::fs::read_to_string(&path).map_err(|error| InventoryError::Io {
            path: path.clone(),
            message: format!("cannot read review decision: {error}"),
        })?;
        let decision: ProposalReviewDecision =
            serde_json::from_str(&json).map_err(|error| InventoryError::Parse {
                path: path.clone(),
                message: format!("invalid reviewed artifact: {error}"),
            })?;
        validate_review_decision_artifact(&path, &decision, &expected_outcome)?;
        let proposal =
            proposals
                .get(&decision.proposal_id)
                .ok_or_else(|| InventoryError::OrphanReview {
                    path: path.clone(),
                    proposal_id: decision.proposal_id.clone(),
                })?;
        validate_review_decision_matches_proposal(&decision, proposal)?;
        let decision_id = decision.proposal_id.clone();
        if decisions.insert(decision_id.clone(), decision).is_some() {
            // Determine the earlier path.
            let mut earlier_path = review_dir.clone();
            earlier_path.push(inbox_json_filename_by_id(&decision_id));
            return Err(InventoryError::DuplicateId {
                proposal_id: decision_id,
                path_a: earlier_path,
                path_b: path,
            });
        }
    }

    Ok(decisions)
}

/// Collect deterministic list rows from `.scryrs/proposals/`, `.scryrs/accepted/`,
/// and `.scryrs/rejected/`.
pub fn collect_list_rows(repo_root: &Path) -> Result<Vec<ProposalListRow>, InventoryError> {
    let proposals_dir = repo_root.join(".scryrs/proposals");
    if !proposals_dir.exists() || !proposals_dir.is_dir() {
        return Err(InventoryError::MissingDirectory {
            path: proposals_dir,
        });
    }

    let proposals = load_proposals(repo_root)?;
    let accepted = load_review_decisions(repo_root, ReviewOutcome::Accepted)?;
    let rejected = load_review_decisions(repo_root, ReviewOutcome::Rejected)?;

    for proposal_id in accepted.keys() {
        if rejected.contains_key(proposal_id) {
            // Construct representative paths for the error.
            let mut accepted_path = repo_root.join(".scryrs/accepted");
            accepted_path.push(inbox_json_filename_by_id(proposal_id));
            let mut rejected_path = repo_root.join(".scryrs/rejected");
            rejected_path.push(inbox_json_filename_by_id(proposal_id));
            return Err(InventoryError::ConflictingState {
                proposal_id: proposal_id.clone(),
                accepted_path,
                rejected_path,
            });
        }
    }

    let mut rows = Vec::new();
    for (proposal_id, proposal) in proposals {
        let state = if accepted.contains_key(&proposal_id) {
            ProposalState::Accepted
        } else if rejected.contains_key(&proposal_id) {
            ProposalState::Rejected
        } else {
            ProposalState::Pending
        };

        rows.push(ProposalListRow {
            proposal_id,
            title: proposal.title,
            target_type: proposal.target_type,
            created_at: proposal.created_at,
            state,
        });
    }

    rows.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
    Ok(rows)
}

/// Validate semantic invariants of a `ProposalDocument` on disk.
pub fn validate_proposal_document(
    path: &Path,
    proposal: &ProposalDocument,
) -> Result<(), InventoryError> {
    proposal
        .validate()
        .map_err(|error| InventoryError::Validation {
            path: path.to_path_buf(),
            message: error,
        })?;
    validate_rfc3339(&proposal.created_at).map_err(|message| InventoryError::Validation {
        path: path.to_path_buf(),
        message: format!("createdAt {message}"),
    })?;

    let expected_filename = inbox_json_filename(proposal);
    let actual_filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if actual_filename != expected_filename {
        return Err(InventoryError::Validation {
            path: path.to_path_buf(),
            message: format!("filename does not match proposalId '{}'", proposal.id),
        });
    }

    let computed_id =
        ProposalDocument::compute_id(&proposal.target_type, &proposal.proposed_content).map_err(
            |error| InventoryError::Validation {
                path: path.to_path_buf(),
                message: format!("cannot compute deterministic proposalId: {error}"),
            },
        )?;
    if proposal.id != computed_id {
        return Err(InventoryError::Validation {
            path: path.to_path_buf(),
            message: format!(
                "proposalId '{}' does not match targetType/proposedContent",
                proposal.id
            ),
        });
    }

    Ok(())
}

/// Validate a review decision artifact file for structural and naming rules.
pub fn validate_review_decision_artifact(
    path: &Path,
    decision: &ProposalReviewDecision,
    expected_outcome: &ReviewOutcome,
) -> Result<(), InventoryError> {
    decision
        .validate()
        .map_err(|error| InventoryError::Validation {
            path: path.to_path_buf(),
            message: error,
        })?;
    validate_rfc3339(&decision.decided_at).map_err(|message| InventoryError::Validation {
        path: path.to_path_buf(),
        message: format!("decidedAt {message}"),
    })?;

    let expected_filename = inbox_json_filename_by_id(&decision.proposal_id);
    let actual_filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if actual_filename != expected_filename {
        return Err(InventoryError::Validation {
            path: path.to_path_buf(),
            message: format!(
                "filename does not match proposalId '{}'",
                decision.proposal_id
            ),
        });
    }
    if decision.outcome != *expected_outcome {
        return Err(InventoryError::Validation {
            path: path.to_path_buf(),
            message: format!(
                "outcome does not match {} directory",
                review_dir_name(expected_outcome)
            ),
        });
    }
    Ok(())
}

/// Validate that a review decision's fields match the original proposal.
pub fn validate_review_decision_matches_proposal(
    decision: &ProposalReviewDecision,
    proposal: &ProposalDocument,
) -> Result<(), InventoryError> {
    if decision.proposal_id != proposal.id {
        return Err(InventoryError::Validation {
            path: PathBuf::from(format!(".scryrs/proposals/{}.json", decision.proposal_id)),
            message: format!(
                "reviewed artifact proposalId '{}' does not match proposal inbox document '{}'",
                decision.proposal_id, proposal.id
            ),
        });
    }
    if decision.source_evidence != proposal.evidence {
        return Err(InventoryError::Validation {
            path: PathBuf::from(format!(".scryrs/proposals/{}.json", decision.proposal_id)),
            message: format!(
                "reviewed artifact for proposal ID '{}' does not preserve sourceEvidence from the proposal",
                proposal.id
            ),
        });
    }
    match decision.outcome {
        ReviewOutcome::Accepted => {
            if decision.target_type.as_ref() != Some(&proposal.target_type) {
                return Err(InventoryError::Validation {
                    path: PathBuf::from(format!(".scryrs/proposals/{}.json", decision.proposal_id)),
                    message: format!(
                        "reviewed artifact for proposal ID '{}' does not preserve targetType",
                        proposal.id
                    ),
                });
            }
            if !is_markdown_target_type(&proposal.target_type)
                && decision.accepted_content.as_ref() != Some(&proposal.proposed_content)
            {
                return Err(InventoryError::Validation {
                    path: PathBuf::from(format!(".scryrs/proposals/{}.json", decision.proposal_id)),
                    message: format!(
                        "reviewed artifact for proposal ID '{}' does not preserve acceptedContent",
                        proposal.id
                    ),
                });
            }
        }
        ReviewOutcome::Rejected => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>, InventoryError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err(InventoryError::Io {
            path: dir.to_path_buf(),
            message: "expected directory".to_string(),
        });
    }
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|error| InventoryError::Io {
        path: dir.to_path_buf(),
        message: format!("cannot read directory: {error}"),
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| InventoryError::Io {
            path: dir.to_path_buf(),
            message: format!("cannot read directory entry: {error}"),
        })?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn inbox_json_filename(proposal: &ProposalDocument) -> String {
    format!("{}.json", proposal.id)
}

fn inbox_json_filename_by_id(proposal_id: &str) -> String {
    format!("{proposal_id}.json")
}

pub fn review_dir_name(outcome: &ReviewOutcome) -> &'static str {
    match outcome {
        ReviewOutcome::Accepted => "accepted",
        ReviewOutcome::Rejected => "rejected",
    }
}

pub fn is_markdown_target_type(target_type: &ProposalTargetType) -> bool {
    matches!(
        target_type,
        ProposalTargetType::DocsNote
            | ProposalTargetType::Adr
            | ProposalTargetType::Skill
            | ProposalTargetType::DebuggingPlaybook
    )
}

// ---------------------------------------------------------------------------
// RFC 3339 validation (extracted from CLI)
// ---------------------------------------------------------------------------

pub fn validate_rfc3339(value: &str) -> Result<(), String> {
    let (date, time_and_offset) = value
        .split_once('T')
        .ok_or_else(|| "must be RFC3339 (missing 'T')".to_string())?;
    validate_date(date)?;
    validate_time_and_offset(time_and_offset)
}

fn validate_date(date: &str) -> Result<(), String> {
    let mut parts = date.split('-');
    let year = parse_fixed_width_u32(parts.next(), 4, "year")?;
    let month = parse_fixed_width_u32(parts.next(), 2, "month")?;
    let day = parse_fixed_width_u32(parts.next(), 2, "day")?;
    if parts.next().is_some() {
        return Err("must be RFC3339 date (too many date fields)".into());
    }
    if !(1..=12).contains(&month) {
        return Err("must be RFC3339 date (month out of range)".into());
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err("must be RFC3339 date (day out of range)".into());
    }
    Ok(())
}

fn validate_time_and_offset(value: &str) -> Result<(), String> {
    if let Some(prefix) = value.strip_suffix('Z') {
        validate_time(prefix)?;
        return Ok(());
    }

    let offset_index = value
        .rfind(['+', '-'])
        .ok_or_else(|| "must be RFC3339 timestamp with Z or ±HH:MM timezone offset".to_string())?;
    let (time, offset) = value.split_at(offset_index);
    validate_time(time)?;
    validate_offset(offset)
}

fn validate_time(time: &str) -> Result<(), String> {
    let (clock, fraction) = match time.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (time, None),
    };
    let mut parts = clock.split(':');
    let hour = parse_fixed_width_u32(parts.next(), 2, "hour")?;
    let minute = parse_fixed_width_u32(parts.next(), 2, "minute")?;
    let second = parse_fixed_width_u32(parts.next(), 2, "second")?;
    if parts.next().is_some() {
        return Err("must be RFC3339 time (too many time fields)".into());
    }
    if hour > 23 {
        return Err("must be RFC3339 time (hour out of range)".into());
    }
    if minute > 59 {
        return Err("must be RFC3339 time (minute out of range)".into());
    }
    if second > 60 {
        return Err("must be RFC3339 time (second out of range)".into());
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("must be RFC3339 time (invalid fractional seconds)".into());
        }
    }
    Ok(())
}

fn validate_offset(offset: &str) -> Result<(), String> {
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return Err("must be RFC3339 timezone offset (expected ±HH:MM)".into());
    }
    let hour = offset[1..3]
        .parse::<u32>()
        .map_err(|_| "must be RFC3339 timezone offset (invalid offset hour)".to_string())?;
    let minute = offset[4..6]
        .parse::<u32>()
        .map_err(|_| "must be RFC3339 timezone offset (invalid offset minute)".to_string())?;
    if hour > 23 {
        return Err("must be RFC3339 timezone offset (hour out of range)".into());
    }
    if minute > 59 {
        return Err("must be RFC3339 timezone offset (minute out of range)".into());
    }
    Ok(())
}

fn parse_fixed_width_u32(
    value: Option<&str>,
    width: usize,
    field_name: &str,
) -> Result<u32, String> {
    let value = value.ok_or_else(|| format!("must be RFC3339 ({field_name} missing)"))?;
    if value.len() != width || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!(
            "must be RFC3339 ({field_name} has invalid width or characters)"
        ));
    }
    value
        .parse::<u32>()
        .map_err(|_| format!("must be RFC3339 ({field_name} is not numeric)"))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3339_utc_valid() {
        assert!(validate_rfc3339("2026-01-15T12:00:00Z").is_ok());
    }

    #[test]
    fn rfc3339_with_offset_valid() {
        assert!(validate_rfc3339("2026-01-15T12:00:00+05:30").is_ok());
    }

    #[test]
    fn rfc3339_missing_t_rejected() {
        assert!(validate_rfc3339("2026-01-15 12:00:00Z").is_err());
    }

    #[test]
    fn rfc3339_invalid_date_rejected() {
        assert!(validate_rfc3339("2026-13-01T12:00:00Z").is_err());
    }

    #[test]
    fn rfc3339_invalid_time_rejected() {
        assert!(validate_rfc3339("2026-01-15T25:00:00Z").is_err());
    }
}
