//! Agent-side routing and retrieval helper foundation.

use scryrs_types::{
    FeatureDescriptor, HINT_SCHEMA_VERSION, RouteHintDocument, RouteHintItem, RouteLoadTargetKind,
    RouteManifestDocument,
};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "runtime",
        title: "scryrs-runtime",
        summary: "agent-side routing and retrieval helper foundation",
    }
}

/// Project a route manifest into a deterministic route hint document.
///
/// Each `RouteEntry` produces exactly one `RouteHintItem`. Rank is the
/// 1-based ordinal position within the manifest's routes array. Relevance
/// is `None` (deferred). Reason is a deterministic template citing the
/// entry identity, evidence count, and subject kind. Evidence is copied
/// directly from the source entry.
pub fn hints_from_manifest(manifest: &RouteManifestDocument) -> RouteHintDocument {
    let hints: Vec<RouteHintItem> = manifest
        .routes
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let rank = (i + 1) as u32;
            let reason = format!(
                "Route '{}' ({}): {} evidence link(s), subject kind {}, load target {}",
                entry.label,
                entry.id,
                entry.evidence_links.len(),
                entry.subject_kind,
                load_target_kind_name(entry.load_target.as_ref())
            );
            RouteHintItem {
                route_id: entry.id.clone(),
                target: entry.target.clone(),
                load_target: entry.load_target.clone(),
                label: entry.label.clone(),
                rank,
                relevance: None,
                reason,
                evidence: entry.evidence_links.clone(),
            }
        })
        .collect();

    RouteHintDocument {
        schema_version: HINT_SCHEMA_VERSION.to_string(),
        hints,
    }
}

fn load_target_kind_name(load_target: Option<&scryrs_types::RouteLoadTarget>) -> &'static str {
    match load_target.map(|target| &target.kind) {
        Some(RouteLoadTargetKind::File) => "file",
        Some(RouteLoadTargetKind::DocPage) => "doc_page",
        Some(RouteLoadTargetKind::NonLoadable) => "non_loadable",
        None => "unknown",
    }
}

/// A match tier for explain_hints ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchTier {
    None = 0,
    Substring = 1,
    Prefix = 2,
    Exact = 3,
}

impl MatchTier {
    fn as_u32(self) -> u32 {
        self as u32
    }
}

const EXPLAIN_RELEVANCE_TIER_MULTIPLIER: u32 = 1_000_000_000;
const EXPLAIN_RELEVANCE_SCORE_MULTIPLIER: u32 = 1_000;
const EXPLAIN_RELEVANCE_SCORE_CAP: u32 = 999_999;
const EXPLAIN_RELEVANCE_COUNT_CAP: u32 = 999;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplainMatch {
    manifest_index: usize,
    route_id: String,
    best_tier: MatchTier,
    total_evidence_score: u32,
    evidence_count: u32,
    matched_fields: Vec<&'static str>,
}

/// Build a sorted, deduplicated, comma-separated list of matched field names.
fn join_matched_fields(fields: &[&str]) -> String {
    let mut sorted: Vec<&str> = fields.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted.join(", ")
}

