use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{Body, to_bytes};
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json as AxumJson, Router as AxumRouter};
use bytes::Bytes;
use futures_util::StreamExt;
use scryrs_core::EventStore;
use scryrs_dashboard::server::router;
use scryrs_dashboard::{Config, SourceMode};
use scryrs_types::{
    EnvelopeEvent, FileOpenedPayload, Outcome, SCHEMA_VERSION, SearchRunPayload,
    ServerIngestEnvelope, TraceEvent, TraceEventPayload, TraceEventType,
};
use tower::ServiceExt;

fn request(path: &str) -> Request<Body> {
    Request::builder()
        .uri(path)
        .body(Body::empty())
        .unwrap_or_else(|err| panic!("request build failed: {err}"))
}

fn json_request(method: &str, path: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|err| panic!("json request build failed: {err}"))
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

async fn mock_live_sessions(
    State(state): State<MockLiveState>,
    AxumPath(repository_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> AxumJson<serde_json::Value> {
    state
        .requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .push(format!(
            "/v1/repositories/{repository_id}/sessions?limit={}",
            query.get("limit").map(String::as_str).unwrap_or("")
        ));
    AxumJson(serde_json::json!({
        "sessions": [{
            "sessionId": "live-s1",
            "startedAt": "2026-07-05T10:00:00Z",
            "endedAt": null,
            "eventCount": 2,
            "source": "pi"
        }],
        "nextCursor": null
    }))
}

async fn mock_invalid_live_sessions() -> AxumJson<serde_json::Value> {
    AxumJson(serde_json::json!({"unexpected": true}))
}

async fn mock_live_session_detail(
    State(state): State<MockLiveState>,
    AxumPath((repository_id, session_id)): AxumPath<(String, String)>,
) -> AxumJson<serde_json::Value> {
    state
        .requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .push(format!(
            "/v1/repositories/{repository_id}/sessions/{session_id}"
        ));
    AxumJson(serde_json::json!({
        "session": {
            "sessionId": session_id,
            "startedAt": "2026-07-05T10:00:00Z",
            "endedAt": null,
            "eventCount": 1,
            "source": "pi"
        },
        "events": [{
            "eventId": 7,
            "sessionId": session_id,
            "eventType": "FileOpened",
            "timestamp": "2026-07-05T10:00:00Z",
            "subjectKind": "file",
            "subject": "src/main.rs",
            "payload": {"path": "src/main.rs"}
        }]
    }))
}

async fn mock_live_events(
    State(state): State<MockLiveState>,
    AxumPath(repository_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> AxumJson<serde_json::Value> {
    let mut query = query.into_iter().collect::<Vec<_>>();
    query.sort();
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
            "/v1/repositories/{repository_id}/events?{query_string}"
        ));
    AxumJson(serde_json::json!({
        "events": [{
            "eventId": 7,
            "sessionId": "live-s1",
            "eventType": "FileOpened",
            "timestamp": "2026-07-05T10:00:00Z",
            "subjectKind": "file",
            "subject": "src/main.rs",
            "payload": {"path": "src/main.rs"}
        }],
        "nextCursor": "6"
    }))
}

async fn mock_live_route_explain(
    State(state): State<MockLiveState>,
    AxumPath(repository_id): AxumPath<String>,
    Query(query): Query<HashMap<String, String>>,
) -> axum::response::Response {
    let query_text = query.get("query").cloned().unwrap_or_default();
    state
        .requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .push(format!(
            "/v1/repositories/{repository_id}/routes/explain?query={query_text}"
        ));
    match query_text.as_str() {
        "missing" => (
            StatusCode::NOT_FOUND,
            AxumJson(serde_json::json!({
                "error": "no route manifest published; publish a route manifest"
            })),
        )
            .into_response(),
        "failure" => (
            StatusCode::INTERNAL_SERVER_ERROR,
            AxumJson(serde_json::json!({"error": "store unavailable"})),
        )
            .into_response(),
        "none" => AxumJson(serde_json::json!({
            "schemaVersion": "1.0.0",
            "hints": []
        }))
        .into_response(),
        _ => AxumJson(serde_json::json!({
            "schemaVersion": "1.0.0",
            "hints": [{
                "routeId": "file:auth",
                "target": "file:auth",
                "loadTarget": {"kind": "file", "reference": "src/auth.rs"},
                "label": "Authentication",
                "rank": 1,
                "relevance": 3000012001_u64,
                "reason": "query match on label",
                "evidence": []
            }]
        }))
        .into_response(),
    }
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

fn populate_live_server_store(path: &std::path::Path) {
    let store = scryrs_server::store::ServerStore::open(path, 10)
        .unwrap_or_else(|err| panic!("open live server store: {err}"));
    let events = [
        event(
            "live-s1",
            "2026-07-05T10:00:00Z",
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
        ),
        event(
            "live-s1",
            "2026-07-05T10:01:00Z",
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "dashboard".into(),
            }),
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, event)| EnvelopeEvent {
        producer_event_id: format!("live-event-{index}"),
        client_timestamp: event.timestamp.clone(),
        event,
    })
    .collect();
    store
        .ingest_batch(&ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-seeded".into(),
            workspace_id: "workspace-seeded".into(),
            agent_id: "pi".into(),
            events,
        })
        .unwrap_or_else(|err| panic!("seed live server store: {err}"));
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
async fn live_sessions_and_events_proxy_repository_scoped_contracts() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state = MockLiveState {
        requests: requests.clone(),
    };
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/sessions",
            get(mock_live_sessions),
        )
        .route(
            "/v1/repositories/:repository_id/sessions/:session_id",
            get(mock_live_session_detail),
        )
        .route(
            "/v1/repositories/:repository_id/events",
            get(mock_live_events),
        )
        .with_state(state);
    let (server_url, live_server) = spawn_live_server(upstream).await;

    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    std::fs::create_dir_all(dir.path().join(".scryrs"))
        .unwrap_or_else(|err| panic!("create local artifacts: {err}"));
    std::fs::write(dir.path().join(".scryrs/scryrs.db"), "not sqlite")
        .unwrap_or_else(|err| panic!("write poison local store: {err}"));
    let config = live_config(dir.path().to_path_buf(), &server_url, "github.com/org/repo");

    let sessions = router(config.clone())
        .oneshot(request("/api/sessions?limit=25"))
        .await
        .unwrap_or_else(|err| panic!("sessions route: {err}"));
    assert_eq!(sessions.status(), StatusCode::OK);
    let sessions_json = response_json(sessions).await;
    assert_eq!(sessions_json.as_array().map(Vec::len), Some(1));
    assert_eq!(sessions_json[0]["sessionId"], "live-s1");

    let detail = router(config.clone())
        .oneshot(request("/api/sessions/live-s1"))
        .await
        .unwrap_or_else(|err| panic!("detail route: {err}"));
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(response_json(detail).await["events"][0]["eventId"], 7);

    let events = router(config)
        .oneshot(request("/api/events?limit=2&cursor=7&sessionId=live-s1"))
        .await
        .unwrap_or_else(|err| panic!("events route: {err}"));
    assert_eq!(events.status(), StatusCode::OK);
    let events_json = response_json(events).await;
    assert_eq!(events_json["events"][0]["sessionId"], "live-s1");
    assert_eq!(events_json["nextCursor"], "6");

    live_server.abort();
    let captured = requests
        .lock()
        .unwrap_or_else(|err| panic!("lock requests: {err}"))
        .clone();
    assert!(captured.contains(&String::from(
        "/v1/repositories/github.com/org/repo/sessions?limit=25"
    )));
    assert!(captured.contains(&String::from(
        "/v1/repositories/github.com/org/repo/sessions/live-s1"
    )));
    assert!(captured.contains(&String::from(
        "/v1/repositories/github.com/org/repo/events?cursor=7&limit=2&session_id=live-s1"
    )));
}

