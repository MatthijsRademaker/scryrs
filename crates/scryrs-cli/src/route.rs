use std::io::Write;

#[cfg(feature = "graph")]
use std::collections::HashMap;

#[cfg(feature = "graph")]
use scryrs_types::{
    EvidenceLink, GRAPH_SCHEMA_VERSION, KnowledgeGraphDocument, ROUTE_SCHEMA_VERSION, RouteEntry,
    RouteGrouping, RouteManifestDocument,
};

#[cfg(feature = "graph")]
pub(crate) fn write_route_json(out: &mut impl Write, err: &mut impl Write, path: &str) -> i32 {
    // Resolve to absolute repo root.
    let repo_root = match std::path::absolute(path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "scryrs route: cannot resolve path '{path}': {e}");
            return 2;
        }
    };

    // Load graph.json (required).
    let graph_path = repo_root.join(".scryrs/graph.json");
    let graph_json = match std::fs::read_to_string(&graph_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(
                err,
                "scryrs route: graph artifact not found at {}",
                graph_path.display()
            );
            return 2;
        }
    };

    let graph_doc: KnowledgeGraphDocument = match serde_json::from_str(&graph_json) {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs route: malformed graph file: {e}");
            return 2;
        }
    };

    // Validate graph schema version.
    if graph_doc.schema_version != GRAPH_SCHEMA_VERSION {
        let _ = writeln!(
            err,
            "scryrs route: graph schema version mismatch: got '{}', expected '{}'",
            graph_doc.schema_version, GRAPH_SCHEMA_VERSION
        );
        return 2;
    }

    // Build parent lookup map from contains edges.
    let parent_map = build_parent_map(&graph_doc.edges);

    // Build route entries from graph nodes.
    let mut routes: Vec<RouteEntry> = graph_doc
        .nodes
        .iter()
        .map(|node| build_route_entry(node, &parent_map))
        .collect();

    // Enrich grouping labels from node list.
    enrich_group_labels(&mut routes, &graph_doc.nodes);

    // Sort routes by id ascending (deterministic).
    routes.sort_by(|a, b| a.id.cmp(&b.id));

    // Sort evidence links within each entry.
    for entry in &mut routes {
        sort_evidence_links(&mut entry.evidence_links);
    }

    // Construct the manifest document.
    let manifest = RouteManifestDocument {
        schema_version: ROUTE_SCHEMA_VERSION.into(),
        metadata: graph_doc.metadata.clone(),
        routes,
    };

    // Serialize as single-line JSON.
    let json = match serde_json::to_string(&manifest) {
        Ok(j) => j,
        Err(e) => {
            let _ = writeln!(err, "scryrs route: serialization error: {e}");
            return 1;
        }
    };

    // Write to stdout.
    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    // Write artifact to .scryrs/routes.json.
    let artifact_path = repo_root.join(".scryrs/routes.json");
    if let Err(e) = std::fs::write(&artifact_path, &json) {
        let _ = writeln!(err, "scryrs route: cannot write artifact file: {e}");
        return 1;
    }

    0
}

/// Build a parent lookup map from `contains` edges.
/// For each edge where `relationship == "contains"`, map
/// `target_node_id → (source_node_id, source_node_label)`.
/// We store `(id, label)` because label comes from the parent node,
/// not from the edge itself — we'll look up the label in build_route_entry.
#[cfg(feature = "graph")]
fn build_parent_map(edges: &[scryrs_types::GraphEdge]) -> HashMap<&str, &str> {
    let mut map = HashMap::new();
    for edge in edges {
        if edge.relationship == "contains" {
            map.insert(edge.target_node_id.as_str(), edge.source_node_id.as_str());
        }
    }
    map
}

/// Build a `RouteEntry` from a single graph node with optional grouping
/// from the parent lookup map.
#[cfg(feature = "graph")]
fn build_route_entry(
    node: &scryrs_types::GraphNode,
    parent_map: &HashMap<&str, &str>,
) -> RouteEntry {
    let grouping = parent_map.get(node.id.as_str()).map(|parent_id| {
        // We don't have the parent's label accessible here — we only mapped
        // target → source_id. The edge stores source_node_id, not label.
        // For the label, we need to look it up from the node list later.
        // For now, construct a placeholder that `write_route_json` will
        // enrich after we have the full node list.
        RouteGrouping {
            group_id: (*parent_id).into(),
            group_label: String::new(),
        }
    });
    let subject = raw_subject(node);

    RouteEntry {
        id: node.id.clone(),
        subject_kind: node.kind.clone(),
        subject,
        label: node.label.clone(),
        target: node.id.clone(),
        kind: node.kind.clone(),
        evidence_links: node.evidence_links.clone(),
        grouping,
        metadata: node.metadata.clone(),
    }
}

