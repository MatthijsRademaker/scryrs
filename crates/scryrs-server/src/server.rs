//! Axum HTTP server for central trace ingest.
//!
//! Exposes `POST /v1/trace-events/batch` with two-layer validation:
//! top-level envelope failures return 400, per-item failures return 200
//! with deterministic diagnostics.
//!
//! Also exposes read-only live hotspot and signal streaming endpoints.

use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::path::Component;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::{Path, Query, State};
use axum::http::header::AUTHORIZATION;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::Config;
use crate::read_models::EventCursor;
use crate::store::ServerStore;
use crate::time::chrono_now;
use scryrs_types::{
    BatchIngestResponse, EventAckStatus, HotspotSignal, ROUTE_SCHEMA_VERSION, RouteLoadTargetKind,
    RouteManifestDocument, ServerIngestEnvelope,
};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// Capacity of the `tokio::sync::broadcast` channel for signal fanout.
/// Large enough to absorb brief consumer stalls; slow consumers that
/// fall behind recover via `after=<signal_id>` replay on reconnect.
const SIGNAL_CHANNEL_CAPACITY: usize = 1024;
const DEFAULT_PAGE_LIMIT: u32 = 50;
const MAX_PAGE_LIMIT: u32 = 500;
/// Maximum accepted route-manifest request body: 2 MiB.
pub const MAX_ROUTE_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
/// JSON environment variable configuring repository-bound route publishers.
pub const ROUTE_PUBLISH_CREDENTIALS_ENV: &str = "SCRYRS_ROUTE_PUBLISH_CREDENTIALS";
/// JSON environment variable configuring repository-bound proposal writers.
pub const PROPOSAL_WRITE_CREDENTIALS_ENV: &str = "SCRYRS_PROPOSAL_WRITE_CREDENTIALS";

/// Repository-bound bearer credential allowed to publish route manifests.
#[derive(Clone, PartialEq, Eq)]
pub struct RoutePublishCredential {
    pub repository_id: String,
    pub publisher_id: String,
    token: String,
}

