//! Reviewable knowledge proposal engine.
//!
//! Generates deterministic `ProposalDocument` candidates from hotspot
//! and graph evidence. Applies concrete heuristics for five target types:
//! `docs_note`, `skill`, `memory_patch`, `adr`, and
//! `semantic_graph_grouping`. `debugging_playbook` is intentionally
//! excluded from V1 generation.

use std::collections::HashMap;

use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, HotspotEntry, KnowledgeGraphDocument,
    PROPOSAL_SCHEMA_VERSION, ProposalDocument, ProposalTargetType, ProposedContent,
    SemanticGraphGrouping,
};
use serde_json::json;

pub fn descriptor() -> scryrs_types::FeatureDescriptor {
    scryrs_types::FeatureDescriptor {
        id: "curator",
        title: "scryrs-curator",
        summary: "reviewable docs, skills, decisions, memory, and grouping proposal foundation",
    }
}

/// Generate proposal documents from hotspot entries and a knowledge graph.
///
/// All proposals carry `created_at` set to `generated_at` for deterministic
/// output. Returns zero or more validated `ProposalDocument` instances;
/// callers should validate each via `ProposalDocument::validate()` before
/// persisting.
pub fn generate_proposals(
    graph: &KnowledgeGraphDocument,
    hotspots: &[HotspotEntry],
    generated_at: &str,
) -> Vec<ProposalDocument> {
    let mut proposals: Vec<ProposalDocument> = Vec::new();

    // docs_note: always generated for every hotspot entry.
    for entry in hotspots {
        if let Some(p) = docs_note_proposal(entry, generated_at) {
            proposals.push(p);
        }
    }

    // skill: entries with at least one Failure outcome event.
    for entry in hotspots {
        if let Some(p) = skill_proposal(entry, generated_at) {
            proposals.push(p);
        }
    }

    // memory_patch: entries with failure-ratio >= 0.5 and score >= 4.
    for entry in hotspots {
        if let Some(p) = memory_patch_proposal(entry, generated_at) {
            proposals.push(p);
        }
    }

    // adr: subject clusters across >=2 distinct subjectKinds with aggregate score >= 10.
    proposals.extend(adr_proposals(hotspots, generated_at));

    // semantic_graph_grouping: cross-kind graph node families with shared hotspot evidence.
    proposals.extend(semantic_grouping_proposals(graph, hotspots, generated_at));

    proposals
}

// ---------------------------------------------------------------------------
// docs_note rule
// ---------------------------------------------------------------------------

fn docs_note_proposal(entry: &HotspotEntry, generated_at: &str) -> Option<ProposalDocument> {
    let title = format!("Document {}", entry.subject);
    let rationale = format!(
        "Hotspot entry [{}:{}] appears with score {} at rank {}",
        entry.subjectKind, entry.subject, entry.score, entry.rank
    );
    let markdown = format!(
        "# {}\n\n**Subject**: {} ({})\n**Score**: {}\n**Rank**: {}\n",
        entry.subject, entry.subject, entry.subjectKind, entry.score, entry.rank
    );
    let content = ProposedContent::Markdown(markdown);
    let id = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
        .unwrap_or_else(|e| panic!("compute_id: {e}"));

    Some(ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type: ProposalTargetType::DocsNote,
        title,
        rationale,
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: entry.subject.clone(),
            row_ids: entry.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(entry.score),
            metadata: None,
        }],
        created_at: generated_at.into(),
    })
}

// ---------------------------------------------------------------------------
// skill rule
// ---------------------------------------------------------------------------

fn skill_proposal(entry: &HotspotEntry, generated_at: &str) -> Option<ProposalDocument> {
    let failure_count = entry.counts.outcome.get("failure").copied().unwrap_or(0);

    if failure_count == 0 {
        return None;
    }

    let title = format!("Skill for {}", entry.subject);
    let rationale = format!(
        "Hotspot entry [{}:{}] has {} failure outcome(s) with score {}",
        entry.subjectKind, entry.subject, failure_count, entry.score
    );
    let markdown = format!(
        "# Skill: {}\n\n**Subject Kind**: {}\n**Failure Count**: {}\n**Score**: {}\n",
        entry.subject, entry.subjectKind, failure_count, entry.score
    );
    let content = ProposedContent::Markdown(markdown);
    let id = ProposalDocument::compute_id(&ProposalTargetType::Skill, &content)
        .unwrap_or_else(|e| panic!("compute_id: {e}"));

    Some(ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type: ProposalTargetType::Skill,
        title,
        rationale,
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: entry.subject.clone(),
            row_ids: entry.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(entry.score),
            metadata: None,
        }],
        created_at: generated_at.into(),
    })
}

