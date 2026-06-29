//! Rspress publishing adapter — publishes accepted Markdown-backed knowledge
//! into `.devagent/docs/docs/accepted-knowledge/` with Rspress frontmatter
//! and deterministic `_nav.json` integration.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use scryrs_adapter_markdown::publish_accepted_markdown;
use scryrs_types::FeatureDescriptor;

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "adapter-rspress",
        title: "scryrs-adapter-rspress",
        summary: "Rspress publishing surface — accepted knowledge → live docs",
    }
}

/// Machine-readable metadata for each published Rspress page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishEntry {
    /// Relative path from docs root, e.g. "accepted-knowledge/docs_note/<id>.md"
    pub path: String,
    /// Target type slug, e.g. "docs_note", "adr"
    pub target_type: String,
    /// Proposal identifier
    pub proposal_id: String,
    /// Navigation display text
    pub nav_text: String,
    /// Navigation link, e.g. "/project-docs/accepted-knowledge/docs_note/<id>"
    pub nav_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishError {
    message: String,
}

impl PublishError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for PublishError {}

const ACCEPTED_KNOWLEDGE_DIR: &str = "accepted-knowledge";
const ACCEPTED_KNOWLEDGE_SECTION: &str = "Accepted Knowledge";

/// Publish accepted Markdown-backed proposals into the Rspress docs source tree.
///
/// # Flow
///
/// 1. Calls `publish_accepted_markdown(repo_root, temp_dir)` for plain Markdown output.
/// 2. Clears `docs_root/accepted-knowledge/`.
/// 3. Transforms each file (adding Rspress frontmatter) and writes into
///    `accepted-knowledge/<target-type>/<proposal-id>.md`.
/// 4. Updates `_nav.json` with a fresh "Accepted Knowledge" section via
///    strip-and-rebuild.
/// 5. Returns machine-readable `PublishEntry` values sorted by `proposal_id`.
pub fn publish_accepted_rspress(
    repository_root: impl AsRef<Path>,
    docs_root: impl AsRef<Path>,
) -> Result<Vec<PublishEntry>, PublishError> {
    let repository_root = repository_root.as_ref();
    let docs_root = docs_root.as_ref();

    // Stage 1: Call markdown adapter into a temp scratch directory.
    let temp_dir = tempfile::TempDir::new().map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-rspress: cannot create temp directory: {error}"
        ))
    })?;
    let temp_path = temp_dir.path();

    let markdown_paths =
        publish_accepted_markdown(repository_root, temp_path).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: markdown publish failed: {error}"
            ))
        })?;

    // Stage 2: Validate _nav.json is parseable before writing any pages.
    // This ensures malformed nav fails early with no partial output.
    validate_nav_json(docs_root)?;

    // Stage 3: Clear the accepted-knowledge subtree.
    clear_accepted_knowledge_dir(docs_root)?;

    // Stage 4: Transform and write Rspress pages.
    let mut publish_entries = read_and_transform_pages(temp_path, &markdown_paths, docs_root)?;

    // Sort entries by proposal_id for deterministic nav output.
    publish_entries.sort_by(|a, b| a.proposal_id.cmp(&b.proposal_id));

    // Stage 5: Update _nav.json.
    update_nav_json(docs_root, &publish_entries)?;

    Ok(publish_entries)
}

fn clear_accepted_knowledge_dir(docs_root: &Path) -> Result<(), PublishError> {
    let accepted_knowledge_dir = docs_root.join(ACCEPTED_KNOWLEDGE_DIR);
    if accepted_knowledge_dir.exists() {
        if !accepted_knowledge_dir.is_dir() {
            return Err(PublishError::new(format!(
                "scryrs-adapter-rspress: {} exists but is not a directory",
                accepted_knowledge_dir.display()
            )));
        }
        std::fs::remove_dir_all(&accepted_knowledge_dir).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot clear accepted-knowledge directory: {error}"
            ))
        })?;
    }
    Ok(())
}