impl RoutePublishCredential {
    pub fn try_new(
        repository_id: impl Into<String>,
        publisher_id: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, String> {
        let credential = Self {
            repository_id: repository_id.into(),
            publisher_id: publisher_id.into(),
            token: token.into(),
        };
        if credential.repository_id.trim().is_empty() {
            return Err("repositoryId must not be empty".into());
        }
        if credential.publisher_id.trim().is_empty() {
            return Err("publisherId must not be empty".into());
        }
        if credential.token.is_empty() {
            return Err("token must not be empty".into());
        }
        Ok(credential)
    }
}

impl fmt::Debug for RoutePublishCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RoutePublishCredential")
            .field("repository_id", &self.repository_id)
            .field("publisher_id", &self.publisher_id)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

/// Repository-bound bearer credential allowed to publish and review proposals.
#[derive(Clone, PartialEq, Eq)]
pub struct ProposalWriteCredential {
    pub repository_id: String,
    pub actor_id: String,
    token: String,
}

impl ProposalWriteCredential {
    pub fn try_new(
        repository_id: impl Into<String>,
        actor_id: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, String> {
        let credential = Self {
            repository_id: repository_id.into(),
            actor_id: actor_id.into(),
            token: token.into(),
        };
        if credential.repository_id.trim().is_empty() {
            return Err("repositoryId must not be empty".into());
        }
        if credential.actor_id.trim().is_empty() {
            return Err("actorId must not be empty".into());
        }
        if credential.token.is_empty() {
            return Err("token must not be empty".into());
        }
        Ok(credential)
    }
}

impl fmt::Debug for ProposalWriteCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProposalWriteCredential")
            .field("repository_id", &self.repository_id)
            .field("actor_id", &self.actor_id)
            .field("token", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone)]
struct AuthorizedPublisher {
    repository_id: String,
    publisher_id: String,
    token_sha256: [u8; 32],
}

#[derive(Clone)]
pub(crate) struct AuthorizedProposalWriter {
    pub(crate) repository_id: String,
    pub(crate) actor_id: String,
    pub(crate) token_sha256: [u8; 32],
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) store: Arc<Mutex<ServerStore>>,
    signal_tx: tokio::sync::broadcast::Sender<(i64, HotspotSignal)>,
    route_publishers: Arc<Vec<AuthorizedPublisher>>,
    pub(crate) proposal_writers: Arc<Vec<AuthorizedProposalWriter>>,
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Deserialize)]
struct PageQuery {
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventPageQuery {
    limit: Option<u32>,
    cursor: Option<String>,
    #[serde(alias = "session_id")]
    session_id: Option<String>,
}

#[derive(Deserialize)]
struct RouteExplainQuery {
    query: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RouteManifestPublicationResponse {
    repository_id: String,
    schema_version: String,
    content_sha256: String,
    publisher_id: String,
    published_at: String,
    unchanged: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoutePublishCredentialInput {
    repository_id: String,
    publisher_id: String,
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposalWriteCredentialInput {
    repository_id: String,
    actor_id: String,
    token: String,
}

/// Parse repository-bound route publication credentials from JSON configuration.
///
/// Expected shape: `[{"repositoryId":"repo","publisherId":"ci","token":"secret"}]`.
pub fn parse_route_publish_credentials(raw: &str) -> Result<Vec<RoutePublishCredential>, String> {
    let inputs: Vec<RoutePublishCredentialInput> = serde_json::from_str(raw)
        .map_err(|error| format!("invalid route publication credentials JSON: {error}"))?;
    let mut token_digests = HashSet::new();
    let mut credentials = Vec::with_capacity(inputs.len());
    for input in inputs {
        let credential =
            RoutePublishCredential::try_new(input.repository_id, input.publisher_id, input.token)?;
        if !token_digests.insert(token_sha256(&credential.token)) {
            return Err("route publication bearer tokens must be unique".into());
        }
        credentials.push(credential);
    }
    Ok(credentials)
}

/// Parse repository-bound proposal write credentials from JSON configuration.
///
/// Expected shape: `[{"repositoryId":"repo","actorId":"alice","token":"secret"}]`.
pub fn parse_proposal_write_credentials(raw: &str) -> Result<Vec<ProposalWriteCredential>, String> {
    let inputs: Vec<ProposalWriteCredentialInput> = serde_json::from_str(raw)
        .map_err(|error| format!("invalid proposal write credentials JSON: {error}"))?;
    let mut token_digests = HashSet::new();
    let mut credentials = Vec::with_capacity(inputs.len());
    for input in inputs {
        let credential =
            ProposalWriteCredential::try_new(input.repository_id, input.actor_id, input.token)?;
        if !token_digests.insert(token_sha256(&credential.token)) {
            return Err("proposal write bearer tokens must be unique".into());
        }
        credentials.push(credential);
    }
    Ok(credentials)
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

/// Build server router over server-owned SQLite store.
pub fn router(
    store_path: &std::path::Path,
    signal_threshold: u32,
) -> Result<Router, rusqlite::Error> {
    router_with_write_credentials(store_path, signal_threshold, Vec::new(), Vec::new())
}

/// Build server router with repository-bound route-manifest publishers.
pub fn router_with_route_publish_credentials(
    store_path: &std::path::Path,
    signal_threshold: u32,
    credentials: Vec<RoutePublishCredential>,
) -> Result<Router, rusqlite::Error> {
    router_with_write_credentials(store_path, signal_threshold, credentials, Vec::new())
}

/// Build server router with repository-bound proposal publishers/reviewers.
pub fn router_with_proposal_write_credentials(
    store_path: &std::path::Path,
    signal_threshold: u32,
    credentials: Vec<ProposalWriteCredential>,
) -> Result<Router, rusqlite::Error> {
    router_with_write_credentials(store_path, signal_threshold, Vec::new(), credentials)
}

fn router_with_write_credentials(
    store_path: &std::path::Path,
    signal_threshold: u32,
    route_credentials: Vec<RoutePublishCredential>,
    proposal_credentials: Vec<ProposalWriteCredential>,
) -> Result<Router, rusqlite::Error> {
    let store = ServerStore::open(store_path, signal_threshold)?;
    let (signal_tx, _) = tokio::sync::broadcast::channel(SIGNAL_CHANNEL_CAPACITY);
    let route_publishers = route_credentials
        .into_iter()
        .map(|credential| AuthorizedPublisher {
            repository_id: credential.repository_id,
            publisher_id: credential.publisher_id,
            token_sha256: token_sha256(&credential.token),
        })
        .collect();
    let proposal_writers = proposal_credentials
        .into_iter()
        .map(|credential| AuthorizedProposalWriter {
            repository_id: credential.repository_id,
            actor_id: credential.actor_id,
            token_sha256: token_sha256(&credential.token),
        })
        .collect();
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        signal_tx,
        route_publishers: Arc::new(route_publishers),
        proposal_writers: Arc::new(proposal_writers),
    };
    Ok(Router::new()
        .route("/v1/trace-events/batch", post(ingest_batch))
        .route(
            "/v1/repositories/:repository_id/hotspots",
            get(get_hotspots),
        )
        .route(
            "/v1/repositories/:repository_id/signals",
            get(get_signals_sse),
        )
        .route(
            "/v1/repositories/:repository_id/sessions",
            get(get_sessions),
        )
        .route(
            "/v1/repositories/:repository_id/sessions/:session_id",
            get(get_session_detail),
        )
        .route("/v1/repositories/:repository_id/events", get(get_events))
        .route(
            "/v1/repositories/:repository_id/proposals",
            get(crate::proposal_api::list_proposals).post(crate::proposal_api::publish_proposal),
        )
        .route(
            "/v1/repositories/:repository_id/proposals/:proposal_id",
            get(crate::proposal_api::get_proposal),
        )
        .route(
            "/v1/repositories/:repository_id/proposals/:proposal_id/accept",
            post(crate::proposal_api::accept_proposal),
        )
        .route(
            "/v1/repositories/:repository_id/proposals/:proposal_id/reject",
            post(crate::proposal_api::reject_proposal),
        )
        .route(
            "/v1/repositories/:repository_id/routes/manifest",
            post(publish_route_manifest),
        )
        .route(
            "/v1/repositories/:repository_id/routes/explain",
            get(get_route_explain),
        )
        .with_state(state))
}

// ---------------------------------------------------------------------------
// Server lifecycle
// ---------------------------------------------------------------------------

pub async fn serve(config: Config) -> Result<(), crate::ServerError> {
    let bind = SocketAddr::new(config.bind_address, config.port);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    write_startup_message(&addr, &config.store_path)?;

    let app = router_with_write_credentials(
        &config.store_path,
        config.signal_threshold,
        config.route_publish_credentials,
        config.proposal_write_credentials,
    )
    .map_err(|e| crate::ServerError::Io(std::io::Error::other(e)))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn write_startup_message(
    addr: &SocketAddr,
    store_path: &std::path::Path,
) -> Result<(), std::io::Error> {
    use std::io::Write;
    writeln!(
        std::io::stderr().lock(),
        "scryrs server listening on http://{}:{}, store {}",
        addr.ip(),
        addr.port(),
        store_path.display()
    )
}

async fn shutdown_signal() {
    #[allow(clippy::expect_used)]
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    #[allow(clippy::expect_used)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(unix)]
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await;
}

// ---------------------------------------------------------------------------
// POST /v1/trace-events/batch
// ---------------------------------------------------------------------------

async fn ingest_batch(State(state): State<AppState>, body: String) -> axum::response::Response {
    // Layer 1: Top-level parse — strict, fail with 400.
    let envelope: ServerIngestEnvelope = match serde_json::from_str(&body) {
        Ok(env) => env,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: format!("malformed request body: {e}"),
                }),
            )
                .into_response();
        }
    };

