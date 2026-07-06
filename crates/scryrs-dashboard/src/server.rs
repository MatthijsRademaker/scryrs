use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::header::{CONTENT_TYPE, HeaderValue};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::{Connection, OpenFlags, params};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::route_explain::route_explain;
use crate::{Config, DashboardError, SourceMode};

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct EmbeddedAssets;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Config,
    pub(crate) http_client: reqwest::Client,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug)]
pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) message: String,
}

impl ApiError {
    pub(crate) fn missing(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub(crate) fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorBody {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LimitQuery {
    limit: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventQuery {
    limit: Option<u32>,
    cursor: Option<String>,
    session_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SignalQuery {
    after: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaResponse {
    mode: &'static str,
    repository_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    session_id: String,
    started_at: String,
    ended_at: Option<String>,
    event_count: u64,
    source: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceEventItem {
    event_id: u64,
    session_id: String,
    event_type: String,
    timestamp: String,
    subject_kind: Option<String>,
    subject: Option<String>,
    payload: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsPage {
    events: Vec<TraceEventItem>,
    next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    session: SessionSummary,
    events: Vec<TraceEventItem>,
}

// --- Proposal DTOs ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalListRow {
    pub proposal_id: String,
    pub title: String,
    pub target_type: scryrs_types::ProposalTargetType,
    pub created_at: String,
    pub state: ProposalStateStr,
}

// --- Accepted Knowledge DTOs ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishSurfaceStatus {
    pub surface: PublishSurface,
    pub status: PublishStatusStr,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishSurface {
    Rspress,
    Markdown,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishStatusStr {
    Published,
    #[serde(rename = "not_published")]
    NotPublished,
    #[serde(rename = "not_publishable")]
    NotPublishable,
    Unknown,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedItemListRow {
    pub proposal_id: String,
    pub title: String,
    pub target_type: scryrs_types::ProposalTargetType,
    pub reviewer: String,
    pub decided_at: String,
    pub evidence_summary: Vec<serde_json::Value>,
    pub publish_status: Vec<PublishSurfaceStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedItemDetail {
    pub proposal_id: String,
    pub title: String,
    pub target_type: scryrs_types::ProposalTargetType,
    pub accepted_content: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_proposed_content: Option<serde_json::Value>,
    pub reviewer: String,
    pub rationale: String,
    pub decided_at: String,
    pub evidence: Vec<scryrs_types::EvidenceLink>,
    pub publish_status: Vec<PublishSurfaceStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStateStr {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposalDetail {
    pub schema_version: String,
    pub id: String,
    pub target_type: scryrs_types::ProposalTargetType,
    pub title: String,
    pub rationale: String,
    pub proposed_content: serde_json::Value,
    pub evidence: Vec<scryrs_types::EvidenceLink>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_decision: Option<ReviewDecisionMeta>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDecisionMeta {
    pub reviewer: String,
    pub outcome: String,
    pub decided_at: String,
    pub rationale: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<scryrs_types::ProposalTargetType>,
}

// --- Router ---

pub fn router(config: Config) -> Router {
    let state = Arc::new(AppState {
        config,
        http_client: reqwest::Client::new(),
    });
    Router::new()
        .route("/", get(index))
        .route("/assets/*path", get(asset))
        .route("/api/meta", get(meta))
        .route("/api/hotspots", get(hotspots))
        .route("/api/signals", get(signals))
        .route("/api/sessions", get(sessions))
        .route("/api/sessions/:session_id", get(session_detail))
        .route("/api/events", get(events))
        .route("/api/proposals", get(proposals_list))
        .route("/api/proposals/:proposal_id", get(proposal_detail))
        .route("/api/accepted", get(accepted_list))
        .route("/api/accepted/:proposal_id", get(accepted_detail))
        .route("/api/routes/explain", get(route_explain))
        .route("/api/*path", get(api_not_found))
        .fallback(spa_fallback)
        .with_state(state)
}

pub async fn serve(config: Config) -> Result<(), DashboardError> {
    let bind = SocketAddr::new(config.bind_address, config.port);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    let url = format!("http://{}:{}", addr.ip(), addr.port());
    write_startup_message(&url, config.dev_mode)?;
    if !config.no_open {
        open_browser(&url);
    }
    axum::serve(listener, router(config)).await?;
    Ok(())
}

fn write_startup_message(url: &str, dev_mode: bool) -> Result<(), std::io::Error> {
    use std::io::Write;
    let suffix = if dev_mode { " (dev mode)" } else { "" };
    writeln!(
        std::io::stderr().lock(),
        "Dashboard available at <{url}>{suffix}"
    )
}

#[allow(clippy::disallowed_methods)]
fn open_browser(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        cmd.arg(url);
        cmd
    } else if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", url]);
        cmd
    } else {
        let mut cmd = Command::new("xdg-open");
        cmd.arg(url);
        cmd
    };
    let _ = command.spawn();
}

async fn meta(State(state): State<Arc<AppState>>) -> Json<MetaResponse> {
    Json(MetaResponse {
        mode: state.config.source_mode.label(),
        repository_path: state.config.repo_root.to_string_lossy().into_owned(),
        repository_id: state
            .config
            .source_mode
            .live_config()
            .map(|config| config.repository_id.clone()),
    })
}

async fn signals(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SignalQuery>,
) -> Result<Response<Body>, ApiError> {
    match &state.config.source_mode {
        SourceMode::Local => Err(ApiError::missing("signal stream unavailable in local mode")),
        SourceMode::Live(live) => proxy_live_signals(&state.http_client, live, query.after).await,
    }
}

async fn hotspots(State(state): State<Arc<AppState>>) -> Result<Response<Body>, ApiError> {
    if let Some(live) = state.config.source_mode.live_config() {
        return proxy_live_hotspots(&state.http_client, live).await;
    }

    let path = state.config.repo_root.join(".scryrs").join("hotspots.json");
    let bytes = std::fs::read(&path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ApiError::missing("hotspot report missing at .scryrs/hotspots.json")
        } else {
            ApiError::bad_gateway(format!("cannot read hotspot report: {err}"))
        }
    })?;
    Ok(bytes_response(bytes, "application/json"))
}

async fn sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<SessionSummary>>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing("/api/sessions unavailable in live mode"));
    }

    let db_path = db_path(&state.config.repo_root);
    let limit = normalize_limit(query.limit, 50);
    let rows = run_blocking(move || query_sessions(db_path, limit)).await?;
    Ok(Json(rows))
}

async fn session_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<SessionDetail>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing(
            "/api/sessions/:session_id unavailable in live mode",
        ));
    }

    let db_path = db_path(&state.config.repo_root);
    let detail = run_blocking(move || query_session_detail(db_path, session_id)).await?;
    Ok(Json(detail))
}

async fn events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EventQuery>,
) -> Result<Json<EventsPage>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing("/api/events unavailable in live mode"));
    }

    let db_path = db_path(&state.config.repo_root);
    let limit = normalize_limit(query.limit, 50);
    let cursor = parse_cursor(query.cursor)?;
    let page = run_blocking(move || query_events(db_path, limit, cursor, query.session_id)).await?;
    Ok(Json(page))
}

