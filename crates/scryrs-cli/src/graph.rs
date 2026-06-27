use std::io::Write;

#[cfg(feature = "graph")]
use std::path::Path;

#[cfg(feature = "graph")]
use scryrs_graph::KnowledgeGraph;
#[cfg(feature = "graph")]
use scryrs_types::{EvidenceLink, EvidenceSourceKind, GraphEdge, GraphNode};

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
    use scryrs_types::{EvidenceSourceKind, GraphNode};

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