    // Validate top-level identity fields.
    if envelope.envelope_version != "1.0.0" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: format!(
                    "unsupported envelope_version: '{}', expected '1.0.0'",
                    envelope.envelope_version
                ),
            }),
        )
            .into_response();
    }

    if envelope.repository_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "missing top-level repository_id".into(),
            }),
        )
            .into_response();
    }
    if envelope.workspace_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "missing top-level workspace_id".into(),
            }),
        )
            .into_response();
    }
    if envelope.agent_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "missing top-level agent_id".into(),
            }),
        )
            .into_response();
    }

    let repo_id = envelope.repository_id.clone();

    // Lock the store for the duration of batch processing.
    let (acks, new_signals) = {
        let store = state.store.lock().unwrap_or_else(|e| e.into_inner());

        // Snapshot max signal id before this batch so we can identify
        // newly created signals afterward.
        let max_before = match store.max_signal_id(&repo_id) {
            Ok(id) => id,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("failed to query max signal id: {e}"),
                    }),
                )
                    .into_response();
            }
        };

        let acks = match store.ingest_batch(&envelope) {
            Ok(acks) => acks,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("batch ingest failed: {e}"),
                    }),
                )
                    .into_response();
            }
        };

        // Collect signals created during this batch.
        let new_signals = store
            .get_signals_after(&repo_id, max_before)
            .unwrap_or_else(|e| {
                use std::io::Write;
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "scryrs server: failed to collect signals after ingest for repo {repo_id}: {e}",
                );
                Vec::new()
            });

        (acks, new_signals)
    };
    // Store lock released here.

    // Publish newly committed signals to broadcast channel (fire-and-forget).
    for signal_row in &new_signals {
        let signal = HotspotSignal {
            repositoryId: repo_id.clone(),
            subjectKind: signal_row.subject_kind.clone(),
            subject: signal_row.subject.clone(),
            score: signal_row.score,
            delta: signal_row.delta,
            window: signal_row.window.clone(),
            threshold: signal_row.threshold,
            evidenceRowIds: serde_json::from_str(&signal_row.evidence_row_ids).unwrap_or_default(),
            createdAt: signal_row.created_at.clone(),
        };
        let _ = state.signal_tx.send((signal_row.id, signal));
    }

    let accepted_count = acks
        .iter()
        .filter(|a| a.status == EventAckStatus::Accepted)
        .count() as u64;
    let duplicate_count = acks
        .iter()
        .filter(|a| a.status == EventAckStatus::Idempotent)
        .count() as u64;
    let rejected_count = acks
        .iter()
        .filter(|a| a.status == EventAckStatus::Rejected)
        .count() as u64;

    let response = BatchIngestResponse {
        accepted_count,
        duplicate_count,
        rejected_count,
        received_count: accepted_count,
        events: acks,
        received_at: chrono_now(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

// ---------------------------------------------------------------------------
// GET /v1/repositories/{repository_id}/hotspots
// ---------------------------------------------------------------------------

async fn get_hotspots(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let window = params
        .get("window")
        .map(|s| s.as_str())
        .unwrap_or("cumulative");
    let session_id = params.get("session_id").map(|s| s.as_str());

    // Validate window parameter — only "cumulative" supported.
    if window != "cumulative" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: format!(
                    "unsupported window '{}': only window=cumulative is supported",
                    window
                ),
            }),
        )
            .into_response();
    }

    let store = state.store.lock().unwrap_or_else(|e| e.into_inner());

    let response = if let Some(sid) = session_id {
        match store.materialize_session_hotspots(&repository_id, sid) {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("session hotspot query failed: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    } else {
        match store.materialize_cumulative_hotspots(&repository_id) {
            Ok(r) => r,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("cumulative hotspot query failed: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    (StatusCode::OK, Json(response)).into_response()
}

// ---------------------------------------------------------------------------
// Route manifest publication and explain
// ---------------------------------------------------------------------------

async fn publish_route_manifest(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    request: Request<Body>,
) -> axum::response::Response {
    let publisher = match authorize_route_publisher(&state, &repository_id, &request) {
        Ok(publisher) => publisher,
        Err(response) => return response,
    };

    if request
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > MAX_ROUTE_MANIFEST_BYTES)
    {
        return route_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("route manifest exceeds {MAX_ROUTE_MANIFEST_BYTES}-byte limit"),
        );
    }

    let bytes = match to_bytes(request.into_body(), MAX_ROUTE_MANIFEST_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return route_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("route manifest exceeds {MAX_ROUTE_MANIFEST_BYTES}-byte limit"),
            );
        }
    };
    let manifest: RouteManifestDocument = match serde_json::from_slice(&bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            return route_error(
                StatusCode::BAD_REQUEST,
                format!("malformed route manifest JSON: {error}"),
            );
        }
    };
    if manifest.schema_version != ROUTE_SCHEMA_VERSION {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "route schema version mismatch: got '{}', expected '{}'",
                manifest.schema_version, ROUTE_SCHEMA_VERSION
            ),
        );
    }
    if manifest.metadata.repository_id.as_deref() != Some(repository_id.as_str()) {
        return route_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "route manifest repository identity must equal path repository '{repository_id}'"
            ),
        );
    }
    if let Err(error) = validate_route_manifest(&manifest) {
        return route_error(StatusCode::UNPROCESSABLE_ENTITY, error);
    }

    let canonical_json = match serde_json::to_string(&manifest) {
        Ok(json) => json,
        Err(error) => {
            return route_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("route manifest serialization failed: {error}"),
            );
        }
    };
    let content_sha256 = sha256_hex(canonical_json.as_bytes());
    let publication = {
        let store = state
            .store
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        store.publish_route_manifest(
            &repository_id,
            ROUTE_SCHEMA_VERSION,
            &canonical_json,
            &content_sha256,
            &publisher.publisher_id,
            &chrono_now(),
        )
    };
    match publication {
        Ok(publication) => (
            StatusCode::OK,
            Json(RouteManifestPublicationResponse {
                repository_id: publication.manifest.repository_id,
                schema_version: publication.manifest.schema_version,
                content_sha256: publication.manifest.content_sha256,
                publisher_id: publication.manifest.publisher_id,
                published_at: publication.manifest.published_at,
                unchanged: publication.unchanged,
            }),
        )
            .into_response(),
        Err(error) => route_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("route manifest publication failed: {error}"),
        ),
    }
}