#[cfg(feature = "graph")]
fn raw_subject(node: &scryrs_types::GraphNode) -> String {
    node.id
        .split_once(':')
        .map(|(_, subject)| subject.to_string())
        .unwrap_or_else(|| node.label.clone())
}

/// Sort evidence links by the documented tie-break chain:
/// `(sourceKind, subject, docRef, description, rowIds, score)` ascending.
#[cfg(feature = "graph")]
fn sort_evidence_links(links: &mut [EvidenceLink]) {
    links.sort_by(|a, b| {
        a.source_kind
            .cmp(&b.source_kind)
            .then_with(|| a.subject.cmp(&b.subject))
            .then_with(|| a.doc_ref.cmp(&b.doc_ref))
            .then_with(|| a.description.cmp(&b.description))
            .then_with(|| a.row_ids.cmp(&b.row_ids))
            .then_with(|| a.score.cmp(&b.score))
    });
}

/// Enrich grouping labels by looking up parent node labels from the node list.
/// Must be called after build_route_entry to fill in group_label.
#[cfg(feature = "graph")]
fn enrich_group_labels(routes: &mut [RouteEntry], nodes: &[scryrs_types::GraphNode]) {
    let label_map: HashMap<&str, &str> = nodes
        .iter()
        .map(|n| (n.id.as_str(), n.label.as_str()))
        .collect();

    for entry in routes.iter_mut() {
        if let Some(ref mut g) = entry.grouping {
            if g.group_label.is_empty() {
                if let Some(label) = label_map.get(g.group_id.as_str()) {
                    g.group_label = (*label).to_string();
                }
            }
        }
    }
}