#[tokio::test]
async fn live_dashboard_reads_sessions_and_events_from_seeded_server_database() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let store_path = dir.path().join("server.db");
    populate_live_server_store(&store_path);
    let upstream = scryrs_server::server::router(&store_path, 10)
        .unwrap_or_else(|err| panic!("server router: {err}"));
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dashboard_root = dir.path().join("dashboard-root");

    let sessions = router(live_config(
        dashboard_root.clone(),
        &server_url,
        "repo-seeded",
    ))
    .oneshot(request("/api/sessions"))
    .await
    .unwrap_or_else(|err| panic!("sessions route: {err}"));
    assert_eq!(sessions.status(), StatusCode::OK);
    let sessions_json = response_json(sessions).await;
    assert_eq!(sessions_json[0]["sessionId"], "live-s1");
    assert_eq!(sessions_json[0]["eventCount"], 2);

    let detail = router(live_config(
        dashboard_root.clone(),
        &server_url,
        "repo-seeded",
    ))
    .oneshot(request("/api/sessions/live-s1"))
    .await
    .unwrap_or_else(|err| panic!("detail route: {err}"));
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = response_json(detail).await;
    assert_eq!(detail_json["events"].as_array().map(Vec::len), Some(2));
    assert_eq!(detail_json["events"][0]["eventId"], 1);

    let events = router(live_config(dashboard_root, &server_url, "repo-seeded"))
        .oneshot(request("/api/events?limit=1"))
        .await
        .unwrap_or_else(|err| panic!("events route: {err}"));
    assert_eq!(events.status(), StatusCode::OK);
    let events_json = response_json(events).await;
    assert_eq!(events_json["events"].as_array().map(Vec::len), Some(1));
    assert_eq!(events_json["events"][0]["eventId"], 2);
    assert_eq!(events_json["nextCursor"], "2");

    live_server.abort();
}