async fn get_route_explain(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(query): Query<RouteExplainQuery>,
) -> axum::response::Response {
    let Some(query) = query.query.map(|query| query.trim().to_owned()) else {
        return route_error(StatusCode::BAD_REQUEST, "query parameter must be non-empty");
    };
    if query.is_empty() {
        return route_error(StatusCode::BAD_REQUEST, "query parameter must be non-empty");
    }

    let stored = {
        let store = state
            .store
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        store.get_route_manifest(&repository_id)
    };
    let stored = match stored {
        Ok(Some(manifest)) => manifest,
        Ok(None) => {
            return route_error(
                StatusCode::NOT_FOUND,
                format!(
                    "no route manifest published for repository '{repository_id}'; publish a route manifest with `scryrs route publish <PATH>`"
                ),
            );
        }
        Err(error) => {
            return route_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("route manifest query failed: {error}"),
            );
        }
    };
    if stored.schema_version != ROUTE_SCHEMA_VERSION {
        return route_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "stored route schema version mismatch: got '{}', expected '{}'",
                stored.schema_version, ROUTE_SCHEMA_VERSION
            ),
        );
    }
    let manifest: RouteManifestDocument = match serde_json::from_str(&stored.manifest_json) {
        Ok(manifest) => manifest,
        Err(error) => {
            return route_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("stored route manifest is invalid: {error}"),
            );
        }
    };

    (
        StatusCode::OK,
        Json(scryrs_runtime::explain_hints(&manifest, &query)),
    )
        .into_response()
}

fn authorize_route_publisher<'a>(
    state: &'a AppState,
    repository_id: &str,
    request: &Request<Body>,
) -> Result<&'a AuthorizedPublisher, axum::response::Response> {
    if state.route_publishers.is_empty() {
        return Err(route_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "route manifest publication is disabled; configure SCRYRS_ROUTE_PUBLISH_CREDENTIALS",
        ));
    }
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            route_error(
                StatusCode::UNAUTHORIZED,
                "route manifest publication requires Authorization: Bearer <token>",
            )
        })?;
    let digest = token_sha256(token);
    let matching_token = state
        .route_publishers
        .iter()
        .filter(|publisher| constant_time_eq(&digest, &publisher.token_sha256))
        .collect::<Vec<_>>();
    if matching_token.is_empty() {
        return Err(route_error(
            StatusCode::UNAUTHORIZED,
            "invalid route manifest publication bearer token",
        ));
    }
    matching_token
        .into_iter()
        .find(|publisher| publisher.repository_id == repository_id)
        .ok_or_else(|| {
            route_error(
                StatusCode::FORBIDDEN,
                format!("authenticated publisher does not own repository '{repository_id}'"),
            )
        })
}