// ---------------------------------------------------------------------------
// memory_patch rule
// ---------------------------------------------------------------------------

fn memory_patch_proposal(entry: &HotspotEntry, generated_at: &str) -> Option<ProposalDocument> {
    if entry.score < 4 {
        return None;
    }

    let total_outcome: u32 = entry.counts.outcome.values().sum();
    let failure_count: u32 = entry.counts.outcome.get("failure").copied().unwrap_or(0);

    if total_outcome == 0 || (failure_count * 2) < total_outcome {
        return None;
    }

    let title = format!("Memory patch for {}", entry.subject);
    let rationale = format!(
        "Hotspot entry [{}:{}] has failure-ratio {}/{} >= 0.5 with score {}",
        entry.subjectKind, entry.subject, failure_count, total_outcome, entry.score
    );

    let structured = json!({
        "subject": entry.subject,
        "subjectKind": entry.subjectKind,
        "score": entry.score,
        "failureCount": failure_count,
        "totalOutcomeCount": total_outcome,
        "failureRatio": if total_outcome > 0 {
            failure_count as f64 / total_outcome as f64
        } else {
            0.0
        },
    });
    let content = ProposedContent::MemoryPatch(structured);
    let id = ProposalDocument::compute_id(&ProposalTargetType::MemoryPatch, &content)
        .unwrap_or_else(|e| panic!("compute_id: {e}"));

    Some(ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type: ProposalTargetType::MemoryPatch,
        title,
        rationale,
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: entry.subject.clone(),
            row_ids: entry.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(entry.score),
            metadata: None,
        }],
        created_at: generated_at.into(),
    })
}

// ---------------------------------------------------------------------------
// adr rule
// ---------------------------------------------------------------------------

fn adr_proposals(hotspots: &[HotspotEntry], generated_at: &str) -> Vec<ProposalDocument> {
    // Group hotspot entries by subject string, collecting (subjectKind, score, entry_index).
    let mut subject_map: HashMap<&str, Vec<(&str, u32, usize)>> = HashMap::new();
    for (i, entry) in hotspots.iter().enumerate() {
        subject_map
            .entry(entry.subject.as_str())
            .or_default()
            .push((entry.subjectKind.as_str(), entry.score, i));
    }

    let mut proposals: Vec<ProposalDocument> = Vec::new();

    for (subject, items) in subject_map {
        // Count distinct subject kinds.
        let mut kinds: Vec<&str> = items.iter().map(|(k, _, _)| *k).collect();
        kinds.sort_unstable();
        kinds.dedup();

        if kinds.len() < 2 {
            continue;
        }

        let aggregate_score: u32 = items.iter().map(|(_, s, _)| s).sum();
        if aggregate_score < 10 {
            continue;
        }

        let title = format!("ADR: {}", subject);
        let rationale = format!(
            "Subject '{}' appears in {} distinct subject kinds ({:?}) with aggregate score {}",
            subject,
            kinds.len(),
            kinds,
            aggregate_score
        );
        let markdown = format!(
            "# ADR Proposal: {}\n\n**Subject Kinds**: {:?}\n**Aggregate Score**: {}\n**Contributing Entries**: {}\n",
            subject,
            kinds,
            aggregate_score,
            items.len()
        );
        let content = ProposedContent::Markdown(markdown);
        let id = ProposalDocument::compute_id(&ProposalTargetType::Adr, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));

        // Collect evidence from all contributing entries.
        let evidence: Vec<EvidenceLink> = items
            .iter()
            .map(|(_, _, idx)| {
                let entry = &hotspots[*idx];
                EvidenceLink {
                    source_kind: EvidenceSourceKind::HotspotSubject,
                    subject: entry.subject.clone(),
                    row_ids: entry.evidence.rowIds.clone(),
                    doc_ref: None,
                    description: None,
                    score: Some(entry.score),
                    metadata: None,
                }
            })
            .collect();

        proposals.push(ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type: ProposalTargetType::Adr,
            title,
            rationale,
            proposed_content: content,
            evidence,
            created_at: generated_at.into(),
        });
    }

    proposals
}

// ---------------------------------------------------------------------------
// semantic_graph_grouping rule
// ---------------------------------------------------------------------------

