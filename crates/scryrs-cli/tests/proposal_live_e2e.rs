use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
    ProposalTargetType, ProposedContent,
};

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or_else(|error| panic!("allocate port: {error}"))
}

fn write_proposal(root: &std::path::Path) -> ProposalDocument {
    let content = ProposedContent::Markdown("# Live knowledge\n".into());
    let proposal = ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id: ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|error| panic!("proposal id: {error}")),
        target_type: ProposalTargetType::DocsNote,
        title: "Live knowledge".into(),
        rationale: "Repeated evidence should become durable knowledge.".into(),
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: "src/auth.rs".into(),
            row_ids: vec![1],
            doc_ref: None,
            description: Some("authentication evidence".into()),
            score: Some(12),
            metadata: None,
        }],
        created_at: "2026-07-26T12:00:00Z".into(),
    };
    let proposal_dir = root.join(".scryrs/proposals");
    std::fs::create_dir_all(&proposal_dir)
        .unwrap_or_else(|error| panic!("create proposal directory: {error}"));
    std::fs::write(
        proposal_dir.join(proposal.inbox_filename()),
        serde_json::to_vec(&proposal).unwrap_or_else(|error| panic!("proposal JSON: {error}")),
    )
    .unwrap_or_else(|error| panic!("write proposal: {error}"));
    proposal
}

fn wait_for_json(url: &str, process: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = process
            .try_wait()
            .unwrap_or_else(|error| panic!("process status: {error}"))
        {
            panic!("process exited before readiness: {status}");
        }
        if ureq::get(url).call().is_ok() {
            return;
        }
        assert!(Instant::now() < deadline, "readiness timed out for {url}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn get_json(url: &str) -> serde_json::Value {
    let body = ureq::get(url)
        .call()
        .unwrap_or_else(|error| panic!("GET {url}: {error}"))
        .into_string()
        .unwrap_or_else(|error| panic!("read {url}: {error}"));
    serde_json::from_str(&body).unwrap_or_else(|error| panic!("JSON {url}: {error}"))
}

#[allow(clippy::disallowed_methods)]
#[test]
fn publishes_views_and_reviews_proposal_through_live_dashboard() {
    const TOKEN: &str = "proposal-e2e-secret";
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let proposal = write_proposal(temp.path());
    let proposal_path = temp
        .path()
        .join(".scryrs/proposals")
        .join(proposal.inbox_filename());
    let original_bytes = std::fs::read(&proposal_path)
        .unwrap_or_else(|error| panic!("read original proposal: {error}"));

    let server_port = free_port();
    let server_url = format!("http://127.0.0.1:{server_port}");
    let credentials = serde_json::json!([{
        "repositoryId": "repo-e2e",
        "actorId": "alice",
        "token": TOKEN
    }])
    .to_string();
    let server_child = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .args([
            "server",
            "--bind",
            "127.0.0.1",
            "--port",
            &server_port.to_string(),
            "--store",
            temp.path()
                .join("server.db")
                .to_str()
                .unwrap_or_else(|| panic!("store path must be UTF-8")),
        ])
        .env("SCRYRS_PROPOSAL_WRITE_CREDENTIALS", credentials)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn server: {error}"));
    let mut server = Process(server_child);
    wait_for_json(
        &format!("{server_url}/v1/repositories/repo-e2e/proposals"),
        &mut server.0,
    );

    let publication = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .args([
            "proposals",
            "publish",
            temp.path()
                .to_str()
                .unwrap_or_else(|| panic!("repository path must be UTF-8")),
            &proposal.id,
            "--server-url",
            &server_url,
            "--repository-id",
            "repo-e2e",
        ])
        .env("SCRYRS_PROPOSAL_WRITE_TOKEN", TOKEN)
        .output()
        .unwrap_or_else(|error| panic!("publish command: {error}"));
    assert!(
        publication.status.success(),
        "publication failed: {}",
        String::from_utf8_lossy(&publication.stderr)
    );
    let publication_json: serde_json::Value = serde_json::from_slice(&publication.stdout)
        .unwrap_or_else(|error| panic!("publication JSON: {error}"));
    assert_eq!(publication_json["proposalId"], proposal.id);
    assert_eq!(publication_json["publisherId"], "alice");

    let dashboard_port = free_port();
    let dashboard_url = format!("http://127.0.0.1:{dashboard_port}");
    let dashboard_child = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .current_dir(temp.path())
        .args([
            "dashboard",
            "--mode",
            "live",
            "--bind",
            "127.0.0.1",
            "--port",
            &dashboard_port.to_string(),
            "--server-url",
            &server_url,
            "--repository-id",
            "repo-e2e",
            "--no-open",
        ])
        .env("SCRYRS_PROPOSAL_WRITE_TOKEN", TOKEN)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn dashboard: {error}"));
    let mut dashboard = Process(dashboard_child);
    wait_for_json(&format!("{dashboard_url}/api/meta"), &mut dashboard.0);

    let inventory = get_json(&format!("{dashboard_url}/api/proposals"));
    assert_eq!(inventory[0]["proposalId"], proposal.id);
    assert_eq!(inventory[0]["state"], "pending");
    let detail_url = format!("{dashboard_url}/api/proposals/{}", proposal.id);
    assert_eq!(get_json(&detail_url)["title"], "Live knowledge");

    let review = ureq::post(&format!("{detail_url}/accept"))
        .set("content-type", "application/json")
        .send_json(serde_json::json!({
            "reviewer": "alice",
            "rationale": "approved through dashboard",
            "decidedAt": "2026-07-26T13:00:00Z"
        }))
        .unwrap_or_else(|error| panic!("review proposal: {error}"));
    assert_eq!(review.status(), 201);
    let reviewed = get_json(&detail_url);
    assert_eq!(reviewed["reviewDecision"]["outcome"], "accepted");
    assert_eq!(reviewed["reviewAudit"]["authenticatedActorId"], "alice");

    assert_eq!(
        std::fs::read(&proposal_path)
            .unwrap_or_else(|error| panic!("read proposal after workflow: {error}")),
        original_bytes
    );
    assert!(!temp.path().join(".scryrs/accepted").exists());
}