/// Project a route manifest into a query-filtered route hint document.
///
/// Calls `hints_from_manifest` internally, then filters and re-orders by
/// case-insensitive substring match against the query. Matched entries sort by
/// `(tier DESC, score DESC, count DESC, manifest_index ASC, route_id ASC)`.
/// `relevance` is populated only for explain matches using the documented packed
/// `u32` formula, while plain `hints_from_manifest` output remains unchanged.
/// Each hint's `reason` field is extended with `"; query match on {fields}"`.
/// Zero matches produces a valid `RouteHintDocument` with an empty `hints` array.
pub fn explain_hints(manifest: &RouteManifestDocument, query: &str) -> RouteHintDocument {
    let base = hints_from_manifest(manifest);
    let query_lower = query.to_lowercase();
    let mut matches: Vec<ExplainMatch> = Vec::new();

    for (manifest_index, entry) in manifest.routes.iter().enumerate() {
        let mut best_tier = MatchTier::None;
        let mut matched_fields: Vec<&'static str> = Vec::new();

        for (field_name, field_value) in [
            ("label", entry.label.as_str()),
            ("subject", entry.subject.as_str()),
            ("id", entry.id.as_str()),
            ("target", entry.target.as_str()),
            ("kind", entry.kind.as_str()),
        ] {
            let tier = classify_match(&field_value.to_lowercase(), &query_lower);
            if tier > MatchTier::None {
                matched_fields.push(field_name);
                if tier > best_tier {
                    best_tier = tier;
                }
            }
        }

        for link in &entry.evidence_links {
            let tier = classify_match(&link.subject.to_lowercase(), &query_lower);
            if tier > MatchTier::None {
                matched_fields.push("evidence.subject");
                if tier > best_tier {
                    best_tier = tier;
                }
            }
        }

        if best_tier > MatchTier::None {
            matches.push(ExplainMatch {
                manifest_index,
                route_id: entry.id.clone(),
                best_tier,
                total_evidence_score: total_evidence_score(entry),
                evidence_count: saturating_evidence_count(entry.evidence_links.len()),
                matched_fields,
            });
        }
    }

    matches.sort_by(|a, b| {
        b.best_tier
            .cmp(&a.best_tier)
            .then_with(|| b.total_evidence_score.cmp(&a.total_evidence_score))
            .then_with(|| b.evidence_count.cmp(&a.evidence_count))
            .then_with(|| a.manifest_index.cmp(&b.manifest_index))
            .then_with(|| a.route_id.cmp(&b.route_id))
    });

    let hints = matches
        .into_iter()
        .map(|matched| {
            let mut hint = base.hints[matched.manifest_index].clone();
            hint.relevance = Some(pack_explain_relevance(
                matched.best_tier,
                matched.total_evidence_score,
                matched.evidence_count,
            ));
            let match_suffix = format!(
                "; query match on {}",
                join_matched_fields(&matched.matched_fields)
            );
            hint.reason.push_str(&match_suffix);
            hint
        })
        .collect();

    RouteHintDocument {
        schema_version: HINT_SCHEMA_VERSION.to_string(),
        hints,
    }
}

fn total_evidence_score(entry: &scryrs_types::RouteEntry) -> u32 {
    entry.evidence_links.iter().fold(0u32, |score, link| {
        score.saturating_add(link.score.unwrap_or(0))
    })
}

fn saturating_evidence_count(count: usize) -> u32 {
    count.min(u32::MAX as usize) as u32
}

fn pack_explain_relevance(
    best_tier: MatchTier,
    total_evidence_score: u32,
    evidence_count: u32,
) -> u32 {
    best_tier.as_u32() * EXPLAIN_RELEVANCE_TIER_MULTIPLIER
        + total_evidence_score.min(EXPLAIN_RELEVANCE_SCORE_CAP) * EXPLAIN_RELEVANCE_SCORE_MULTIPLIER
        + evidence_count.min(EXPLAIN_RELEVANCE_COUNT_CAP)
}