#[tokio::test]
async fn live_events_returns_bad_gateway_when_upstream_is_unreachable() {
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
    .oneshot(request("/api/events"))
    .await
    .unwrap_or_else(|err| panic!("events route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("live events upstream request failed"))
    );
}

#[tokio::test]
async fn live_sessions_rejects_invalid_upstream_contract() {
    let upstream = AxumRouter::new().route(
        "/v1/repositories/:repository_id/sessions",
        get(mock_invalid_live_sessions),
    );
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(live_config(dir.path().to_path_buf(), &server_url, "repo-a"))
        .oneshot(request("/api/sessions"))
        .await
        .unwrap_or_else(|err| panic!("sessions route: {err}"));
    live_server.abort();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("response contract invalid"))
    );
}

// ── Route explain tests ────────────────────────────────────────────────────

fn minimal_routes_json() -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "routes": [
            {
                "id": "file:auth",
                "subjectKind": "file",
                "subject": "auth_handler",
                "label": "auth_handler",
                "target": "file:auth",
                "loadTarget": { "kind": "file", "reference": "auth_handler" },
                "kind": "file",
                "evidenceLinks": [
                    {
                        "sourceKind": "local_trace_row",
                        "subject": "auth_handler",
                        "rowIds": [1, 2]
                    }
                ],
                "relatedEdges": []
            },
            {
                "id": "search:auth",
                "subjectKind": "search",
                "subject": "authentication",
                "label": "authentication",
                "target": "search:auth",
                "loadTarget": { "kind": "non_loadable" },
                "kind": "search",
                "evidenceLinks": [
                    {
                        "sourceKind": "local_trace_row",
                        "subject": "authentication",
                        "rowIds": [3]
                    }
                ],
                "relatedEdges": []
            }
        ]
    })
}

fn write_routes_json(dir: &std::path::Path, json: &serde_json::Value) {
    let scryrs = dir.join(".scryrs");
    std::fs::create_dir_all(&scryrs).unwrap_or_else(|err| panic!("create .scryrs: {err}"));
    std::fs::write(
        scryrs.join("routes.json"),
        serde_json::to_string(json).unwrap_or_else(|err| panic!("serialize routes: {err}")),
    )
    .unwrap_or_else(|err| panic!("write routes: {err}"));
}

#[tokio::test]
async fn route_explain_success_returns_route_hint_document() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    write_routes_json(dir.path(), &minimal_routes_json());

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query=auth"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["schemaVersion"], "1.0.0");
    let hints = json["hints"]
        .as_array()
        .unwrap_or_else(|| panic!("hints is array"));
    // "auth" should match both "auth_handler" (file:auth) and "authentication" (search:auth)
    assert_eq!(hints.len(), 2);
    // file:auth matches exact in target; search:auth matches substring
    assert!(hints.iter().any(|h| h["routeId"] == "file:auth"));
    assert!(hints.iter().any(|h| h["routeId"] == "search:auth"));
}

#[tokio::test]
async fn route_explain_missing_artifact_returns_404() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query=auth"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let json = response_json(response).await;
    let error = json["error"]
        .as_str()
        .unwrap_or_else(|| panic!("error field"));
    assert!(error.contains("route artifact not found"));
    assert!(error.contains("scryrs route"));
}

#[tokio::test]
async fn route_explain_malformed_json_returns_502() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let scryrs = dir.path().join(".scryrs");
    std::fs::create_dir_all(&scryrs).unwrap_or_else(|err| panic!("create .scryrs: {err}"));
    std::fs::write(scryrs.join("routes.json"), "not json")
        .unwrap_or_else(|err| panic!("write: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query=auth"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(response).await;
    let error = json["error"]
        .as_str()
        .unwrap_or_else(|| panic!("error field"));
    assert!(error.contains("malformed"));
}

#[tokio::test]
async fn route_explain_schema_version_mismatch_returns_502() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let bad = serde_json::json!({
        "schemaVersion": "99.0.0",
        "metadata": {},
        "routes": []
    });
    write_routes_json(dir.path(), &bad);

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query=auth"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let json = response_json(response).await;
    let error = json["error"]
        .as_str()
        .unwrap_or_else(|| panic!("error field"));
    assert!(error.contains("schema version mismatch"));
    assert!(error.contains("99.0.0"));
    assert!(error.contains("1.0.0"));
}

#[tokio::test]
async fn route_explain_empty_query_returns_400() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    write_routes_json(dir.path(), &minimal_routes_json());

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query="))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = response_json(response).await;
    assert!(
        json["error"]
            .as_str()
            .is_some_and(|msg| msg.contains("non-empty"))
    );
}

#[tokio::test]
async fn route_explain_missing_query_returns_400() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    write_routes_json(dir.path(), &minimal_routes_json());

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = response_json(response).await;
    assert!(
        json["error"]
            .as_str()
            .is_some_and(|msg| msg.contains("non-empty"))
    );
}

