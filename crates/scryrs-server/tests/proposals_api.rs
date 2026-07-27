use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use scryrs_server::server::{ProposalWriteCredential, router_with_proposal_write_credentials};
use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
    ProposalTargetType, ProposedContent,
};
use serde_json::Value;
use tempfile::tempdir;
use tower::ServiceExt;

const TOKEN_A: &str = "proposal-token-a";
const TOKEN_B: &str = "proposal-token-b";

fn proposal(title: &str) -> ProposalDocument {
    let content = ProposedContent::Markdown("# Durable knowledge".into());
    ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id: ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|error| panic!("proposal id: {error}")),
        target_type: ProposalTargetType::DocsNote,
        title: title.into(),
        rationale: "Repeated evidence warrants documentation.".into(),
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::ServerTraceRow,
            subject: "src/auth.rs".into(),
            row_ids: vec![7],
            doc_ref: None,
            description: None,
            score: Some(12),
            metadata: None,
        }],
        created_at: "2026-07-26T12:00:00Z".into(),
    }
}

fn app() -> axum::Router {
    let dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    router_with_proposal_write_credentials(
        &dir.keep().join("server.db"),
        10,
        vec![
            ProposalWriteCredential::try_new("repo-a", "alice", TOKEN_A)
                .unwrap_or_else(|error| panic!("credential: {error}")),
            ProposalWriteCredential::try_new("repo-b", "bob", TOKEN_B)
                .unwrap_or_else(|error| panic!("credential: {error}")),
        ],
    )
    .unwrap_or_else(|error| panic!("router: {error}"))
}

fn publish_request(
    repository_id: &str,
    token: Option<&str>,
    proposal: &ProposalDocument,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/v1/repositories/{repository_id}/proposals"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder
        .body(Body::from(
            serde_json::to_vec(proposal).unwrap_or_else(|error| panic!("proposal JSON: {error}")),
        ))
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

fn review_request(
    repository_id: &str,
    proposal_id: &str,
    outcome: &str,
    token: Option<&str>,
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!(
            "/v1/repositories/{repository_id}/proposals/{proposal_id}/{outcome}"
        ))
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|error| panic!("request: {error}"))
}

#[tokio::test]
async fn authenticated_publication_is_visible_in_repository_inventory_and_detail() {
    let app = app();
    let proposal = proposal("Authentication notes");

    let publication = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &proposal))
        .await
        .unwrap_or_else(|error| panic!("publish: {error}"));
    assert_eq!(publication.status(), StatusCode::CREATED);
    let publication = json_body(publication).await;
    assert_eq!(publication["proposalId"], proposal.id);
    assert_eq!(publication["publisherId"], "alice");
    assert_eq!(publication["unchanged"], false);
    assert_eq!(
        publication["revisionSha256"].as_str().map(str::len),
        Some(64)
    );

    let inventory = app
        .clone()
        .oneshot(get_request("/v1/repositories/repo-a/proposals"))
        .await
        .unwrap_or_else(|error| panic!("inventory: {error}"));
    assert_eq!(inventory.status(), StatusCode::OK);
    let inventory = json_body(inventory).await;
    assert_eq!(inventory.as_array().map(Vec::len), Some(1));
    assert_eq!(inventory[0]["proposalId"], proposal.id);
    assert_eq!(inventory[0]["state"], "pending");

    let detail = app
        .oneshot(get_request(&format!(
            "/v1/repositories/repo-a/proposals/{}",
            proposal.id
        )))
        .await
        .unwrap_or_else(|error| panic!("detail: {error}"));
    assert_eq!(detail.status(), StatusCode::OK);
    let detail = json_body(detail).await;
    assert_eq!(detail["title"], "Authentication notes");
    assert_eq!(detail["proposedContent"], "# Durable knowledge");
    assert_eq!(detail["reviewDecision"], Value::Null);
    assert_eq!(detail["publication"]["publisherId"], "alice");
}

#[tokio::test]
async fn authenticated_accept_records_terminal_decision_and_audit() {
    let app = app();
    let proposal = proposal("Authentication notes");
    let publication = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &proposal))
        .await
        .unwrap_or_else(|error| panic!("publish: {error}"));
    assert_eq!(publication.status(), StatusCode::CREATED);

    let review = serde_json::json!({
        "reviewer": "alice",
        "rationale": "Evidence is sufficient.",
        "decidedAt": "2026-07-26T13:00:00Z",
        "reviewedContent": "# Reviewed durable knowledge"
    });
    let response = app
        .clone()
        .oneshot(review_request(
            "repo-a",
            &proposal.id,
            "accept",
            Some(TOKEN_A),
            review,
        ))
        .await
        .unwrap_or_else(|error| panic!("accept: {error}"));
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = json_body(response).await;
    assert_eq!(response["outcome"], "accepted");
    assert_eq!(response["reviewer"], "alice");
    assert_eq!(response["authenticatedActorId"], "alice");
    assert_eq!(response["unchanged"], false);

    let detail = app
        .oneshot(get_request(&format!(
            "/v1/repositories/repo-a/proposals/{}",
            proposal.id
        )))
        .await
        .unwrap_or_else(|error| panic!("detail: {error}"));
    let detail = json_body(detail).await;
    assert_eq!(detail["reviewDecision"]["outcome"], "accepted");
    assert_eq!(
        detail["reviewDecision"]["acceptedContent"],
        "# Reviewed durable knowledge"
    );
    assert_eq!(detail["reviewAudit"]["authenticatedActorId"], "alice");
}

