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
    let state = Arc::new(AppState {
        store: Arc::new(Mutex::new(store)),
    });
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

async fn ingest_batch(
    State(state): State<Arc<AppState>>,
    body: String,
) -> axum::response::Response {
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

/// Return the current wall-clock time as an RFC 3339 string.
/// Mirrors `store::chrono_now`.
fn chrono_now() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days_since_epoch = secs / 86400;
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let seconds = remaining % 60;
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

fn civil_from_days(mut days: i64) -> (i64, u32, u32) {
    days += 719468;
    let era = if days >= 0 {
        days / 146097
    } else {
        (days - 146096) / 146097
    };
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}