#[tokio::test]
async fn route_explain_live_mode_proxies_repository_and_ignores_local_artifact() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/routes/explain",
            get(mock_live_route_explain),
        )
        .with_state(MockLiveState {
            requests: requests.clone(),
        });
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    write_routes_json(
        dir.path(),
        &serde_json::json!({"invalid": "local artifact"}),
    );

    let response = router(live_config(dir.path().to_path_buf(), &server_url, "repo-a"))
        .oneshot(request("/api/routes/explain?query=auth"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    live_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["hints"][0]["label"], "Authentication");
    assert_eq!(
        requests
            .lock()
            .unwrap_or_else(|err| panic!("lock requests: {err}"))
            .as_slice(),
        ["/v1/repositories/repo-a/routes/explain?query=auth"]
    );
}

#[tokio::test]
async fn route_explain_live_missing_publication_preserves_404() {
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/routes/explain",
            get(mock_live_route_explain),
        )
        .with_state(MockLiveState {
            requests: Arc::new(Mutex::new(Vec::new())),
        });
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(live_config(dir.path().to_path_buf(), &server_url, "repo-a"))
        .oneshot(request("/api/routes/explain?query=missing"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    live_server.abort();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("publish a route manifest"))
    );
}

#[tokio::test]
async fn route_explain_live_upstream_failure_returns_502() {
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/routes/explain",
            get(mock_live_route_explain),
        )
        .with_state(MockLiveState {
            requests: Arc::new(Mutex::new(Vec::new())),
        });
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(live_config(dir.path().to_path_buf(), &server_url, "repo-a"))
        .oneshot(request("/api/routes/explain?query=failure"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    live_server.abort();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("upstream returned"))
    );
}

#[tokio::test]
async fn route_explain_live_zero_match_returns_empty_hints() {
    let upstream = AxumRouter::new()
        .route(
            "/v1/repositories/:repository_id/routes/explain",
            get(mock_live_route_explain),
        )
        .with_state(MockLiveState {
            requests: Arc::new(Mutex::new(Vec::new())),
        });
    let (server_url, live_server) = spawn_live_server(upstream).await;
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(live_config(dir.path().to_path_buf(), &server_url, "repo-a"))
        .oneshot(request("/api/routes/explain?query=none"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));
    live_server.abort();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response_json(response).await["hints"],
        serde_json::json!([])
    );
}

#[tokio::test]
async fn route_explain_zero_match_returns_empty_hints() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    write_routes_json(dir.path(), &minimal_routes_json());

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/routes/explain?query=zzz_nonexistent"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["schemaVersion"], "1.0.0");
    let hints = json["hints"]
        .as_array()
        .unwrap_or_else(|| panic!("hints is array"));
    assert!(hints.is_empty());
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

// --- Proposal API tests ---

fn write_proposal(
    root: &std::path::Path,
    id: &str,
    title: &str,
    target_type: &str,
    content_md: &str,
    created_at: &str,
    row_ids: Vec<u64>,
) {
    write_proposal_value(
        root,
        id,
        title,
        target_type,
        serde_json::json!(content_md),
        created_at,
        row_ids,
    );
}

fn write_proposal_value(
    root: &std::path::Path,
    id: &str,
    title: &str,
    target_type: &str,
    proposed_content: serde_json::Value,
    created_at: &str,
    row_ids: Vec<u64>,
) {
    let dir = root.join(".scryrs/proposals");
    std::fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create proposals dir: {err}"));
    let json = serde_json::json!({
        "schemaVersion": "1.0.0",
        "id": id,
        "targetType": target_type,
        "title": title,
        "rationale": format!("Rationale for {title}"),
        "proposedContent": proposed_content,
        "evidence": [{
            "sourceKind": "hotspot_subject",
            "subject": "test-subject",
            "rowIds": row_ids
        }],
        "createdAt": created_at
    });
    std::fs::write(dir.join(format!("{id}.json")), json.to_string())
        .unwrap_or_else(|err| panic!("write proposal: {err}"));
}

fn write_review(
    root: &std::path::Path,
    outcome: &str,
    proposal_id: &str,
    reviewer: &str,
    decided_at: &str,
    rationale: &str,
) {
    let dir = root.join(format!(".scryrs/{outcome}"));
    std::fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("create review dir: {err}"));
    let target_type = if outcome == "accepted" {
        serde_json::json!("docs_note")
    } else {
        serde_json::json!(null)
    };
    let accepted_content = if outcome == "accepted" {
        serde_json::json!("reviewed content")
    } else {
        serde_json::json!(null)
    };
    let json = serde_json::json!({
        "schemaVersion": "1.0.0",
        "proposalId": proposal_id,
        "reviewer": reviewer,
        "decidedAt": decided_at,
        "rationale": rationale,
        "sourceEvidence": [{
            "sourceKind": "hotspot_subject",
            "subject": "test-subject",
            "rowIds": [1]
        }],
        "outcome": outcome,
        "targetType": target_type,
        "acceptedContent": accepted_content
    });
    std::fs::write(dir.join(format!("{proposal_id}.json")), json.to_string())
        .unwrap_or_else(|err| panic!("write review: {err}"));
}

