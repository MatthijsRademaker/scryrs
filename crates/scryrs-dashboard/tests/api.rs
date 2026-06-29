use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json as AxumJson, Router as AxumRouter};
use bytes::Bytes;
use futures_util::StreamExt;
use scryrs_core::EventStore;
use scryrs_dashboard::server::router;
use scryrs_dashboard::{Config, SourceMode};
use scryrs_types::{
    FileOpenedPayload, Outcome, SCHEMA_VERSION, SearchRunPayload, TraceEvent, TraceEventPayload,
    TraceEventType,
};
use tower::ServiceExt;

fn request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .body(Body::empty())
        .unwrap_or_else(|err| panic!("request build failed: {err}"))
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap_or_else(|err| panic!("read response body: {err}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|err| panic!("parse response json: {err}"))
}

fn config(repo_root: std::path::PathBuf) -> Config {
    Config::try_new(
        8080,
        "127.0.0.1"
            .parse()
            .unwrap_or_else(|err| panic!("parse localhost: {err}")),
        true,
        false,
        repo_root,
        SourceMode::Local,
    )
    .unwrap_or_else(|err| panic!("config: {err}"))
}

fn live_config(repo_root: std::path::PathBuf, server_url: &str, repository_id: &str) -> Config {
    Config::try_new(
        8080,
        "127.0.0.1"
            .parse()
            .unwrap_or_else(|err| panic!("parse localhost: {err}")),
        true,
        false,
        repo_root,
        SourceMode::live(server_url, repository_id)
            .unwrap_or_else(|err| panic!("live mode: {err}")),
    )
    .unwrap_or_else(|err| panic!("config: {err}"))
}

#[derive(Clone)]
struct MockLiveState {
    requests: Arc<Mutex<Vec<String>>>,
}

async fn mock_live_hotspots(
    State(state): State<MockLiveState>,
    AxumPath(repository_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> AxumJson<serde_json::Value> {
    let query_string = query
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    state
        .requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .push(format!(
            "/v1/repositories/{repository_id}/hotspots?{query_string}"
        ));
    AxumJson(serde_json::json!({
        "schemaVersion": "1.0.0",
        "repositoryId": repository_id,
        "cursor": "cursor-7",
        "generatedAt": "2026-06-29T19:00:00Z",
        "entries": [
            {
                "rank": 1,
                "subjectKind": "file",
                "subject": "/srv/repo/src/main.rs",
                "score": 13,
                "counts": {
                    "eventType": {"EditMade": 2},
                    "outcome": {"success": 2}
                },
                "sessionCount": 2,
                "firstSeen": "2026-06-29T18:00:00Z",
                "lastSeen": "2026-06-29T18:59:00Z",
                "evidence": {"rowIds": [41, 42]}
            }
        ]
    }))
}

async fn mock_live_signals(
    State(state): State<MockLiveState>,
    AxumPath(repository_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let query_string = query
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    state
        .requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .push(format!(
            "/v1/repositories/{repository_id}/signals?{query_string}"
        ));

    let stream = futures_util::stream::unfold(0, |step| async move {
        match step {
            0 => Some((
                Ok::<Bytes, Infallible>(Bytes::from_static(
                    b"id: 43\ndata: {\"repositoryId\":\"repo-a\",\"subjectKind\":\"file\",\"subject\":\"src/main.rs\",\"score\":10,\"delta\":1,\"window\":\"cumulative\",\"threshold\":10,\"evidenceRowIds\":[41],\"createdAt\":\"2026-06-29T19:00:00Z\"}\n\n",
                )),
                1,
            )),
            1 => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                Some((
                    Ok::<Bytes, Infallible>(Bytes::from_static(
                        b"id: 44\ndata: {\"repositoryId\":\"repo-a\",\"subjectKind\":\"file\",\"subject\":\"src/lib.rs\",\"score\":11,\"delta\":1,\"window\":\"cumulative\",\"threshold\":10,\"evidenceRowIds\":[42],\"createdAt\":\"2026-06-29T19:00:01Z\"}\n\n",
                    )),
                    2,
                ))
            }
            _ => None,
        }
    });

    let mut response = axum::response::Response::new(Body::from_stream(stream));
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/event-stream"),
    );
    response
}

async fn spawn_live_server(app: AxumRouter) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|err| panic!("bind live server: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| panic!("live server addr: {err}"));
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|err| panic!("serve live server: {err}"));
    });
    (format!("http://{}:{}", addr.ip(), addr.port()), handle)
}

