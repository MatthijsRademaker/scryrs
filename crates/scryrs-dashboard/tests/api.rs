use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use scryrs_core::EventStore;
use scryrs_dashboard::Config;
use scryrs_dashboard::server::router;
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
    )
    .unwrap_or_else(|err| panic!("config: {err}"))
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
async fn hotspots_returns_artifact_json() {
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    std::fs::create_dir_all(dir.path().join(".scryrs"))
        .unwrap_or_else(|err| panic!("create .scryrs: {err}"));
    std::fs::write(
        dir.path().join(".scryrs").join("hotspots.json"),
        r#"{"entries":[{"rank":1,"subject":"src/main.rs"}]}"#,
    )
    .unwrap_or_else(|err| panic!("write hotspots: {err}"));

    let response = router(config(dir.path().to_path_buf()))
        .oneshot(request("/api/hotspots"))
        .await
        .unwrap_or_else(|err| panic!("route: {err}"));

    assert_eq!(response.status(), StatusCode::OK);
    let json = response_json(response).await;
    assert_eq!(json["entries"][0]["subject"], "src/main.rs");
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
