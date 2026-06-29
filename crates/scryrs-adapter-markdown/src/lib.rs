//! Generic Markdown publishing foundation.

use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Write as _};
use std::path::{Path, PathBuf};

use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, FeatureDescriptor, ProposalReviewDecision,
    ProposalTargetType, ProposedContent, ReviewOutcome,
};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "adapter-markdown",
        title: "scryrs-adapter-markdown",
        summary: "generic Markdown publishing surface foundation",
    }
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

pub fn publish_accepted_markdown(
    repository_root: impl AsRef<Path>,
    output_root: impl AsRef<Path>,
) -> Result<Vec<PathBuf>, PublishError> {
    let repository_root = repository_root.as_ref();
    let output_root = output_root.as_ref();
    let accepted_dir = repository_root.join(".scryrs/accepted");

    if !accepted_dir.exists() {
        return Ok(Vec::new());
    }
    if !accepted_dir.is_dir() {
        return Err(PublishError::new(format!(
            "scryrs-adapter-markdown: expected directory {}",
            accepted_dir.display()
        )));
    }

    let decisions = load_accepted_decisions(&accepted_dir)?;
    let publishable = decisions
        .iter()
        .filter_map(PublishableDecision::from_decision)
        .collect::<Vec<_>>();

    let mut written_paths = Vec::with_capacity(publishable.len());
    for decision in publishable {
        let target_dir = output_root.join(target_type_slug(decision.target_type));
        std::fs::create_dir_all(&target_dir).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-markdown: cannot create output directory {}: {error}",
                target_dir.display()
            ))
        })?;

        let output_path = target_dir.join(format!("{}.md", decision.proposal_id));
        let markdown = render_reviewed_markdown(decision);
        std::fs::write(&output_path, markdown).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-markdown: cannot write markdown file {}: {error}",
                output_path.display()
            ))
        })?;
        written_paths.push(output_path);
    }

    Ok(written_paths)
}

fn load_accepted_decisions(
    accepted_dir: &Path,
) -> Result<Vec<ProposalReviewDecision>, PublishError> {
    let mut decisions = Vec::new();

    for path in json_files_in_dir(accepted_dir)? {
        let json = std::fs::read_to_string(&path).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-markdown: cannot read accepted artifact {}: {error}",
                path.display()
            ))
        })?;
        let decision: ProposalReviewDecision = serde_json::from_str(&json).map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-markdown: invalid accepted artifact {}: {error}",
                path.display()
            ))
        })?;
        validate_accepted_decision(&path, &decision)?;
        decisions.push(decision);
    }

    decisions.sort_by(|left, right| left.proposal_id.cmp(&right.proposal_id));
    Ok(decisions)
}

fn validate_accepted_decision(
    path: &Path,
    decision: &ProposalReviewDecision,
) -> Result<(), PublishError> {
    decision.validate().map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-markdown: invalid accepted artifact {}: {error}",
            path.display()
        ))
    })?;

    let expected_filename = format!("{}.json", decision.proposal_id);
    let actual_filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if actual_filename != expected_filename {
        return Err(PublishError::new(format!(
            "scryrs-adapter-markdown: invalid accepted artifact {}: filename does not match proposalId '{}'",
            path.display(),
            decision.proposal_id
        )));
    }

    if decision.outcome != ReviewOutcome::Accepted {
        return Err(PublishError::new(format!(
            "scryrs-adapter-markdown: invalid accepted artifact {}: outcome does not match accepted directory",
            path.display()
        )));
    }

    Ok(())
}

fn json_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>, PublishError> {
    let entries = std::fs::read_dir(dir).map_err(|error| {
        PublishError::new(format!(
            "scryrs-adapter-markdown: cannot read directory {}: {error}",
            dir.display()
        ))
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            PublishError::new(format!(
                "scryrs-adapter-markdown: cannot read directory {}: {error}",
                dir.display()
            ))
        })?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Debug, Clone, Copy)]
struct PublishableDecision<'a> {
    proposal_id: &'a str,
    target_type: &'a ProposalTargetType,
    body: &'a str,
    reviewer: &'a str,
    decided_at: &'a str,
    rationale: &'a str,
    source_evidence: &'a [EvidenceLink],
}

