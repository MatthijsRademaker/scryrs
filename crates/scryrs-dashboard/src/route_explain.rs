//! Route explain handler for the dashboard.
//!
//! This module provides the `GET /api/routes/explain?query=<text>` endpoint,
//! which exposes the deterministic route hint matching from `scryrs_runtime` in
//! the browser dashboard.
//!
//! ## Manifest loading: deliberate duplication over CLI `route_common.rs`
//!
//! The CLI path (`crates/scryrs-cli/src/route_common.rs`) uses `writeln!` to
//! stderr and returns exit codes — incompatible with the dashboard's `ApiError`
//! HTTP pattern. This module contains a standalone load-and-validate helper that
//! maps missing → 404, malformed JSON → 502, and schema mismatch → 502.
//! The two paths are intentionally independent; they are documented here so
//! future maintainers understand the relationship.

use std::path::Path;
use std::sync::Arc;

use axum::extract::{Query, State};

use crate::server::{ApiError, AppState, live_api_url};
use axum::Json;
use serde::Deserialize;
use serde_json::Value;

/// Query parameter deserialization for `GET /api/routes/explain?query=...`.
#[derive(Deserialize)]
pub(crate) struct RouteExplainQuery {
    /// The search query text. Must be non-empty.
    pub(crate) query: Option<String>,
}

/// Load `.scryrs/routes.json` and validate its schema version.
///
/// Returns `Ok(RouteManifestDocument)` on success, or `ApiError` for missing
/// artifact (404), malformed JSON (502), or schema version mismatch (502).
///
/// ## Relationship to CLI `route_common.rs`
///
/// This is a deliberate fork of the CLI's `load_route_manifest()`.
/// The CLI writes diagnostic messages to stderr and returns exit codes (2),
/// which work for command-line UX but not for HTTP API error responses.
/// This function maps the same three failure modes to `ApiError` variants
/// that the dashboard frontend handles with distinct remediation messages.
fn load_route_manifest(repo_root: &Path) -> Result<scryrs_types::RouteManifestDocument, ApiError> {
    let routes_path = repo_root.join(".scryrs/routes.json");
    let routes_json = std::fs::read_to_string(&routes_path).map_err(|err| {
        if err.kind() == std::io::ErrorKind::NotFound {
            ApiError::missing(format!(
                "route artifact not found at {} — run `scryrs route <PATH>` to generate the route manifest",
                routes_path.display()
            ))
        } else {
            ApiError::bad_gateway(format!("cannot read route artifact: {err}"))
        }
    })?;

    let manifest: scryrs_types::RouteManifestDocument =
        serde_json::from_str(&routes_json).map_err(|err| {
            ApiError::bad_gateway(format!(
                "route artifact is malformed (invalid JSON): {err} — run `scryrs route <PATH>` to regenerate"
            ))
        })?;

    if manifest.schema_version != scryrs_types::ROUTE_SCHEMA_VERSION {
        return Err(ApiError::bad_gateway(format!(
            "route artifact schema version mismatch: got '{}', expected '{}' — run `scryrs route <PATH>` to regenerate",
            manifest.schema_version,
            scryrs_types::ROUTE_SCHEMA_VERSION
        )));
    }

    Ok(manifest)
}

/// `GET /api/routes/explain?query=<text>`
///
/// Read-only, deterministic route explain endpoint using mode-selected data.
///
/// # Behavior
///
/// - **Empty or missing `?query=`:** HTTP 400 with a descriptive error.
/// - **Live mode:** proxies the configured repository's published manifest.
/// - **Missing `.scryrs/routes.json`:** HTTP 404 with remediation guidance.
/// - **Malformed JSON or schema mismatch:** HTTP 502 with regeneration guidance.
/// - **Zero matches:** HTTP 200 with an empty `hints` array.
/// - **Matches found:** HTTP 200 with a populated `RouteHintDocument`.
pub(crate) async fn route_explain(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RouteExplainQuery>,
) -> Result<Json<Value>, ApiError> {
    // Reject empty or missing query — empty queries match every route (unbounded).
    let query_text = match query.query.as_deref().map(str::trim) {
        None | Some("") => {
            return Err(ApiError {
                status: axum::http::StatusCode::BAD_REQUEST,
                message: "query parameter must be non-empty".into(),
            });
        }
        Some(query) => query,
    };

    if let Some(live) = state.config.source_mode.live_config() {
        return proxy_live_route_explain(&state.http_client, live, query_text).await;
    }

    let manifest = load_route_manifest(&state.config.repo_root)?;
    let hint_doc = scryrs_runtime::explain_hints(&manifest, query_text);

    let json = serde_json::to_value(&hint_doc).map_err(|err| {
        ApiError::bad_gateway(format!("route explain serialization failure: {err}"))
    })?;

    Ok(Json(json))
}

async fn proxy_live_route_explain(
    client: &reqwest::Client,
    live: &crate::LiveSourceConfig,
    query: &str,
) -> Result<Json<Value>, ApiError> {
    let response = client
        .get(live_api_url(
            live,
            &[
                "v1",
                "repositories",
                &live.repository_id,
                "routes",
                "explain",
            ],
        )?)
        .query(&[("query", query)])
        .send()
        .await
        .map_err(|error| {
            ApiError::bad_gateway(format!(
                "live route explain upstream request failed: {error}"
            ))
        })?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("upstream response body unavailable"));
        let message = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|json| json["error"].as_str().map(str::to_owned))
            .unwrap_or(body);
        return Err(ApiError::missing(message));
    }
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("upstream response body unavailable"));
        return Err(ApiError::bad_gateway(format!(
            "live route explain upstream returned {status}: {body}"
        )));
    }
    let document = response
        .json::<scryrs_types::RouteHintDocument>()
        .await
        .map_err(|error| {
            ApiError::bad_gateway(format!(
                "live route explain upstream response contract invalid: {error}"
            ))
        })?;
    let json = serde_json::to_value(document).map_err(|error| {
        ApiError::bad_gateway(format!("route explain serialization failure: {error}"))
    })?;
    Ok(Json(json))
}