fn read_and_transform_pages(
    temp_path: &Path,
    markdown_paths: &[PathBuf],
    docs_root: &Path,
) -> Result<Vec<PublishEntry>, PublishError> {
    let accepted_knowledge_dir = docs_root.join(ACCEPTED_KNOWLEDGE_DIR);
    let mut publish_entries = Vec::with_capacity(markdown_paths.len());

    for src_path in markdown_paths {
        // Path is <temp_dir>/<target_type>/<proposal_id>.md
        let relative = src_path.strip_prefix(temp_path).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot strip temp prefix from {}: {error}",
                src_path.display()
            ))
        })?;

        // Parse target_type from the parent directory name.
        let target_type = relative.parent().and_then(|p| p.to_str()).ok_or_else(|| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: unexpected path structure (no parent dir): {}",
                src_path.display()
            ))
        })?;
        // Guard against empty target type slug.
        if target_type.is_empty() {
            return Err(PublishError::new(format!(
                "scryrs-adapter-rspress: empty target type dir in path: {}",
                src_path.display()
            )));
        }

        // Parse proposal_id from the file stem (without .md extension).
        let proposal_id = relative
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                PublishError::new(format!(
                    "scryrs-adapter-rspress: unexpected file stem in path: {}",
                    src_path.display()
                ))
            })?;
        if proposal_id.is_empty() {
            return Err(PublishError::new(format!(
                "scryrs-adapter-rspress: empty proposal id stem in path: {}",
                src_path.display()
            )));
        }

        // Read the plain Markdown body written by the Markdown adapter.
        let body = std::fs::read_to_string(src_path).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot read markdown {}: {error}",
                src_path.display()
            ))
        })?;

        // Prepend Rspress frontmatter.
        let frontmatter = render_frontmatter(target_type, proposal_id);
        let rspress_page = format!("{frontmatter}\n{body}");

        // Write to docs_root/accepted-knowledge/<target_type>/<proposal_id>.md
        let dest_dir = accepted_knowledge_dir.join(target_type);
        std::fs::create_dir_all(&dest_dir).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot create directory {}: {error}",
                dest_dir.display()
            ))
        })?;
        let dest_path = dest_dir.join(format!("{proposal_id}.md"));
        std::fs::write(&dest_path, rspress_page).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot write {}: {error}",
                dest_path.display()
            ))
        })?;

        // Build PublishEntry.
        let nav_text = format!("{target_type}:{}", truncate_id(proposal_id));
        let nav_link =
            format!("/project-docs/{ACCEPTED_KNOWLEDGE_DIR}/{target_type}/{proposal_id}");
        let rel_path = format!("{ACCEPTED_KNOWLEDGE_DIR}/{target_type}/{proposal_id}.md");

        publish_entries.push(PublishEntry {
            path: rel_path,
            target_type: target_type.to_string(),
            proposal_id: proposal_id.to_string(),
            nav_text,
            nav_link,
        });
    }

    Ok(publish_entries)
}

fn render_frontmatter(target_type: &str, proposal_id: &str) -> String {
    let title = format!("{target_type} {proposal_id}");
    let truncated_id = truncate_id(proposal_id);
    let sidebar_label = format!("{target_type}:{truncated_id}");
    format!("---\ntitle: {title}\nsidebar_label: {sidebar_label}\n---")
}

fn truncate_id(proposal_id: &str) -> &str {
    if proposal_id.len() > 8 {
        &proposal_id[..8]
    } else {
        proposal_id
    }
}

/// Early validation: confirm `_nav.json` is parseable as a JSON array.
///
/// Missing `_nav.json` passes validation (it will be created on write).
/// Malformed or non-array `_nav.json` fails loudly. Called before any pages
/// are written so that nav errors produce no partial output.
fn validate_nav_json(docs_root: &Path) -> Result<(), PublishError> {
    let nav_path = docs_root.join("_nav.json");
    if !nav_path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(&nav_path).map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-rspress: cannot read _nav.json: {error}"
        ))
    })?;
    let nav: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-rspress: malformed _nav.json: {error}"
        ))
    })?;
    if !nav.is_array() {
        return Err(PublishError::new(
            "scryrs-adapter-rspress: _nav.json is not a JSON array",
        ));
    }
    Ok(())
}

