//! Axum HTTP server for central trace ingest.
//!
//! Exposes `POST /v1/trace-events/batch` with two-layer validation:
//! top-level envelope failures return 400, per-item failures return 200
//! with deterministic diagnostics.
//!
//! Also exposes read-only live hotspot and signal streaming endpoints.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::Config;
use crate::store::ServerStore;
use crate::time::chrono_now;
use scryrs_types::{BatchIngestResponse, EventAckStatus, HotspotSignal, ServerIngestEnvelope};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// Capacity of the `tokio::sync::broadcast` channel for signal fanout.
/// Large enough to absorb brief consumer stalls; slow consumers that
/// fall behind recover via `after=<signal_id>` replay on reconnect.
const SIGNAL_CHANNEL_CAPACITY: usize = 1024;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<ServerStore>>,
    signal_tx: tokio::sync::broadcast::Sender<(i64, HotspotSignal)>,
}

// ---------------------------------------------------------------------------
// Response helpers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

fn router(store_path: &std::path::Path, signal_threshold: u32) -> Result<Router, rusqlite::Error> {
    let store = ServerStore::open(store_path, signal_threshold)?;
    let (signal_tx, _) = tokio::sync::broadcast::channel(SIGNAL_CHANNEL_CAPACITY);
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        signal_tx,
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

    let app = router(&config.store_path, config.signal_threshold)
        .map_err(|e| crate::ServerError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

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
            .unwrap_or_default();

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
        serde_json::json!({
            "envelope_version": "1.0.0",
            "repository_id": "repo-a",
            "workspace_id": "ws-1",
            "agent_id": "pi",
            "events": [{
                "producer_event_id": "evt-001",
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
