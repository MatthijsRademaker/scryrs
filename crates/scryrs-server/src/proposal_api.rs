use axum::Json;
use axum::body::{Body, to_bytes};
use axum::extract::{Path, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use scryrs_types::{ProposalDocument, ProposalReviewDecision, ProposedContent, ReviewOutcome};
use serde::{Deserialize, Serialize};

use crate::proposal_records::{ProposalRecordError, StoredProposal};
use crate::server::{
    AppState, AuthorizedProposalWriter, constant_time_eq, sha256_hex, token_sha256,
};
use crate::time::chrono_now;

/// Maximum accepted proposal publication body: 1 MiB.
pub const MAX_PROPOSAL_BYTES: usize = 1024 * 1024;
/// Maximum accepted proposal review body: 64 KiB.
pub const MAX_PROPOSAL_REVIEW_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalPublicationResponse {
    repository_id: String,
    proposal_id: String,
    schema_version: String,
    revision_sha256: String,
    publisher_id: String,
    published_at: String,
    unchanged: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicationAudit {
    revision_sha256: String,
    publisher_id: String,
    published_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewAudit {
    authenticated_actor_id: String,
    recorded_at: String,
    decision_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalDetailResponse {
    #[serde(flatten)]
    proposal: ProposalDocument,
    review_decision: Option<ProposalReviewDecision>,
    publication: PublicationAudit,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_audit: Option<ReviewAudit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposalReviewRequest {
    reviewer: String,
    rationale: String,
    decided_at: String,
    reviewed_content: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalReviewResponse {
    outcome: ReviewOutcome,
    reviewer: String,
    authenticated_actor_id: String,
    decided_at: String,
    recorded_at: String,
    decision_sha256: String,
    unchanged: bool,
}

pub(crate) async fn publish_proposal(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    request: Request<Body>,
) -> Response {
    let writer = match authorize(&state, &repository_id, &request) {
        Ok(writer) => writer,
        Err(response) => return response,
    };
    let body = match to_bytes(request.into_body(), MAX_PROPOSAL_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("proposal body exceeds {MAX_PROPOSAL_BYTES} bytes"),
            );
        }
    };
    let proposal: ProposalDocument = match serde_json::from_slice(&body) {
        Ok(proposal) => proposal,
        Err(error) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("invalid proposal JSON: {error}"),
            );
        }
    };
    let proposal_json = match scryrs_curator::proposals::wire::serialize_proposal(&proposal) {
        Ok(json) => json,
        Err(error) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, error),
    };
    let revision_sha256 = sha256_hex(proposal_json.as_bytes());
    let published_at = chrono_now();
    let publication = match state
        .store
        .lock()
        .map_err(|error| error.to_string())
        .and_then(|store| {
            store
                .publish_proposal(
                    &repository_id,
                    &proposal,
                    &proposal_json,
                    &revision_sha256,
                    &writer.actor_id,
                    &published_at,
                )
                .map_err(|error| match error {
                    ProposalRecordError::Conflict(message) => format!("conflict:{message}"),
                    ProposalRecordError::Sql(error) => format!("storage:{error}"),
                })
        }) {
        Ok(publication) => publication,
        Err(error) if error.starts_with("conflict:") => {
            return error_response(StatusCode::CONFLICT, error.trim_start_matches("conflict:"));
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("proposal storage failed: {error}"),
            );
        }
    };
    let status = if publication.unchanged {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    let stored = publication.stored;
    (
        status,
        Json(ProposalPublicationResponse {
            repository_id,
            proposal_id: stored.proposal.id,
            schema_version: stored.proposal.schema_version,
            revision_sha256: stored.revision_sha256,
            publisher_id: stored.publisher_id,
            published_at: stored.published_at,
            unchanged: publication.unchanged,
        }),
    )
        .into_response()
}

pub(crate) async fn list_proposals(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
) -> Response {
    match state
        .store
        .lock()
        .map_err(|error| error.to_string())
        .and_then(|store| {
            store
                .list_proposals(&repository_id)
                .map_err(|error| error.to_string())
        }) {
        Ok(rows) => Json(rows).into_response(),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("proposal inventory failed: {error}"),
        ),
    }
}

pub(crate) async fn get_proposal(
    State(state): State<AppState>,
    Path((repository_id, proposal_id)): Path<(String, String)>,
) -> Response {
    let stored = match state
        .store
        .lock()
        .map_err(|error| error.to_string())
        .and_then(|store| {
            store
                .get_proposal(&repository_id, &proposal_id)
                .map_err(|error| error.to_string())
        }) {
        Ok(Some(stored)) => stored,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                format!("proposal not found: {proposal_id}"),
            );
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("proposal detail failed: {error}"),
            );
        }
    };
    Json(detail_response(stored)).into_response()
}

pub(crate) async fn accept_proposal(
    State(state): State<AppState>,
    Path((repository_id, proposal_id)): Path<(String, String)>,
    request: Request<Body>,
) -> Response {
    review_proposal(
        state,
        repository_id,
        proposal_id,
        ReviewOutcome::Accepted,
        request,
    )
    .await
}

pub(crate) async fn reject_proposal(
    State(state): State<AppState>,
    Path((repository_id, proposal_id)): Path<(String, String)>,
    request: Request<Body>,
) -> Response {
    review_proposal(
        state,
        repository_id,
        proposal_id,
        ReviewOutcome::Rejected,
        request,
    )
    .await
}

