use std::io::Write;

#[cfg(feature = "graph")]
use std::{
    collections::HashSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[cfg(feature = "graph")]
use scryrs_graph::KnowledgeGraph;
#[cfg(feature = "graph")]
use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, GraphEdge, GraphNode, ProposalReviewDecision,
    ProposalTargetType, ProposedContent, ReviewOutcome,
};

#[cfg(feature = "graph")]
pub(crate) fn write_graph_json(out: &mut impl Write, err: &mut impl Write, path: &str) -> i32 {
    // Resolve to absolute repo root.
    let repo_root = match std::path::absolute(path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "scryrs graph: cannot resolve path '{path}': {e}");
            return 2;
        }
    };

    // Load hotspots.json (required).
    let hotspots_path = repo_root.join(".scryrs/hotspots.json");
    let hotspots_json = match std::fs::read_to_string(&hotspots_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(
                err,
                "scryrs graph: hotspots artifact not found at {}",
                hotspots_path.display()
            );
            return 2;
        }
    };
    let hotspots_entries: HotspotsEntries = match serde_json::from_str(&hotspots_json) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "scryrs graph: malformed hotspots file: {e}");
            return 2;
        }
    };

    // Scan docs (optional — tolerated-missing).
    let docs_dir = repo_root.join(".devagent/docs/docs");
    let nav_path = docs_dir.join("_nav.json");
    let (docs_exist, nav_groups) = load_docs(&docs_dir, &nav_path, err);

    // Build the graph.
    let mut kg = KnowledgeGraph::new();

    // Hotspot nodes — all five subject kinds included.
    for entry in &hotspots_entries.entries {
        let node_id = format!("{}:{}", entry.subjectKind, entry.subject);
        let evidence_link = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: entry.subject.clone(),
            row_ids: entry.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(entry.score),
            metadata: None,
        };
        kg.add_node(GraphNode {
            id: node_id,
            label: entry.subject.clone(),
            description: None,
            kind: entry.subjectKind.clone(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![evidence_link],
            metadata: None,
        });
    }

    // Doc nodes and nav-hierarchy contains edges.
    if docs_exist {
        build_doc_layer(&mut kg, &nav_groups);
    }

    if let Err(error) = load_accepted_evidence(&repo_root, &mut kg, err) {
        let _ = writeln!(err, "scryrs graph: {error}");
        return 2;
    }

    // Validate and materialize.
    let document = match kg.to_document(None) {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs graph: validation error: {e}");
            return 1;
        }
    };

    // Serialize.
    let json = match serde_json::to_string(&document) {
        Ok(j) => j,
        Err(e) => {
            let _ = writeln!(err, "scryrs graph: serialization error: {e}");
            return 1;
        }
    };

    // Write to stdout.
    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    // Write artifact to .scryrs/graph.json.
    let artifact_path = repo_root.join(".scryrs/graph.json");
    if let Err(e) = std::fs::write(&artifact_path, &json) {
        let _ = writeln!(err, "scryrs graph: cannot write artifact file: {e}");
        return 1;
    }

    0
}

/// Load docs directory metadata. Returns `(docs_exist, nav_groups)`.
/// Prints a stderr warning when the docs directory is missing or empty.
#[cfg(feature = "graph")]
fn load_docs(docs_dir: &Path, nav_path: &Path, err: &mut impl Write) -> (bool, Vec<NavGroup>) {
    if !docs_dir.is_dir() {
        let _ = writeln!(
            err,
            "scryrs graph: warning: docs directory not found; producing hotspot-only graph"
        );
        return (false, vec![]);
    }

    // Enumerate .md / .mdx files (sorted for determinism).
    let mut page_slugs: Vec<String> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(docs_dir) {
        for entry in read_dir.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                if (ext == "md" || ext == "mdx")
                    && p.file_stem().and_then(|s| s.to_str()) != Some("_nav")
                {
                    if let Some(slug) = p.file_stem().and_then(|s| s.to_str()) {
                        page_slugs.push(slug.to_string());
                    }
                }
            }
        }
    }
    page_slugs.sort();
    // De-duplicate (shouldn't happen, but safe).
    page_slugs.dedup();

    if page_slugs.is_empty() {
        let _ = writeln!(
            err,
            "scryrs graph: warning: docs directory is empty; producing hotspot-only graph"
        );
        return (false, vec![]);
    }

    // Load _nav.json for hierarchy.
    let nav_groups: Vec<NavGroup> = match std::fs::read_to_string(nav_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(groups) => groups,
            Err(_) => {
                let _ = writeln!(
                    err,
                    "scryrs graph: warning: _nav.json is malformed; producing hotspot-only graph"
                );
                return (false, vec![]);
            }
        },
        Err(_) => {
            let _ = writeln!(
                err,
                "scryrs graph: warning: _nav.json not found; producing hotspot-only graph"
            );
            return (false, vec![]);
        }
    };

    if nav_groups.is_empty() {
        let _ = writeln!(
            err,
            "scryrs graph: warning: _nav.json is empty; producing hotspot-only graph"
        );
        return (false, vec![]);
    }

    (true, nav_groups)
}