fn event(session_id: &str, timestamp: &str, payload: TraceEventPayload) -> TraceEvent {
    let event_type = match payload {
        TraceEventPayload::FileOpened(_) => TraceEventType::FileOpened,
        TraceEventPayload::SearchRun(_) => TraceEventType::SearchRun,
        _ => panic!("unsupported test payload"),
    };
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type,
        tool_name: Some("pi".into()),
        payload,
        outcome: Outcome::Success,
    }
}

fn populate_store(root: &std::path::Path) {
    let store_path = root.join(".scryrs").join("scryrs.db");
    let mut store = EventStore::open(store_path).unwrap_or_else(|err| panic!("open store: {err}"));
    store
        .begin_transaction()
        .unwrap_or_else(|err| panic!("begin: {err}"));
    for trace_event in [
        event(
            "s1",
            "2026-06-21T09:00:00Z",
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
        ),
        event(
            "s2",
            "2026-06-21T10:00:00Z",
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "dashboard".into(),
            }),
        ),
        event(
            "s2",
            "2026-06-21T10:01:00Z",
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/lib.rs".into(),
            }),
        ),
    ] {
        store
            .append(&trace_event)
            .unwrap_or_else(|err| panic!("append: {err}"));
    }
    store
        .commit_transaction()
        .unwrap_or_else(|err| panic!("commit: {err}"));
}

#[tokio::test]
async fn meta_returns_repository_path() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let repo_root = dir.path().to_path_buf();

    let response = router(config(repo_root.clone()))
        .oneshot(request("/api/meta"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["mode"], "local");
    assert_eq!(json["repositoryPath"], repo_root.to_string_lossy().as_ref());
    assert!(json["repositoryId"].is_null());
}

#[tokio::test]
async fn meta_returns_live_mode_and_repository_id() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let repo_root = dir.path().to_path_buf();
    let live = SourceMode::live("http://localhost:8081", "repo-a")
        .unwrap_or_else(|err| panic!("live mode: {err}"));

    let response = router(
        Config::try_new(
            8080,
            "127.0.0.1"
                .parse()
                .unwrap_or_else(|err| panic!("parse localhost: {err}")),
            true,
            false,
            repo_root.clone(),
            live,
        )
        .unwrap_or_else(|err| panic!("config: {err}")),
    )
    .oneshot(request("/api/meta"))
    .await
    .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["mode"], "live");
    assert_eq!(json["repositoryId"], "repo-a");
    assert_eq!(json["repositoryPath"], repo_root.to_string_lossy().as_ref());
}

#[tokio::test]
async fn hotspots_returns_artifact_json() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    std::fs::create_dir_all(dir.path().join(".scryrs"))
        .unwrap_or_else(|err| panic!("create .scryrs: {err}"));
    std::fs::write(
        dir.path().join(".scryrs").join("hotspots.json"),
        r#"{"repositoryPath":"/work/scryrs","entries":[{"rank":1,"subject":"src/main.rs"}]}"#,
    )
    .unwrap_or_else(|err| panic!("write hotspots: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/hotspots"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["entries"][0]["subject"], "src/main.rs");
    // `/api/hotspots` serves the report file raw, so `repositoryPath` round-trips to the client.
    assert_eq!(json["repositoryPath"], "/work/scryrs");
}

#[tokio::test]
async fn sessions_are_ordered_by_start_desc_with_limit() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    populate_store(dir.path());

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/sessions?limit=1"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json.as_array().map(Vec::len), Some(1));
    assert_eq!(json[0]["sessionId"], "s2");
    assert_eq!(json[0]["eventCount"], 2);
}

#[tokio::test]
async fn events_use_cursor_pagination() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    populate_store(dir.path());

    let first = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/events?limit=2"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    assert_eq!(first.status(), StatusCode::OK);
    let first_json = response_json(first).await;
    assert_eq!(first_json["events"].as_array().map(Vec::len), Some(2));
    let cursor = first_json["nextCursor"]
        .as_str()
        .unwrap_or_else(|| panic!("next cursor missing"));

    let second = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/events?limit=2&cursor={cursor}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    assert_eq!(second.status(), StatusCode::OK);
    let second_json = response_json(second).await;
    assert_eq!(second_json["events"].as_array().map(Vec::len), Some(1));
    assert!(second_json["nextCursor"].is_null());
}