fn make_valid_proposal_id(content_md: &str) -> String {
    use scryrs_types::{ProposalDocument, ProposalTargetType, ProposedContent};
    ProposalDocument::compute_id(
        &ProposalTargetType::DocsNote,
        &ProposedContent::Markdown(content_md.to_string()),
    )
    .unwrap_or_else(|err| panic!("compute proposal id: {err}"))
}

fn make_memory_patch_proposal_id(content: serde_json::Value) -> String {
    use scryrs_types::{ProposalDocument, ProposalTargetType, ProposedContent};
    ProposalDocument::compute_id(
        &ProposalTargetType::MemoryPatch,
        &ProposedContent::MemoryPatch(content),
    )
    .unwrap_or_else(|err| panic!("compute proposal id: {err}"))
}

#[tokio::test]
async fn live_proposal_inventory_detail_and_authenticated_review_are_proxied() {
    const TOKEN: &str = "proposal-dashboard-token";
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let server_store = dir.path().join("server.db");
    let upstream = scryrs_server::server::router_with_proposal_write_credentials(
        &server_store,
        10,
        vec![
            scryrs_server::server::ProposalWriteCredential::try_new("repo-live", "alice", TOKEN)
                .unwrap_or_else(|err| panic!("credential: {err}")),
        ],
    )
    .unwrap_or_else(|err| panic!("server router: {err}"));
    let (server_url, live_server) = spawn_live_server(upstream).await;

    let proposal_root = dir.path().join("proposal-source");
    let proposal_id = make_valid_proposal_id("live content");
    write_proposal(
        &proposal_root,
        &proposal_id,
        "Live proposal",
        "docs_note",
        "live content",
        "2026-07-26T12:00:00Z",
        vec![1],
    );
    let proposal_bytes = std::fs::read(
        proposal_root
            .join(".scryrs/proposals")
            .join(format!("{proposal_id}.json")),
    )
    .unwrap_or_else(|err| panic!("read proposal: {err}"));
    let publication = reqwest::Client::new()
        .post(format!("{server_url}/v1/repositories/repo-live/proposals"))
        .bearer_auth(TOKEN)
        .header("content-type", "application/json")
        .body(proposal_bytes)
        .send()
        .await
        .unwrap_or_else(|err| panic!("publish proposal: {err}"));
    assert_eq!(publication.status(), reqwest::StatusCode::CREATED);

    let dashboard_root = dir.path().join("dashboard-root");
    let source_mode = SourceMode::live(&server_url, "repo-live")
        .unwrap_or_else(|err| panic!("live mode: {err}"))
        .with_proposal_write_token(TOKEN)
        .unwrap_or_else(|err| panic!("proposal token: {err}"));
    let dashboard_config = Config::try_new(
        8080,
        "127.0.0.1"
            .parse()
            .unwrap_or_else(|err| panic!("parse localhost: {err}")),
        true,
        false,
        dashboard_root,
        source_mode,
    )
    .unwrap_or_else(|err| panic!("dashboard config: {err}"));

    let meta = router(dashboard_config.clone())
        .oneshot(request("/api/meta"))
        .await
        .unwrap_or_else(|err| panic!("meta: {err}"));
    let meta = response_json(meta).await;
    assert_eq!(meta["proposalReadsAvailable"], true);
    assert_eq!(meta["proposalReviewWritesAvailable"], true);

    let inventory = router(dashboard_config.clone())
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("inventory: {err}"));
    assert_eq!(inventory.status(), StatusCode::OK);
    assert_eq!(response_json(inventory).await[0]["proposalId"], proposal_id);

    let detail = router(dashboard_config.clone())
        .oneshot(request(&format!("/api/proposals/{proposal_id}")))
        .await
        .unwrap_or_else(|err| panic!("detail: {err}"));
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(response_json(detail).await["title"], "Live proposal");

    let accepted = router(dashboard_config.clone())
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{proposal_id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "approved",
                "decidedAt": "2026-07-26T13:00:00Z",
                "reviewedContent": "reviewed live content"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("accept: {err}"));
    assert_eq!(accepted.status(), StatusCode::CREATED);
    assert_eq!(response_json(accepted).await["outcome"], "accepted");

    let conflict = router(dashboard_config)
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{proposal_id}/reject"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "changed mind",
                "decidedAt": "2026-07-26T14:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("reject conflict: {err}"));
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    live_server.abort();
}