/// Build doc-layer nodes and contains edges from nav hierarchy.
#[cfg(feature = "graph")]
fn build_doc_layer(kg: &mut KnowledgeGraph, nav_groups: &[NavGroup]) {
    // Synthetic docs_root node.
    kg.add_node(GraphNode {
        id: "docs_root".into(),
        label: "Documentation Root".into(),
        description: None,
        kind: "doc_group".into(),
        tags: vec![],
        aliases: vec![],
        evidence_links: vec![],
        metadata: None,
    });

    for group in nav_groups {
        let group_id = slugify(&group.text);

        // Nav group node.
        kg.add_node(GraphNode {
            id: group_id.clone(),
            label: group.text.clone(),
            description: None,
            kind: "doc_group".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        });

        // docs_root → group edge.
        kg.add_edge(GraphEdge {
            id: format!("docs_root_contains_{group_id}"),
            source_node_id: "docs_root".into(),
            target_node_id: group_id.clone(),
            relationship: "contains".into(),
            label: None,
            tags: vec![],
            evidence_links: vec![],
            metadata: None,
        });

        // Items in array order (deterministic from JSON order).
        for item in &group.items {
            let slug = item.link.trim_start_matches('/').to_string();
            let doc_node_id = format!("doc_page:{slug}");

            kg.add_node(GraphNode {
                id: doc_node_id.clone(),
                label: slug.clone(),
                description: None,
                kind: "doc_page".into(),
                tags: vec![],
                aliases: vec![],
                evidence_links: vec![EvidenceLink {
                    source_kind: EvidenceSourceKind::DocReference,
                    subject: slug.clone(),
                    row_ids: vec![],
                    doc_ref: Some(slug.clone()),
                    description: None,
                    score: None,
                    metadata: None,
                }],
                metadata: None,
            });

            // group → page edge.
            kg.add_edge(GraphEdge {
                id: format!("{group_id}_contains_{doc_node_id}"),
                source_node_id: group_id.clone(),
                target_node_id: doc_node_id,
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: vec![],
                metadata: None,
            });
        }
    }
}

/// Derive a stable nav group node ID from human-readable text.
#[cfg(feature = "graph")]
fn slugify(text: &str) -> String {
    text.to_lowercase().replace(' ', "-")
}