#[tokio::test]
async fn missing_files_return_json_404_and_corrupt_store_returns_502() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let missing_hotspots = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/hotspots"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    assert_eq!(missing_hotspots.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(missing_hotspots).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("hotspot report missing"))
    );

    let missing_store = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/sessions"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    assert_eq!(missing_store.status(), StatusCode::NOT_FOUND);

    std::fs::create_dir_all(dir.path().join(".scryrs"))
        .unwrap_or_else(|err| panic!("create .scryrs: {err}"));
    std::fs::write(dir.path().join(".scryrs").join("scryrs.db"), "not sqlite")
        .unwrap_or_else(|err| panic!("write corrupt db: {err}"));
    let corrupt = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/events"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    assert_eq!(corrupt.status(), StatusCode::BAD_GATEWAY);
}

#[tokio::test]
async fn live_hotspots_proxy_forwards_repository_and_cursor() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = MockLiveState {
        requests: requests.clone(),
    };
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/hotspots",
            get(mock_live_hotspots),
        )
        .with_state(state);
    let (server_url, live_server) = spawn_live_server(upstream).await;

    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let repository_id = "github.com/org/repo";
    let response = router(live_config(
        dir.path().to_path_buf(),
        &server_url,
        repository_id,
    ))
    .oneshot(request("/api/hotspots"))
    .await
    .unwrap_or_else(|err| panic!("dashboard route: {err}"));

    live_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["repositoryId"], repository_id);
    assert_eq!(json["cursor"], "cursor-7");
    assert_eq!(json["entries"][0]["subject"], "/srv/repo/src/main.rs");
    assert_eq!(
        requests
            .lock()
            .unwrap_or_else(|err| panic!("lock requests: {err}"))
            .as_slice(),
        ["/v1/repositories/github.com/org/repo/hotspots?window=cumulative"]
    );
}

#[tokio::test]
async fn live_hotspots_returns_bad_gateway_when_upstream_is_unreachable() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|err| panic!("reserve port: {err}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|err| panic!("reserved addr: {err}"));
    drop(listener);

    let response = router(live_config(
        dir.path().to_path_buf(),
        &format!("http://{}:{}", addr.ip(), addr.port()),
        "repo-a",
    ))
    .oneshot(request("/api/hotspots"))
    .await
    .unwrap_or_else(|err| panic!("dashboard route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("live hotspots upstream request failed"))
    );
}

#[tokio::test]
async fn live_mode_rejects_local_only_endpoints() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let config = live_config(dir.path().to_path_buf(), "http://localhost:8081", "repo-a");

    let sessions = router(config.clone())
        .oneshot(request("/api/sessions"))
        .await
        .unwrap_or_else(|err| panic!("sessions route: {err}"));
    assert_eq!(sessions.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(sessions).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("unavailable in live mode"))
    );

    let session_detail = router(config.clone())
        .oneshot(request("/api/sessions/s1"))
        .await
        .unwrap_or_else(|err| panic!("session detail route: {err}"));
    assert_eq!(session_detail.status(), StatusCode::NOT_FOUND);

    let events = router(config)
        .oneshot(request("/api/events"))
        .await
        .unwrap_or_else(|err| panic!("events route: {err}"));
    assert_eq!(events.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(events).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("unavailable in live mode"))
    );
}

#[tokio::test]
async fn live_signals_proxy_forwards_after_cursor_and_streams_first_chunk() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = MockLiveState {
        requests: requests.clone(),
    };
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/signals",
            get(mock_live_signals),
        )
        .with_state(state);
    let (server_url, live_server) = spawn_live_server(upstream).await;

    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let repository_id = "github.com/org/repo";
    let response = router(live_config(
        dir.path().to_path_buf(),
        &server_url,
        repository_id,
    ))
    .oneshot(request("/api/signals?after=42"))
    .await
    .unwrap_or_else(|err| panic!("dashboard route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    let mut stream = response.into_body().into_data_stream();
    let first_chunk = tokio::time::timeout(Duration::from_millis(100), stream.next())
        .await
        .unwrap_or_else(|_| panic!("first SSE chunk timed out"))
        .unwrap_or_else(|| panic!("expected first SSE chunk"))
        .unwrap_or_else(|err| panic!("read first SSE chunk: {err}"));
    let first_text = std::str::from_utf8(&first_chunk)
        .unwrap_or_else(|err| panic!("decode first SSE chunk: {err}"));

    live_server.abort();

    assert!(first_text.contains("id: 43"));
    assert!(first_text.contains("src/main.rs"));
    assert_eq!(
        requests
            .lock()
            .unwrap_or_else(|err| panic!("lock requests: {err}"))
            .as_slice(),
        ["/v1/repositories/github.com/org/repo/signals?after=42"]
    );
}