async fn accepted_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AcceptedItemListRow>>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing("/api/accepted unavailable in live mode"));
    }

    let repo_root = state.config.repo_root.clone();
    let rows = run_blocking(move || {
        let accepted_dir = repo_root.join(".scryrs/accepted");
        if !accepted_dir.exists() || !accepted_dir.is_dir() {
            return Err(ApiError::missing(
                "accepted decisions directory not found: .scryrs/accepted",
            ));
        }
        let proposals =
            scryrs_curator::proposals::inventory::load_proposals(&repo_root)
                .map_err(|err| {
                    // MissingDirectory maps to 404; everything else to 502.
                    map_inventory_error(err)
                })?;
        let accepted =
            scryrs_curator::proposals::inventory::load_review_decisions(
                &repo_root,
                scryrs_types::ReviewOutcome::Accepted,
            )
            .map_err(map_inventory_error)?;

        // Detect conflicting accepted + rejected artifacts for the same ID.
        let rejected_dir = repo_root.join(".scryrs/rejected");
        if rejected_dir.exists() && rejected_dir.is_dir() {
            for proposal_id in accepted.keys() {
                let rejected_path = rejected_dir.join(format!("{proposal_id}.json"));
                if rejected_path.exists() {
                    let accepted_path = repo_root
                        .join(".scryrs/accepted")
                        .join(format!("{proposal_id}.json"));
                    return Err(ApiError::bad_gateway(format!(
                        "conflicting terminal state for proposal ID '{}': accepted at {} and rejected at {}",
                        proposal_id,
                        accepted_path.display(),
                        rejected_path.display()
                    )));
                }
            }
        }

        let mut rows: Vec<AcceptedItemListRow> = Vec::new();
        for (proposal_id, decision) in &accepted {
            let proposal = match proposals.get(proposal_id) {
                Some(p) => p,
                None => continue,
            };
            let evidence_summary: Vec<serde_json::Value> = decision
                .source_evidence
                .iter()
                .map(|link| {
                    serde_json::json!({
                        "sourceKind": link.source_kind,
                        "subject": link.subject,
                        "rowIds": link.row_ids
                    })
                })
                .collect();
            let publish_status = compute_publish_status(&repo_root, proposal_id, &decision.target_type);
            rows.push(AcceptedItemListRow {
                proposal_id: proposal_id.clone(),
                title: proposal.title.clone(),
                target_type: proposal.target_type.clone(),
                reviewer: decision.reviewer.clone(),
                decided_at: decision.decided_at.clone(),
                evidence_summary,
                publish_status,
            });
        }
        rows.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));
        Ok(rows)
    })
    .await?;
    Ok(Json(rows))
}

