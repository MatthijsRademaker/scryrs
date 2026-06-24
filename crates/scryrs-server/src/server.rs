//! Axum HTTP server for central trace ingest.
//!
//! Exposes `POST /v1/trace-events/batch` with two-layer validation:
//! top-level envelope failures return 400, per-item failures return 200
//! with deterministic diagnostics.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::Serialize;

use crate::Config;
use crate::store::ServerStore;
use crate::time::chrono_now;
use scryrs_types::{BatchIngestResponse, EventAckStatus, ServerIngestEnvelope};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<ServerStore>>,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

fn router(store_path: &std::path::Path) -> Result<Router, rusqlite::Error> {
    let store = ServerStore::open(store_path)?;
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
    };
    Ok(Router::new()
        .route("/v1/trace-events/batch", post(ingest_batch))
        .with_state(state))
}

pub async fn serve(config: Config) -> Result<(), crate::ServerError> {
    let bind = SocketAddr::new(config.bind_address, config.port);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    let addr = listener.local_addr()?;
    write_startup_message(&addr, &config.store_path)?;

    let app = router(&config.store_path)
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
        store.ingest_batch(&envelope)
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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();

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
        let app = router(&store_path).unwrap();
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
        let app = router(&store_path).unwrap();

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
}