async fn review_proposal(
    state: AppState,
    repository_id: String,
    proposal_id: String,
    outcome: ReviewOutcome,
    request: Request<Body>,
) -> Response {
    let writer = match authorize(&state, &repository_id, &request) {
        Ok(writer) => writer,
        Err(response) => return response,
    };
    let body = match to_bytes(request.into_body(), MAX_PROPOSAL_REVIEW_BYTES).await {
        Ok(body) => body,
        Err(_) => {
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("proposal review body exceeds {MAX_PROPOSAL_REVIEW_BYTES} bytes"),
            );
        }
    };
    let body: ProposalReviewRequest = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(error) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("invalid proposal review JSON: {error}"),
            );
        }
    };
    if body.reviewer != writer.actor_id {
        return error_response(
            StatusCode::FORBIDDEN,
            format!(
                "reviewer '{}' does not match authenticated actor '{}'",
                body.reviewer, writer.actor_id
            ),
        );
    }
    if outcome == ReviewOutcome::Rejected && body.reviewed_content.is_some() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "reviewedContent is not supported on reject",
        );
    }
    let proposal = match state
        .store
        .lock()
        .map_err(|error| error.to_string())
        .and_then(|store| {
            store
                .get_proposal(&repository_id, &proposal_id)
                .map_err(|error| error.to_string())
        }) {
        Ok(Some(stored)) => stored.proposal,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                format!("proposal not found: {proposal_id}"),
            );
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("proposal detail failed: {error}"),
            );
        }
    };
    let decision = match scryrs_curator::proposals::wire::build_review_decision(
        &proposal,
        &scryrs_curator::proposals::review_write::ReviewWriteRequest {
            proposal_id,
            outcome,
            reviewer: body.reviewer,
            rationale: body.rationale,
            decided_at: body.decided_at,
            override_content: body.reviewed_content.map(ProposedContent::Markdown),
        },
    ) {
        Ok(decision) => decision,
        Err(error) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, error.to_string()),
    };
    let decision_json = match scryrs_curator::proposals::wire::serialize_review_decision(&decision)
    {
        Ok(json) => json,
        Err(error) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, error),
    };
    let decision_sha256 = sha256_hex(decision_json.as_bytes());
    let recorded_at = chrono_now();
    let record = match state
        .store
        .lock()
        .map_err(|error| error.to_string())
        .and_then(|store| {
            store
                .record_proposal_review(
                    &repository_id,
                    &decision,
                    &decision_json,
                    &decision_sha256,
                    &writer.actor_id,
                    &recorded_at,
                )
                .map_err(|error| match error {
                    ProposalRecordError::Conflict(message) => format!("conflict:{message}"),
                    ProposalRecordError::Sql(error) => format!("storage:{error}"),
                })
        }) {
        Ok(record) => record,
        Err(error) if error.starts_with("conflict:") => {
            return error_response(StatusCode::CONFLICT, error.trim_start_matches("conflict:"));
        }
        Err(error) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("proposal review storage failed: {error}"),
            );
        }
    };
    let status = if record.unchanged {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    let stored = record.stored;
    (
        status,
        Json(ProposalReviewResponse {
            outcome: stored.decision.outcome,
            reviewer: stored.decision.reviewer,
            authenticated_actor_id: stored.authenticated_actor_id,
            decided_at: stored.decision.decided_at,
            recorded_at: stored.recorded_at,
            decision_sha256: stored.decision_sha256,
            unchanged: record.unchanged,
        }),
    )
        .into_response()
}

fn detail_response(stored: StoredProposal) -> ProposalDetailResponse {
    let review_audit = match (
        stored.review_actor_id,
        stored.review_recorded_at,
        stored.decision_sha256,
    ) {
        (Some(authenticated_actor_id), Some(recorded_at), Some(decision_sha256)) => {
            Some(ReviewAudit {
                authenticated_actor_id,
                recorded_at,
                decision_sha256,
            })
        }
        _ => None,
    };
    ProposalDetailResponse {
        proposal: stored.proposal,
        review_decision: stored.review_decision,
        publication: PublicationAudit {
            revision_sha256: stored.revision_sha256,
            publisher_id: stored.publisher_id,
            published_at: stored.published_at,
        },
        review_audit,
    }
}

fn authorize<'a>(
    state: &'a AppState,
    repository_id: &str,
    request: &Request<Body>,
) -> Result<&'a AuthorizedProposalWriter, Response> {
    if state.proposal_writers.is_empty() {
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "proposal writes are disabled; configure SCRYRS_PROPOSAL_WRITE_CREDENTIALS",
        ));
    }
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            error_response(
                StatusCode::UNAUTHORIZED,
                "proposal writes require Authorization: Bearer <token>",
            )
        })?;
    let digest = token_sha256(token);
    let matching = state
        .proposal_writers
        .iter()
        .filter(|writer| constant_time_eq(&digest, &writer.token_sha256))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid proposal write bearer token",
        ));
    }
    matching
        .into_iter()
        .find(|writer| writer.repository_id == repository_id)
        .ok_or_else(|| {
            error_response(
                StatusCode::FORBIDDEN,
                format!("authenticated actor does not own repository '{repository_id}'"),
            )
        })
}

fn error_response(status: StatusCode, error: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorBody {
            error: error.into(),
        }),
    )
        .into_response()
}