impl<'a> PublishableDecision<'a> {
    fn from_decision(decision: &'a ProposalReviewDecision) -> Option<Self> {
        let target_type = decision.target_type.as_ref()?;
        let accepted_content = decision.accepted_content.as_ref()?;
        match (target_type, accepted_content) {
            (
                ProposalTargetType::DocsNote
                | ProposalTargetType::Adr
                | ProposalTargetType::Skill
                | ProposalTargetType::DebuggingPlaybook,
                ProposedContent::Markdown(body),
            ) => Some(Self {
                proposal_id: decision.proposal_id.as_str(),
                target_type,
                body: body.as_str(),
                reviewer: decision.reviewer.as_str(),
                decided_at: decision.decided_at.as_str(),
                rationale: decision.rationale.as_str(),
                source_evidence: decision.source_evidence.as_slice(),
            }),
            (ProposalTargetType::MemoryPatch, ProposedContent::MemoryPatch(_))
            | (
                ProposalTargetType::SemanticGraphGrouping,
                ProposedContent::SemanticGraphGrouping(_),
            ) => None,
            _ => None,
        }
    }
}

fn render_reviewed_markdown(decision: PublishableDecision<'_>) -> String {
    let mut markdown = String::new();
    let target_type = target_type_slug(decision.target_type);

    writeln!(
        &mut markdown,
        "# {} {}\n\n## Review Metadata\n",
        target_type, decision.proposal_id
    )
    .unwrap_or_else(|error| panic!("render heading: {error}"));
    writeln!(&mut markdown, "- `proposalId`: `{}`", decision.proposal_id)
        .unwrap_or_else(|error| panic!("render proposalId: {error}"));
    writeln!(&mut markdown, "- `targetType`: `{}`", target_type)
        .unwrap_or_else(|error| panic!("render targetType: {error}"));
    writeln!(&mut markdown, "- `reviewer`: `{}`", decision.reviewer)
        .unwrap_or_else(|error| panic!("render reviewer: {error}"));
    writeln!(&mut markdown, "- `decidedAt`: `{}`", decision.decided_at)
        .unwrap_or_else(|error| panic!("render decidedAt: {error}"));
    markdown.push_str("- `rationale`:\n");
    push_indented_block(&mut markdown, decision.rationale, 2);
    markdown.push('\n');
    markdown.push('\n');
    markdown.push_str(decision.body);
    if !decision.body.ends_with('\n') {
        markdown.push('\n');
    }
    markdown.push_str("\n## Evidence backlinks\n\n");

    for (index, evidence) in decision.source_evidence.iter().enumerate() {
        render_evidence_entry(&mut markdown, index + 1, evidence);
    }

    markdown
}

fn render_evidence_entry(markdown: &mut String, index: usize, evidence: &EvidenceLink) {
    writeln!(markdown, "### Evidence {index}\n")
        .unwrap_or_else(|error| panic!("render evidence heading: {error}"));
    writeln!(
        markdown,
        "- `sourceKind`: `{}`",
        evidence_source_kind_slug(&evidence.source_kind)
    )
    .unwrap_or_else(|error| panic!("render sourceKind: {error}"));
    writeln!(markdown, "- `subject`: `{}`", evidence.subject)
        .unwrap_or_else(|error| panic!("render subject: {error}"));
    if !evidence.row_ids.is_empty() {
        let row_ids = evidence
            .row_ids
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(markdown, "- `rowIds`: {row_ids}")
            .unwrap_or_else(|error| panic!("render rowIds: {error}"));
    }
    if let Some(doc_ref) = &evidence.doc_ref {
        writeln!(markdown, "- `docRef`: `{doc_ref}`")
            .unwrap_or_else(|error| panic!("render docRef: {error}"));
    }
    if let Some(description) = &evidence.description {
        markdown.push_str("- `description`:\n");
        push_indented_block(markdown, description, 2);
        markdown.push('\n');
    }
    if let Some(score) = evidence.score {
        writeln!(markdown, "- `score`: {score}")
            .unwrap_or_else(|error| panic!("render score: {error}"));
    }
    markdown.push('\n');
}

fn push_indented_block(markdown: &mut String, value: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    for line in value.lines() {
        markdown.push_str(&prefix);
        markdown.push_str(line);
        markdown.push('\n');
    }
}

fn target_type_slug(target_type: &ProposalTargetType) -> &'static str {
    match target_type {
        ProposalTargetType::DocsNote => "docs_note",
        ProposalTargetType::Adr => "adr",
        ProposalTargetType::Skill => "skill",
        ProposalTargetType::DebuggingPlaybook => "debugging_playbook",
        ProposalTargetType::MemoryPatch => "memory_patch",
        ProposalTargetType::SemanticGraphGrouping => "semantic_graph_grouping",
    }
}