async fn accepted_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(proposal_id): AxumPath<String>,
) -> Result<Json<AcceptedItemDetail>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing(
            "/api/accepted/:proposal_id unavailable in live mode",
        ));
    }

    let repo_root = state.config.repo_root.clone();
    let detail = run_blocking(move || {
        let proposals = scryrs_curator::proposals::inventory::load_proposals(&repo_root)
            .map_err(map_inventory_error)?;
        let accepted = scryrs_curator::proposals::inventory::load_review_decisions(
            &repo_root,
            scryrs_types::ReviewOutcome::Accepted,
        )
        .map_err(map_inventory_error)?;

        let decision = accepted.get(&proposal_id).ok_or_else(|| {
            ApiError::missing(format!(
                "proposal not found among accepted decisions: {proposal_id}"
            ))
        })?;
        let proposal = match proposals.get(&proposal_id) {
            Some(p) => p,
            None => {
                return Err(ApiError::bad_gateway(format!(
                    "orphan accepted artifact: no matching proposal for {proposal_id}"
                )));
            }
        };

        let accepted_content = decision
            .accepted_content
            .as_ref()
            .and_then(|content| serde_json::to_value(content).ok())
            .unwrap_or(serde_json::Value::Null);

        let proposed_content = serde_json::to_value(&proposal.proposed_content).ok();

        let original_proposed_content = proposed_content.as_ref().and_then(|pc| {
            if pc == &accepted_content {
                None
            } else {
                Some(pc.clone())
            }
        });

        let publish_status =
            compute_publish_status(&repo_root, &proposal_id, &decision.target_type);

        Ok(AcceptedItemDetail {
            proposal_id: proposal_id.clone(),
            title: proposal.title.clone(),
            target_type: proposal.target_type.clone(),
            accepted_content,
            original_proposed_content,
            reviewer: decision.reviewer.clone(),
            rationale: decision.rationale.clone(),
            decided_at: decision.decided_at.clone(),
            evidence: decision.source_evidence.clone(),
            publish_status,
        })
    })
    .await?;
    Ok(Json(detail))
}