/// Classify how `query_lower` matches `field_lower`.
fn classify_match(field_lower: &str, query_lower: &str) -> MatchTier {
    if field_lower == query_lower {
        MatchTier::Exact
    } else if field_lower.starts_with(query_lower) {
        MatchTier::Prefix
    } else if field_lower.contains(query_lower) {
        MatchTier::Substring
    } else {
        MatchTier::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::{
        EvidenceLink, EvidenceSourceKind, GraphMetadata, RouteEntry, RouteLoadTarget,
        RouteLoadTargetKind,
    };

    fn make_evidence_link(
        source_kind: EvidenceSourceKind,
        subject: &str,
        row_ids: Vec<u64>,
    ) -> EvidenceLink {
        make_scored_evidence_link(source_kind, subject, row_ids, None)
    }

    fn make_scored_evidence_link(
        source_kind: EvidenceSourceKind,
        subject: &str,
        row_ids: Vec<u64>,
        score: Option<u32>,
    ) -> EvidenceLink {
        EvidenceLink {
            source_kind,
            subject: subject.into(),
            row_ids,
            doc_ref: None,
            description: None,
            score,
            metadata: None,
        }
    }

    fn make_route_entry(
        id: &str,
        label: &str,
        target: &str,
        subject_kind: &str,
        evidence: Vec<EvidenceLink>,
    ) -> RouteEntry {
        RouteEntry {
            id: id.into(),
            subject_kind: subject_kind.into(),
            subject: label.into(),
            label: label.into(),
            target: target.into(),
            load_target: Some(default_load_target(subject_kind, label)),
            kind: subject_kind.into(),
            evidence_links: evidence,
            grouping: None,
            metadata: None,
        }
    }

    fn default_load_target(subject_kind: &str, label: &str) -> RouteLoadTarget {
        match subject_kind {
            "file" => RouteLoadTarget {
                kind: RouteLoadTargetKind::File,
                reference: Some(label.into()),
            },
            "doc_page" => RouteLoadTarget {
                kind: RouteLoadTargetKind::DocPage,
                reference: Some(format!("project-docs/{label}")),
            },
            _ => RouteLoadTarget {
                kind: RouteLoadTargetKind::NonLoadable,
                reference: None,
            },
        }
    }

    fn make_manifest(routes: Vec<RouteEntry>) -> RouteManifestDocument {
        RouteManifestDocument {
            schema_version: "1.0.0".into(),
            metadata: GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            routes,
        }
    }

    #[test]
    fn hints_from_manifest_preserves_identity_boundaries() {
        let manifest = make_manifest(vec![
            make_route_entry("file:auth", "auth", "file:auth", "file", vec![]),
            make_route_entry("search:auth", "auth", "search:auth", "search", vec![]),
            make_route_entry("symbol:auth", "auth", "symbol:auth", "symbol", vec![]),
        ]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 3);

        let ids: Vec<&str> = doc.hints.iter().map(|h| h.route_id.as_str()).collect();
        assert_eq!(ids, vec!["file:auth", "search:auth", "symbol:auth"]);
        assert!(!doc.hints.iter().any(|h| h.route_id.is_empty()));
    }

    #[test]
    fn hints_from_manifest_assigns_ordinal_rank() {
        let manifest = make_manifest(vec![
            make_route_entry("file:aaa.rs", "aaa.rs", "file:aaa.rs", "file", vec![]),
            make_route_entry("file:zzz.rs", "zzz.rs", "file:zzz.rs", "file", vec![]),
            make_route_entry(
                "search:routing",
                "routing",
                "search:routing",
                "search",
                vec![],
            ),
        ]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints[0].rank, 1);
        assert_eq!(doc.hints[1].rank, 2);
        assert_eq!(doc.hints[2].rank, 3);
    }

    #[test]
    fn hints_from_manifest_copies_evidence_links() {
        let evidence = vec![
            make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![1, 2]),
            make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![3]),
        ];
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth",
            "auth",
            "file:auth",
            "file",
            evidence.clone(),
        )]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(doc.hints[0].evidence.len(), 2);
        assert_eq!(doc.hints[0].evidence, evidence);
    }

    #[test]
    fn hints_from_manifest_reason_template() {
        let manifest = make_manifest(vec![make_route_entry(
            "search:auth",
            "auth",
            "search:auth",
            "search",
            vec![
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![1]),
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![2]),
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![3]),
            ],
        )]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(
            doc.hints[0].reason,
            "Route 'auth' (search:auth): 3 evidence link(s), subject kind search, load target non_loadable"
        );
    }

    #[test]
    fn hints_from_manifest_copies_load_target() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:src/main.rs",
            "src/main.rs",
            "file:src/main.rs",
            "file",
            vec![],
        )]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 1);
        let Some(load_target) = doc.hints[0].load_target.as_ref() else {
            panic!("load target");
        };
        assert_eq!(load_target.kind, RouteLoadTargetKind::File);
        assert_eq!(load_target.reference.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn hints_from_manifest_keeps_non_loadable_routes_explicit() {
        let manifest = make_manifest(vec![make_route_entry(
            "domain_term:auth",
            "auth",
            "domain_term:auth",
            "domain_term",
            vec![],
        )]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 1);
        let Some(load_target) = doc.hints[0].load_target.as_ref() else {
            panic!("load target");
        };
        assert_eq!(load_target.kind, RouteLoadTargetKind::NonLoadable);
        assert_eq!(load_target.reference, None);
        assert!(doc.hints[0].reason.contains("load target non_loadable"));
    }

    #[test]
    fn hints_from_manifest_is_deterministic() {
        let manifest = make_manifest(vec![
            make_route_entry("file:auth", "auth", "file:auth", "file", vec![]),
            make_route_entry("search:auth", "auth", "search:auth", "search", vec![]),
        ]);
        let doc1 = hints_from_manifest(&manifest);
        let doc2 = hints_from_manifest(&manifest);
        assert_eq!(doc1, doc2);
    }

    #[test]
    fn hints_from_manifest_empty_manifest() {
        let manifest = make_manifest(vec![]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.schema_version, HINT_SCHEMA_VERSION);
        assert!(doc.hints.is_empty());
    }

    #[test]
    fn hints_from_manifest_relevance_is_none() {
        let manifest = make_manifest(vec![
            make_route_entry("file:a", "a", "file:a", "file", vec![]),
            make_route_entry("file:b", "b", "file:b", "file", vec![]),
        ]);
        let doc = hints_from_manifest(&manifest);
        for hint in &doc.hints {
            assert!(
                hint.relevance.is_none(),
                "relevance must be None, got {:?}",
                hint.relevance
            );
        }
    }

    // --- explain_hints tests ---

    #[test]
    fn explain_hints_matches_by_label() {
        let manifest = make_manifest(vec![
            make_route_entry("file:zzz", "Authentication", "file:zzz", "file", vec![]),
            make_route_entry(
                "file:unrelated",
                "unrelated",
                "file:unrelated",
                "file",
                vec![],
            ),
        ]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.schema_version, HINT_SCHEMA_VERSION);
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(doc.hints[0].route_id, "file:zzz");
        assert!(doc.hints[0].reason.contains("query match on"));
        // Only label and subject (which equals label in the test helper) should match.
        assert!(doc.hints[0].reason.contains("label, subject"));
    }

    #[test]
    fn explain_hints_matches_by_subject() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:zzz",
            "auth_handler",
            "file:zzz",
            "file",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "handler");
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(doc.hints[0].route_id, "file:zzz");
        // label and subject both contain "handler" (subject equals label in test helper).
        assert!(
            doc.hints[0]
                .reason
                .contains("query match on label, subject")
        );
    }

    #[test]
    fn explain_hints_matches_by_id() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth_module",
            "zzz",
            "file:auth_module",
            "file",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "module");
        assert_eq!(doc.hints.len(), 1);
        // "module" matches id and target (target equals id in test helper).
        assert!(doc.hints[0].reason.contains("query match on id, target"));
    }

    #[test]
    fn explain_hints_matches_by_target() {
        // Use a target that differs from id to isolate target matching.
        let entry = RouteEntry {
            id: "file:zzz".into(),
            subject_kind: "file".into(),
            subject: "zzz".into(),
            label: "zzz".into(),
            target: "file:config".into(),
            load_target: Some(RouteLoadTarget {
                kind: RouteLoadTargetKind::File,
                reference: Some("config".into()),
            }),
            kind: "file".into(),
            evidence_links: vec![],
            grouping: None,
            metadata: None,
        };
        let manifest = make_manifest(vec![entry]);
        let doc = explain_hints(&manifest, "conf");
        assert_eq!(doc.hints.len(), 1);
        assert!(doc.hints[0].reason.contains("query match on target"));
    }

    #[test]
    fn explain_hints_matches_by_kind() {
        let manifest = make_manifest(vec![make_route_entry(
            "zzz:graph",
            "graph",
            "zzz:graph",
            "doc_page",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "doc");
        assert_eq!(doc.hints.len(), 1);
        // "doc" appears in kind. Also in id and target but "zzz:graph" doesn't contain "doc".
        assert!(doc.hints[0].reason.contains("query match on kind"));
    }

    #[test]
    fn explain_hints_matches_by_evidence_subject() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth",
            "auth",
            "file:auth",
            "file",
            vec![make_evidence_link(
                EvidenceSourceKind::LocalTraceRow,
                "auth_handler",
                vec![1],
            )],
        )]);
        let doc = explain_hints(&manifest, "handler");
        assert_eq!(doc.hints.len(), 1);
        assert!(
            doc.hints[0]
                .reason
                .contains("query match on evidence.subject")
        );
    }

    #[test]
    fn explain_hints_zero_matches_emits_empty_hints() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth",
            "auth",
            "file:auth",
            "file",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "zzz_nonexistent");
        assert_eq!(doc.schema_version, HINT_SCHEMA_VERSION);
        assert!(doc.hints.is_empty());
    }

    #[test]
    fn explain_hints_reason_mentions_doc_page_load_target() {
        let manifest = make_manifest(vec![make_route_entry(
            "doc_page:graph",
            "graph",
            "doc_page:graph",
            "doc_page",
            vec![make_evidence_link(
                EvidenceSourceKind::DocReference,
                "graph",
                vec![],
            )],
        )]);
        let doc = explain_hints(&manifest, "graph");
        assert_eq!(doc.hints.len(), 1);
        let Some(load_target) = doc.hints[0].load_target.as_ref() else {
            panic!("load target");
        };
        assert_eq!(load_target.kind, RouteLoadTargetKind::DocPage);
        assert_eq!(load_target.reference.as_deref(), Some("project-docs/graph"));
        assert!(
            doc.hints[0]
                .reason
                .contains("load target doc_page; query match on")
        );
    }

    #[test]
    fn explain_hints_reason_mentions_non_loadable_target() {
        let manifest = make_manifest(vec![make_route_entry(
            "domain_term:auth",
            "auth",
            "domain_term:auth",
            "domain_term",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 1);
        let Some(load_target) = doc.hints[0].load_target.as_ref() else {
            panic!("load target");
        };
        assert_eq!(load_target.kind, RouteLoadTargetKind::NonLoadable);
        assert_eq!(load_target.reference, None);
        assert!(
            doc.hints[0]
                .reason
                .contains("load target non_loadable; query match on")
        );
    }

    #[test]
    fn explain_hints_exact_outranks_higher_evidence_prefix_match() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:auth-prefix",
                "auth-prefix",
                "file:auth-prefix",
                "file",
                vec![make_scored_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth-prefix",
                    vec![1],
                    Some(999),
                )],
            ),
            make_route_entry("file:auth", "auth", "file:auth", "file", vec![]),
        ]);

        let doc = explain_hints(&manifest, "auth");
        let ids: Vec<&str> = doc
            .hints
            .iter()
            .map(|hint| hint.route_id.as_str())
            .collect();
        assert_eq!(ids, vec!["file:auth", "file:auth-prefix"]);
    }

    #[test]
    fn explain_hints_higher_evidence_score_wins_within_same_tier() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:auth-low-score",
                "auth-low-score",
                "file:auth-low-score",
                "file",
                vec![make_scored_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth-low-score",
                    vec![1],
                    Some(10),
                )],
            ),
            make_route_entry(
                "file:auth-high-score",
                "auth-high-score",
                "file:auth-high-score",
                "file",
                vec![
                    make_scored_evidence_link(
                        EvidenceSourceKind::LocalTraceRow,
                        "auth-high-score-a",
                        vec![2],
                        Some(10),
                    ),
                    make_scored_evidence_link(
                        EvidenceSourceKind::LocalTraceRow,
                        "auth-high-score-b",
                        vec![3],
                        Some(5),
                    ),
                ],
            ),
        ]);

        let doc = explain_hints(&manifest, "auth");
        let ids: Vec<&str> = doc
            .hints
            .iter()
            .map(|hint| hint.route_id.as_str())
            .collect();
        assert_eq!(ids, vec!["file:auth-high-score", "file:auth-low-score"]);
    }

    #[test]
    fn explain_hints_higher_evidence_count_wins_when_score_ties() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:auth-one-link",
                "auth-one-link",
                "file:auth-one-link",
                "file",
                vec![make_scored_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth-one-link",
                    vec![1],
                    Some(10),
                )],
            ),
            make_route_entry(
                "file:auth-two-links",
                "auth-two-links",
                "file:auth-two-links",
                "file",
                vec![
                    make_scored_evidence_link(
                        EvidenceSourceKind::LocalTraceRow,
                        "auth-two-links-a",
                        vec![2],
                        Some(4),
                    ),
                    make_scored_evidence_link(
                        EvidenceSourceKind::LocalTraceRow,
                        "auth-two-links-b",
                        vec![3],
                        Some(6),
                    ),
                ],
            ),
        ]);

        let doc = explain_hints(&manifest, "auth");
        let ids: Vec<&str> = doc
            .hints
            .iter()
            .map(|hint| hint.route_id.as_str())
            .collect();
        assert_eq!(ids, vec!["file:auth-two-links", "file:auth-one-link"]);
    }

    #[test]
    fn explain_hints_manifest_order_breaks_full_ties() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:auth-first",
                "auth-first",
                "file:auth-first",
                "file",
                vec![make_scored_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth-first",
                    vec![1],
                    Some(10),
                )],
            ),
            make_route_entry(
                "file:auth-second",
                "auth-second",
                "file:auth-second",
                "file",
                vec![make_scored_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth-second",
                    vec![2],
                    Some(10),
                )],
            ),
        ]);

        let doc = explain_hints(&manifest, "auth");
        let ids: Vec<&str> = doc
            .hints
            .iter()
            .map(|hint| hint.route_id.as_str())
            .collect();
        assert_eq!(ids, vec!["file:auth-first", "file:auth-second"]);
    }

    #[test]
    fn explain_hints_best_match_tier_across_fields_drives_ranking() {
        let exact_id = RouteEntry {
            id: "auth".into(),
            subject_kind: "file".into(),
            subject: "zzz auth zzz".into(),
            label: "zzz auth zzz".into(),
            target: "file:zzz-auth".into(),
            load_target: Some(RouteLoadTarget {
                kind: RouteLoadTargetKind::File,
                reference: Some("zzz-auth".into()),
            }),
            kind: "file".into(),
            evidence_links: vec![],
            grouping: None,
            metadata: None,
        };
        let prefix_subject = RouteEntry {
            id: "file:zzz".into(),
            subject_kind: "file".into(),
            subject: "auth-prefix".into(),
            label: "auth-prefix".into(),
            target: "file:zzz".into(),
            load_target: Some(RouteLoadTarget {
                kind: RouteLoadTargetKind::File,
                reference: Some("zzz".into()),
            }),
            kind: "file".into(),
            evidence_links: vec![],
            grouping: None,
            metadata: None,
        };

        let manifest = make_manifest(vec![prefix_subject, exact_id]);
        let doc = explain_hints(&manifest, "auth");
        let ids: Vec<&str> = doc
            .hints
            .iter()
            .map(|hint| hint.route_id.as_str())
            .collect();
        assert_eq!(ids, vec!["auth", "file:zzz"]);
    }

    #[test]
    fn explain_hints_populates_relevance_with_saturating_packed_score() {
        let mut evidence_links = vec![make_scored_evidence_link(
            EvidenceSourceKind::LocalTraceRow,
            "auth-overflow",
            vec![1],
            Some(u32::MAX),
        )];
        evidence_links.extend((2..=1001).map(|row_id| {
            make_scored_evidence_link(
                EvidenceSourceKind::LocalTraceRow,
                "auth-overflow",
                vec![row_id],
                Some(1),
            )
        }));

        let manifest = make_manifest(vec![make_route_entry(
            "file:auth-overflow",
            "auth-overflow",
            "file:auth-overflow",
            "file",
            evidence_links,
        )]);

        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(doc.hints[0].relevance, Some(2_999_999_999));
    }

    #[test]
    fn explain_hints_exact_before_substring() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:authentication",
                "authentication",
                "file:authentication",
                "file",
                vec![],
            ),
            make_route_entry("file:auth", "auth", "file:auth", "file", vec![]),
        ]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 2);
        // "file:auth" (exact match) should come before "file:authentication" (prefix match).
        assert_eq!(doc.hints[0].route_id, "file:auth");
        assert_eq!(doc.hints[1].route_id, "file:authentication");
    }

    #[test]
    fn explain_hints_prefix_before_substring() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:zzz_auth_zzz",
                "zzz_auth_zzz",
                "file:zzz_auth_zzz",
                "file",
                vec![],
            ),
            make_route_entry(
                "file:auth_prefix",
                "auth_prefix",
                "file:auth_prefix",
                "file",
                vec![],
            ),
        ]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 2);
        // "file:auth_prefix" (prefix match) should come before "file:zzz_auth_zzz" (substring match).
        assert_eq!(doc.hints[0].route_id, "file:auth_prefix");
        assert_eq!(doc.hints[1].route_id, "file:zzz_auth_zzz");
    }

    #[test]
    fn explain_hints_is_deterministic() {
        let manifest = make_manifest(vec![
            make_route_entry(
                "file:authentication",
                "authentication",
                "file:authentication",
                "file",
                vec![],
            ),
            make_route_entry("file:auth", "auth", "file:auth", "file", vec![]),
        ]);
        let doc1 = explain_hints(&manifest, "auth");
        let doc2 = explain_hints(&manifest, "auth");
        assert_eq!(doc1, doc2);
    }

    #[test]
    fn explain_hints_reason_preserves_base_template() {
        let manifest = make_manifest(vec![make_route_entry(
            "search:auth",
            "auth",
            "search:auth",
            "search",
            vec![
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![1]),
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![2]),
            ],
        )]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 1);
        let reason = &doc.hints[0].reason;
        assert!(
            reason
                .starts_with("Route 'auth' (search:auth): 2 evidence link(s), subject kind search"),
            "reason should start with base template, got: {reason}"
        );
        assert!(reason.contains("query match on"));
    }

    #[test]
    fn explain_hints_multiple_matched_fields() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth",
            "auth",
            "file:auth",
            "file",
            vec![],
        )]);
        // "auth" appears in id, label, subject, target.
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 1);
        let reason = &doc.hints[0].reason;
        // Fields should be sorted alphabetically.
        assert!(reason.contains("query match on id, label, subject, target"));
    }

    #[test]
    fn explain_hints_case_insensitive() {
        let manifest = make_manifest(vec![make_route_entry(
            "file:AUTH",
            "Auth",
            "file:AUTH",
            "file",
            vec![],
        )]);
        let doc = explain_hints(&manifest, "auth");
        assert_eq!(doc.hints.len(), 1);
    }

    #[test]
    fn explain_hints_evidence_subject_deduplicated_in_reason() {
        // Multiple evidence links whose subjects all match must not produce
        // duplicate "evidence.subject" in the reason suffix.
        let manifest = make_manifest(vec![make_route_entry(
            "file:auth",
            "auth",
            "file:auth",
            "file",
            vec![
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth_handler", vec![1]),
                make_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth_handler_v2",
                    vec![2],
                ),
                make_evidence_link(
                    EvidenceSourceKind::LocalTraceRow,
                    "auth_handler_legacy",
                    vec![3],
                ),
            ],
        )]);
        let doc = explain_hints(&manifest, "handler");
        assert_eq!(doc.hints.len(), 1);
        // "handler" matches all three evidence subjects but evidence.subject
        // must appear exactly once in the reason.
        let reason = &doc.hints[0].reason;
        assert!(
            reason.contains("query match on evidence.subject"),
            "reason should contain evidence.subject, got: {reason}"
        );
        // Count occurrences of "evidence.subject" — must be exactly 1.
        let count = reason.matches("evidence.subject").count();
        assert_eq!(
            count, 1,
            "evidence.subject must appear exactly once in reason, appeared {count} times: {reason}"
        );
    }

    #[test]
    fn explain_hints_hints_from_manifest_unchanged() {
        // Verify hints_from_manifest reason template is untouched.
        let manifest = make_manifest(vec![make_route_entry(
            "search:auth",
            "auth",
            "search:auth",
            "search",
            vec![
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![1]),
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![2]),
                make_evidence_link(EvidenceSourceKind::LocalTraceRow, "auth", vec![3]),
            ],
        )]);
        let doc = hints_from_manifest(&manifest);
        assert_eq!(doc.hints.len(), 1);
        assert_eq!(
            doc.hints[0].reason,
            "Route 'auth' (search:auth): 3 evidence link(s), subject kind search, load target non_loadable"
        );
        // Ensure no query-match suffix leaked into hints_from_manifest.
        assert!(!doc.hints[0].reason.contains("query match on"));
    }
}