fn validate_route_manifest(manifest: &RouteManifestDocument) -> Result<(), String> {
    let mut route_ids = HashSet::with_capacity(manifest.routes.len());
    for route in &manifest.routes {
        for (field, value) in [
            ("id", route.id.as_str()),
            ("subjectKind", route.subject_kind.as_str()),
            ("subject", route.subject.as_str()),
            ("label", route.label.as_str()),
            ("target", route.target.as_str()),
            ("kind", route.kind.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(format!("route {field} must not be empty"));
            }
        }
        if !route_ids.insert(route.id.as_str()) {
            return Err(format!("duplicate route id '{}'", route.id));
        }
        if let Some(load_target) = &route.load_target {
            match load_target.kind {
                RouteLoadTargetKind::File | RouteLoadTargetKind::DocPage => {
                    let reference = load_target.reference.as_deref().ok_or_else(|| {
                        format!("route '{}' load target requires a reference", route.id)
                    })?;
                    if reference.trim().is_empty() {
                        return Err(format!(
                            "route '{}' load target reference must not be empty",
                            route.id
                        ));
                    }
                    if matches!(load_target.kind, RouteLoadTargetKind::File)
                        && (std::path::Path::new(reference).is_absolute()
                            || std::path::Path::new(reference)
                                .components()
                                .any(|component| matches!(component, Component::ParentDir)))
                    {
                        return Err(format!(
                            "route '{}' file load target must be repository-relative",
                            route.id
                        ));
                    }
                }
                RouteLoadTargetKind::NonLoadable if load_target.reference.is_some() => {
                    return Err(format!(
                        "route '{}' non-loadable target must not include a reference",
                        route.id
                    ));
                }
                RouteLoadTargetKind::NonLoadable => {}
            }
        }
    }
    for route in &manifest.routes {
        for edge in &route.related_edges {
            if !route_ids.contains(edge.target_route_id.as_str()) {
                return Err(format!(
                    "route '{}' references missing related route '{}'",
                    route.id, edge.target_route_id
                ));
            }
        }
    }
    Ok(())
}

fn route_error(status: StatusCode, error: impl Into<String>) -> axum::response::Response {
    (
        status,
        Json(ErrorBody {
            error: error.into(),
        }),
    )
        .into_response()
}

pub(crate) fn token_sha256(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

pub(crate) fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

// ---------------------------------------------------------------------------
// GET /v1/repositories/{repository_id}/sessions
// ---------------------------------------------------------------------------

async fn get_sessions(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(query): Query<PageQuery>,
) -> axum::response::Response {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let cursor = match parse_cursor(query.cursor) {
        Ok(cursor) => cursor,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(ErrorBody { error })).into_response();
        }
    };
    let store = state
        .store
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    match store.list_sessions(&repository_id, limit, cursor) {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: format!("session query failed: {error}"),
            }),
        )
            .into_response(),
    }
}

async fn get_session_detail(
    State(state): State<AppState>,
    Path((repository_id, session_id)): Path<(String, String)>,
) -> axum::response::Response {
    let store = state
        .store
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    match store.get_session_detail(&repository_id, &session_id) {
        Ok(Some(detail)) => (StatusCode::OK, Json(detail)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: "session not found".into(),
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: format!("session detail query failed: {error}"),
            }),
        )
            .into_response(),
    }
}

async fn get_events(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(query): Query<EventPageQuery>,
) -> axum::response::Response {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .clamp(1, MAX_PAGE_LIMIT);
    let cursor = match parse_cursor(query.cursor) {
        Ok(cursor) => cursor,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(ErrorBody { error })).into_response();
        }
    };
    let store = state
        .store
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    match store.list_events(&repository_id, limit, cursor, query.session_id.as_deref()) {
        Ok(page) => (StatusCode::OK, Json(page)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorBody {
                error: format!("event query failed: {error}"),
            }),
        )
            .into_response(),
    }
}

fn parse_cursor(raw: Option<String>) -> Result<Option<EventCursor>, String> {
    raw.map(|value| {
        value
            .parse::<EventCursor>()
            .map_err(|error| error.to_string())
    })
    .transpose()
}

// ---------------------------------------------------------------------------
// GET /v1/repositories/{repository_id}/signals (SSE)
// ---------------------------------------------------------------------------