fn compute_publish_status(
    repo_root: &std::path::Path,
    proposal_id: &str,
    target_type: &Option<scryrs_types::ProposalTargetType>,
) -> Vec<PublishSurfaceStatus> {
    let mut statuses = Vec::new();

    // Rspress status
    let rspress_status = compute_rspress_status(repo_root, proposal_id, target_type);
    statuses.push(rspress_status);

    // Markdown status — always unknown
    statuses.push(PublishSurfaceStatus {
        surface: PublishSurface::Markdown,
        status: PublishStatusStr::Unknown,
        reason: "output root not persisted — publish path is operator-chosen at publish time"
            .to_string(),
        path: None,
    });

    statuses
}

fn compute_rspress_status(
    repo_root: &std::path::Path,
    proposal_id: &str,
    target_type: &Option<scryrs_types::ProposalTargetType>,
) -> PublishSurfaceStatus {
    let rspress_slug = match target_type {
        Some(tt) => target_type_slug(tt),
        None => {
            return PublishSurfaceStatus {
                surface: PublishSurface::Rspress,
                status: PublishStatusStr::NotPublishable,
                reason: "target type unknown — accepted artifact missing targetType".to_string(),
                path: None,
            };
        }
    };

    match target_type {
        Some(
            scryrs_types::ProposalTargetType::MemoryPatch
            | scryrs_types::ProposalTargetType::SemanticGraphGrouping,
        ) => PublishSurfaceStatus {
            surface: PublishSurface::Rspress,
            status: PublishStatusStr::NotPublishable,
            reason: format!("target type '{rspress_slug}' is not publishable to Rspress"),
            path: None,
        },
        _ => {
            let expected_path = repo_root
                .join(".devagent/docs/docs/accepted-knowledge")
                .join(rspress_slug)
                .join(format!("{proposal_id}.md"));
            if expected_path.exists() {
                PublishSurfaceStatus {
                    surface: PublishSurface::Rspress,
                    status: PublishStatusStr::Published,
                    reason: String::new(),
                    path: Some(
                        expected_path
                            .strip_prefix(repo_root)
                            .unwrap_or(&expected_path)
                            .to_string_lossy()
                            .into_owned(),
                    ),
                }
            } else {
                PublishSurfaceStatus {
                    surface: PublishSurface::Rspress,
                    status: PublishStatusStr::NotPublished,
                    reason: String::new(),
                    path: None,
                }
            }
        }
    }
}

fn target_type_slug(target_type: &scryrs_types::ProposalTargetType) -> &'static str {
    match target_type {
        scryrs_types::ProposalTargetType::DocsNote => "docs_note",
        scryrs_types::ProposalTargetType::Adr => "adr",
        scryrs_types::ProposalTargetType::Skill => "skill",
        scryrs_types::ProposalTargetType::DebuggingPlaybook => "debugging_playbook",
        scryrs_types::ProposalTargetType::MemoryPatch => "memory_patch",
        scryrs_types::ProposalTargetType::SemanticGraphGrouping => "semantic_graph_grouping",
    }
}