#[cfg(not(feature = "graph"))]
pub(crate) fn write_route_json(_out: &mut impl Write, err: &mut impl Write, _path: &str) -> i32 {
    let _ = writeln!(err, "scryrs route: unavailable (graph feature not enabled)");
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

#[cfg(all(test, feature = "graph"))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use scryrs_types::{
        EvidenceSourceKind, GraphEdge, GraphMetadata, GraphNode, KnowledgeGraphDocument,
    };

    fn make_test_doc(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> KnowledgeGraphDocument {
        KnowledgeGraphDocument {
            schema_version: GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes,
            edges,
        }
    }

    #[test]
    fn parent_map_from_contains_edges() {
        let edges = vec![
            GraphEdge {
                id: "e1".into(),
                source_node_id: "technical".into(),
                target_node_id: "doc_page:graph".into(),
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: vec![],
                metadata: None,
            },
            GraphEdge {
                id: "e2".into(),
                source_node_id: "docs_root".into(),
                target_node_id: "technical".into(),
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: vec![],
                metadata: None,
            },
        ];

        let map = build_parent_map(&edges);
        assert_eq!(map.get("doc_page:graph"), Some(&"technical"));
        assert_eq!(map.get("technical"), Some(&"docs_root"));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn parent_map_ignores_non_contains_edges() {
        let edges = vec![GraphEdge {
            id: "e1".into(),
            source_node_id: "file:a".into(),
            target_node_id: "file:b".into(),
            relationship: "references".into(),
            label: None,
            tags: vec![],
            evidence_links: vec![],
            metadata: None,
        }];

        let map = build_parent_map(&edges);
        assert!(map.is_empty());
    }

    #[test]
    fn sort_evidence_links_deterministic_order() {
        let mut links = vec![
            EvidenceLink {
                source_kind: EvidenceSourceKind::DocReference,
                subject: "graph".into(),
                row_ids: vec![],
                doc_ref: Some("graph".into()),
                description: None,
                score: None,
                metadata: None,
            },
            EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "src/main.rs".into(),
                row_ids: vec![5, 1],
                doc_ref: None,
                description: None,
                score: Some(10),
                metadata: None,
            },
            EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "aaa.rs".into(),
                row_ids: vec![1],
                doc_ref: None,
                description: None,
                score: Some(5),
                metadata: None,
            },
        ];

        sort_evidence_links(&mut links);

        // Sorted by sourceKind: HotspotSubject < LocalTraceRow < DocReference
        // LocalTraceRow("aaa.rs", score 5) < LocalTraceRow("src/main.rs", score 10) < DocReference
        assert_eq!(links[0].source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(links[0].subject, "aaa.rs");
        assert_eq!(links[1].source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(links[1].subject, "src/main.rs");
        assert_eq!(links[2].source_kind, EvidenceSourceKind::DocReference);
    }

    #[test]
    fn build_route_entry_from_graph_node_no_grouping() {
        let node = GraphNode {
            id: "file:src/main.rs".into(),
            label: "src/main.rs".into(),
            description: None,
            kind: "file".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::LocalTraceRow,
                subject: "src/main.rs".into(),
                row_ids: vec![1, 2],
                doc_ref: None,
                description: None,
                score: Some(10),
                metadata: None,
            }],
            metadata: None,
        };

        let parent_map = HashMap::new();
        let entry = build_route_entry(&node, &parent_map);

        assert_eq!(entry.id, "file:src/main.rs");
        assert_eq!(entry.subject_kind, "file");
        assert_eq!(entry.subject, "src/main.rs");
        assert_eq!(entry.label, "src/main.rs");
        assert_eq!(entry.target, "file:src/main.rs");
        assert_eq!(entry.kind, "file");
        assert_eq!(entry.evidence_links.len(), 1);
        assert_eq!(
            entry.evidence_links[0].source_kind,
            EvidenceSourceKind::LocalTraceRow
        );
        assert!(entry.grouping.is_none());
    }

    #[test]
    fn build_route_entry_with_grouping() {
        let node = GraphNode {
            id: "doc_page:graph".into(),
            label: "graph".into(),
            description: None,
            kind: "doc_page".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::DocReference,
                subject: "graph".into(),
                row_ids: vec![],
                doc_ref: Some("graph".into()),
                description: None,
                score: None,
                metadata: None,
            }],
            metadata: None,
        };

        let mut parent_map = HashMap::new();
        parent_map.insert("doc_page:graph", "technical");

        let entry = build_route_entry(&node, &parent_map);

        assert_eq!(entry.id, "doc_page:graph");
        assert!(entry.grouping.is_some());
        let g = entry.grouping.as_ref().unwrap();
        assert_eq!(g.group_id, "technical");
    }

    #[test]
    fn build_route_entry_uses_raw_subject_from_prefixed_node_id() {
        let node = GraphNode {
            id: "domain_term:auth".into(),
            label: "Auth".into(),
            description: None,
            kind: "domain_term".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        };

        let entry = build_route_entry(&node, &HashMap::new());

        assert_eq!(entry.subject_kind, "domain_term");
        assert_eq!(entry.subject, "auth");
        assert_eq!(entry.label, "Auth");
    }

    #[test]
    fn enrich_group_labels_fills_empty_labels() {
        let nodes = vec![GraphNode {
            id: "technical".into(),
            label: "Technical".into(),
            description: None,
            kind: "doc_group".into(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        }];

        let mut routes = vec![RouteEntry {
            id: "doc_page:graph".into(),
            subject_kind: "doc_page".into(),
            subject: "graph".into(),
            label: "graph".into(),
            target: "doc_page:graph".into(),
            kind: "doc_page".into(),
            evidence_links: vec![],
            grouping: Some(RouteGrouping {
                group_id: "technical".into(),
                group_label: String::new(),
            }),
            metadata: None,
        }];

        enrich_group_labels(&mut routes, &nodes);

        let g = routes[0].grouping.as_ref().unwrap();
        assert_eq!(g.group_label, "Technical");
    }

    #[test]
    fn full_route_manifest_pipeline() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        fs::create_dir(&scryrs_dir).expect("create .scryrs");

        // Build a test graph document.
        let graph_doc = make_test_doc(
            vec![
                GraphNode {
                    id: "file:src/main.rs".into(),
                    label: "src/main.rs".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::LocalTraceRow,
                        subject: "src/main.rs".into(),
                        row_ids: vec![1],
                        doc_ref: None,
                        description: None,
                        score: Some(10),
                        metadata: None,
                    }],
                    metadata: None,
                },
                GraphNode {
                    id: "technical".into(),
                    label: "Technical".into(),
                    description: None,
                    kind: "doc_group".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
                GraphNode {
                    id: "doc_page:graph".into(),
                    label: "graph".into(),
                    description: None,
                    kind: "doc_page".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::DocReference,
                        subject: "graph".into(),
                        row_ids: vec![],
                        doc_ref: Some("graph".into()),
                        description: None,
                        score: None,
                        metadata: None,
                    }],
                    metadata: None,
                },
            ],
            vec![GraphEdge {
                id: "e1".into(),
                source_node_id: "technical".into(),
                target_node_id: "doc_page:graph".into(),
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: vec![],
                metadata: None,
            }],
        );

        let graph_json = serde_json::to_string(&graph_doc).expect("serialize graph");
        fs::write(scryrs_dir.join("graph.json"), &graph_json).expect("write graph.json");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_route_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        let manifest: RouteManifestDocument =
            serde_json::from_str(stdout.trim()).expect("must be valid JSON");

        assert_eq!(manifest.schema_version, ROUTE_SCHEMA_VERSION);
        assert_eq!(manifest.routes.len(), 3);

        // Routes should be sorted by id ascending.
        assert_eq!(manifest.routes[0].id, "doc_page:graph");
        assert_eq!(manifest.routes[1].id, "file:src/main.rs");
        assert_eq!(manifest.routes[2].id, "technical");

        // doc_page:graph should have grouping.
        let doc_entry = &manifest.routes[0];
        assert!(doc_entry.grouping.is_some());
        assert_eq!(doc_entry.grouping.as_ref().unwrap().group_id, "technical");
        assert_eq!(
            doc_entry.grouping.as_ref().unwrap().group_label,
            "Technical"
        );

        // file:src/main.rs should NOT have grouping.
        let file_entry = &manifest.routes[1];
        assert!(file_entry.grouping.is_none());

        // Artifact was written.
        assert!(tmp.path().join(".scryrs/routes.json").exists());
    }

    #[test]
    fn accepted_grouping_reaches_route_manifest_via_graph_contains_edges_only() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        fs::create_dir(&scryrs_dir).expect("create .scryrs");
        fs::create_dir(scryrs_dir.join("accepted")).expect("create accepted dir");
        fs::write(scryrs_dir.join("accepted/bad.json"), "not-json")
            .expect("write ignored accepted artifact");

        let graph_doc = make_test_doc(
            vec![
                GraphNode {
                    id: "domain_term:auth".into(),
                    label: "Auth".into(),
                    description: None,
                    kind: "domain_term".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
                GraphNode {
                    id: "file:auth".into(),
                    label: "auth".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::LocalTraceRow,
                        subject: "auth".into(),
                        row_ids: vec![1],
                        doc_ref: None,
                        description: None,
                        score: Some(10),
                        metadata: None,
                    }],
                    metadata: None,
                },
            ],
            vec![GraphEdge {
                id: "domain_term:auth_contains_file:auth".into(),
                source_node_id: "domain_term:auth".into(),
                target_node_id: "file:auth".into(),
                relationship: "contains".into(),
                label: None,
                tags: vec![],
                evidence_links: vec![],
                metadata: None,
            }],
        );

        let graph_json = serde_json::to_string(&graph_doc).expect("serialize graph");
        fs::write(scryrs_dir.join("graph.json"), &graph_json).expect("write graph.json");

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_route_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let manifest: RouteManifestDocument =
            serde_json::from_slice(&out).expect("must be valid JSON");
        let file_entry = manifest
            .routes
            .iter()
            .find(|entry| entry.id == "file:auth")
            .expect("file route exists");
        let grouping = file_entry.grouping.as_ref().expect("grouping exists");
        assert_eq!(grouping.group_id, "domain_term:auth");
        assert_eq!(grouping.group_label, "Auth");
        assert!(String::from_utf8_lossy(&err).is_empty());
    }

    #[test]
    fn route_determinism_two_runs_produce_identical_output() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        fs::create_dir(&scryrs_dir).expect("create .scryrs");

        let graph_doc = make_test_doc(
            vec![
                GraphNode {
                    id: "file:zzz.rs".into(),
                    label: "zzz.rs".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
                GraphNode {
                    id: "file:aaa.rs".into(),
                    label: "aaa.rs".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
                GraphNode {
                    id: "search:routing".into(),
                    label: "routing".into(),
                    description: None,
                    kind: "search".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
            ],
            vec![],
        );

        let graph_json = serde_json::to_string(&graph_doc).expect("serialize graph");
        fs::write(scryrs_dir.join("graph.json"), &graph_json).expect("write graph.json");

        let mut out1 = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_route_json(&mut out1, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let mut out2 = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_route_json(&mut out2, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        assert_eq!(out1, out2, "repeated runs must be byte-identical");
    }

    #[test]
    fn route_empty_routes_on_empty_graph() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        fs::create_dir(&scryrs_dir).expect("create .scryrs");

        let graph_doc = make_test_doc(vec![], vec![]);
        let graph_json = serde_json::to_string(&graph_doc).expect("serialize graph");
        fs::write(scryrs_dir.join("graph.json"), &graph_json).expect("write graph.json");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_route_json(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let stdout = String::from_utf8_lossy(&out);
        let manifest: RouteManifestDocument =
            serde_json::from_str(stdout.trim()).expect("must be valid JSON");
        assert!(manifest.routes.is_empty());
    }
}
