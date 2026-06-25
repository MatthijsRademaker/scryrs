//! Axum HTTP server for central trace ingest.
//!
//! Exposes `POST /v1/trace-events/batch` with two-layer validation:
//! top-level envelope failures return 400, per-item failures return 200
//! with deterministic diagnostics.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::Stream;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::Config;
use crate::store::ServerStore;
use crate::time::chrono_now;
use scryrs_types::{
    BatchIngestResponse, EventAckStatus, HotspotQueryParams, LiveHotspotsResponse,
    ServerIngestEnvelope,
};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<ServerStore>>,
    store_path: PathBuf,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Deserialize)]
struct SignalStreamParams {
    #[serde(default)]
    after: Option<i64>,
}

fn router(store_path: &std::path::Path, signal_threshold: u32) -> Result<Router, rusqlite::Error> {
    let store = ServerStore::open(store_path, signal_threshold)?;
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
        store_path: store_path.to_path_buf(),
    };
    Ok(Router::new()
        .route("/v1/trace-events/batch", post(ingest_batch))
        .route(
            "/v1/repositories/:repository_id/hotspots",
            get(get_hotspots),
        )
        .route(
            "/v1/repositories/:repository_id/signals",
            get(signal_stream),
        )
        .with_state(state))
}

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

    // Layer 2: Per-item processing with raw-value iteration.
    // Lock the store for the duration of batch processing.
    let acks = {
        let store = state.store.lock().unwrap_or_else(|e| e.into_inner());
        match store.ingest_batch(&envelope) {
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
        }
    };

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
        received_count: accepted_count + duplicate_count,
        events: acks,
        received_at: chrono_now(),
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn get_hotspots(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(params): Query<HotspotQueryParams>,
) -> axum::response::Response {
    // Validate window: only cumulative is supported.
    let window = params.window.as_deref().unwrap_or("cumulative");
    if window != "cumulative" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: format!(
                    "unsupported window '{}'; only 'cumulative' is supported",
                    window
                ),
            }),
        )
            .into_response();
    }

    // Reject session_id: session-scoped queries are deferred.
    if params.session_id.is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "session-scoped hotspot queries are not yet supported; omit session_id or use the cumulative window without session filter.".into(),
            }),
        )
            .into_response();
    }

    let entries = {
        let store = state.store.lock().unwrap_or_else(|e| e.into_inner());
        match store.query_hotspots(&repository_id, window) {
            Ok(entries) => entries,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody {
                        error: format!("query_hotspots failed: {e}"),
                    }),
                )
                    .into_response();
            }
        }
    };

    let generated_at = chrono_now();
    let response = LiveHotspotsResponse {
        schemaVersion: scryrs_types::LIVE_HOTSPOT_SCHEMA_VERSION.into(),
        repositoryId: repository_id,
        cursor: generated_at.clone(),
        generatedAt: generated_at,
        entries,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn signal_stream(
    State(state): State<AppState>,
    Path(repository_id): Path<String>,
    Query(params): Query<SignalStreamParams>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let store_path = state.store_path.clone();
    let repo = repository_id.clone();
    let start_after: Option<i64> = params.after;

    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<Result<Event, std::convert::Infallible>>();

    std::thread::spawn(move || {
        // Open a separate read-only connection for this SSE stream.
        let flags =
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = match rusqlite::Connection::open_with_flags(&store_path, flags) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Ok(Event::default()
                    .comment(format!("failed to open read-only store connection: {e}"))));
                return;
            }
        };

        let mut cursor: Option<i64> = start_after;

        loop {
            // Poll for new signals.
            let sql = if cursor.is_some() {
                "SELECT id, subject_kind, subject, score, delta, window,
                        threshold, evidence_row_ids, created_at
                 FROM hotspot_signals
                 WHERE repository_id = ?1 AND id > ?2
                 ORDER BY id ASC
                 LIMIT 50"
                    .to_string()
            } else {
                "SELECT id, subject_kind, subject, score, delta, window,
                        threshold, evidence_row_ids, created_at
                 FROM hotspot_signals
                 WHERE repository_id = ?1
                 ORDER BY id ASC
                 LIMIT 50"
                    .to_string()
            };

            let rows: Vec<scryrs_types::HotspotSignalEvent> = match (|| -> rusqlite::Result<_> {
                let mut stmt = conn.prepare(&sql)?;
                if let Some(after) = cursor {
                    stmt.query_map(rusqlite::params![&repo, after], |r| {
                        Ok(scryrs_types::HotspotSignalEvent {
                            id: r.get(0)?,
                            repositoryId: repo.clone(),
                            subjectKind: r.get(1)?,
                            subject: r.get(2)?,
                            score: r.get(3)?,
                            delta: r.get(4)?,
                            window: r.get(5)?,
                            threshold: r.get(6)?,
                            evidenceRowIds: serde_json::from_str(&r.get::<_, String>(7)?)
                                .unwrap_or_default(),
                            createdAt: r.get(8)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()
                } else {
                    stmt.query_map(rusqlite::params![&repo], |r| {
                        Ok(scryrs_types::HotspotSignalEvent {
                            id: r.get(0)?,
                            repositoryId: repo.clone(),
                            subjectKind: r.get(1)?,
                            subject: r.get(2)?,
                            score: r.get(3)?,
                            delta: r.get(4)?,
                            window: r.get(5)?,
                            threshold: r.get(6)?,
                            evidenceRowIds: serde_json::from_str(&r.get::<_, String>(7)?)
                                .unwrap_or_default(),
                            createdAt: r.get(8)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()
                }
            })() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Ok(Event::default().comment(format!("DB poll error: {e}"))));
                    return;
                }
            };

            for signal in &rows {
                let data = match serde_json::to_string(signal) {
                    Ok(json) => json,
                    Err(e) => {
                        let _ = tx.send(Ok(
                            Event::default().comment(format!("serialization error: {e}"))
                        ));
                        return;
                    }
                };
                let event = Event::default().id(signal.id.to_string()).data(data);
                if tx.send(Ok(event)).is_err() {
                    // Client disconnected.
                    return;
                }
            }

            // Update cursor to last seen id.
            if let Some(last) = rows.last() {
                cursor = Some(last.id);
            }

            // Poll interval.
            std::thread::sleep(Duration::from_secs(2));
        }
    });

    Sse::new(UnboundedReceiverStream::new(rx))
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)))
}

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

    fn request(body: String) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/v1/trace-events/batch")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    // --- Valid batch tests ---

    #[tokio::test]
    async fn valid_batch_returns_200_with_accepted_event() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app.oneshot(request(make_valid_body())).await.unwrap();
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
        let r1 = app.clone().oneshot(request(body.clone())).await.unwrap();
        assert_eq!(r1.status(), StatusCode::OK);

        // Second submission with same body.
        let r2 = app.oneshot(request(body)).await.unwrap();
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

        let response = app.oneshot(request("not json".into())).await.unwrap();
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

        let response = app.oneshot(request(body)).await.unwrap();
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

        let response = app.oneshot(request(body)).await.unwrap();
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

        let response = app.oneshot(request(body)).await.unwrap();
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

        let response = app.oneshot(request(body)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
                let resp = app.oneshot(request(body)).await.unwrap();
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

        // Count accepted vs idempotent per-item statuses.
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
                let resp = app.oneshot(request(body)).await.unwrap();
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

    // --- Hotspot query endpoint tests (3.4) ---

    fn hotspot_request(repo: &str, query: &str) -> Request<Body> {
        let uri = format!("/v1/repositories/{repo}/hotspots{query}");
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn hotspot_query_valid_returns_ranked_entries() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        // Ingest events to populate accumulators.
        let body = make_valid_body();
        let _ = app.clone().oneshot(request(body)).await.unwrap();

        let response = app.oneshot(hotspot_request("repo-a", "")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(json["schemaVersion"], "1.0.0");
        assert_eq!(json["repositoryId"], "repo-a");
        assert!(!json["cursor"].as_str().unwrap_or("").is_empty());
        assert!(!json["generatedAt"].as_str().unwrap_or("").is_empty());
        assert!(
            !json["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .is_empty()
        );
    }

    #[tokio::test]
    async fn hotspot_query_unknown_repo_returns_empty() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(hotspot_request("unknown-repo", ""))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(json["repositoryId"], "unknown-repo");
        assert!(
            json["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .is_empty()
        );
    }

    #[tokio::test]
    async fn hotspot_query_unsupported_window_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(hotspot_request("repo-a", "?window=recent"))
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
    async fn hotspot_query_session_id_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(hotspot_request("repo-a", "?session_id=s1"))
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
                .contains("session-scoped")
        );
    }

    #[tokio::test]
    async fn hotspot_query_session_id_with_valid_window_still_returns_400() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(hotspot_request(
                "repo-a",
                "?window=cumulative&session_id=s1",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn hotspot_query_response_has_no_filesystem_path_fields() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let body = make_valid_body();
        let _ = app.clone().oneshot(request(body)).await.unwrap();

        let response = app.oneshot(hotspot_request("repo-a", "")).await.unwrap();
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert!(!body_str.contains("repositoryPath"));
        assert!(!body_str.contains("storePath"));
    }

    // --- SSE signal stream endpoint tests (4.5) ---

    fn signal_request(repo: &str, after: Option<i64>) -> Request<Body> {
        let uri = if let Some(a) = after {
            format!("/v1/repositories/{repo}/signals?after={a}")
        } else {
            format!("/v1/repositories/{repo}/signals")
        };
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn sse_content_type_header() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app.oneshot(signal_request("repo-a", None)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with("text/event-stream"),
            "expected text/event-stream, got: {ct}"
        );
    }

    #[tokio::test]
    async fn sse_empty_repository_returns_200() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(signal_request("unknown-repo", None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.starts_with("text/event-stream"));
    }

    #[tokio::test]
    async fn sse_with_after_parameter_returns_200() {
        let dir = temp_dir();
        let store_path = dir.path().join("server.db");
        let app = router(&store_path, 10).unwrap();

        let response = app
            .oneshot(signal_request("repo-a", Some(42)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let ct = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(ct.starts_with("text/event-stream"));
    }
}