#[cfg(feature = "graph")]
fn load_accepted_evidence(
    repo_root: &Path,
    kg: &mut KnowledgeGraph,
    err: &mut impl Write,
) -> Result<(), String> {
    let accepted_dir = repo_root.join(".scryrs/accepted");
    let accepted_paths = json_files_in_dir(&accepted_dir)?;
    let baseline_node_ids: HashSet<String> =
        kg.nodes().iter().map(|node| node.id.clone()).collect();
    let mut projected_group_ids = HashSet::new();
    let mut grouped_source_node_ids = HashSet::new();

    for path in accepted_paths {
        let decision = load_accepted_decision(&path)?;
        let target_type = decision.target_type.as_ref().ok_or_else(|| {
            format!(
                "invalid accepted artifact {}: missing targetType",
                path.display()
            )
        })?;

        if *target_type != ProposalTargetType::SemanticGraphGrouping {
            let _ = writeln!(
                err,
                "scryrs graph: warning: skipping accepted targetType '{}' from {}",
                proposal_target_type_name(target_type),
                path.display()
            );
            continue;
        }

        let grouping = match decision.accepted_content.as_ref() {
            Some(ProposedContent::SemanticGraphGrouping(grouping)) => grouping,
            Some(_) => {
                return Err(format!(
                    "invalid accepted artifact {}: semantic_graph_grouping targetType requires semantic_graph_grouping acceptedContent",
                    path.display()
                ));
            }
            None => {
                return Err(format!(
                    "invalid accepted artifact {}: missing acceptedContent",
                    path.display()
                ));
            }
        };

        if baseline_node_ids.contains(&grouping.target_group_node_id) {
            return Err(format!(
                "accepted target group node ID '{}' collides with existing graph node ID",
                grouping.target_group_node_id
            ));
        }

        if !projected_group_ids.insert(grouping.target_group_node_id.clone()) {
            return Err(format!(
                "conflicting accepted grouping for target group node ID '{}'",
                grouping.target_group_node_id
            ));
        }

        let group_kind =
            group_kind_from_node_id(&grouping.target_group_node_id).ok_or_else(|| {
                format!(
                    "cannot derive node kind from accepted target group node ID '{}' in {}",
                    grouping.target_group_node_id,
                    path.display()
                )
            })?;

        for source_node_id in &grouping.source_node_ids {
            if !baseline_node_ids.contains(source_node_id) {
                return Err(format!(
                    "accepted decision '{}' references missing source node ID '{}'",
                    decision.proposal_id, source_node_id
                ));
            }
            if !grouped_source_node_ids.insert(source_node_id.clone()) {
                return Err(format!(
                    "conflicting accepted grouping for source node ID '{}'",
                    source_node_id
                ));
            }
        }

        kg.add_node(GraphNode {
            id: grouping.target_group_node_id.clone(),
            label: grouping.target_group_label.clone(),
            description: None,
            kind: group_kind.to_string(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::RecordedEvidence,
                subject: decision.proposal_id.clone(),
                row_ids: vec![],
                doc_ref: None,
                description: Some(path.display().to_string()),
                score: None,
                metadata: None,
            }],
            metadata: None,
        });

        for source_node_id in &grouping.source_node_ids {
            kg.add_edge(GraphEdge {
                id: format!(
                    "{}_contains_{}",
                    grouping.target_group_node_id, source_node_id
                ),
                source_node_id: grouping.target_group_node_id.clone(),
                target_node_id: source_node_id.clone(),
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: decision.source_evidence.clone(),
                metadata: None,
            });
        }
    }

    Ok(())
}

#[cfg(feature = "graph")]
fn load_accepted_decision(path: &Path) -> Result<ProposalReviewDecision, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read accepted artifact {}: {error}", path.display()))?;
    let decision: ProposalReviewDecision = serde_json::from_str(&json)
        .map_err(|error| format!("invalid accepted artifact {}: {error}", path.display()))?;
    decision
        .validate()
        .map_err(|error| format!("invalid accepted artifact {}: {error}", path.display()))?;
    if decision.outcome != ReviewOutcome::Accepted {
        return Err(format!(
            "invalid accepted artifact {}: outcome must be accepted",
            path.display()
        ));
    }
    Ok(decision)
}

#[cfg(feature = "graph")]
fn json_files_in_dir(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err(format!("expected directory {}", dir.display()));
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("cannot read directory {}: {error}", dir.display()))?
    {
        let entry =
            entry.map_err(|error| format!("cannot read directory {}: {error}", dir.display()))?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(feature = "graph")]
fn group_kind_from_node_id(node_id: &str) -> Option<&str> {
    let (kind, _) = node_id.split_once(':')?;
    if kind.is_empty() {
        return None;
    }
    Some(kind)
}

#[cfg(feature = "graph")]
fn proposal_target_type_name(target_type: &ProposalTargetType) -> String {
    serde_json::to_string(target_type)
        .map(|value| value.trim_matches('"').to_string())
        .unwrap_or_else(|_| "<unknown>".into())
}

/// Partial deserialization target for hotspots.json — only the entries array.
#[cfg(feature = "graph")]
#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
struct HotspotsEntries {
    entries: Vec<scryrs_types::HotspotEntry>,
}

/// Nav group from `_nav.json` — parsed as ordered array.
#[cfg(feature = "graph")]
#[derive(Debug, serde::Deserialize)]
struct NavGroup {
    text: String,
    items: Vec<NavItem>,
}

#[cfg(feature = "graph")]
#[derive(Debug, serde::Deserialize)]
struct NavItem {
    #[allow(dead_code)]
    text: String,
    link: String,
}

