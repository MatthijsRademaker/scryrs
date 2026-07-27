use rusqlite::{OptionalExtension, params};
use scryrs_curator::proposals::inventory::{ProposalListRow, ProposalState};
use scryrs_types::{ProposalDocument, ProposalReviewDecision, ReviewOutcome};

use crate::store::ServerStore;

#[derive(Debug)]
pub(crate) enum ProposalRecordError {
    Conflict(String),
    Sql(rusqlite::Error),
}

impl From<rusqlite::Error> for ProposalRecordError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StoredProposal {
    pub(crate) proposal: ProposalDocument,
    pub(crate) revision_sha256: String,
    pub(crate) publisher_id: String,
    pub(crate) published_at: String,
    pub(crate) review_decision: Option<ProposalReviewDecision>,
    pub(crate) review_actor_id: Option<String>,
    pub(crate) review_recorded_at: Option<String>,
    pub(crate) decision_sha256: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ProposalPublication {
    pub(crate) stored: StoredProposal,
    pub(crate) unchanged: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct StoredReview {
    pub(crate) decision: ProposalReviewDecision,
    pub(crate) decision_sha256: String,
    pub(crate) authenticated_actor_id: String,
    pub(crate) recorded_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct ProposalReviewRecord {
    pub(crate) stored: StoredReview,
    pub(crate) unchanged: bool,
}

impl ServerStore {
    pub(crate) fn publish_proposal(
        &self,
        repository_id: &str,
        proposal: &ProposalDocument,
        proposal_json: &str,
        revision_sha256: &str,
        publisher_id: &str,
        published_at: &str,
    ) -> Result<ProposalPublication, ProposalRecordError> {
        if let Some(existing) = self.get_proposal(repository_id, &proposal.id)? {
            if existing.revision_sha256 == revision_sha256 {
                return Ok(ProposalPublication {
                    stored: existing,
                    unchanged: true,
                });
            }
            return Err(ProposalRecordError::Conflict(format!(
                "proposal '{}' already exists with revision {}; refusing revision {}",
                proposal.id, existing.revision_sha256, revision_sha256
            )));
        }

        self.conn.execute(
            "INSERT INTO repository_proposals (
                repository_id, proposal_id, schema_version, proposal_json,
                revision_sha256, publisher_id, published_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                repository_id,
                proposal.id,
                proposal.schema_version,
                proposal_json,
                revision_sha256,
                publisher_id,
                published_at,
            ],
        )?;

        Ok(ProposalPublication {
            stored: StoredProposal {
                proposal: proposal.clone(),
                revision_sha256: revision_sha256.to_owned(),
                publisher_id: publisher_id.to_owned(),
                published_at: published_at.to_owned(),
                review_decision: None,
                review_actor_id: None,
                review_recorded_at: None,
                decision_sha256: None,
            },
            unchanged: false,
        })
    }

    pub(crate) fn record_proposal_review(
        &self,
        repository_id: &str,
        decision: &ProposalReviewDecision,
        decision_json: &str,
        decision_sha256: &str,
        authenticated_actor_id: &str,
        recorded_at: &str,
    ) -> Result<ProposalReviewRecord, ProposalRecordError> {
        if let Some(existing) = self.get_proposal(repository_id, &decision.proposal_id)? {
            if let Some(existing_decision) = existing.review_decision {
                if existing.decision_sha256.as_deref() == Some(decision_sha256) {
                    return Ok(ProposalReviewRecord {
                        stored: StoredReview {
                            decision: existing_decision,
                            decision_sha256: existing
                                .decision_sha256
                                .unwrap_or_else(|| decision_sha256.to_owned()),
                            authenticated_actor_id: existing
                                .review_actor_id
                                .unwrap_or_else(|| authenticated_actor_id.to_owned()),
                            recorded_at: existing
                                .review_recorded_at
                                .unwrap_or_else(|| recorded_at.to_owned()),
                        },
                        unchanged: true,
                    });
                }
                return Err(ProposalRecordError::Conflict(format!(
                    "terminal {} decision already exists for proposal '{}'; refusing overwrite",
                    outcome_name(&existing_decision.outcome),
                    decision.proposal_id
                )));
            }
        }

        self.conn.execute(
            "INSERT INTO repository_proposal_reviews (
                repository_id, proposal_id, schema_version, decision_json,
                decision_sha256, outcome, reviewer, authenticated_actor_id,
                decided_at, recorded_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                repository_id,
                decision.proposal_id,
                decision.schema_version,
                decision_json,
                decision_sha256,
                outcome_name(&decision.outcome),
                decision.reviewer,
                authenticated_actor_id,
                decision.decided_at,
                recorded_at,
            ],
        )?;
        Ok(ProposalReviewRecord {
            stored: StoredReview {
                decision: decision.clone(),
                decision_sha256: decision_sha256.to_owned(),
                authenticated_actor_id: authenticated_actor_id.to_owned(),
                recorded_at: recorded_at.to_owned(),
            },
            unchanged: false,
        })
    }

    pub(crate) fn list_proposals(
        &self,
        repository_id: &str,
    ) -> rusqlite::Result<Vec<ProposalListRow>> {
        let mut statement = self.conn.prepare(
            "SELECT p.proposal_json, r.outcome
             FROM repository_proposals p
             LEFT JOIN repository_proposal_reviews r
               ON r.repository_id = p.repository_id
              AND r.proposal_id = p.proposal_id
             WHERE p.repository_id = ?1
             ORDER BY p.proposal_id ASC",
        )?;
        statement
            .query_map([repository_id], |row| {
                let proposal_json: String = row.get(0)?;
                let outcome: Option<String> = row.get(1)?;
                let proposal: ProposalDocument = serde_json::from_str(&proposal_json)
                    .map_err(|error| json_read_error(0, error))?;
                let state = match outcome.as_deref() {
                    None => ProposalState::Pending,
                    Some("accepted") => ProposalState::Accepted,
                    Some("rejected") => ProposalState::Rejected,
                    Some(other) => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            1,
                            other.to_owned(),
                            rusqlite::types::Type::Text,
                        ));
                    }
                };
                Ok(ProposalListRow {
                    proposal_id: proposal.id,
                    title: proposal.title,
                    target_type: proposal.target_type,
                    created_at: proposal.created_at,
                    state,
                })
            })?
            .collect()
    }

    pub(crate) fn get_proposal(
        &self,
        repository_id: &str,
        proposal_id: &str,
    ) -> rusqlite::Result<Option<StoredProposal>> {
        self.conn
            .query_row(
                "SELECT p.proposal_json, p.revision_sha256, p.publisher_id,
                        p.published_at, r.decision_json, r.authenticated_actor_id,
                        r.recorded_at, r.decision_sha256
                 FROM repository_proposals p
                 LEFT JOIN repository_proposal_reviews r
                   ON r.repository_id = p.repository_id
                  AND r.proposal_id = p.proposal_id
                 WHERE p.repository_id = ?1 AND p.proposal_id = ?2",
                params![repository_id, proposal_id],
                |row| {
                    let proposal_json: String = row.get(0)?;
                    let decision_json: Option<String> = row.get(4)?;
                    Ok(StoredProposal {
                        proposal: serde_json::from_str(&proposal_json)
                            .map_err(|error| json_read_error(0, error))?,
                        revision_sha256: row.get(1)?,
                        publisher_id: row.get(2)?,
                        published_at: row.get(3)?,
                        review_decision: decision_json
                            .map(|json| {
                                serde_json::from_str(&json)
                                    .map_err(|error| json_read_error(4, error))
                            })
                            .transpose()?,
                        review_actor_id: row.get(5)?,
                        review_recorded_at: row.get(6)?,
                        decision_sha256: row.get(7)?,
                    })
                },
            )
            .optional()
    }
}

pub(crate) fn outcome_name(outcome: &ReviewOutcome) -> &'static str {
    match outcome {
        ReviewOutcome::Accepted => "accepted",
        ReviewOutcome::Rejected => "rejected",
    }
}

fn json_read_error(index: usize, error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(error))
}