async fn proposals_list(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProposalListRow>>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing("/api/proposals unavailable in live mode"));
    }

    let repo_root = state.config.repo_root.clone();
    let rows = run_blocking(move || {
        let curator_rows = scryrs_curator::proposals::inventory::collect_list_rows(&repo_root)
            .map_err(map_inventory_error)?;
        Ok(curator_rows
            .into_iter()
            .map(|r| ProposalListRow {
                proposal_id: r.proposal_id,
                title: r.title,
                target_type: r.target_type,
                created_at: r.created_at,
                state: match r.state {
                    scryrs_curator::proposals::inventory::ProposalState::Pending => {
                        ProposalStateStr::Pending
                    }
                    scryrs_curator::proposals::inventory::ProposalState::Accepted => {
                        ProposalStateStr::Accepted
                    }
                    scryrs_curator::proposals::inventory::ProposalState::Rejected => {
                        ProposalStateStr::Rejected
                    }
                },
            })
            .collect::<Vec<_>>())
    })
    .await?;
    Ok(Json(rows))
}

async fn proposal_detail(
    State(state): State<Arc<AppState>>,
    AxumPath(proposal_id): AxumPath<String>,
) -> Result<Json<ProposalDetail>, ApiError> {
    if state.config.source_mode.live_config().is_some() {
        return Err(ApiError::missing(
            "/api/proposals/:proposal_id unavailable in live mode",
        ));
    }

    let repo_root = state.config.repo_root.clone();
    let detail = run_blocking(move || load_proposal_detail(&repo_root, &proposal_id)).await?;
    Ok(Json(detail))
}

fn load_proposal_detail(
    repo_root: &std::path::Path,
    proposal_id: &str,
) -> Result<ProposalDetail, ApiError> {
    use scryrs_types::ProposedContent;

    // Use the shared inventory loader so validation is identical to the list
    // endpoint.  load_proposals validates every proposal document in the inbox
    // (schema version, filename match, deterministic-id match, RFC 3339
    // timestamps) and detects duplicate proposal IDs.
    let proposals = scryrs_curator::proposals::inventory::load_proposals(repo_root)
        .map_err(map_inventory_error)?;

    let proposal = proposals
        .get(proposal_id)
        .ok_or_else(|| ApiError::missing(format!("proposal not found: {proposal_id}")))?;

    let proposed_content_json = match &proposal.proposed_content {
        ProposedContent::Markdown(text) => serde_json::Value::String(text.clone()),
        ProposedContent::SemanticGraphGrouping(g) => serde_json::to_value(g).map_err(|err| {
            ApiError::bad_gateway(format!("cannot serialize proposed content: {err}"))
        })?,
        ProposedContent::MemoryPatch(v) => v.clone(),
    };

    let review_decision = load_optional_review_decision(repo_root, proposal)?;

    Ok(ProposalDetail {
        schema_version: proposal.schema_version.clone(),
        id: proposal.id.clone(),
        target_type: proposal.target_type.clone(),
        title: proposal.title.clone(),
        rationale: proposal.rationale.clone(),
        proposed_content: proposed_content_json,
        evidence: proposal.evidence.clone(),
        created_at: proposal.created_at.clone(),
        review_decision,
    })
}

fn load_optional_review_decision(
    repo_root: &std::path::Path,
    proposal: &scryrs_types::ProposalDocument,
) -> Result<Option<ReviewDecisionMeta>, ApiError> {
    use scryrs_types::{ProposalReviewDecision, ReviewOutcome};

    let accepted_path = repo_root
        .join(".scryrs/accepted")
        .join(format!("{}.json", proposal.id));
    let rejected_path = repo_root
        .join(".scryrs/rejected")
        .join(format!("{}.json", proposal.id));

    let (decision_path, outcome) = if accepted_path.is_file() && rejected_path.is_file() {
        return Err(ApiError::bad_gateway(format!(
            "conflicting terminal state for proposal ID '{}': accepted and rejected artifacts both exist",
            proposal.id
        )));
    } else if accepted_path.is_file() {
        (accepted_path, ReviewOutcome::Accepted)
    } else if rejected_path.is_file() {
        (rejected_path, ReviewOutcome::Rejected)
    } else {
        return Ok(None);
    };

    let json = std::fs::read_to_string(&decision_path).map_err(|err| {
        ApiError::bad_gateway(format!(
            "cannot read review decision {}: {err}",
            decision_path.display()
        ))
    })?;
    let decision: ProposalReviewDecision = serde_json::from_str(&json).map_err(|err| {
        ApiError::bad_gateway(format!(
            "invalid reviewed artifact {}: {err}",
            decision_path.display()
        ))
    })?;

    scryrs_curator::proposals::inventory::validate_review_decision_artifact(
        &decision_path,
        &decision,
        &outcome,
    )
    .map_err(map_inventory_error)?;

    scryrs_curator::proposals::inventory::validate_review_decision_matches_proposal(
        &decision, proposal,
    )
    .map_err(map_inventory_error)?;

    let outcome_str = serde_json::to_string(&decision.outcome)
        .unwrap_or_else(|_| String::from("unknown"))
        .trim_matches('"')
        .to_string();

    let accepted_content = decision
        .accepted_content
        .as_ref()
        .and_then(|content| serde_json::to_value(content).ok());

    Ok(Some(ReviewDecisionMeta {
        reviewer: decision.reviewer,
        outcome: outcome_str,
        decided_at: decision.decided_at,
        rationale: decision.rationale,
        accepted_content,
        target_type: decision.target_type,
    }))
}