fn semantic_grouping_proposals(
    graph: &KnowledgeGraphDocument,
    hotspots: &[HotspotEntry],
    generated_at: &str,
) -> Vec<ProposalDocument> {
    // Build a set of hotspot subjects for evidence matching.
    let hotspot_subjects: std::collections::HashSet<&str> =
        hotspots.iter().map(|e| e.subject.as_str()).collect();

    // Group graph nodes by subject stem (the part after `kind:` in the node id).
    // Node IDs are in `{subjectKind}:{subject}` format.
    let mut stem_map: HashMap<&str, Vec<&scryrs_types::GraphNode>> = HashMap::new();
    for node in &graph.nodes {
        // Extract the subject part (after the first colon).
        if let Some(colon_pos) = node.id.find(':') {
            let stem = &node.id[colon_pos + 1..];
            stem_map.entry(stem).or_default().push(node);
        }
    }

    let mut proposals: Vec<ProposalDocument> = Vec::new();

    for (stem, nodes) in stem_map {
        // Need >=2 distinct subject kinds.
        let mut kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
        kinds.sort_unstable();
        kinds.dedup();

        if kinds.len() < 2 {
            continue;
        }

        // At least one node in the family must have hotspot-backed evidence
        // (evidence link with HotspotSubject source kind and a subject matching
        // a hotspot entry).
        let has_hotspot_evidence = nodes.iter().any(|node| {
            node.evidence_links.iter().any(|link| {
                link.source_kind == EvidenceSourceKind::HotspotSubject
                    && hotspot_subjects.contains(link.subject.as_str())
            })
        });

        if !has_hotspot_evidence {
            continue;
        }

        let source_node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
        let target_group_node_id = format!("domain_term:{stem}");
        let target_group_label = stem.to_string();

        let title = format!("Grouping: {}", stem);
        let rationale = format!(
            "{} nodes across {} subject kinds ({:?}) share subject stem '{}' with hotspot evidence",
            source_node_ids.len(),
            kinds.len(),
            kinds,
            stem
        );

        let grouping = SemanticGraphGrouping {
            source_node_ids: source_node_ids.clone(),
            target_group_node_id: target_group_node_id.clone(),
            target_group_label: target_group_label.clone(),
        };
        let content = ProposedContent::SemanticGraphGrouping(grouping);
        let id = ProposalDocument::compute_id(&ProposalTargetType::SemanticGraphGrouping, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));

        // Gather evidence from hotspot-backed links across all family nodes.
        let mut evidence: Vec<EvidenceLink> = Vec::new();
        for node in &nodes {
            for link in &node.evidence_links {
                if link.source_kind == EvidenceSourceKind::HotspotSubject
                    && hotspot_subjects.contains(link.subject.as_str())
                {
                    evidence.push(link.clone());
                }
            }
        }

        proposals.push(ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type: ProposalTargetType::SemanticGraphGrouping,
            title,
            rationale,
            proposed_content: content,
            evidence,
            created_at: generated_at.into(),
        });
    }

    proposals
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::{
        GraphMetadata, GraphNode, HotspotCounts, HotspotEvidence, ProposalTargetType,
    };
    use std::collections::HashMap;

    const TEST_GENERATED_AT: &str = "2026-06-27T12:00:00Z";

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

    fn make_empty_graph() -> KnowledgeGraphDocument {
        KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![],
            edges: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // docs_note tests
    // -----------------------------------------------------------------------

    #[test]
    fn docs_note_generates_one_per_hotspot() {
        let hotspots = vec![
            make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5),
            make_hotspot("search", "routing", 5, 2, vec![2], 0, 3),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let docs: Vec<&ProposalDocument> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::DocsNote)
            .collect();
        assert_eq!(docs.len(), 2, "each hotspot entry yields one docs_note");

        for p in &docs {
            assert!(!p.title.is_empty());
            assert!(!p.rationale.is_empty());
            assert!(!p.evidence.is_empty());
            assert_eq!(
                p.evidence[0].source_kind,
                EvidenceSourceKind::HotspotSubject
            );
            assert_eq!(p.created_at, TEST_GENERATED_AT);
        }
    }

    #[test]
    fn docs_note_empty_hotspots_yields_none() {
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &[], TEST_GENERATED_AT);
        let docs: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::DocsNote)
            .collect();
        assert!(docs.is_empty());
    }

    #[test]
    fn docs_note_cites_hotspot_evidence() {
        let hotspots = vec![make_hotspot(
            "symbol",
            "MyStruct",
            42,
            1,
            vec![5, 12, 23],
            0,
            3,
        )];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let p = match proposals
            .iter()
            .find(|p| p.target_type == ProposalTargetType::DocsNote)
        {
            Some(p) => p,
            None => panic!("docs_note must exist"),
        };

        assert_eq!(p.evidence.len(), 1);
        assert_eq!(p.evidence[0].subject, "MyStruct");
        assert_eq!(p.evidence[0].row_ids, vec![5, 12, 23]);
        assert_eq!(p.evidence[0].score, Some(42));
    }

    // -----------------------------------------------------------------------
    // skill tests
    // -----------------------------------------------------------------------

    #[test]
    fn skill_only_for_failure_entries() {
        let hotspots = vec![
            make_hotspot("file", "src/a.rs", 5, 1, vec![1], 3, 2), // has failures
            make_hotspot("file", "src/b.rs", 5, 2, vec![2], 0, 10), // no failures
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let skills: Vec<&ProposalDocument> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Skill)
            .collect();
        assert_eq!(skills.len(), 1, "only entry with failures gets skill");
        assert_eq!(skills[0].evidence[0].subject, "src/a.rs");
    }

    #[test]
    fn skill_empty_hotspots_yields_none() {
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &[], TEST_GENERATED_AT);
        let skills: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Skill)
            .collect();
        assert!(skills.is_empty());
    }

    #[test]
    fn skill_has_non_empty_rationale_and_evidence() {
        let hotspots = vec![make_hotspot(
            "command",
            "cargo build",
            8,
            1,
            vec![10],
            5,
            10,
        )];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let p = match proposals
            .iter()
            .find(|p| p.target_type == ProposalTargetType::Skill)
        {
            Some(p) => p,
            None => panic!("skill must exist"),
        };

        assert!(!p.rationale.is_empty());
        assert!(!p.evidence.is_empty());
        assert_eq!(p.target_type, ProposalTargetType::Skill);
    }

    // -----------------------------------------------------------------------
    // memory_patch tests
    // -----------------------------------------------------------------------

    #[test]
    fn memory_patch_threshold_behavior() {
        let hotspots = vec![
            // score >= 4, failure-ratio 3/5 = 0.6 >= 0.5 -> YES
            make_hotspot("file", "high_failure.rs", 5, 1, vec![1], 3, 2),
            // score >= 4, failure-ratio 1/5 = 0.2 < 0.5 -> NO
            make_hotspot("file", "low_failure.rs", 5, 2, vec![2], 1, 4),
            // score < 4, failure-ratio 5/5 = 1.0 >= 0.5 -> NO (score too low)
            make_hotspot("file", "low_score.rs", 3, 3, vec![3], 5, 0),
            // score >= 4, no outcomes at all -> NO
            make_hotspot("file", "no_events.rs", 6, 4, vec![4], 0, 0),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let patches: Vec<&ProposalDocument> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::MemoryPatch)
            .collect();
        assert_eq!(patches.len(), 1, "only high_failure.rs qualifies");
        assert_eq!(patches[0].evidence[0].subject, "high_failure.rs");
    }

    #[test]
    fn memory_patch_uses_structured_json_content() {
        let hotspots = vec![make_hotspot("file", "buggy.rs", 7, 1, vec![1], 4, 2)];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let p = match proposals
            .iter()
            .find(|p| p.target_type == ProposalTargetType::MemoryPatch)
        {
            Some(p) => p,
            None => panic!("memory_patch must exist"),
        };

        match &p.proposed_content {
            ProposedContent::MemoryPatch(v) => {
                assert!(v.is_object());
                assert_eq!(v["subject"], "buggy.rs");
                assert_eq!(v["score"], 7);
                assert_eq!(v["failureCount"], 4);
                assert_eq!(v["totalOutcomeCount"], 6);
            }
            other => panic!("expected MemoryPatch content, got: {:?}", other),
        }
    }

    #[test]
    fn memory_patch_empty_hotspots_yields_none() {
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &[], TEST_GENERATED_AT);
        let patches: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::MemoryPatch)
            .collect();
        assert!(patches.is_empty());
    }

    // -----------------------------------------------------------------------
    // adr tests
    // -----------------------------------------------------------------------

    #[test]
    fn adr_cross_kind_cluster() {
        let hotspots = vec![
            make_hotspot("file", "routing", 6, 1, vec![1], 0, 5),
            make_hotspot("search", "routing", 6, 2, vec![2], 0, 3),
            make_hotspot("symbol", "routing", 0, 3, vec![3], 0, 1),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let adrs: Vec<&ProposalDocument> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Adr)
            .collect();
        assert_eq!(
            adrs.len(),
            1,
            "routing appears in 3 kinds with score 12 >= 10"
        );
        assert_eq!(
            adrs[0].evidence.len(),
            3,
            "all 3 entries contribute evidence"
        );
    }

    #[test]
    fn adr_single_kind_no_cluster() {
        let hotspots = vec![
            make_hotspot("file", "config", 6, 1, vec![1], 0, 5),
            make_hotspot("file", "config", 6, 2, vec![2], 0, 3),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let adrs: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Adr)
            .collect();
        assert!(adrs.is_empty(), "single kind does not trigger ADR");
    }

    #[test]
    fn adr_aggregate_score_below_10() {
        let hotspots = vec![
            make_hotspot("file", "logging", 4, 1, vec![1], 0, 3),
            make_hotspot("search", "logging", 4, 2, vec![2], 0, 2),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let adrs: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Adr)
            .collect();
        assert!(adrs.is_empty(), "aggregate score 8 < 10, no ADR");
    }

    #[test]
    fn adr_empty_hotspots_yields_none() {
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &[], TEST_GENERATED_AT);
        let adrs: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::Adr)
            .collect();
        assert!(adrs.is_empty());
    }

    // -----------------------------------------------------------------------
    // semantic_graph_grouping tests
    // -----------------------------------------------------------------------

    fn make_test_graph() -> KnowledgeGraphDocument {
        KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![
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
                        score: Some(5),
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
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::HotspotSubject,
                        subject: "auth".into(),
                        row_ids: vec![2],
                        doc_ref: None,
                        description: None,
                        score: Some(3),
                        metadata: None,
                    }],
                    metadata: None,
                },
                GraphNode {
                    id: "symbol:auth".into(),
                    label: "auth".into(),
                    description: None,
                    kind: "symbol".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
                // Unrelated node.
                GraphNode {
                    id: "file:other".into(),
                    label: "other".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
            ],
            edges: vec![],
        }
    }

    #[test]
    fn semantic_grouping_for_cross_kind_node_family() {
        let hotspots = vec![make_hotspot("file", "auth", 5, 1, vec![1], 0, 5)];
        let graph = make_test_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let groupings: Vec<&ProposalDocument> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::SemanticGraphGrouping)
            .collect();
        assert_eq!(
            groupings.len(),
            1,
            "auth appears in 3 kinds with hotspot evidence"
        );

        let g = &groupings[0];
        match &g.proposed_content {
            ProposedContent::SemanticGraphGrouping(sgg) => {
                assert_eq!(sgg.source_node_ids.len(), 3);
                assert!(sgg.source_node_ids.contains(&"file:auth".to_string()));
                assert!(sgg.source_node_ids.contains(&"search:auth".to_string()));
                assert!(sgg.source_node_ids.contains(&"symbol:auth".to_string()));
                assert_eq!(sgg.target_group_node_id, "domain_term:auth");
                assert_eq!(sgg.target_group_label, "auth");
            }
            other => panic!("expected SemanticGraphGrouping, got: {:?}", other),
        }
    }

    #[test]
    fn semantic_grouping_requires_hotspot_evidence() {
        // Graph with cross-kind nodes but NO hotspot evidence links with matching subjects.
        let graph = KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![
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
                        score: Some(5),
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
            edges: vec![],
        };

        let hotspots = vec![make_hotspot("file", "auth", 5, 1, vec![1], 0, 5)];
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let groupings: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::SemanticGraphGrouping)
            .collect();
        assert!(
            groupings.is_empty(),
            "no grouping when nodes lack hotspot-backed evidence links"
        );
    }

    #[test]
    fn semantic_grouping_single_kind_no_grouping() {
        let graph = KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![
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
                        score: Some(5),
                        metadata: None,
                    }],
                    metadata: None,
                },
                GraphNode {
                    id: "file:auth_v2".into(),
                    label: "auth_v2".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![],
                    metadata: None,
                },
            ],
            edges: vec![],
        };

        let hotspots = vec![make_hotspot("file", "auth", 5, 1, vec![1], 0, 5)];
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let groupings: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::SemanticGraphGrouping)
            .collect();
        assert!(
            groupings.is_empty(),
            "same kind nodes do not trigger grouping"
        );
    }

    #[test]
    fn semantic_grouping_empty_graph_yields_none() {
        let graph = make_empty_graph();
        let hotspots = vec![make_hotspot("file", "auth", 5, 1, vec![1], 0, 5)];
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let groupings: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::SemanticGraphGrouping)
            .collect();
        assert!(groupings.is_empty());
    }

    // -----------------------------------------------------------------------
    // Cross-cutting tests
    // -----------------------------------------------------------------------

    #[test]
    fn all_proposals_have_created_at_set() {
        let hotspots = vec![
            make_hotspot("file", "config", 4, 1, vec![1], 3, 2),
            make_hotspot("search", "config", 7, 2, vec![2], 0, 5),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, "2026-06-27T12:00:00Z");

        for p in &proposals {
            assert_eq!(
                p.created_at, "2026-06-27T12:00:00Z",
                "all proposals must use generated_at timestamp"
            );
        }
    }

    #[test]
    fn no_debugging_playbook_proposals() {
        let hotspots = vec![make_hotspot("file", "bug.rs", 10, 1, vec![1], 10, 0)];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        let playbooks: Vec<_> = proposals
            .iter()
            .filter(|p| p.target_type == ProposalTargetType::DebuggingPlaybook)
            .collect();
        assert!(
            playbooks.is_empty(),
            "debugging_playbook must never be generated"
        );
    }

    #[test]
    fn all_proposals_have_deterministic_64_char_hex_id() {
        let hotspots = vec![
            make_hotspot("file", "src/main.rs", 10, 1, vec![1], 3, 2),
            make_hotspot("search", "routing", 5, 2, vec![2], 0, 3),
            make_hotspot("file", "config", 7, 3, vec![3], 0, 5),
        ];
        let graph = make_empty_graph();
        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        assert!(!proposals.is_empty(), "should generate proposals");
        for p in &proposals {
            assert_eq!(p.id.len(), 64, "id must be 64 hex chars");
            assert!(
                p.id.chars().all(|c| c.is_ascii_hexdigit()),
                "id must be hex: {}",
                p.id
            );
        }
    }

    #[test]
    fn all_proposals_validate() {
        let hotspots = vec![
            make_hotspot("file", "src/main.rs", 10, 1, vec![1], 3, 2),
            make_hotspot("search", "routing", 6, 2, vec![2], 0, 3),
            make_hotspot("symbol", "routing", 5, 3, vec![3], 0, 2),
        ];

        // Add graph with cross-kind nodes for grouping.
        let graph = KnowledgeGraphDocument {
            schema_version: scryrs_types::GRAPH_SCHEMA_VERSION.into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: vec![
                GraphNode {
                    id: "file:routing".into(),
                    label: "routing".into(),
                    description: None,
                    kind: "file".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::HotspotSubject,
                        subject: "routing".into(),
                        row_ids: vec![1],
                        doc_ref: None,
                        description: None,
                        score: Some(6),
                        metadata: None,
                    }],
                    metadata: None,
                },
                GraphNode {
                    id: "search:routing".into(),
                    label: "routing".into(),
                    description: None,
                    kind: "search".into(),
                    tags: vec![],
                    aliases: vec![],
                    evidence_links: vec![EvidenceLink {
                        source_kind: EvidenceSourceKind::HotspotSubject,
                        subject: "routing".into(),
                        row_ids: vec![2],
                        doc_ref: None,
                        description: None,
                        score: Some(5),
                        metadata: None,
                    }],
                    metadata: None,
                },
            ],
            edges: vec![],
        };

        let proposals = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);
        assert!(!proposals.is_empty());

        for p in &proposals {
            if let Err(e) = p.validate() {
                panic!("proposal {:?} failed validation: {}", p.target_type, e);
            }
        }
    }

    #[test]
    fn deterministic_id_same_input_same_id() {
        let hotspots = vec![make_hotspot("file", "src/main.rs", 10, 1, vec![1], 0, 5)];
        let graph = make_empty_graph();
        let proposals1 = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);
        let proposals2 = generate_proposals(&graph, &hotspots, TEST_GENERATED_AT);

        assert_eq!(proposals1.len(), proposals2.len());
        for (p1, p2) in proposals1.iter().zip(proposals2.iter()) {
            assert_eq!(p1.id, p2.id, "same inputs must produce same IDs");
            assert_eq!(p1.target_type, p2.target_type);
        }
    }

    #[test]
    fn descriptor_returns_valid_feature_descriptor() {
        let d = descriptor();
        assert_eq!(d.id, "curator");
        assert_eq!(d.title, "scryrs-curator");
        assert!(!d.summary.is_empty());
    }
}