/// Update `docs_root/_nav.json` with strip-and-rebuild semantics.
///
/// # Behavior
///
/// - Reads the existing `_nav.json` (if absent, starts with an empty array).
/// - Strips any section where `text == "Accepted Knowledge"`.
/// - If `published_entries` is non-empty, appends a fresh "Accepted Knowledge"
///   section with per-target-type sub-sections and items sorted by `proposal_id`.
/// - If `published_entries` is empty, the "Accepted Knowledge" section is
///   removed entirely.
/// - Reruns with identical input produce byte-identical `_nav.json`.
fn update_nav_json(
    docs_root: impl AsRef<Path>,
    published_entries: &[PublishEntry],
) -> Result<(), PublishError> {
    let docs_root = docs_root.as_ref();
    let nav_path = docs_root.join("_nav.json");

    let mut nav: serde_json::Value = if nav_path.exists() {
        let raw = std::fs::read_to_string(&nav_path).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: cannot read _nav.json: {error}"
            ))
        })?;
        serde_json::from_str(&raw).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-rspress: malformed _nav.json: {error}"
            ))
        })?
    } else {
        serde_json::Value::Array(Vec::new())
    };

    // Strip any existing "Accepted Knowledge" section.
    let mut sections: Vec<serde_json::Value> = match nav {
        serde_json::Value::Array(ref arr) => arr
            .iter()
            .filter(|item| {
                item.get("text")
                    .and_then(|t| t.as_str())
                    .is_none_or(|t| t != ACCEPTED_KNOWLEDGE_SECTION)
            })
            .cloned()
            .collect(),
        _ => {
            return Err(PublishError::new(
                "scryrs-adapter-rspress: _nav.json is not a JSON array",
            ));
        }
    };

    // Only append the section when there are published entries.
    if !published_entries.is_empty() {
        let accepted_section = build_nav_section(published_entries);
        sections.push(accepted_section);
    }

    nav = serde_json::Value::Array(sections);

    let json_output = serde_json::to_string_pretty(&nav).map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-rspress: cannot serialize _nav.json: {error}"
        ))
    })?;
    std::fs::write(&nav_path, format!("{json_output}\n")).map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-rspress: cannot write _nav.json: {error}"
        ))
    })?;

    Ok(())
}

