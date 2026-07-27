use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use scryrs_server::server::{RoutePublishCredential, router_with_route_publish_credentials};
use serde_json::{Value, json};
use tempfile::tempdir;
use tower::ServiceExt;

const TOKEN_A: &str = "route-token-a";
const TOKEN_B: &str = "route-token-b";

fn app() -> axum::Router {
    let dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let path = dir.keep().join("server.db");
    router_with_route_publish_credentials(
        &path,
        10,
        vec![
            RoutePublishCredential::try_new("repo-a", "publisher-a", TOKEN_A)
                .unwrap_or_else(|error| panic!("credential: {error}")),
            RoutePublishCredential::try_new("repo-b", "publisher-b", TOKEN_B)
                .unwrap_or_else(|error| panic!("credential: {error}")),
        ],
    )
    .unwrap_or_else(|error| panic!("router: {error}"))
}

fn manifest(repository_id: &str, label: &str) -> Value {
    json!({
        "schemaVersion": "1.0.0",
        "metadata": {"repositoryId": repository_id},
        "routes": [{
            "id": "file:src/auth.rs",
            "subjectKind": "file",
            "subject": "src/auth.rs",
            "label": label,
            "target": "file:src/auth.rs",
            "loadTarget": {"kind": "file", "reference": "src/auth.rs"},
            "kind": "file",
            "evidenceLinks": [{
                "sourceKind": "server_trace_row",
                "subject": "src/auth.rs",
                "rowIds": [7],
                "score": 12
            }],
            "relatedEdges": [],
            "grouping": null
        }]
    })
}

fn publish_request(
    repository_id: &str,
    token: Option<&str>,
    body: impl Into<Body>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/v1/repositories/{repository_id}/routes/manifest"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder
        .body(body.into())
        .unwrap_or_else(|error| panic!("request: {error}"))
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .unwrap_or_else(|error| panic!("request: {error}"))
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|error| panic!("response body: {error}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("response JSON: {error}"))
}

#[tokio::test]
async fn authenticated_publication_and_explain_are_deterministic() {
    let app = app();
    let response = app
        .clone()
        .oneshot(publish_request(
            "repo-a",
            Some(TOKEN_A),
            manifest("repo-a", "Authentication").to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("publish: {error}"));

    assert_eq!(response.status(), StatusCode::OK);
    let publication = json_body(response).await;
    assert_eq!(publication["repositoryId"], "repo-a");
    assert_eq!(publication["schemaVersion"], "1.0.0");
    assert_eq!(publication["publisherId"], "publisher-a");
    assert_eq!(publication["unchanged"], false);
    assert_eq!(
        publication["contentSha256"].as_str().map(str::len),
        Some(64)
    );
    assert!(publication["publishedAt"].as_str().is_some());

    let first = app
        .clone()
        .oneshot(get_request(
            "/v1/repositories/repo-a/routes/explain?query=auth",
        ))
        .await
        .unwrap_or_else(|error| panic!("first explain: {error}"));
    let second = app
        .oneshot(get_request(
            "/v1/repositories/repo-a/routes/explain?query=auth",
        ))
        .await
        .unwrap_or_else(|error| panic!("second explain: {error}"));

    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let first_json = json_body(first).await;
    let second_json = json_body(second).await;
    assert_eq!(first_json, second_json);
    assert_eq!(first_json["schemaVersion"], "1.0.0");
    assert_eq!(first_json["hints"][0]["label"], "Authentication");
    assert_eq!(first_json["hints"][0]["rank"], 1);
}

#[tokio::test]
async fn publication_requires_repository_bound_bearer_token() {
    let app = app();
    let missing = app
        .clone()
        .oneshot(publish_request(
            "repo-a",
            None,
            manifest("repo-a", "Authentication").to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("missing auth: {error}"));
    let wrong_repository = app
        .oneshot(publish_request(
            "repo-a",
            Some(TOKEN_B),
            manifest("repo-a", "Authentication").to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("wrong repository: {error}"));

    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_repository.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_publications_do_not_replace_latest_manifest() {
    let app = app();
    let valid = app
        .clone()
        .oneshot(publish_request(
            "repo-a",
            Some(TOKEN_A),
            manifest("repo-a", "Original").to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("valid publish: {error}"));
    assert_eq!(valid.status(), StatusCode::OK);

    let malformed = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), "{"))
        .await
        .unwrap_or_else(|error| panic!("malformed publish: {error}"));
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let mut mismatch = manifest("repo-a", "Replacement");
    mismatch["schemaVersion"] = json!("99.0.0");
    let mismatch_response = app
        .clone()
        .oneshot(publish_request(
            "repo-a",
            Some(TOKEN_A),
            mismatch.to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("schema mismatch publish: {error}"));
    assert_eq!(mismatch_response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut invalid_reference = manifest("repo-a", "Replacement");
    invalid_reference["routes"][0]["relatedEdges"] = json!([{
        "relationship": "uses",
        "targetRouteId": "missing-route",
        "evidenceLinks": []
    }]);
    let invalid_reference_response = app
        .clone()
        .oneshot(publish_request(
            "repo-a",
            Some(TOKEN_A),
            invalid_reference.to_string(),
        ))
        .await
        .unwrap_or_else(|error| panic!("invalid reference publish: {error}"));
    assert_eq!(
        invalid_reference_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let explain = app
        .oneshot(get_request(
            "/v1/repositories/repo-a/routes/explain?query=original",
        ))
        .await
        .unwrap_or_else(|error| panic!("explain original: {error}"));
    assert_eq!(explain.status(), StatusCode::OK);
    assert_eq!(json_body(explain).await["hints"][0]["label"], "Original");
}

#[tokio::test]
async fn publication_is_idempotent_and_repository_scoped() {
    let app = app();
    let body = manifest("repo-a", "Authentication").to_string();
    let first = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), body.clone()))
        .await
        .unwrap_or_else(|error| panic!("first publish: {error}"));
    let first_json = json_body(first).await;
    let replay = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), body))
        .await
        .unwrap_or_else(|error| panic!("replay publish: {error}"));
    assert_eq!(replay.status(), StatusCode::OK);
    let replay_json = json_body(replay).await;
    assert_eq!(replay_json["unchanged"], true);
    assert_eq!(replay_json["publishedAt"], first_json["publishedAt"]);
    assert_eq!(replay_json["contentSha256"], first_json["contentSha256"]);

    let isolated = app
        .oneshot(get_request(
            "/v1/repositories/repo-b/routes/explain?query=auth",
        ))
        .await
        .unwrap_or_else(|error| panic!("isolated explain: {error}"));
    assert_eq!(isolated.status(), StatusCode::NOT_FOUND);
    assert!(
        json_body(isolated).await["error"]
            .as_str()
            .is_some_and(|error| error.contains("publish a route manifest"))
    );
}

#[tokio::test]
async fn empty_query_and_oversized_manifest_are_rejected() {
    let app = app();
    let empty_query = app
        .clone()
        .oneshot(get_request(
            "/v1/repositories/repo-a/routes/explain?query=%20%20",
        ))
        .await
        .unwrap_or_else(|error| panic!("empty query: {error}"));
    assert_eq!(empty_query.status(), StatusCode::BAD_REQUEST);

    let oversized = vec![b' '; scryrs_server::server::MAX_ROUTE_MANIFEST_BYTES + 1];
    let response = app
        .oneshot(publish_request("repo-a", Some(TOKEN_A), oversized))
        .await
        .unwrap_or_else(|error| panic!("oversized publish: {error}"));
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