#[tokio::test]
async fn proposals_list_returns_rows_sorted_by_proposal_id() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id_a = make_valid_proposal_id("content a");
    let id_b = make_valid_proposal_id("content b");
    write_proposal(
        dir.path(),
        &id_a,
        "Proposal A",
        "docs_note",
        "content a",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_proposal(
        dir.path(),
        &id_b,
        "Proposal B",
        "docs_note",
        "content b",
        "2026-07-02T00:00:00Z",
        vec![2],
    );
    // Accept id_a; id_b stays pending.
    write_review(
        dir.path(),
        "accepted",
        &id_a,
        "reviewer1",
        "2026-07-03T00:00:00Z",
        "ok",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    let rows = json
        .as_array()
        .unwrap_or_else(|| panic!("expected array, got: {json}"));
    assert_eq!(rows.len(), 2);
    // Sorted by proposalId ascending. id_b < id_a lexicographically.
    assert_eq!(rows[0]["proposalId"], id_b);
    assert_eq!(rows[0]["state"], "pending");
    assert_eq!(rows[0]["title"], "Proposal B");
    assert_eq!(rows[1]["proposalId"], id_a);
    assert_eq!(rows[1]["state"], "accepted");
    assert_eq!(rows[1]["title"], "Proposal A");
}

#[tokio::test]
async fn proposals_list_returns_404_when_dir_missing() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("not found"))
    );
}

#[tokio::test]
async fn proposals_list_returns_502_for_malformed_json() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let proposals_dir = dir.path().join(".scryrs/proposals");
    std::fs::create_dir_all(&proposals_dir)
        .unwrap_or_else(|err| panic!("create proposals dir: {err}"));
    std::fs::write(proposals_dir.join("bad.json"), "not json")
        .unwrap_or_else(|err| panic!("write bad proposal: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("invalid JSON"))
    );
}

#[tokio::test]
async fn proposals_list_returns_502_for_conflicting_terminal_state() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("conflicting content");
    write_proposal(
        dir.path(),
        &id,
        "Conflicting",
        "docs_note",
        "conflicting content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_review(
        dir.path(),
        "accepted",
        &id,
        "r1",
        "2026-07-02T00:00:00Z",
        "accept",
    );
    write_review(
        dir.path(),
        "rejected",
        &id,
        "r1",
        "2026-07-02T00:00:00Z",
        "reject",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("conflicting"))
    );
}

#[tokio::test]
async fn proposal_detail_returns_full_document() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("detail content");
    write_proposal(
        dir.path(),
        &id,
        "Detail Proposal",
        "docs_note",
        "detail content",
        "2026-07-01T00:00:00Z",
        vec![1, 2],
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/proposals/{id}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["id"], id);
    assert_eq!(json["title"], "Detail Proposal");
    assert_eq!(json["targetType"], "docs_note");
    assert!(!json["rationale"].as_str().unwrap_or("").is_empty());
    assert_eq!(json["proposedContent"], "detail content");
    assert_eq!(json["evidence"].as_array().map(Vec::len), Some(1));
    assert!(json["reviewDecision"].is_null());
}

#[tokio::test]
async fn proposal_detail_includes_review_decision_meta() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("reviewed content");
    write_proposal(
        dir.path(),
        &id,
        "Reviewed",
        "docs_note",
        "reviewed content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_review(
        dir.path(),
        "accepted",
        &id,
        "alice",
        "2026-07-02T12:00:00Z",
        "looks good",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/proposals/{id}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    let rd = &json["reviewDecision"];
    assert!(!rd.is_null());
    assert_eq!(rd["outcome"], "accepted");
    assert_eq!(rd["reviewer"], "alice");
    assert_eq!(rd["decidedAt"], "2026-07-02T12:00:00Z");
    assert_eq!(rd["rationale"], "looks good");
    assert_eq!(rd["acceptedContent"], "reviewed content");
    assert_eq!(rd["targetType"], "docs_note");
}

#[tokio::test]
async fn proposal_detail_returns_404_for_unknown_id() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("existing");
    write_proposal(
        dir.path(),
        &id,
        "Existing",
        "docs_note",
        "existing",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/proposals/nonexistent"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("not found"))
    );
}

#[tokio::test]
async fn live_proposal_reads_report_unreachable_upstream() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let config = live_config(dir.path().to_path_buf(), "http://localhost:8081", "repo-a");

    let list = router(config.clone())
        .oneshot(request("/api/proposals"))
        .await
        .unwrap_or_else(|err| panic!("list route: {err}"));
    assert_eq!(list.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(list).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("upstream request failed"))
    );

    let detail = router(config)
        .oneshot(request("/api/proposals/any-id"))
        .await
        .unwrap_or_else(|err| panic!("detail route: {err}"));
    assert_eq!(detail.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(detail).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("upstream request failed"))
    );
}