fn map_inventory_error(err: scryrs_curator::proposals::inventory::InventoryError) -> ApiError {
    use scryrs_curator::proposals::inventory::InventoryError;
    match &err {
        InventoryError::MissingDirectory { path: _ } => ApiError::missing(format!("{err}")),
        InventoryError::Io { .. }
        | InventoryError::Parse { .. }
        | InventoryError::Validation { .. }
        | InventoryError::DuplicateId { .. }
        | InventoryError::ConflictingState { .. }
        | InventoryError::OrphanReview { .. } => ApiError::bad_gateway(format!("{err}")),
    }
}

async fn run_blocking<T>(
    f: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError>
where
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|err| ApiError::bad_gateway(format!("dashboard query task failed: {err}")))?
}

async fn proxy_live_hotspots(
    client: &reqwest::Client,
    live: &crate::LiveSourceConfig,
) -> Result<Response<Body>, ApiError> {
    let response = client
        .get(live_api_url(
            live,
            &["v1", "repositories", &live.repository_id, "hotspots"],
        )?)
        .query(&[("window", "cumulative")])
        .send()
        .await
        .map_err(|err| {
            ApiError::bad_gateway(format!("live hotspots upstream request failed: {err}"))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("upstream response body unavailable"));
        return Err(ApiError::bad_gateway(format!(
            "live hotspots upstream returned {status}: {body}"
        )));
    }

    let bytes = response.bytes().await.map_err(|err| {
        ApiError::bad_gateway(format!("live hotspots upstream body read failed: {err}"))
    })?;
    Ok(bytes_response(bytes.to_vec(), "application/json"))
}

async fn proxy_live_signals(
    client: &reqwest::Client,
    live: &crate::LiveSourceConfig,
    after: Option<String>,
) -> Result<Response<Body>, ApiError> {
    let mut request = client.get(live_api_url(
        live,
        &["v1", "repositories", &live.repository_id, "signals"],
    )?);
    if let Some(after) = after.as_deref() {
        request = request.query(&[("after", after)]);
    }

    let response = request.send().await.map_err(|err| {
        ApiError::bad_gateway(format!("live signals upstream request failed: {err}"))
    })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("upstream response body unavailable"));
        return Err(ApiError::bad_gateway(format!(
            "live signals upstream returned {status}: {body}"
        )));
    }

    let mut proxied = Response::new(Body::from_stream(response.bytes_stream()));
    proxied
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
    Ok(proxied)
}