#[tokio::test]
async fn publication_is_authenticated_idempotent_conflict_safe_and_repository_scoped() {
    let app = app();
    let original = proposal("Original title");

    let missing_auth = app
        .clone()
        .oneshot(publish_request("repo-a", None, &original))
        .await
        .unwrap_or_else(|error| panic!("missing auth: {error}"));
    assert_eq!(missing_auth.status(), StatusCode::UNAUTHORIZED);

    let wrong_repository = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_B), &original))
        .await
        .unwrap_or_else(|error| panic!("wrong repository: {error}"));
    assert_eq!(wrong_repository.status(), StatusCode::FORBIDDEN);

    let first = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &original))
        .await
        .unwrap_or_else(|error| panic!("first: {error}"));
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = json_body(first).await;

    let replay = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &original))
        .await
        .unwrap_or_else(|error| panic!("replay: {error}"));
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = json_body(replay).await;
    assert_eq!(replay["unchanged"], true);
    assert_eq!(replay["publishedAt"], first["publishedAt"]);
    assert_eq!(replay["revisionSha256"], first["revisionSha256"]);

    let mut changed_revision = original.clone();
    changed_revision.title = "Changed title".into();
    let conflict = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &changed_revision))
        .await
        .unwrap_or_else(|error| panic!("conflict: {error}"));
    assert_eq!(conflict.status(), StatusCode::CONFLICT);

    let isolated = app
        .oneshot(get_request("/v1/repositories/repo-b/proposals"))
        .await
        .unwrap_or_else(|error| panic!("isolated list: {error}"));
    assert_eq!(isolated.status(), StatusCode::OK);
    assert_eq!(json_body(isolated).await.as_array().map(Vec::len), Some(0));
}

#[tokio::test]
async fn malformed_incompatible_and_oversized_publications_are_rejected() {
    let app = app();
    let malformed = Request::builder()
        .method("POST")
        .uri("/v1/repositories/repo-a/proposals")
        .header("authorization", format!("Bearer {TOKEN_A}"))
        .body(Body::from("{"))
        .unwrap_or_else(|error| panic!("request: {error}"));
    let malformed = app
        .clone()
        .oneshot(malformed)
        .await
        .unwrap_or_else(|error| panic!("malformed: {error}"));
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);

    let mut incompatible = proposal("Incompatible");
    incompatible.schema_version = "99.0.0".into();
    let incompatible = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &incompatible))
        .await
        .unwrap_or_else(|error| panic!("incompatible: {error}"));
    assert_eq!(incompatible.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let oversized = Request::builder()
        .method("POST")
        .uri("/v1/repositories/repo-a/proposals")
        .header("authorization", format!("Bearer {TOKEN_A}"))
        .body(Body::from(vec![b' '; 1024 * 1024 + 1]))
        .unwrap_or_else(|error| panic!("request: {error}"));
    let oversized = app
        .oneshot(oversized)
        .await
        .unwrap_or_else(|error| panic!("oversized: {error}"));
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn review_retries_are_idempotent_and_conflicting_terminal_decisions_fail() {
    let app = app();
    let proposal = proposal("Review semantics");
    let publication = app
        .clone()
        .oneshot(publish_request("repo-a", Some(TOKEN_A), &proposal))
        .await
        .unwrap_or_else(|error| panic!("publish: {error}"));
    assert_eq!(publication.status(), StatusCode::CREATED);

    let review = serde_json::json!({
        "reviewer": "alice",
        "rationale": "approved",
        "decidedAt": "2026-07-26T13:00:00Z"
    });
    let first = app
        .clone()
        .oneshot(review_request(
            "repo-a",
            &proposal.id,
            "accept",
            Some(TOKEN_A),
            review.clone(),
        ))
        .await
        .unwrap_or_else(|error| panic!("first review: {error}"));
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = json_body(first).await;

    let replay = app
        .clone()
        .oneshot(review_request(
            "repo-a",
            &proposal.id,
            "accept",
            Some(TOKEN_A),
            review.clone(),
        ))
        .await
        .unwrap_or_else(|error| panic!("review replay: {error}"));
    assert_eq!(replay.status(), StatusCode::OK);
    let replay = json_body(replay).await;
    assert_eq!(replay["unchanged"], true);
    assert_eq!(replay["recordedAt"], first["recordedAt"]);
    assert_eq!(replay["decisionSha256"], first["decisionSha256"]);

    let opposite = app
        .clone()
        .oneshot(review_request(
            "repo-a",
            &proposal.id,
            "reject",
            Some(TOKEN_A),
            review,
        ))
        .await
        .unwrap_or_else(|error| panic!("opposite review: {error}"));
    assert_eq!(opposite.status(), StatusCode::CONFLICT);

    let impersonation = app
        .oneshot(review_request(
            "repo-b",
            &proposal.id,
            "accept",
            Some(TOKEN_B),
            serde_json::json!({
                "reviewer": "mallory",
                "rationale": "spoofed",
                "decidedAt": "2026-07-26T14:00:00Z"
            }),
        ))
        .await
        .unwrap_or_else(|error| panic!("impersonation: {error}"));
    assert_eq!(impersonation.status(), StatusCode::FORBIDDEN);
}