#[tokio::test]
async fn proposals_accept_writes_review_decision() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("accept content");
    write_proposal(
        dir.path(),
        &id,
        "Accept Me",
        "docs_note",
        "accept content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "looks good",
                "decidedAt": "2026-07-03T00:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let accepted_path = dir.path().join(format!(".scryrs/accepted/{id}.json"));
    let accepted_json = std::fs::read_to_string(&accepted_path)
        .unwrap_or_else(|err| panic!("read accepted artifact {}: {err}", accepted_path.display()));
    let accepted: serde_json::Value = serde_json::from_str(&accepted_json)
        .unwrap_or_else(|err| panic!("parse accepted artifact: {err}"));
    assert_eq!(accepted["proposalId"], id);
    assert_eq!(accepted["outcome"], "accepted");
    assert_eq!(accepted["reviewer"], "alice");
    assert_eq!(accepted["acceptedContent"], "accept content");

    let proposal_json =
        std::fs::read_to_string(dir.path().join(format!(".scryrs/proposals/{id}.json")))
            .unwrap_or_else(|err| panic!("read proposal artifact: {err}"));
    let proposal: serde_json::Value = serde_json::from_str(&proposal_json)
        .unwrap_or_else(|err| panic!("parse proposal artifact: {err}"));
    assert_eq!(proposal["proposedContent"], "accept content");
}

#[tokio::test]
async fn proposals_accept_is_idempotent_for_identical_bytes() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("accept content");
    write_proposal(
        dir.path(),
        &id,
        "Accept Me",
        "docs_note",
        "accept content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let request_body = serde_json::json!({
        "reviewer": "alice",
        "rationale": "looks good",
        "decidedAt": "2026-07-03T00:00:00Z",
        "reviewedContent": "reviewed content"
    });

    let first = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            request_body.clone(),
        ))
        .await
        .unwrap_or_else(|err| panic!("first route: {err}"));
    assert_eq!(first.status(), StatusCode::OK);

    let accepted_path = dir.path().join(format!(".scryrs/accepted/{id}.json"));
    let first_bytes = std::fs::read_to_string(&accepted_path)
        .unwrap_or_else(|err| panic!("read accepted artifact {}: {err}", accepted_path.display()));

    let second = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            request_body,
        ))
        .await
        .unwrap_or_else(|err| panic!("second route: {err}"));
    assert_eq!(second.status(), StatusCode::OK);

    let second_bytes = std::fs::read_to_string(&accepted_path)
        .unwrap_or_else(|err| panic!("read accepted artifact {}: {err}", accepted_path.display()));
    assert_eq!(first_bytes, second_bytes);
}

#[tokio::test]
async fn proposals_accept_rejects_missing_metadata() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("missing metadata content");
    write_proposal(
        dir.path(),
        &id,
        "Needs Metadata",
        "docs_note",
        "missing metadata content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "decidedAt": "2026-07-03T00:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("invalid JSON body"))
    );
    assert!(
        !dir.path()
            .join(format!(".scryrs/accepted/{id}.json"))
            .exists()
    );
}

#[tokio::test]
async fn proposals_accept_rejects_reviewed_content_for_structured_targets() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let content = serde_json::json!({"patch": "alpha"});
    let id = make_memory_patch_proposal_id(content.clone());
    write_proposal_value(
        dir.path(),
        &id,
        "Structured Proposal",
        "memory_patch",
        content,
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "edited",
                "decidedAt": "2026-07-03T00:00:00Z",
                "reviewedContent": "override"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("reviewed content is not supported"))
    );
    assert!(
        !dir.path()
            .join(format!(".scryrs/accepted/{id}.json"))
            .exists()
    );
}

#[tokio::test]
async fn proposals_accept_returns_conflict_for_different_existing_bytes() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("conflict content");
    write_proposal(
        dir.path(),
        &id,
        "Conflict Proposal",
        "docs_note",
        "conflict content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_review(
        dir.path(),
        "accepted",
        &id,
        "bob",
        "2026-07-02T00:00:00Z",
        "previous decision",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "looks good",
                "decidedAt": "2026-07-03T00:00:00Z",
                "reviewedContent": "reviewed content"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("existing review decision differs"))
    );
}