fn evidence_source_kind_slug(kind: &EvidenceSourceKind) -> &'static str {
    match kind {
        EvidenceSourceKind::HotspotSubject => "hotspot_subject",
        EvidenceSourceKind::LocalTraceRow => "local_trace_row",
        EvidenceSourceKind::ServerTraceRow => "server_trace_row",
        EvidenceSourceKind::DocReference => "doc_reference",
        EvidenceSourceKind::RecordedEvidence => "recorded_evidence",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use scryrs_types::{
        PROPOSAL_SCHEMA_VERSION, ProposalDocument, REVIEW_DECISION_SCHEMA_VERSION,
        SemanticGraphGrouping,
    };
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn make_evidence(subject: &str) -> Vec<EvidenceLink> {
        vec![EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: subject.to_string(),
            row_ids: vec![7, 9],
            doc_ref: Some(".devagent/docs/docs/proposals.md".into()),
            description: Some("captured from accepted review".into()),
            score: Some(42),
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
            accepted_content: Some(ProposedContent::MemoryPatch(serde_json::json!({
                "patch": "value"
            }))),
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
            serde_json::to_string(proposal).expect("serialize proposal"),
        )
        .expect("write proposal");
    }

    fn write_accepted_decision(repo_root: &Path, decision: &ProposalReviewDecision) {
        let accepted_dir = repo_root.join(".scryrs/accepted");
        fs::create_dir_all(&accepted_dir).expect("create accepted dir");
        fs::write(
            accepted_dir.join(format!("{}.json", decision.proposal_id)),
            serde_json::to_string(decision).expect("serialize decision"),
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

    #[test]
    fn descriptor_marks_markdown_adapter() {
        assert_eq!(descriptor().id, "adapter-markdown");
        assert!(!descriptor().title.is_empty());
        assert!(!descriptor().summary.is_empty());
    }

    #[test]
    fn pending_proposals_alone_do_not_publish() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("release-output");
        let pending = make_markdown_proposal(
            ProposalTargetType::DocsNote,
            "pending-note",
            "## Pending only\n",
        );
        write_pending_proposal(repo.path(), &pending);

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");

        assert!(written.is_empty());
        assert!(!output.exists());
    }

    #[test]
    fn mixed_pending_and_accepted_repositories_publish_only_accepted_ids() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("chosen-output-root");
        let pending = make_markdown_proposal(
            ProposalTargetType::DocsNote,
            "pending-note",
            "## Pending only\n",
        );
        let accepted = make_markdown_proposal(ProposalTargetType::Adr, "accepted-adr", "## ADR\n");
        write_pending_proposal(repo.path(), &pending);
        write_pending_proposal(repo.path(), &accepted);
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&accepted, "alice", "accepted for publication"),
        );

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");

        assert_eq!(written.len(), 1);
        assert_eq!(
            written[0],
            output.join("adr").join(format!("{}.md", accepted.id))
        );
        assert!(
            !output
                .join("docs_note")
                .join(format!("{}.md", pending.id))
                .exists()
        );
    }

    #[test]
    fn accepted_markdown_target_types_use_stable_paths_without_deleting_stale_output() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("arbitrary-output-root");
        fs::create_dir_all(output.join("docs_note")).expect("create output dir");
        let stale_path = output.join("docs_note/stale.md");
        fs::write(&stale_path, "stale bytes\n").expect("write stale file");

        let docs = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        let adr = make_markdown_proposal(ProposalTargetType::Adr, "adr", "## ADR\n");
        let skill = make_markdown_proposal(ProposalTargetType::Skill, "skill", "## Skill\n");
        let playbook = make_markdown_proposal(
            ProposalTargetType::DebuggingPlaybook,
            "playbook",
            "## Playbook\n",
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&docs, "alice", "publish docs"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&adr, "alice", "publish adr"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&skill, "alice", "publish skill"),
        );
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&playbook, "alice", "publish playbook"),
        );

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");

        let mut expected_ids = vec![
            docs.id.clone(),
            adr.id.clone(),
            skill.id.clone(),
            playbook.id.clone(),
        ];
        expected_ids.sort();
        let written_ids: Vec<String> = written
            .iter()
            .map(|path| {
                path.file_stem()
                    .expect("stem")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(written_ids, expected_ids);
        assert!(
            output
                .join("docs_note")
                .join(format!("{}.md", docs.id))
                .is_file()
        );
        assert!(output.join("adr").join(format!("{}.md", adr.id)).is_file());
        assert!(
            output
                .join("skill")
                .join(format!("{}.md", skill.id))
                .is_file()
        );
        assert!(
            output
                .join("debugging_playbook")
                .join(format!("{}.md", playbook.id))
                .is_file()
        );
        assert_eq!(
            fs::read_to_string(&stale_path).expect("stale bytes"),
            "stale bytes\n"
        );
    }

    #[test]
    fn repeated_publish_runs_are_byte_stable() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("output-root");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "stable publish"),
        );

        publish_accepted_markdown(repo.path(), &output).expect("first publish");
        let first_snapshot = snapshot_dir(&output);
        publish_accepted_markdown(repo.path(), &output).expect("second publish");
        let second_snapshot = snapshot_dir(&output);

        assert_eq!(first_snapshot, second_snapshot);
    }

    #[test]
    fn rendered_markdown_includes_review_metadata_body_and_evidence_backlinks() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("published-markdown");
        let proposal = make_markdown_proposal(
            ProposalTargetType::DocsNote,
            "docs",
            "## Accepted body\n\nDetails preserved.\n",
        );
        let decision = make_accepted_decision(&proposal, "alice", "approved with evidence");
        write_accepted_decision(repo.path(), &decision);

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");
        let markdown = fs::read_to_string(&written[0]).expect("read markdown");

        assert!(markdown.contains(&format!("# docs_note {}", proposal.id)));
        assert!(markdown.contains("## Review Metadata"));
        assert!(markdown.contains(&format!("`proposalId`: `{}`", proposal.id)));
        assert!(markdown.contains("`targetType`: `docs_note`"));
        assert!(markdown.contains("`reviewer`: `alice`"));
        assert!(markdown.contains("`decidedAt`: `2026-06-29T18:00:00Z`"));
        assert!(markdown.contains("`rationale`:"));
        assert!(markdown.contains("  approved with evidence"));
        assert!(markdown.contains("## Accepted body\n\nDetails preserved."));
        assert!(markdown.contains("## Evidence backlinks"));
        assert!(markdown.contains("### Evidence 1"));
        assert!(markdown.contains("`sourceKind`: `local_trace_row`"));
        assert!(markdown.contains("`subject`: `docs`"));
        assert!(markdown.contains("`rowIds`: 7, 9"));
        assert!(markdown.contains("`docRef`: `.devagent/docs/docs/proposals.md`"));
        assert!(markdown.contains("`description`:"));
        assert!(markdown.contains("  captured from accepted review"));
        assert!(markdown.contains("`score`: 42"));
    }

    #[test]
    fn malformed_accepted_artifacts_fail_loudly_without_partial_output() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("output-root");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "valid but blocked by malformed sibling"),
        );
        let accepted_dir = repo.path().join(".scryrs/accepted");
        fs::create_dir_all(&accepted_dir).expect("create accepted dir");
        fs::write(accepted_dir.join("broken.json"), "{not valid json").expect("write invalid json");

        let error = publish_accepted_markdown(repo.path(), &output).expect_err("publish fails");

        assert!(error.to_string().contains("invalid accepted artifact"));
        assert!(snapshot_dir(&output).is_empty());
    }

    #[test]
    fn non_markdown_accepted_artifacts_are_skipped_without_error() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("output-root");
        let proposal = make_markdown_proposal(ProposalTargetType::DocsNote, "docs", "## Docs\n");
        write_accepted_decision(
            repo.path(),
            &make_accepted_decision(&proposal, "alice", "publish markdown"),
        );
        write_accepted_decision(repo.path(), &make_memory_patch_decision("memory-proposal"));
        write_accepted_decision(repo.path(), &make_grouping_decision("grouping-proposal"));

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");

        assert_eq!(written.len(), 1);
        assert!(written[0].ends_with(format!("docs_note/{}.md", proposal.id)));
        assert!(!output.join("memory_patch/memory-proposal.md").exists());
        assert!(
            !output
                .join("semantic_graph_grouping/grouping-proposal.md")
                .exists()
        );
    }

    #[test]
    fn missing_accepted_directory_is_a_no_op_success() {
        let repo = TempDir::new().expect("tempdir");
        let output = repo.path().join("output-root");

        let written = publish_accepted_markdown(repo.path(), &output).expect("publish succeeds");

        assert!(written.is_empty());
        assert!(!output.exists());
    }
}