fn live_api_url(
    live: &crate::LiveSourceConfig,
    path_segments: &[&str],
) -> Result<reqwest::Url, ApiError> {
    let mut url = reqwest::Url::parse(&live.server_url)
        .map_err(|err| ApiError::bad_gateway(format!("invalid live server URL: {err}")))?;
    let mut segments = url.path_segments_mut().map_err(|_| {
        ApiError::bad_gateway("invalid live server URL: cannot append path segments")
    })?;
    segments.pop_if_empty();
    for segment in path_segments {
        segments.push(segment);
    }
    drop(segments);
    Ok(url)
}

fn db_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".scryrs").join("scryrs.db")
}

fn normalize_limit(limit: Option<u32>, default: u32) -> u32 {
    limit.unwrap_or(default).clamp(1, 500)
}

fn parse_cursor(cursor: Option<String>) -> Result<Option<u64>, ApiError> {
    cursor
        .map(|raw| {
            raw.parse::<u64>()
                .map_err(|err| ApiError::bad_gateway(format!("invalid cursor: {err}")))
        })
        .transpose()
}

fn open_readonly(db_path: &Path) -> Result<Connection, ApiError> {
    if !db_path.exists() {
        return Err(ApiError::missing(
            "scryrs datastore missing at .scryrs/scryrs.db",
        ));
    }
    Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|err| ApiError::bad_gateway(format!("cannot open scryrs datastore: {err}")))
}

fn query_sessions(db_path: PathBuf, limit: u32) -> Result<Vec<SessionSummary>, ApiError> {
    let conn = open_readonly(&db_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT session_id,
                    MIN(timestamp) AS started_at,
                    MAX(CASE WHEN event_type = 'SessionEnd' THEN timestamp END) AS ended_at,
                    COUNT(*) AS event_count,
                    COALESCE(MIN(tool_name), 'unknown') AS source
             FROM trace_events
             GROUP BY session_id
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .map_err(sql_error)?;
    let rows = stmt
        .query_map([i64::from(limit)], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                started_at: row.get(1)?,
                ended_at: row.get(2)?,
                event_count: row.get::<_, i64>(3)? as u64,
                source: row.get(4)?,
            })
        })
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)?;
    Ok(rows)
}

fn query_session_detail(db_path: PathBuf, session_id: String) -> Result<SessionDetail, ApiError> {
    let conn = open_readonly(&db_path)?;
    let session = conn
        .query_row(
            "SELECT session_id,
                    MIN(timestamp) AS started_at,
                    MAX(CASE WHEN event_type = 'SessionEnd' THEN timestamp END) AS ended_at,
                    COUNT(*) AS event_count,
                    COALESCE(MIN(tool_name), 'unknown') AS source
             FROM trace_events
             WHERE session_id = ?1
             GROUP BY session_id",
            [&session_id],
            |row| {
                Ok(SessionSummary {
                    session_id: row.get(0)?,
                    started_at: row.get(1)?,
                    ended_at: row.get(2)?,
                    event_count: row.get::<_, i64>(3)? as u64,
                    source: row.get(4)?,
                })
            },
        )
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => ApiError::missing("session not found"),
            other => sql_error(other),
        })?;
    let events = query_events_for_session(&conn, &session_id, 500)?;
    Ok(SessionDetail { session, events })
}

fn query_events(
    db_path: PathBuf,
    limit: u32,
    cursor: Option<u64>,
    session_id: Option<String>,
) -> Result<EventsPage, ApiError> {
    let conn = open_readonly(&db_path)?;
    let fetch_limit = limit + 1;
    let mut rows = if let Some(session_id) = session_id {
        query_events_by_sql(
            &conn,
            "SELECT id, session_id, event_type, timestamp, subject_kind, subject, event_json
             FROM trace_events
             WHERE (?1 IS NULL OR id < ?1) AND session_id = ?2
             ORDER BY id DESC
             LIMIT ?3",
            params![cursor, session_id, i64::from(fetch_limit)],
        )?
    } else {
        query_events_by_sql(
            &conn,
            "SELECT id, session_id, event_type, timestamp, subject_kind, subject, event_json
             FROM trace_events
             WHERE (?1 IS NULL OR id < ?1)
             ORDER BY id DESC
             LIMIT ?2",
            params![cursor, i64::from(fetch_limit)],
        )?
    };

    let next_cursor = if rows.len() > limit as usize {
        let _extra = rows
            .pop()
            .ok_or_else(|| ApiError::bad_gateway("pagination invariant failed"))?;
        rows.last().map(|event| event.event_id.to_string())
    } else {
        None
    };
    Ok(EventsPage {
        events: rows,
        next_cursor,
    })
}