#[tokio::test]
async fn proposals_reject_returns_conflict_for_different_existing_bytes() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("reject conflict content");
    write_proposal(
        dir.path(),
        &id,
        "Reject Conflict Proposal",
        "docs_note",
        "reject conflict content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let first = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/reject"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "off-scope",
                "decidedAt": "2026-07-03T00:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("first route: {err}"));
    assert_eq!(first.status(), StatusCode::OK);

    let rejected_path = dir.path().join(format!(".scryrs/rejected/{id}.json"));
    let first_bytes = std::fs::read_to_string(&rejected_path)
        .unwrap_or_else(|err| panic!("read rejected artifact {}: {err}", rejected_path.display()));

    let second = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{id}/reject"),
            serde_json::json!({
                "reviewer": "bob",
                "rationale": "duplicate but different",
                "decidedAt": "2026-07-04T00:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("second route: {err}"));

    assert_eq!(second.status(), StatusCode::CONFLICT);
    assert!(
        response_json(second).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("existing review decision differs"))
    );

    let second_bytes = std::fs::read_to_string(&rejected_path)
        .unwrap_or_else(|err| panic!("read rejected artifact {}: {err}", rejected_path.display()));
    assert_eq!(first_bytes, second_bytes);
}

#[tokio::test]
async fn proposals_review_write_rejects_invalid_decided_at() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let accept_id = make_valid_proposal_id("accept invalid decidedAt");
    write_proposal(
        dir.path(),
        &accept_id,
        "Invalid Accept Timestamp",
        "docs_note",
        "accept invalid decidedAt",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let reject_id = make_valid_proposal_id("reject invalid decidedAt");
    write_proposal(
        dir.path(),
        &reject_id,
        "Invalid Reject Timestamp",
        "docs_note",
        "reject invalid decidedAt",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    let accept = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{accept_id}/accept"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "looks good",
                "decidedAt": "not-a-timestamp"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("accept route: {err}"));
    assert_eq!(accept.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(accept).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("invalid decidedAt"))
    );
    assert!(
        !dir.path()
            .join(format!(".scryrs/accepted/{accept_id}.json"))
            .exists()
    );

    let reject = router(config(dir.path().to_path_buf()))
        .oneshot(json_request(
            "POST",
            &format!("/api/proposals/{reject_id}/reject"),
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "off-scope",
                "decidedAt": "not-a-timestamp"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("reject route: {err}"));
    assert_eq!(reject.status(), StatusCode::BAD_REQUEST);
    assert!(
        response_json(reject).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("invalid decidedAt"))
    );
    assert!(
        !dir.path()
            .join(format!(".scryrs/rejected/{reject_id}.json"))
            .exists()
    );
}

#[tokio::test]
async fn live_proposal_review_requires_configured_write_authorization() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    let config = live_config(dir.path().to_path_buf(), "http://localhost:8081", "repo-a");

    let response = router(config)
        .oneshot(json_request(
            "POST",
            "/api/proposals/any-id/accept",
            serde_json::json!({
                "reviewer": "alice",
                "rationale": "ok",
                "decidedAt": "2026-07-03T00:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("authorization is not configured"))
    );
}

#[tokio::test]
async fn proposal_detail_rejected_shows_review_decision() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("rejected content");
    write_proposal(
        dir.path(),
        &id,
        "Rejected",
        "docs_note",
        "rejected content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_review(
        dir.path(),
        "rejected",
        &id,
        "bob",
        "2026-07-03T00:00:00Z",
        "needs work",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/proposals/{id}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    let rd = &json["reviewDecision"];
    assert_eq!(rd["outcome"], "rejected");
    assert_eq!(rd["reviewer"], "bob");
}

#[tokio::test]
async fn proposal_detail_returns_502_when_review_evidence_does_not_match_proposal() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("mismatched evidence content");
    write_proposal(
        dir.path(),
        &id,
        "Mismatched Evidence",
        "docs_note",
        "mismatched evidence content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );

    // Write an accepted review with different sourceEvidence than the proposal.
    let accepted_dir = dir.path().join(".scryrs/accepted");
    std::fs::create_dir_all(&accepted_dir)
        .unwrap_or_else(|err| panic!("create accepted dir: {err}"));
    let review_json = serde_json::json!({
        "schemaVersion": "1.0.0",
        "proposalId": id,
        "reviewer": "bob",
        "decidedAt": "2026-07-02T00:00:00Z",
        "rationale": "ok",
        "sourceEvidence": [{
            "sourceKind": "hotspot_subject",
            "subject": "different-subject",
            "rowIds": [99]
        }],
        "outcome": "accepted",
        "targetType": "docs_note",
        "acceptedContent": "reviewed content"
    });
    std::fs::write(
        accepted_dir.join(format!("{id}.json")),
        review_json.to_string(),
    )
    .unwrap_or_else(|err| panic!("write review: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/proposals/{id}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    // Before the fix: this returned 200 with the mismatched review.
    // After the fix: validate_review_decision_matches_proposal rejects it.
    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("sourceEvidence"))
    );
}

#[tokio::test]
async fn proposal_detail_returns_502_for_conflicting_terminal_state() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));

    let id = make_valid_proposal_id("conflicting detail content");
    write_proposal(
        dir.path(),
        &id,
        "Conflicting Detail",
        "docs_note",
        "conflicting detail content",
        "2026-07-01T00:00:00Z",
        vec![1],
    );
    write_review(
        dir.path(),
        "accepted",
        &id,
        "r1",
        "2026-07-02T00:00:00Z",
        "accept",
    );
    write_review(
        dir.path(),
        "rejected",
        &id,
        "r1",
        "2026-07-02T00:00:00Z",
        "reject",
    );

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request(&format!("/api/proposals/{id}")))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert!(
        response_json(response).await["error"]
            .as_str()
            .is_some_and(|message| message.contains("conflicting"))
    );
}
