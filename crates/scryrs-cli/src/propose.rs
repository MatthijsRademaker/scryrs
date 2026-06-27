use std::io::Write;

#[cfg(feature = "curator")]
use scryrs_types::KnowledgeGraphDocument;

#[cfg(feature = "curator")]
pub(crate) fn write_proposals(out: &mut impl Write, err: &mut impl Write, path: &str) -> i32 {
    // Resolve to absolute repo root.
    let repo_root = match std::path::absolute(path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(err, "scryrs propose: cannot resolve path '{path}': {e}");
            return 2;
        }
    };

    // Load hotspots.json (required) — full report to extract generatedAt.
    let hotspots_path = repo_root.join(".scryrs/hotspots.json");
    let hotspots_json = match std::fs::read_to_string(&hotspots_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(
                err,
                "scryrs propose: hotspots artifact not found at {}",
                hotspots_path.display()
            );
            return 2;
        }
    };
    let hotspots_report: HotspotsReportPartial = match serde_json::from_str(&hotspots_json) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(err, "scryrs propose: malformed hotspots file: {e}");
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
                "scryrs propose: graph artifact not found at {}",
                graph_path.display()
            );
            return 2;
        }
    };
    let graph_doc: KnowledgeGraphDocument = match serde_json::from_str(&graph_json) {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs propose: malformed graph file: {e}");
            return 2;
        }
    };

    // Generate proposals via the curator engine.
    let proposals = scryrs_curator::generate_proposals(
        &graph_doc,
        &hotspots_report.entries,
        &hotspots_report.generated_at,
    );

    // Validate every proposal.
    for p in &proposals {
        if let Err(e) = p.validate() {
            let _ = writeln!(err, "scryrs propose: invalid proposal: {e}");
            return 1;
        }
    }

    // Ensure proposals directory exists.
    let proposals_dir = repo_root.join(".scryrs/proposals");
    if let Err(e) = std::fs::create_dir_all(&proposals_dir) {
        let _ = writeln!(
            err,
            "scryrs propose: cannot create proposals directory: {e}"
        );
        return 1;
    }

    // Write each proposal as .scryrs/proposals/{id}.json.
    let count = proposals.len();
    for p in &proposals {
        let filename = p.inbox_filename();
        let file_path = proposals_dir.join(&filename);
        let json = match serde_json::to_string(p) {
            Ok(j) => j,
            Err(e) => {
                let _ = writeln!(err, "scryrs propose: serialization error: {e}");
                return 1;
            }
        };
        if let Err(e) = std::fs::write(&file_path, &json) {
            let _ = writeln!(
                err,
                "scryrs propose: cannot write proposal {}: {e}",
                file_path.display()
            );
            return 1;
        }
    }

    // Report count to stdout (single-line JSON summary, graph/route pattern).
    let _ = writeln!(out, "{}", count);
    0
}

/// Partial deserialization target for hotspots.json — captures `generatedAt`
/// and the `entries` array.
#[cfg(feature = "curator")]
#[derive(Debug, serde::Deserialize)]
struct HotspotsReportPartial {
    #[serde(rename = "generatedAt")]
    generated_at: String,
    entries: Vec<scryrs_types::HotspotEntry>,
}