async fn get_signals_sse(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let after: i64 = params
        .get("after")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Subscribe to broadcast BEFORE replay to avoid missing signals
    // published between replay and subscription.
    let rx = state.signal_tx.subscribe();

    // Phase 1: Replay persisted signals with id > after, under store lock.
    let replay_signals = {
        let store = state.store.lock().unwrap_or_else(|e| e.into_inner());
        store
            .get_signals_after(&repository_id, after)
            .unwrap_or_default()
    };

    // Track max replayed signal id to deduplicate live signals.
    let max_replayed_id = replay_signals.last().map(|sr| sr.id).unwrap_or(after);

    // Phase 2: Live signal broadcast (filtered to this repository and new signals only).
    let live_stream = BroadcastStream::new(rx);

    // Build replay events.
    let replay_events: Vec<Result<Event, Infallible>> = replay_signals
        .into_iter()
        .map(|sr| {
            let signal = HotspotSignal {
                repositoryId: sr.repository_id.clone(),
                subjectKind: sr.subject_kind.clone(),
                subject: sr.subject.clone(),
                score: sr.score,
                delta: sr.delta,
                window: sr.window.clone(),
                threshold: sr.threshold,
                evidenceRowIds: serde_json::from_str(&sr.evidence_row_ids).unwrap_or_default(),
                createdAt: sr.created_at.clone(),
            };
            let json = serde_json::to_string(&signal).unwrap_or_default();
            Ok(Event::default().id(sr.id.to_string()).data(json))
        })
        .collect();

    // Filter live stream: only this repo, only new signals.
    let filtered_live = live_stream.filter_map(move |result| {
        let (signal_id, signal) = match result {
            Ok(tuple) => tuple,
            Err(e) => {
                // BroadcastStreamRecvError wraps RecvError (Lagged or Closed).
                // Log lagged consumers so operators can detect slow subscribers;
                // recovery is via after=<signal_id> cursor on reconnect.
                use std::io::Write;
                drop(writeln!(
                    std::io::stderr().lock(),
                    "SSE subscriber error for repo {repository_id}: {e:?}"
                ));
                return None;
            }
        };
        // Filter to only this repository's signals, and only signals newer than
        // what we already replayed.
        if signal.repositoryId != repository_id || signal_id <= max_replayed_id {
            return None;
        }
        let json = match serde_json::to_string(&signal) {
            Ok(j) => j,
            Err(_) => return None,
        };
        let event = Event::default().id(signal_id.to_string()).data(json);
        Some(Ok(event))
    });

    // Combine replay + live.
    let replay_stream = tokio_stream::iter(replay_events);
    let combined = replay_stream.chain(filtered_live);

    Sse::new(combined).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn make_valid_body() -> String {
        make_batch_body("repo-a", vec![("evt-001", "s1", "2026-06-24T10:00:00Z")])
    }

    fn make_batch_body(repository_id: &str, events: Vec<(&str, &str, &str)>) -> String {
        let events = events
            .into_iter()
            .map(|(producer_event_id, session_id, timestamp)| {
                serde_json::json!({
                    "producer_event_id": producer_event_id,
                    "client_timestamp": timestamp,
                    "event": {
                        "schema_version": "0.1.0",
                        "timestamp": timestamp,
                        "session_id": session_id,
                        "event_type": "DocRetrieved",
                        "tool_name": "read",
                        "payload": { "type": "DocRetrieved", "doc_ref": "doc/a.md" },
                        "outcome": { "result": "Success" }
                    }
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": repository_id,
            "workspace_id": "ws-1",
            "agent_id": "pi",
            "events": events
        })
        .to_string()
    }

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"))
    }

    fn post_request(body: String) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/v1/trace-events/batch")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    fn get_request(uri: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    // --- Valid batch tests ---

    #[tokio::test]
    async fn valid_batch_returns_200_with_accepted_event() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app.oneshot(post_request(make_valid_body())).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["accepted_count"], 1);
        assert_eq!(json["duplicate_count"], 0);
        assert_eq!(json["rejected_count"], 0);
        assert_eq!(json["received_count"], 1);
        assert_eq!(json["events"][0]["index"], 0);
        assert_eq!(json["events"][0]["status"], "accepted");
    }

    #[tokio::test]
    async fn duplicate_replay_returns_idempotent() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = make_valid_body();

        // First submission.
        let r1 = app
            .clone()
            .oneshot(post_request(body.clone()))
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::OK);

        // Second submission with same body.
        let r2 = app.oneshot(post_request(body)).await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(r2.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["duplicate_count"], 1);
        assert_eq!(json["accepted_count"], 0);
        assert_eq!(json["events"][0]["status"], "idempotent");
    }

    #[tokio::test]
    async fn sessions_endpoint_returns_repository_scoped_page() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();
        let body = make_batch_body(
            "repo-a",
            vec![
                ("evt-old", "session-old", "2026-06-24T09:00:00Z"),
                ("evt-new-1", "session-new", "2026-06-24T10:00:00Z"),
                ("evt-new-2", "session-new", "2026-06-24T10:01:00Z"),
            ],
        );
        let ingest = app.clone().oneshot(post_request(body)).await.unwrap();
        assert_eq!(ingest.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(get_request("/v1/repositories/repo-a/sessions?limit=1"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["sessions"].as_array().map(Vec::len), Some(1));
        assert_eq!(json["sessions"][0]["sessionId"], "session-new");
        assert_eq!(json["sessions"][0]["eventCount"], 2);
        assert_eq!(json["sessions"][0]["source"], "read");
        let cursor = json["nextCursor"]
            .as_str()
            .unwrap_or_else(|| panic!("next cursor missing"));

        let second = app
            .clone()
            .oneshot(get_request(&format!(
                "/v1/repositories/repo-a/sessions?limit=1&cursor={cursor}"
            )))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        let second_body = axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap();
        let second_json: serde_json::Value = serde_json::from_slice(&second_body).unwrap();
        assert_eq!(second_json["sessions"][0]["sessionId"], "session-old");
        assert!(second_json["nextCursor"].is_null());

        let empty = app
            .clone()
            .oneshot(get_request("/v1/repositories/repo-b/sessions"))
            .await
            .unwrap();
        assert_eq!(empty.status(), StatusCode::OK);
        let empty_body = axum::body::to_bytes(empty.into_body(), usize::MAX)
            .await
            .unwrap();
        let empty_json: serde_json::Value = serde_json::from_slice(&empty_body).unwrap();
        assert_eq!(empty_json["sessions"].as_array().map(Vec::len), Some(0));

        let malformed = app
            .oneshot(get_request(
                "/v1/repositories/repo-a/sessions?cursor=invalid",
            ))
            .await
            .unwrap();
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn session_detail_and_events_endpoints_are_repository_scoped() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();
        let body = make_batch_body(
            "repo-a",
            vec![
                ("evt-1", "s1", "2026-06-24T09:00:00Z"),
                ("evt-2", "s1", "2026-06-24T09:01:00Z"),
                ("evt-3", "s2", "2026-06-24T10:00:00Z"),
            ],
        );
        let ingest = app.clone().oneshot(post_request(body)).await.unwrap();
        assert_eq!(ingest.status(), StatusCode::OK);

        let detail = app
            .clone()
            .oneshot(get_request("/v1/repositories/repo-a/sessions/s1"))
            .await
            .unwrap();
        assert_eq!(detail.status(), StatusCode::OK);
        let detail_body = axum::body::to_bytes(detail.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail_json: serde_json::Value = serde_json::from_slice(&detail_body).unwrap();
        assert_eq!(detail_json["session"]["sessionId"], "s1");
        assert_eq!(detail_json["events"][0]["eventId"], 1);
        assert_eq!(detail_json["events"][1]["eventId"], 2);

        let cross_repository = app
            .clone()
            .oneshot(get_request("/v1/repositories/repo-b/sessions/s1"))
            .await
            .unwrap();
        assert_eq!(cross_repository.status(), StatusCode::NOT_FOUND);

        let first = app
            .clone()
            .oneshot(get_request("/v1/repositories/repo-a/events?limit=2"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let first_body = axum::body::to_bytes(first.into_body(), usize::MAX)
            .await
            .unwrap();
        let first_json: serde_json::Value = serde_json::from_slice(&first_body).unwrap();
        assert_eq!(first_json["events"][0]["eventId"], 3);
        assert_eq!(first_json["events"][1]["eventId"], 2);
        assert_eq!(first_json["nextCursor"], "2");

        let filtered = app
            .clone()
            .oneshot(get_request(
                "/v1/repositories/repo-a/events?limit=10&session_id=s1",
            ))
            .await
            .unwrap();
        assert_eq!(filtered.status(), StatusCode::OK);
        let filtered_body = axum::body::to_bytes(filtered.into_body(), usize::MAX)
            .await
            .unwrap();
        let filtered_json: serde_json::Value = serde_json::from_slice(&filtered_body).unwrap();
        assert_eq!(filtered_json["events"].as_array().map(Vec::len), Some(2));
        assert!(
            filtered_json["events"]
                .as_array()
                .is_some_and(|events| events.iter().all(|event| event["sessionId"] == "s1"))
        );

        let malformed = app
            .oneshot(get_request(
                "/v1/repositories/repo-a/events?cursor=not-a-cursor",
            ))
            .await
            .unwrap();
        assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    }

    // --- 400 Bad Request tests ---

    #[tokio::test]
    async fn malformed_json_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app.oneshot(post_request("not json".into())).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap_or("")
                .contains("malformed request body")
        );
    }

    #[tokio::test]
    async fn unsupported_envelope_version_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = serde_json::json!({
            "envelope_version": "9.9.9",
            "repository_id": "repo-a",
            "workspace_id": "ws-1",
            "agent_id": "pi",
            "events": []
        })
        .to_string();

        let response = app.oneshot(post_request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap_or("")
                .contains("unsupported envelope_version")
        );
    }

    #[tokio::test]
    async fn missing_repository_id_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": "",
            "workspace_id": "ws-1",
            "agent_id": "pi",
            "events": []
        })
        .to_string();

        let response = app.oneshot(post_request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap_or("")
                .contains("missing top-level repository_id")
        );
    }

    #[tokio::test]
    async fn missing_workspace_id_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": "repo-a",
            "workspace_id": "",
            "agent_id": "pi",
            "events": []
        })
        .to_string();

        let response = app.oneshot(post_request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_agent_id_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": "repo-a",
            "workspace_id": "ws-1",
            "agent_id": "",
            "events": []
        })
        .to_string();

        let response = app.oneshot(post_request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // --- Hotspot endpoint tests ---

    #[tokio::test]
    async fn cumulative_hotspots_returns_live_response() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        // Ingest an event first.
        let _ = app
            .clone()
            .oneshot(post_request(make_valid_body()))
            .await
            .unwrap();

        let response = app
            .oneshot(get_request(
                "/v1/repositories/repo-a/hotspots?window=cumulative",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["schemaVersion"], "1.0.0");
        assert_eq!(json["repositoryId"], "repo-a");
        assert!(json["generatedAt"].as_str().unwrap_or("").len() >= 20);
        assert!(json["entries"].is_array());
    }

    #[tokio::test]
    async fn unknown_repository_returns_empty_entries() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(get_request(
                "/v1/repositories/unknown-repo/hotspots?window=cumulative",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["repositoryId"], "unknown-repo");
        assert_eq!(json["entries"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn unsupported_window_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(get_request(
                "/v1/repositories/repo-a/hotspots?window=recent",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert!(
            json["error"]
                .as_str()
                .unwrap_or("")
                .contains("unsupported window")
        );
    }

    #[tokio::test]
    async fn hotspot_response_has_no_filesystem_fields() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let _ = app
            .clone()
            .oneshot(post_request(make_valid_body()))
            .await
            .unwrap();

        let response = app
            .oneshot(get_request(
                "/v1/repositories/repo-a/hotspots?window=cumulative",
            ))
            .await
            .unwrap();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        assert!(!body_str.contains("repositoryPath"));
        assert!(!body_str.contains("storePath"));
    }

    // --- Concurrency tests ---

    #[tokio::test]
    async fn concurrent_duplicate_submissions_yield_one_accepted_one_idempotent() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();
        let body = make_valid_body();

        // Spawn 3 concurrent submissions with identical composite key.
        let mut handles = Vec::new();
        for _ in 0..3 {
            let app = app.clone();
            let body = body.clone();
            handles.push(tokio::spawn(async move {
                let resp = app.oneshot(post_request(body)).await.unwrap();
                let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
                    .await
                    .unwrap();
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            results.push(h.await.unwrap());
        }

        let mut accepted = 0u64;
        let mut idempotent = 0u64;
        for r in &results {
            accepted += r["accepted_count"].as_u64().unwrap_or(0);
            idempotent += r["duplicate_count"].as_u64().unwrap_or(0);
        }

        assert_eq!(
            accepted, 1,
            "concurrent identical submissions must produce exactly 1 accepted, got {accepted}"
        );
        assert_eq!(
            idempotent, 2,
            "concurrent identical submissions must produce exactly 2 idempotent, got {idempotent}"
        );
    }

    #[tokio::test]
    async fn concurrent_submissions_with_distinct_keys_all_accepted() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body_template = |pid: &str| -> String {
            serde_json::json!({
                "envelope_version": "1.0.0",
                "repository_id": "repo-a",
                "workspace_id": "ws-1",
                "agent_id": "pi",
                "events": [{
                    "producer_event_id": pid,
                    "client_timestamp": "2026-06-24T10:00:05Z",
                    "event": {
                        "schema_version": "0.1.0",
                        "timestamp": "2026-06-24T10:00:00Z",
                        "session_id": "s1",
                        "event_type": "DocRetrieved",
                        "tool_name": "read",
                        "payload": { "type": "DocRetrieved", "doc_ref": "doc/a.md" },
                        "outcome": { "result": "Success" }
                    }
                }]
            })
            .to_string()
        };

        let mut handles = Vec::new();
        for i in 0..5 {
            let app = app.clone();
            let body = body_template(&format!("evt-{i:03}"));
            handles.push(tokio::spawn(async move {
                let resp = app.oneshot(post_request(body)).await.unwrap();
                let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
                    .await
                    .unwrap();
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            results.push(h.await.unwrap());
        }

        let mut total_accepted = 0u64;
        let mut total_idempotent = 0u64;
        for r in &results {
            total_accepted += r["accepted_count"].as_u64().unwrap_or(0);
            total_idempotent += r["duplicate_count"].as_u64().unwrap_or(0);
        }

        assert_eq!(
            total_accepted, 5,
            "concurrent distinct-key submissions must all be accepted, got {total_accepted}"
        );
        assert_eq!(
            total_idempotent, 0,
            "concurrent distinct-key submissions must have zero idempotent, got {total_idempotent}"
        );
    }

    // ------------------------------------------------------------------
    // Task 4.2: SSE endpoint tests
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn signals_endpoint_returns_text_event_stream_content_type() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(get_request("/v1/repositories/repo-a/signals"))
            .await
            .unwrap();

        let content_type = response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            content_type.starts_with("text/event-stream"),
            "expected text/event-stream, got {content_type}"
        );
    }

    #[tokio::test]
    async fn signals_endpoint_replays_persisted_signals() {
        // Verify that persisted signals are replayed by the SSE endpoint.
        // The SSE stream is infinite (replay + live broadcast), so we verify
        // replay content via the store directly and check the endpoint
        // returns 200 with correct content type.
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 2).unwrap(); // threshold=2 for quick signals

        // Create signals by ingesting events.
        let body = serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": "repo-a",
            "workspace_id": "ws-1",
            "agent_id": "pi",
            "events": [
                {
                    "producer_event_id": "evt-001",
                    "client_timestamp": "2026-06-24T10:00:05Z",
                    "event": {
                        "schema_version": "0.1.0",
                        "timestamp": "2026-06-24T10:00:00Z",
                        "session_id": "s1",
                        "event_type": "FileOpened",
                        "tool_name": "read",
                        "payload": { "type": "FileOpened", "path": "src/a.rs" },
                        "outcome": { "result": "Success" }
                    }
                },
                {
                    "producer_event_id": "evt-002",
                    "client_timestamp": "2026-06-24T10:00:06Z",
                    "event": {
                        "schema_version": "0.1.0",
                        "timestamp": "2026-06-24T10:01:00Z",
                        "session_id": "s1",
                        "event_type": "FileOpened",
                        "tool_name": "read",
                        "payload": { "type": "FileOpened", "path": "src/a.rs" },
                        "outcome": { "result": "Success" }
                    }
                }
            ]
        })
        .to_string();

        let post_response = app.clone().oneshot(post_request(body)).await.unwrap();
        assert_eq!(post_response.status(), StatusCode::OK);

        // Verify replay content: open the store directly and check persisted
        // signals. This avoids consuming the infinite SSE body stream.
        let store = ServerStore::open(&store_path, 2).unwrap();
        let signals = store.get_signals_after("repo-a", 0).unwrap();
        assert!(
            !signals.is_empty(),
            "should have persisted signals after ingest (threshold=2, 2 events for same subject)"
        );

        // Each signal should have required fields.
        for signal in &signals {
            assert_eq!(signal.repository_id, "repo-a");
            assert!(signal.id > 0, "signal id must be positive");
            assert!(!signal.subject_kind.is_empty());
            assert!(!signal.subject.is_empty());
            assert!(signal.score > 0, "signal must have positive score");
            assert!(!signal.created_at.is_empty());
        }

        // Verify SSE endpoint returns 200 with correct content type.
        let sse_response = app
            .oneshot(get_request("/v1/repositories/repo-a/signals?after=0"))
            .await
            .unwrap();
        assert_eq!(sse_response.status(), StatusCode::OK);
        let content_type = sse_response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(
            content_type.starts_with("text/event-stream"),
            "SSE endpoint must return text/event-stream"
        );
    }
}