/// Build a fresh "Accepted Knowledge" nav section from published entries.
///
/// Groups entries by `target_type` into per-type sub-sections. Display names
/// are derived from the target type slug:
///   `adr` → "ADRs"
///   `docs_note` → "Docs Notes"
///   `skill` → "Skills"
///   `debugging_playbook` → "Debugging Playbooks"
/// Unrecognized types use the raw slug as the display name.
fn build_nav_section(entries: &[PublishEntry]) -> serde_json::Value {
    // Group entries by target_type (use BTreeMap for deterministic ordering).
    let mut type_map: BTreeMap<&str, Vec<&PublishEntry>> = BTreeMap::new();
    for entry in entries {
        type_map.entry(&entry.target_type).or_default().push(entry);
    }

    let mut sub_sections: Vec<serde_json::Value> = Vec::new();
    for (target_type, type_entries) in &type_map {
        // Entries are already sorted by proposal_id from the caller.
        let items: Vec<serde_json::Value> = type_entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "text": entry.nav_text,
                    "link": entry.nav_link,
                })
            })
            .collect();

        let display_name = match *target_type {
            "docs_note" => "Docs Notes",
            "adr" => "ADRs",
            "skill" => "Skills",
            "debugging_playbook" => "Debugging Playbooks",
            other => other,
        };

        sub_sections.push(serde_json::json!({
            "text": display_name,
            "items": items,
        }));
    }

    serde_json::json!({
        "text": ACCEPTED_KNOWLEDGE_SECTION,
        "items": sub_sections,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use scryrs_types::{
        EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
        ProposalReviewDecision, ProposalTargetType, ProposedContent,
        REVIEW_DECISION_SCHEMA_VERSION, ReviewOutcome, SemanticGraphGrouping,
    };
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    // --- helpers ---

    fn make_evidence(subject: &str) -> Vec<EvidenceLink> {
        vec![EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: subject.to_string(),
            row_ids: vec![1],
            doc_ref: Some(".devagent/docs/docs/proposals.md".into()),
            description: Some("evidence description".into()),
            score: Some(10),
            metadata: None,
        }]
    }

    fn make_markdown_proposal(
        target_type: ProposalTargetType,
        title: &str,
        body: &str,
    ) -> ProposalDocument {
        let proposed_content = ProposedContent::Markdown(body.to_string());
        let id = ProposalDocument::compute_id(&target_type, &proposed_content).expect("compute id");
        ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type,
            title: title.into(),
            rationale: format!("justify {title}"),
            proposed_content,
            evidence: make_evidence(title),
            created_at: "2026-06-29T17:00:00Z".into(),
        }
    }

    fn make_accepted_decision(
        proposal: &ProposalDocument,
        reviewer: &str,
        rationale: &str,
    ) -> ProposalReviewDecision {
        ProposalReviewDecision {
            schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: proposal.id.clone(),
            reviewer: reviewer.into(),
            decided_at: "2026-06-29T18:00:00Z".into(),
            rationale: rationale.into(),
            source_evidence: proposal.evidence.clone(),
            outcome: ReviewOutcome::Accepted,
            target_type: Some(proposal.target_type.clone()),
            accepted_content: Some(proposal.proposed_content.clone()),
        }
    }

    fn make_memory_patch_decision(proposal_id: &str) -> ProposalReviewDecision {
        ProposalReviewDecision {
            schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: proposal_id.into(),
            reviewer: "alice".into(),
            decided_at: "2026-06-29T18:00:00Z".into(),
            rationale: "structured memory".into(),
            source_evidence: make_evidence("memory"),
            outcome: ReviewOutcome::Accepted,
            target_type: Some(ProposalTargetType::MemoryPatch),
            accepted_content: Some(ProposedContent::MemoryPatch(serde_json::json!(
                {"patch": "value"}
            ))),
        }
    }

    fn make_grouping_decision(proposal_id: &str) -> ProposalReviewDecision {
        ProposalReviewDecision {
            schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: proposal_id.into(),
            reviewer: "alice".into(),
            decided_at: "2026-06-29T18:00:00Z".into(),
            rationale: "graph grouping".into(),
            source_evidence: make_evidence("graph"),
            outcome: ReviewOutcome::Accepted,
            target_type: Some(ProposalTargetType::SemanticGraphGrouping),
            accepted_content: Some(ProposedContent::SemanticGraphGrouping(
                SemanticGraphGrouping {
                    source_node_ids: vec!["node-a".into()],
                    target_group_node_id: "group-1".into(),
                    target_group_label: "Group 1".into(),
                },
            )),
        }
    }

    fn write_pending_proposal(repo_root: &Path, proposal: &ProposalDocument) {
        let proposals_dir = repo_root.join(".scryrs/proposals");
        fs::create_dir_all(&proposals_dir).expect("create proposals dir");
        fs::write(
            proposals_dir.join(proposal.inbox_filename()),
            serde_json::to_string(proposal).expect("serialize"),
        )
        .expect("write proposal");
    }

    fn write_accepted_decision(repo_root: &Path, decision: &ProposalReviewDecision) {
        let accepted_dir = repo_root.join(".scryrs/accepted");
        fs::create_dir_all(&accepted_dir).expect("create accepted dir");
        fs::write(
            accepted_dir.join(format!("{}.json", decision.proposal_id)),
            serde_json::to_string(decision).expect("serialize"),
        )
        .expect("write decision");
    }

    fn snapshot_dir(root: &Path) -> HashMap<PathBuf, Vec<u8>> {
        let mut snapshot = HashMap::new();
        if !root.exists() {
            return snapshot;
        }
        walk_snapshot(root, root, &mut snapshot);
        snapshot
    }

    fn walk_snapshot(root: &Path, dir: &Path, snapshot: &mut HashMap<PathBuf, Vec<u8>>) {
        let mut entries: Vec<PathBuf> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| entry.expect("entry").path())
            .collect();
        entries.sort();
        for entry in entries {
            if entry.is_dir() {
                walk_snapshot(root, &entry, snapshot);
                continue;
            }
            let relative = entry
                .strip_prefix(root)
                .expect("strip prefix")
                .to_path_buf();
            snapshot.insert(relative, fs::read(&entry).expect("read file"));
        }
    }

    // --- tests ---

    #[test]
    fn descriptor_marks_rspress_adapter() {
        assert_eq!(descriptor().id, "adapter-rspress");
        assert!(!descriptor().title.is_empty());
        assert!(!descriptor().summary.is_empty());
    }

    #[test]
    fn publish_entry_stores_all_fields() {
        let entry = PublishEntry {
            path: "accepted-knowledge/docs_note/abc.123.md".to_string(),
            target_type: "docs_note".to_string(),
            proposal_id: "abc.123".to_string(),
            nav_text: "docs_note:abc.123".to_string(),
            nav_link: "/project-docs/accepted-knowledge/docs_note/abc.123".to_string(),
        };
        assert_eq!(entry.path, "accepted-knowledge/docs_note/abc.123.md");
        assert_eq!(entry.target_type, "docs_note");
        assert_eq!(entry.proposal_id, "abc.123");
        assert_eq!(entry.nav_text, "docs_note:abc.123");
        assert_eq!(
            entry.nav_link,
            "/project-docs/accepted-knowledge/docs_note/abc.123"
        );
    }

    // --- 3.1: Only publishes from accepted artifacts ---

    #[test]
    fn pending_proposals_alone_produce_no_rspress_pages() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");
        let pending =
            make_markdown_proposal(ProposalTargetType::DocsNote, "pending", "## Pending\n");
        write_pending_proposal(repo.path(), &pending);

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        assert!(entries.is_empty());
        let ak_dir = docs.path().join(ACCEPTED_KNOWLEDGE_DIR);
        assert!(!ak_dir.exists());

        // _nav.json should either not exist or have no Accepted Knowledge section.
        let nav_path = docs.path().join("_nav.json");
        if nav_path.exists() {
            let nav: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&nav_path).unwrap()).unwrap();
            let sections = nav.as_array().unwrap();
            assert!(
                !sections
                    .iter()
                    .any(|s| s["text"].as_str() == Some(ACCEPTED_KNOWLEDGE_SECTION))
            );
        }
    }

    #[test]
    fn mixed_pending_and_accepted_only_publishes_accepted() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");
        let pending =
            make_markdown_proposal(ProposalTargetType::DocsNote, "pending", "## Pending\n");
        let accepted = make_markdown_proposal(ProposalTargetType::Adr, "accepted-adr", "## ADR\n");
        write_pending_proposal(repo.path(), &pending);
        write_pending_proposal(repo.path(), &accepted);
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&accepted, "alice", "accepted"),
        );

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].proposal_id, accepted.id);
        assert_eq!(entries[0].target_type, "adr");

        // pending proposal id should not appear in nav.
        let nav: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(docs.path().join("_nav.json")).unwrap())
                .unwrap();
        let nav_str = serde_json::to_string(&nav).unwrap();
        assert!(!nav_str.contains(&pending.id));
        assert!(nav_str.contains(&accepted.id));
    }

    // --- 3.2: Deterministic reruns ---

    #[test]
    fn repeated_publish_runs_are_byte_stable() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "stable"),
        );

        publish_accepted_rspress(repo.path(), docs.path()).expect("first publish");
        let first_snapshot = snapshot_dir(docs.path());
        publish_accepted_rspress(repo.path(), docs.path()).expect("second publish");
        let second_snapshot = snapshot_dir(docs.path());

        assert_eq!(first_snapshot, second_snapshot);
        // Verify the page and _nav.json exist.
        assert!(first_snapshot.contains_key(&Path::new("_nav.json").to_path_buf()));
    }

    // --- 3.3: Subtree cleared and regenerated ---

    #[test]
    fn stale_pages_are_removed_on_regeneration() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");

        // First publish with proposal A.
        let proposal_a = make_markdown_proposal(ProposalTargetType::DocsNote, "a", "## A\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal_a, "alice", "publish a"),
        );
        publish_accepted_rspress(repo.path(), docs.path()).expect("first publish");

        let page_a = docs
            .path()
            .join("accepted-knowledge/docs_note")
            .join(format!("{}.md", proposal_a.id));
        assert!(page_a.is_file());

        // Remove A's accepted decision, add B.
        let accepted_dir = repo.path().join(".scryrs/accepted");
        fs::remove_file(accepted_dir.join(format!("{}.json", proposal_a.id)))
            .expect("remove decision A");
        let proposal_b = make_markdown_proposal(ProposalTargetType::DocsNote, "b", "## B\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal_b, "alice", "publish b"),
        );

        publish_accepted_rspress(repo.path(), docs.path()).expect("second publish");

        // Stale page A is gone; page B exists.
        assert!(!page_a.exists());
        let page_b = docs
            .path()
            .join("accepted-knowledge/docs_note")
            .join(format!("{}.md", proposal_b.id));
        assert!(page_b.is_file());
    }

    // --- 3.4: Non-Markdown accepted decisions ---

    #[test]
    fn non_markdown_decisions_produce_no_rspress_output() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");
        let markdown_proposal =
            make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&markdown_proposal, "alice", "publish"),
        );
        write_accepted_decision(repo.path(), &make_memory_patch_decision("memory-proposal"));
        write_accepted_decision(repo.path(), &make_grouping_decision("grouping-proposal"));

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].proposal_id, markdown_proposal.id);

        // No pages for non-Markdown types.
        let ak = docs.path().join(ACCEPTED_KNOWLEDGE_DIR);
        assert!(!ak.join("memory_patch").exists());
        assert!(!ak.join("semantic_graph_grouping").exists());

        // Nav does not contain non-Markdown ids.
        let nav: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(docs.path().join("_nav.json")).unwrap())
                .unwrap();
        let nav_str = serde_json::to_string(&nav).unwrap();
        assert!(!nav_str.contains("memory-proposal"));
        assert!(!nav_str.contains("grouping-proposal"));
    }

    // --- 3.5: Nav merge ---

    #[test]
    fn nav_hand_authored_sections_preserved_accepted_knowledge_appended() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");

        // Write hand-authored _nav.json.
        let original_nav = serde_json::json!([
            { "text": "Hand Section", "items": [{ "text": "Page 1", "link": "/page1" }] }
        ]);
        fs::write(
            docs.path().join("_nav.json"),
            serde_json::to_string_pretty(&original_nav).unwrap() + "\n",
        )
        .expect("write nav");

        // Publish accepted artifacts.
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );

        publish_accepted_rspress(repo.path(), docs.path()).expect("publish");

        let nav: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(docs.path().join("_nav.json")).unwrap())
                .unwrap();
        let sections = nav.as_array().unwrap();

        // First section is the hand-authored one.
        assert_eq!(sections[0]["text"].as_str(), Some("Hand Section"));
        assert_eq!(sections[0]["items"][0]["text"].as_str(), Some("Page 1"));

        // Last section is Accepted Knowledge.
        let ak_section = &sections[1];
        assert_eq!(
            ak_section["text"].as_str(),
            Some(ACCEPTED_KNOWLEDGE_SECTION)
        );
        // Accepted Knowledge contains a sub-section for docs_note.
        assert_eq!(ak_section["items"][0]["text"].as_str(), Some("Docs Notes"));
    }

    #[test]
    fn nav_accepted_knowledge_section_replaced_on_rerun() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");

        let nav_content = serde_json::json!([
            { "text": "Hand Section", "items": [{ "text": "Page 1", "link": "/page1" }] }
        ]);
        fs::write(
            docs.path().join("_nav.json"),
            serde_json::to_string_pretty(&nav_content).unwrap() + "\n",
        )
        .expect("write nav");

        // First publish: proposal A.
        let proposal_a = make_markdown_proposal(ProposalTargetType::DocsNote, "a", "## A\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal_a, "alice", "publish a"),
        );
        publish_accepted_rspress(repo.path(), docs.path()).expect("first publish");

        let nav1 = fs::read_to_string(docs.path().join("_nav.json")).expect("read nav1");
        assert!(nav1.contains(&proposal_a.id));

        // Remove A, add B, republish.
        fs::remove_file(
            repo.path()
                .join(".scryrs/accepted")
                .join(format!("{}.json", proposal_a.id)),
        )
        .expect("remove A");
        let proposal_b = make_markdown_proposal(ProposalTargetType::DocsNote, "b", "## B\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal_b, "alice", "publish b"),
        );
        publish_accepted_rspress(repo.path(), docs.path()).expect("second publish");

        let nav2 = fs::read_to_string(docs.path().join("_nav.json")).expect("read nav2");
        // A is gone, B is present.
        assert!(!nav2.contains(&proposal_a.id));
        assert!(nav2.contains(&proposal_b.id));
        // Hand-authored section still present.
        assert!(nav2.contains("Hand Section"));
        assert!(nav2.contains("Page 1"));
    }

    #[test]
    fn nav_empty_published_set_removes_accepted_knowledge_section() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");

        // First publish adds Accepted Knowledge.
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );
        publish_accepted_rspress(repo.path(), docs.path()).expect("first publish");

        let nav1 = fs::read_to_string(docs.path().join("_nav.json")).expect("read nav1");
        assert!(nav1.contains(ACCEPTED_KNOWLEDGE_SECTION));

        // Remove accepted decision, republish.
        fs::remove_file(
            repo.path()
                .join(".scryrs/accepted")
                .join(format!("{}.json", proposal.id)),
        )
        .expect("remove decision");
        publish_accepted_rspress(repo.path(), docs.path()).expect("second publish");

        let nav2 = fs::read_to_string(docs.path().join("_nav.json")).expect("read nav2");
        assert!(!nav2.contains(ACCEPTED_KNOWLEDGE_SECTION));
        // File should be `[]\n` (empty array).
        let trimmed = nav2.trim();
        assert_eq!(trimmed, "[]");
    }

    #[test]
    fn nav_missing_file_created_with_accepted_knowledge_only() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );

        // No _nav.json exists.
        assert!(!docs.path().join("_nav.json").exists());

        publish_accepted_rspress(repo.path(), docs.path()).expect("publish");

        let nav_path = docs.path().join("_nav.json");
        assert!(nav_path.exists());
        let nav = fs::read_to_string(&nav_path).expect("read nav");
        assert!(nav.contains(ACCEPTED_KNOWLEDGE_SECTION));
        assert!(nav.contains(&proposal.id));
    }

    // --- 3.6: Malformed _nav.json fails loudly ---

    #[test]
    fn malformed_nav_json_fails_loudly() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );

        fs::write(docs.path().join("_nav.json"), "this is not json").expect("write bad json");

        let result = publish_accepted_rspress(repo.path(), docs.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("malformed _nav.json"));

        // No pages written despite accepted artifact existing.
        assert!(!docs.path().join(ACCEPTED_KNOWLEDGE_DIR).exists());
    }

    #[test]
    fn nav_not_array_fails_loudly() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );

        fs::write(docs.path().join("_nav.json"), r#"{"text": "not an array"}"#)
            .expect("write object json");

        let result = publish_accepted_rspress(repo.path(), docs.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a JSON array"));
    }

    // --- 3.7: PublishEntry return values match written files and nav ---

    #[test]
    fn publish_entries_match_written_files_and_nav() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");
        let docs_proposal =
            make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        let adr_proposal = make_markdown_proposal(ProposalTargetType::Adr, "adr", "## ADR\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&docs_proposal, "alice", "publish docs"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&adr_proposal, "alice", "publish adr"),
        );

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        // Two entries, sorted by proposal_id.
        // Compute sorted order.
        let mut ids = [docs_proposal.id.clone(), adr_proposal.id.clone()];
        ids.sort();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].proposal_id, ids[0]);
        assert_eq!(entries[1].proposal_id, ids[1]);

        for entry in &entries {
            // Entry path should be a real file.
            let file_path = docs.path().join(&entry.path);
            assert!(file_path.is_file(), "missing file: {file_path:?}");

            // File should contain the frontmatter.
            let content = fs::read_to_string(&file_path).expect("read page");
            assert!(content.starts_with("---\n"));
            assert!(content.contains(&format!(
                "title: {} {}",
                entry.target_type, entry.proposal_id
            )));

            // Nav should contain the entry's link and nav_text.
            let nav = fs::read_to_string(docs.path().join("_nav.json")).expect("read nav");
            assert!(
                nav.contains(&entry.nav_text),
                "nav missing nav_text: {}",
                entry.nav_text
            );
            assert!(
                nav.contains(&entry.nav_link),
                "nav missing nav_link: {}",
                entry.nav_link
            );
        }
    }

    // --- Frontmatter ---

    #[test]
    fn rspress_frontmatter_includes_title_and_sidebar_label() {
        let frontmatter = render_frontmatter(
            "docs_note",
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
        );
        assert!(frontmatter.starts_with("---\n"));
        assert!(frontmatter.contains(
            "title: docs_note abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        ));
        assert!(frontmatter.contains("sidebar_label: docs_note:abcdef01"));
        assert!(frontmatter.ends_with("---"));
    }

    #[test]
    fn short_proposal_id_not_truncated() {
        let frontmatter = render_frontmatter("adr", "short");
        assert!(frontmatter.contains("sidebar_label: adr:short"));
        assert!(frontmatter.contains("title: adr short"));
    }

    // --- Edge cases ---

    #[test]
    fn missing_accepted_directory_is_no_op() {
        let repo = TempDir::new().expect("tempdir");
        let docs = TempDir::new().expect("tempdir docs");

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        assert!(entries.is_empty());
        assert!(!docs.path().join(ACCEPTED_KNOWLEDGE_DIR).exists());
    }

    #[test]
    fn non_directory_accepted_knowledge_path_fails() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish"),
        );

        // Create accepted-knowledge as a file, not a directory.
        fs::write(docs.path().join(ACCEPTED_KNOWLEDGE_DIR), "not a dir").expect("write file");

        let result = publish_accepted_rspress(repo.path(), docs.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[test]
    fn update_nav_json_is_idempotent_on_rerun() {
        let docs = TempDir::new().expect("tempdir docs");

        let entries = vec![PublishEntry {
            path: "accepted-knowledge/docs_note/abc.md".to_string(),
            target_type: "docs_note".to_string(),
            proposal_id: "abc".to_string(),
            nav_text: "docs_note:abc".to_string(),
            nav_link: "/project-docs/accepted-knowledge/docs_note/abc".to_string(),
        }];

        // First call: creates _nav.json.
        update_nav_json(docs.path(), &entries).expect("first update");
        let first = fs::read_to_string(docs.path().join("_nav.json")).expect("read first");

        // Second call: same input, byte-identical output.
        update_nav_json(docs.path(), &entries).expect("second update");
        let second = fs::read_to_string(docs.path().join("_nav.json")).expect("read second");

        assert_eq!(first, second);
    }

    #[test]
    fn multiple_target_types_grouped_in_nav() {
        let docs = TempDir::new().expect("tempdir docs");
        let repo = TempDir::new().expect("tempdir");
        let docs_proposal =
            make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        let adr_proposal = make_markdown_proposal(ProposalTargetType::Adr, "adr", "## ADR\n");
        let skill_proposal =
            make_markdown_proposal(ProposalTargetType::Skill, "skill", "## Skill\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&docs_proposal, "alice", "publish"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&adr_proposal, "alice", "publish"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&skill_proposal, "alice", "publish"),
        );

        let entries = publish_accepted_rspress(repo.path(), docs.path()).expect("publish succeeds");

        // 3 entries across 3 target types.
        assert_eq!(entries.len(), 3);

        let nav: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(docs.path().join("_nav.json")).unwrap())
                .unwrap();
        let ak_section = &nav.as_array().unwrap()[0];
        let sub_sections = ak_section["items"].as_array().unwrap();

        // 3 sub-sections (one per target type).
        assert_eq!(sub_sections.len(), 3);

        let sub_texts: Vec<&str> = sub_sections
            .iter()
            .map(|s| s["text"].as_str().unwrap())
            .collect();
        // BTreeMap orders: ADRs, Debugging Playbooks, Docs Notes, Skills
        assert_eq!(sub_texts, vec!["ADRs", "Docs Notes", "Skills"]);
    }
}