#[cfg(not(feature = "curator"))]
pub(crate) fn write_proposals(_out: &mut impl Write, err: &mut impl Write, _path: &str) -> i32 {
    let _ = writeln!(
        err,
        "scryrs propose: unavailable (curator feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

#[cfg(all(test, feature = "curator"))]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use scryrs_types::{
        EvidenceLink, EvidenceSourceKind, GraphEdge, GraphMetadata, GraphNode, HotspotCounts,
        HotspotEntry, HotspotEvidence, KnowledgeGraphDocument, ProposalDocument,
        ProposalTargetType,
    };
    use std::collections::HashMap;
    use std::path::Path;

    fn make_graph_doc(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> KnowledgeGraphDocument {
        KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes,
            edges,
        }
    }

    fn write_test_hotspots(dir: &Path, entries: &[HotspotEntry], generated_at: &str) {
        let scryrs_dir = dir.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        let report = serde_json::json!({
            "generatedAt": generated_at,
            "entries": entries,
        });
        std::fs::write(
            scryrs_dir.join("hotspots.json"),
            serde_json::to_string(&report).expect("serialize"),
        )
        .expect("write hotspots");
    }

    fn write_test_graph(dir: &Path, doc: &KnowledgeGraphDocument) {
        let scryrs_dir = dir.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        std::fs::write(
            scryrs_dir.join("graph.json"),
            serde_json::to_string(doc).expect("serialize"),
        )
        .expect("write graph");
    }

    fn make_hotspot(
        subject_kind: &str,
        subject: &str,
        score: u32,
        rank: u32,
        row_ids: Vec<u64>,
        outcome_failure: u32,
        outcome_success: u32,
    ) -> HotspotEntry {
        let mut outcome = HashMap::new();
        if outcome_success > 0 {
            outcome.insert("success".to_string(), outcome_success);
        }
        if outcome_failure > 0 {
            outcome.insert("failure".to_string(), outcome_failure);
        }
        HotspotEntry {
            rank,
            subjectKind: subject_kind.to_string(),
            subject: subject.to_string(),
            score,
            counts: HotspotCounts {
                eventType: HashMap::new(),
                outcome,
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: row_ids },
        }
    }

    // --- Integration tests ---

    #[test]
    fn valid_inputs_produce_proposal_files() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![
            make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5),
            make_hotspot("search", "routing", 5, 2, vec![2], 0, 3),
        ];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let proposals_dir = tmp.path().join(".scryrs/proposals");
        assert!(proposals_dir.is_dir());

        // Verify files were written.
        let files: Vec<_> = std::fs::read_dir(&proposals_dir)
            .expect("read proposals dir")
            .filter_map(|e| e.ok())
            .collect();
        assert!(!files.is_empty(), "proposal files must be written");

        // Each file should deserialize as a ProposalDocument and validate.
        for entry in files {
            let content = std::fs::read_to_string(entry.path()).expect("read proposal");
            let doc: ProposalDocument =
                serde_json::from_str(&content).expect("valid ProposalDocument");
            assert!(doc.validate().is_ok(), "proposal must validate");
        }
    }

    #[test]
    fn missing_hotspots_exits_2() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            2
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("hotspots artifact not found"),
            "got: {err_str}"
        );

        // No proposals directory created.
        assert!(!tmp.path().join(".scryrs/proposals").exists());
    }

    #[test]
    fn malformed_hotspots_exits_2() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let scryrs_dir = tmp.path().join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        std::fs::write(scryrs_dir.join("hotspots.json"), "not json").expect("write");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            2
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("malformed hotspots file"),
            "got: {err_str}"
        );
        assert!(!tmp.path().join(".scryrs/proposals").exists());
    }

    #[test]
    fn missing_graph_exits_2() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("graph artifact not found"),
            "got: {err_str}"
        );
    }

    #[test]
    fn malformed_graph_exits_2() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let scryrs_dir = tmp.path().join(".scryrs");
        std::fs::write(scryrs_dir.join("graph.json"), "not json").expect("write");

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("malformed graph file"), "got: {err_str}");
    }

    #[test]
    fn deterministic_rerun_same_inputs_same_output() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![
            make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5),
            make_hotspot("search", "routing", 5, 2, vec![2], 0, 3),
        ];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        // First run.
        let mut out1 = Vec::new();
        let mut err1 = Vec::new();
        assert_eq!(
            write_proposals(&mut out1, &mut err1, tmp.path().to_str().unwrap()),
            0
        );

        // Second run.
        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        assert_eq!(
            write_proposals(&mut out2, &mut err2, tmp.path().to_str().unwrap()),
            0
        );

        // Same file set.
        let files1: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        let files2: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(files1, files2, "same inputs produce identical filenames");
    }

    #[test]
    fn source_of_truth_not_mutated() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        // Snapshot original content.
        let graph_original =
            std::fs::read_to_string(tmp.path().join(".scryrs/graph.json")).expect("read graph");
        let hotspots_original = std::fs::read_to_string(tmp.path().join(".scryrs/hotspots.json"))
            .expect("read hotspots");

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        // Verify source files unchanged.
        let graph_after =
            std::fs::read_to_string(tmp.path().join(".scryrs/graph.json")).expect("read graph");
        let hotspots_after = std::fs::read_to_string(tmp.path().join(".scryrs/hotspots.json"))
            .expect("read hotspots");

        assert_eq!(
            graph_original, graph_after,
            ".scryrs/graph.json must not be modified"
        );
        assert_eq!(
            hotspots_original, hotspots_after,
            ".scryrs/hotspots.json must not be modified"
        );
    }

    #[test]
    fn upsert_behavior_same_inputs_overwrites() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        let files_before: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        // Run again with same inputs.
        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        assert_eq!(
            write_proposals(&mut out2, &mut err2, tmp.path().to_str().unwrap()),
            0
        );

        let files_after: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        assert_eq!(
            files_before, files_after,
            "same inputs overwrite same files"
        );
    }

    #[test]
    fn upsert_different_inputs_adds_not_removes() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");

        // First run: one entry.
        let entries1 = vec![make_hotspot("file", "src/a.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries1, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );
        let files_first: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        // Second run: different entry.
        let entries2 = vec![make_hotspot("file", "src/b.rs", 10, 1, vec![2], 0, 5)];
        write_test_hotspots(tmp.path(), &entries2, "2026-06-27T12:00:00Z");

        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        assert_eq!(
            write_proposals(&mut out2, &mut err2, tmp.path().to_str().unwrap()),
            0
        );
        let files_second: Vec<String> = std::fs::read_dir(tmp.path().join(".scryrs/proposals"))
            .expect("read dir")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();

        // Check that old files are still present (upsert, not clean).
        for f in &files_first {
            assert!(
                files_second.contains(f),
                "old proposal file {} must persist (upsert-only)",
                f
            );
        }
    }

    #[test]
    fn semantic_grouping_proposal_carries_correct_fields() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let entries = vec![make_hotspot("file", "auth", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");

        let graph = make_graph_doc(
            vec![
                GraphNode {
                    id: "file:auth".into(),
                    label: "auth".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::HotspotSubject,
                        subject: "auth".into(),
                        row_ids: vec![1],
                        doc_ref: None,
                        description: None,
                        score: Some(10),
                        metadata: None,
                    }],
                    metadata: None,
                },
                GraphNode {
                    id: "search:auth".into(),
                    label: "auth".into(),
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
        write_test_graph(tmp.path(), &graph);

        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_proposals(&mut out, &mut err, tmp.path().to_str().unwrap()),
            0
        );

        // Find the grouping proposal among written files.
        let proposals_dir = tmp.path().join(".scryrs/proposals");
        let mut found = false;
        for entry in std::fs::read_dir(&proposals_dir)
            .expect("read dir")
            .flatten()
        {
            let content = std::fs::read_to_string(entry.path()).expect("read proposal");
            let doc: ProposalDocument =
                serde_json::from_str(&content).expect("valid ProposalDocument");
            if doc.target_type == ProposalTargetType::SemanticGraphGrouping {
                found = true;
                match &doc.proposed_content {
                    scryrs_types::ProposedContent::SemanticGraphGrouping(sgg) => {
                        assert_eq!(sgg.source_node_ids.len(), 2);
                        assert!(sgg.source_node_ids.contains(&"file:auth".to_string()));
                        assert!(sgg.source_node_ids.contains(&"search:auth".to_string()));
                        assert_eq!(sgg.target_group_node_id, "domain_term:auth");
                        assert_eq!(sgg.target_group_label, "auth");
                    }
                    other => panic!("expected SemanticGraphGrouping, got: {:?}", other),
                }
            }
        }
        assert!(found, "semantic_graph_grouping proposal must exist");
    }


}