#[cfg(not(feature = "graph"))]
pub(crate) fn write_graph_json(_out: &mut impl Write, err: &mut impl Write, _path: &str) -> i32 {
    let _ = writeln!(err, "scryrs graph: unavailable (graph feature not enabled)");
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

#[cfg(all(test, feature = "graph"))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use scryrs_types::{
        EvidenceSourceKind, GraphNode, KnowledgeGraphDocument, ProposalReviewDecision,
        ProposalTargetType, ProposedContent, ReviewOutcome, SemanticGraphGrouping,
    };

    fn write_hotspots_json(
        repo_root: &std::path::Path,
        entries: serde_json::Value,
    ) -> std::path::PathBuf {
        let scryrs_dir = repo_root.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        let path = scryrs_dir.join("hotspots.json");
        std::fs::write(
            &path,
            serde_json::to_string(&serde_json::json!({ "entries": entries }))
                .expect("serialize hotspots"),
        )
        .expect("write hotspots");
        path
    }

    fn write_review_artifact(
        repo_root: &std::path::Path,
        state_dir: &str,
        file_name: &str,
        decision: &ProposalReviewDecision,
    ) {
        let dir = repo_root.join(format!(".scryrs/{state_dir}"));
        std::fs::create_dir_all(&dir).expect("create review dir");
        std::fs::write(
            dir.join(file_name),
            serde_json::to_string(decision).expect("serialize decision"),
        )
        .expect("write decision");
    }

    fn accepted_grouping_decision(
        proposal_id: &str,
        source_node_ids: Vec<&str>,
        target_group_node_id: &str,
        target_group_label: &str,
    ) -> ProposalReviewDecision {
        ProposalReviewDecision {
            schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: proposal_id.into(),
            reviewer: "reviewer".into(),
            decided_at: "2026-01-02T00:00:00Z".into(),
            rationale: "group nodes".into(),
            source_evidence: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: proposal_id.into(),
                row_ids: vec![11, 12],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            outcome: ReviewOutcome::Accepted,
            target_type: Some(ProposalTargetType::SemanticGraphGrouping),
            accepted_content: Some(ProposedContent::SemanticGraphGrouping(
                SemanticGraphGrouping {
                    source_node_ids: source_node_ids.into_iter().map(Into::into).collect(),
                    target_group_node_id: target_group_node_id.into(),
                    target_group_label: target_group_label.into(),
                },
            )),
        }
    }

    fn run_graph_build_raw(repo_root: &std::path::Path) -> (i32, Vec<u8>, Vec<u8>) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let exit_code = write_graph_json(&mut out, &mut err, repo_root.to_str().unwrap());
        (exit_code, out, err)
    }

    fn run_graph_build(
        repo_root: &std::path::Path,
    ) -> (i32, Vec<u8>, Vec<u8>, KnowledgeGraphDocument) {
        let (exit_code, out, err) = run_graph_build_raw(repo_root);
        let document = serde_json::from_slice(&out).expect("valid graph document");
        (exit_code, out, err, document)
    }

    #[test]
    fn node_id_derivation_from_hotspot_entry() {
        let entry = scryrs_types::HotspotEntry {
            rank: 1,
            subjectKind: "file".into(),
            subject: "src/main.rs".into(),
            score: 10,
            counts: scryrs_types::HotspotCounts {
                eventType: Default::default(),
                outcome: Default::default(),
            },
            sessionCount: 1,
            firstSeen: "2026-01-01T00:00:00Z".into(),
            lastSeen: "2026-01-01T00:00:00Z".into(),
            evidence: scryrs_types::HotspotEvidence { rowIds: vec![1, 2] },
        };

        let node_id = format!("{}:{}", entry.subjectKind, entry.subject);
        assert_eq!(node_id, "file:src/main.rs");

        // All five subject kinds.
        for kind in &["file", "search", "symbol", "command", "document"] {
            let id = format!("{kind}:subj");
            assert!(id.starts_with(kind));
        }
    }

    #[test]
    fn evidence_link_from_hotspot_preserves_row_ids_and_score() {
        let entry = scryrs_types::HotspotEntry {
            rank: 1,
            subjectKind: "symbol".into(),
            subject: "MyStruct".into(),
            score: 42,
            counts: scryrs_types::HotspotCounts {
                eventType: Default::default(),
                outcome: Default::default(),
            },
            sessionCount: 1,
            firstSeen: "2026-01-01T00:00:00Z".into(),
            lastSeen: "2026-01-01T00:00:00Z".into(),
            evidence: scryrs_types::HotspotEvidence {
                rowIds: vec![5, 12, 23],
            },
        };

        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: entry.subject.clone(),
            row_ids: entry.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(entry.score),
            metadata: None,
        };

        assert_eq!(link.source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(link.subject, "MyStruct");
        assert_eq!(link.row_ids, vec![5, 12, 23]);
        assert_eq!(link.score, Some(42));
    }

    #[test]
    fn doc_page_node_id_is_doc_page_colon_slug() {
        let slug = "graph";
        let node_id = format!("doc_page:{slug}");
        assert_eq!(node_id, "doc_page:graph");
    }

    #[test]
    fn doc_page_evidence_link_has_doc_reference_kind() {
        let slug = "graph".to_string();
        let link = EvidenceLink {
            source_kind: EvidenceSourceKind::DocReference,
            subject: slug.clone(),
            row_ids: vec![],
            doc_ref: Some(slug.clone()),
            description: None,
            score: None,
            metadata: None,
        };

        assert_eq!(link.source_kind, EvidenceSourceKind::DocReference);
        assert_eq!(link.doc_ref.as_deref(), Some("graph"));
        assert!(link.row_ids.is_empty());
    }

    #[test]
    fn slugify_replaces_spaces_with_hyphens() {
        assert_eq!(slugify("Vision & Strategy"), "vision-&-strategy");
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Technical"), "technical");
    }

    #[test]
    fn build_doc_layer_creates_docs_root_node() {
        let nav = vec![NavGroup {
            text: "Technical".into(),
            items: vec![NavItem {
                text: "Graph".into(),
                link: "/graph".into(),
            }],
        }];

        let mut kg = KnowledgeGraph::new();
        build_doc_layer(&mut kg, &nav);

        // docs_root node must exist.
        let nodes = kg.nodes();
        let root = nodes.iter().find(|n| n.id == "docs_root");
        assert!(root.is_some(), "docs_root node must be created");
        assert_eq!(root.unwrap().kind, "doc_group");

        // Nav group node must exist.
        let group = nodes.iter().find(|n| n.id == "technical");
        assert!(group.is_some(), "nav group node must be created");

        // Doc page node must exist.
        let page = nodes.iter().find(|n| n.id == "doc_page:graph");
        assert!(page.is_some(), "doc page node must be created");
        assert_eq!(page.unwrap().kind, "doc_page");
    }

    #[test]
    fn build_doc_layer_creates_contains_edges() {
        let nav = vec![NavGroup {
            text: "Technical".into(),
            items: vec![
                NavItem {
                    text: "Graph".into(),
                    link: "/graph".into(),
                },
                NavItem {
                    text: "CLI".into(),
                    link: "/cli-v0-contract".into(),
                },
            ],
        }];

        let mut kg = KnowledgeGraph::new();
        build_doc_layer(&mut kg, &nav);

        let edges = kg.edges();

        // docs_root → technical edge.
        let root_edge = edges
            .iter()
            .find(|e| e.source_node_id == "docs_root" && e.target_node_id == "technical");
        assert!(
            root_edge.is_some(),
            "docs_root -> technical edge must exist"
        );
        assert_eq!(root_edge.unwrap().relationship, "contains");

        // technical → doc_page:graph edge.
        let page_edge1 = edges
            .iter()
            .find(|e| e.source_node_id == "technical" && e.target_node_id == "doc_page:graph");
        assert!(
            page_edge1.is_some(),
            "technical -> doc_page:graph edge must exist"
        );

        // technical → doc_page:cli-v0-contract edge.
        let page_edge2 = edges.iter().find(|e| {
            e.source_node_id == "technical" && e.target_node_id == "doc_page:cli-v0-contract"
        });
        assert!(
            page_edge2.is_some(),
            "technical -> doc_page:cli-v0-contract edge must exist"
        );
    }

    #[test]
    fn no_cross_domain_edges_in_v1() {
        let nav = vec![NavGroup {
            text: "Tech".into(),
            items: vec![NavItem {
                text: "Graph".into(),
                link: "/graph".into(),
            }],
        }];

        let mut kg = KnowledgeGraph::new();

        // Add a hotspot node.
        kg.add_node(GraphNode {
            id: "file:src/main.rs".into(),
            label: "src/main.rs".into(),
            description: None,
            kind: "file".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        });

        build_doc_layer(&mut kg, &nav);

        // Validate — should not reject dangling edges (no edges connect hotspot to doc).
        assert!(kg.validate().is_ok());

        // No edges should involve the hotspot node.
        for edge in kg.edges() {
            assert_ne!(
                edge.source_node_id, "file:src/main.rs",
                "no edge should originate from hotspot node"
            );
            assert_ne!(
                edge.target_node_id, "file:src/main.rs",
                "no edge should target hotspot node"
            );
        }
    }

    #[test]
    fn empty_nav_groups_produce_no_doc_nodes() {
        let mut kg = KnowledgeGraph::new();
        build_doc_layer(&mut kg, &[]);

        assert_eq!(
            kg.nodes().len(),
            1,
            "only docs_root should exist for empty nav"
        );
        assert_eq!(kg.nodes()[0].id, "docs_root");
    }

    #[test]
    fn to_document_produces_sorted_output() {
        let mut kg = KnowledgeGraph::new();

        // Add nodes in non-sorted order.
        kg.add_node(GraphNode {
            id: "file:zzz.rs".into(),
            label: "zzz.rs".into(),
            description: None,
            kind: "file".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        });
        kg.add_node(GraphNode {
            id: "file:aaa.rs".into(),
            label: "aaa.rs".into(),
            description: None,
            kind: "file".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        });

        let doc = kg.to_document(None).expect("valid graph");
        assert_eq!(doc.nodes[0].id, "file:aaa.rs");
        assert_eq!(doc.nodes[1].id, "file:zzz.rs");
    }

    #[test]
    fn accepted_semantic_grouping_creates_group_node_and_contains_edges() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        std::fs::create_dir(&scryrs_dir).expect("create .scryrs");

        let hotspots = serde_json::json!({
            "entries": [
                {
                    "rank": 1,
                    "subjectKind": "file",
                    "subject": "auth",
                    "score": 10,
                    "counts": {"eventType": {}, "outcome": {}},
                    "sessionCount": 1,
                    "firstSeen": "2026-01-01T00:00:00Z",
                    "lastSeen": "2026-01-01T00:00:00Z",
                    "evidence": {"rowIds": [1]}
                },
                {
                    "rank": 2,
                    "subjectKind": "search",
                    "subject": "auth",
                    "score": 7,
                    "counts": {"eventType": {}, "outcome": {}},
                    "sessionCount": 1,
                    "firstSeen": "2026-01-01T00:00:00Z",
                    "lastSeen": "2026-01-01T00:00:00Z",
                    "evidence": {"rowIds": [2]}
                }
            ]
        });
        std::fs::write(
            scryrs_dir.join("hotspots.json"),
            serde_json::to_string(&hotspots).expect("serialize hotspots"),
        )
        .expect("write hotspots");

        let accepted_dir = scryrs_dir.join("accepted");
        std::fs::create_dir(&accepted_dir).expect("create accepted dir");
        let decision = ProposalReviewDecision {
            schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: "proposal-auth-group".into(),
            reviewer: "reviewer".into(),
            decided_at: "2026-01-02T00:00:00Z".into(),
            rationale: "group auth nodes".into(),
            source_evidence: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "auth".into(),
                row_ids: vec![11, 12],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            outcome: ReviewOutcome::Accepted,
            target_type: Some(ProposalTargetType::SemanticGraphGrouping),
            accepted_content: Some(ProposedContent::SemanticGraphGrouping(
                SemanticGraphGrouping {
                    source_node_ids: vec!["file:auth".into(), "search:auth".into()],
                    target_group_node_id: "domain_term:auth".into(),
                    target_group_label: "Auth".into(),
                },
            )),
        };
        std::fs::write(
            accepted_dir.join("z-group.json"),
            serde_json::to_string(&decision).expect("serialize decision"),
        )
        .expect("write decision");

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_graph_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        let document: scryrs_types::KnowledgeGraphDocument =
            serde_json::from_str(stdout.trim()).expect("valid graph document");

        let group = document
            .nodes
            .iter()
            .find(|node| node.id == "domain_term:auth")
            .expect("group node exists");
        assert_eq!(group.label, "Auth");
        assert_eq!(group.kind, "domain_term");
        assert!(group.evidence_links.iter().any(|link| {
            link.source_kind == EvidenceSourceKind::RecordedEvidence
                && link.subject == "proposal-auth-group"
        }));

        let file_edge = document
            .edges
            .iter()
            .find(|edge| edge.id == "domain_term:auth_contains_file:auth")
            .expect("file contains edge exists");
        assert_eq!(file_edge.relationship, "contains");
        assert_eq!(file_edge.evidence_links, decision.source_evidence);

        let search_edge = document
            .edges
            .iter()
            .find(|edge| edge.id == "domain_term:auth_contains_search:auth")
            .expect("search contains edge exists");
        assert_eq!(search_edge.relationship, "contains");
        assert_eq!(search_edge.evidence_links, decision.source_evidence);
    }

    #[test]
    fn accepted_non_semantic_decision_is_skipped_with_warning() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        std::fs::create_dir(&scryrs_dir).expect("create .scryrs");

        let hotspots = serde_json::json!({
            "entries": [{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]
        });
        std::fs::write(
            scryrs_dir.join("hotspots.json"),
            serde_json::to_string(&hotspots).expect("serialize hotspots"),
        )
        .expect("write hotspots");

        let accepted_dir = scryrs_dir.join("accepted");
        std::fs::create_dir(&accepted_dir).expect("create accepted dir");
        let decision = ProposalReviewDecision {
            schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: "proposal-memory-patch".into(),
            reviewer: "reviewer".into(),
            decided_at: "2026-01-02T00:00:00Z".into(),
            rationale: "patch memory".into(),
            source_evidence: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "auth".into(),
                row_ids: vec![7],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            outcome: ReviewOutcome::Accepted,
            target_type: Some(ProposalTargetType::MemoryPatch),
            accepted_content: Some(ProposedContent::MemoryPatch(serde_json::json!({
                "memory": "auth"
            }))),
        };
        std::fs::write(
            accepted_dir.join("memory.json"),
            serde_json::to_string(&decision).expect("serialize decision"),
        )
        .expect("write decision");

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_graph_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        let document: scryrs_types::KnowledgeGraphDocument =
            serde_json::from_str(stdout.trim()).expect("valid graph document");
        assert_eq!(document.nodes.len(), 1, "only hotspot node should remain");
        assert!(
            document.edges.is_empty(),
            "no grouping edges should be added"
        );

        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("warning: skipping accepted targetType 'memory_patch'"));
    }

    #[test]
    fn malformed_accepted_artifact_fails_without_writing_graph_artifact() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(tmp.path(), serde_json::json!([]));
        let accepted_dir = tmp.path().join(".scryrs/accepted");
        std::fs::create_dir_all(&accepted_dir).expect("create accepted dir");
        std::fs::write(accepted_dir.join("bad.json"), "not-json").expect("write bad decision");

        let (exit_code, out, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        assert!(
            out.is_empty(),
            "failing build should not emit stdout graph JSON"
        );
        assert!(
            String::from_utf8_lossy(&err).contains("invalid accepted artifact"),
            "stderr should identify the accepted artifact as invalid"
        );
        assert!(
            !tmp.path().join(".scryrs/graph.json").exists(),
            "graph artifact should not be written on failure"
        );
    }

    #[test]
    fn missing_accepted_source_node_fails_loudly() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "group.json",
            &accepted_grouping_decision(
                "proposal-auth-group",
                vec!["file:missing.rs"],
                "domain_term:auth",
                "Auth",
            ),
        );

        let (exit_code, _, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("proposal-auth-group"));
        assert!(stderr.contains("file:missing.rs"));
    }

    #[test]
    fn accepted_target_group_without_kind_prefix_fails_loudly() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "group.json",
            &accepted_grouping_decision("proposal-auth-group", vec!["file:auth"], "auth", "Auth"),
        );

        let (exit_code, _, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("cannot derive node kind"));
        assert!(stderr.contains("auth"));
    }

    #[test]
    fn duplicate_target_group_conflict_fails_loudly() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "a.json",
            &accepted_grouping_decision(
                "proposal-auth-group-a",
                vec!["file:auth"],
                "domain_term:auth",
                "Auth",
            ),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "b.json",
            &accepted_grouping_decision(
                "proposal-auth-group-b",
                vec!["file:auth"],
                "domain_term:auth",
                "Auth Duplicate",
            ),
        );

        let (exit_code, _, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("conflicting accepted grouping"));
        assert!(stderr.contains("domain_term:auth"));
    }

    #[test]
    fn duplicate_grouped_source_node_conflict_fails_loudly() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "a.json",
            &accepted_grouping_decision(
                "proposal-auth-domain",
                vec!["file:auth"],
                "domain_term:auth",
                "Auth",
            ),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "b.json",
            &accepted_grouping_decision(
                "proposal-auth-concept",
                vec!["file:auth"],
                "concept:auth",
                "Auth Concept",
            ),
        );

        let (exit_code, _, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("conflicting accepted grouping for source node ID"));
        assert!(stderr.contains("file:auth"));
    }

    #[test]
    fn accepted_target_group_id_collision_with_existing_node_fails_loudly() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "group.json",
            &accepted_grouping_decision(
                "proposal-auth-group",
                vec!["file:auth"],
                "file:auth",
                "Auth",
            ),
        );

        let (exit_code, _, err) = run_graph_build_raw(tmp.path());
        assert_ne!(exit_code, 0);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("collides with existing graph node ID"));
        assert!(stderr.contains("file:auth"));
    }

    #[test]
    fn pending_proposals_and_rejected_decisions_do_not_affect_graph() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([{
                "rank": 1,
                "subjectKind": "file",
                "subject": "auth",
                "score": 10,
                "counts": {"eventType": {}, "outcome": {}},
                "sessionCount": 1,
                "firstSeen": "2026-01-01T00:00:00Z",
                "lastSeen": "2026-01-01T00:00:00Z",
                "evidence": {"rowIds": [1]}
            }]),
        );
        let proposals_dir = tmp.path().join(".scryrs/proposals");
        std::fs::create_dir_all(&proposals_dir).expect("create proposals dir");
        std::fs::write(proposals_dir.join("pending.json"), "{ definitely-bad }")
            .expect("write pending proposal");
        let rejected = ProposalReviewDecision {
            schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
            proposal_id: "proposal-auth-group".into(),
            reviewer: "reviewer".into(),
            decided_at: "2026-01-02T00:00:00Z".into(),
            rationale: "reject".into(),
            source_evidence: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "auth".into(),
                row_ids: vec![1],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            outcome: ReviewOutcome::Rejected,
            target_type: None,
            accepted_content: None,
        };
        write_review_artifact(
            tmp.path(),
            "rejected",
            "proposal-auth-group.json",
            &rejected,
        );

        let (exit_code, _, _, document) = run_graph_build(tmp.path());
        assert_eq!(exit_code, 0);
        assert_eq!(document.nodes.len(), 1, "only hotspot node should remain");
        assert!(document.edges.is_empty());
    }

    #[test]
    fn accepted_artifacts_are_processed_deterministically() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        write_hotspots_json(
            tmp.path(),
            serde_json::json!([
                {
                    "rank": 1,
                    "subjectKind": "file",
                    "subject": "auth",
                    "score": 10,
                    "counts": {"eventType": {}, "outcome": {}},
                    "sessionCount": 1,
                    "firstSeen": "2026-01-01T00:00:00Z",
                    "lastSeen": "2026-01-01T00:00:00Z",
                    "evidence": {"rowIds": [1]}
                },
                {
                    "rank": 2,
                    "subjectKind": "file",
                    "subject": "billing",
                    "score": 9,
                    "counts": {"eventType": {}, "outcome": {}},
                    "sessionCount": 1,
                    "firstSeen": "2026-01-01T00:00:00Z",
                    "lastSeen": "2026-01-01T00:00:00Z",
                    "evidence": {"rowIds": [2]}
                }
            ]),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "z-billing.json",
            &accepted_grouping_decision(
                "proposal-billing-group",
                vec!["file:billing"],
                "domain_term:billing",
                "Billing",
            ),
        );
        write_review_artifact(
            tmp.path(),
            "accepted",
            "a-auth.json",
            &accepted_grouping_decision(
                "proposal-auth-group",
                vec!["file:auth"],
                "domain_term:auth",
                "Auth",
            ),
        );

        let (_, out1, _, _) = run_graph_build(tmp.path());
        let (_, out2, _, _) = run_graph_build(tmp.path());
        assert_eq!(out1, out2, "repeated runs must be byte-identical");
    }

    #[test]
    fn full_pipeline_empty_hotspots_with_docs() {
        let tmp = tempfile::TempDir::new().expect("tempdir");

        // Write hotspots.
        let scryrs_dir = tmp.path().join(".scryrs");
        std::fs::create_dir(&scryrs_dir).expect("create .scryrs");
        let hotspots = serde_json::json!({"entries": []});
        std::fs::write(
            scryrs_dir.join("hotspots.json"),
            serde_json::to_string(&hotspots).expect("serialize"),
        )
        .expect("write");

        // Write docs.
        let docs = tmp.path().join(".devagent/docs/docs");
        std::fs::create_dir_all(&docs).expect("create docs");
        std::fs::write(docs.join("graph.md"), "# Graph").expect("write page");
        let nav = serde_json::json!([{
            "text": "Tech",
            "items": [{"text": "Graph", "link": "/graph"}]
        }]);
        std::fs::write(
            docs.join("_nav.json"),
            serde_json::to_string(&nav).expect("serialize"),
        )
        .expect("write nav");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_graph_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        let doc: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("must be valid JSON");
        assert!(doc.get("schemaVersion").is_some());
        assert!(doc.get("nodes").is_some());
    }
}