fn query_events_for_session(
    conn: &Connection,
    session_id: &str,
    limit: u32,
) -> Result<Vec<TraceEventItem>, ApiError> {
    query_events_by_sql(
        conn,
        "SELECT id, session_id, event_type, timestamp, subject_kind, subject, event_json
         FROM trace_events
         WHERE session_id = ?1
         ORDER BY id ASC
         LIMIT ?2",
        params![session_id, i64::from(limit)],
    )
}

fn query_events_by_sql<P>(
    conn: &Connection,
    sql: &str,
    params: P,
) -> Result<Vec<TraceEventItem>, ApiError>
where
    P: rusqlite::Params,
{
    let mut stmt = conn.prepare(sql).map_err(sql_error)?;
    stmt.query_map(params, event_from_row)
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TraceEventItem> {
    let event_json: String = row.get(6)?;
    let payload = serde_json::from_str::<Value>(&event_json)
        .ok()
        .and_then(|value| value.get("payload").cloned())
        .unwrap_or(Value::Null);
    Ok(TraceEventItem {
        event_id: row.get::<_, i64>(0)? as u64,
        session_id: row.get(1)?,
        event_type: row.get(2)?,
        timestamp: row.get(3)?,
        subject_kind: row.get(4)?,
        subject: row.get(5)?,
        payload,
    })
}

fn sql_error(err: rusqlite::Error) -> ApiError {
    ApiError::bad_gateway(format!("cannot query scryrs datastore: {err}"))
}

async fn index(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    serve_index(&state.config)
}

async fn spa_fallback(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    serve_index(&state.config)
}

async fn asset(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> impl IntoResponse {
    let normalized = format!("assets/{}", path.trim_start_matches('/'));
    serve_asset(&state.config, &normalized, false)
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorBody {
            error: "dashboard API endpoint not found".into(),
        }),
    )
}

fn serve_index(config: &Config) -> Response<Body> {
    serve_asset(config, "index.html", true).into_response()
}

fn serve_asset(config: &Config, path: &str, fallback_index: bool) -> Response<Body> {
    if config.dev_mode {
        return serve_dev_asset(config, path, fallback_index);
    }
    match EmbeddedAssets::get(path) {
        Some(file) => bytes_response(file.data.into_owned(), mime_for(path)),
        None if fallback_index => EmbeddedAssets::get("index.html")
            .map(|file| bytes_response(file.data.into_owned(), "text/html"))
            .unwrap_or_else(not_found_response),
        None => not_found_response(),
    }
}

fn serve_dev_asset(config: &Config, path: &str, fallback_index: bool) -> Response<Body> {
    let target = config.frontend_dist_dir().join(path);
    match std::fs::read(&target) {
        Ok(bytes) => bytes_response(bytes, mime_for(path)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && fallback_index => {
            let index = config.frontend_dist_dir().join("index.html");
            std::fs::read(index)
                .map(|bytes| bytes_response(bytes, "text/html"))
                .unwrap_or_else(|_| not_found_response())
        }
        Err(_) => not_found_response(),
    }
}

fn bytes_response(bytes: Vec<u8>, mime: &str) -> Response<Body> {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime).unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    response
}

fn not_found_response() -> Response<Body> {
    let mut response = Response::new(Body::from("not found"));
    *response.status_mut() = StatusCode::NOT_FOUND;
    response
}

fn mime_for(path: &str) -> &'static str {
    mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
}
