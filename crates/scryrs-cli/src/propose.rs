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
    use sha2::{Digest, Sha256};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

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

    fn write_test_routes(dir: &Path, content: &str) {
        let scryrs_dir = dir.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        std::fs::write(scryrs_dir.join("routes.json"), content).expect("write routes");
    }

    fn write_test_docs(dir: &Path) {
        let docs_dir = dir.join(".devagent/docs/docs");
        std::fs::create_dir_all(&docs_dir).expect("create .devagent/docs/docs");
        std::fs::write(docs_dir.join("_nav.json"), "[]").expect("write _nav.json");
        std::fs::write(
            docs_dir.join("vision.md"),
            "# Vision\n\nProject vision document.\n",
        )
        .expect("write vision.md");
    }

    /// Returns relative file paths mapped to SHA-256 hex digests of file contents.
    fn compute_file_inventory(root: &Path) -> HashMap<PathBuf, String> {
        let mut inventory = HashMap::new();
        walk_dir(root, root, &mut inventory);
        inventory
    }

    fn walk_dir(root: &Path, dir: &Path, inventory: &mut HashMap<PathBuf, String>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(root, &path, inventory);
            } else if path.is_file() {
                let contents = std::fs::read(&path).expect("read file for inventory");
                let hash = format!("{:x}", Sha256::digest(&contents));
                let relative = path.strip_prefix(root).expect("relative path");
                inventory.insert(relative.to_path_buf(), hash);
            }
        }
    }

    /// Verification helper: runs `write_proposals` and asserts:
    /// - Every protected path is byte-for-byte identical before and after.
    /// - Any file added or modified after the run is under `.scryrs/proposals/`.
    fn verify_proposal_writes_confined(root: &Path, protected_paths: &[&str]) {
        // (a) Snapshot byte-for-byte content of every protected path.
        let mut protected_snapshots: HashMap<&str, Vec<u8>> = HashMap::new();
        for pp in protected_paths {
            protected_snapshots.insert(pp, snapshot_dir_or_file(&root.join(pp)));
        }

        // (b) Compute full file inventory before write_proposals.
        let inventory_before = compute_file_inventory(root);

        // (c) Run write_proposals.
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            write_proposals(&mut out, &mut err, root.to_str().unwrap()),
            0
        );

        // (d) Assert byte-for-byte identity of every protected path.
        for pp in protected_paths {
            let after = snapshot_dir_or_file(&root.join(pp));
            let before = protected_snapshots
                .get(pp)
                .expect("protected snapshot must exist");
            assert_eq!(before, &after, "protected path must not be modified: {pp}");
        }

        // (e) Compute full file inventory after write_proposals.
        let inventory_after = compute_file_inventory(root);

        // (f) Assert that any file added or modified is under .scryrs/proposals/.
        for (path, hash_after) in &inventory_after {
            let path_str = path.to_string_lossy();
            if path_str.starts_with(".scryrs/proposals/") || path_str == ".scryrs/proposals" {
                continue;
            }
            match inventory_before.get(path) {
                Some(hash_before) => {
                    assert_eq!(
                        hash_before, hash_after,
                        "file outside .scryrs/proposals/ must not be modified: {path_str}"
                    );
                }
                None => {
                    panic!("new file created outside .scryrs/proposals/: {path_str}");
                }
            }
        }

        // No files should be deleted either (non-proposals files still present).
        for path in inventory_before.keys() {
            assert!(
                inventory_after.contains_key(path),
                "file must not be deleted: {}",
                path.display()
            );
        }
    }

    /// Recursively read a file or directory into a single byte buffer for
    /// deterministic comparison. Directory snapshots concatenate relative
    /// file paths and their contents in sorted order.
    fn snapshot_dir_or_file(path: &Path) -> Vec<u8> {
        if path.is_file() {
            return std::fs::read(path).expect("read protected file");
        }
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
                .expect("read protected dir")
                .flatten()
                .map(|e| e.path())
                .collect();
            entries.sort();
            let mut buf = Vec::new();
            for entry in entries {
                let relative = entry.strip_prefix(path).expect("relative path");
                buf.extend_from_slice(relative.to_string_lossy().as_bytes());
                buf.push(b'\n');
                let contents = snapshot_dir_or_file(&entry);
                buf.extend_from_slice(&contents);
            }
            return buf;
        }
        // If the path doesn't exist, snapshot is empty.
        Vec::new()
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

        // Seed input artifacts.
        let entries = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        write_test_hotspots(tmp.path(), &entries, "2026-06-27T12:00:00Z");
        let graph = make_graph_doc(vec![], vec![]);
        write_test_graph(tmp.path(), &graph);

        // Seed protected source-of-truth artifacts.
        write_test_routes(tmp.path(), "{}");
        write_test_docs(tmp.path());

        // Verify: no protected path is mutated, and all writes are confined.
        verify_proposal_writes_confined(
            tmp.path(),
            &[
                ".scryrs/graph.json",
                ".scryrs/routes.json",
                ".devagent/docs/",
            ],
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
