//! Agent-side routing and retrieval helper foundation.

use scryrs_types::{
    FeatureDescriptor, HINT_SCHEMA_VERSION, RouteHintDocument, RouteHintItem, RouteManifestDocument,
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
                "Route '{}' ({}): {} evidence link(s), subject kind {}",
                entry.label,
                entry.id,
                entry.evidence_links.len(),
                entry.subject_kind
            );
            RouteHintItem {
                route_id: entry.id.clone(),
                target: entry.target.clone(),
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

/// A match tier for explain_hints ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchTier {
    None = 0,
    Substring = 1,
    Prefix = 2,
    Exact = 3,
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
/// case-insensitive substring match against the query. Matched entries are
/// tiered: exact match (tier 3) > prefix match (tier 2) > substring match (tier 1).
/// Within each tier, manifest entry order (rank ascending) is the final tie-break.
/// Only entries that match at least one field appear in the output.
/// Each hint's `reason` field is extended with `"; query match on {fields}"`.
/// Zero matches produces a valid `RouteHintDocument` with an empty `hints` array.
pub fn explain_hints(manifest: &RouteManifestDocument, query: &str) -> RouteHintDocument {
    let base = hints_from_manifest(manifest);
    let query_lower = query.to_lowercase();

    // Collect (hint_index, tier, matched_fields) for each matching entry.
    let mut matches: Vec<(usize, MatchTier, Vec<&str>)> = Vec::new();

    for (i, _hint) in base.hints.iter().enumerate() {
        // Find the source route entry by index.
        let entry = &manifest.routes[i];
        let mut best_tier = MatchTier::None;
        let mut matched_fields: Vec<&str> = Vec::new();

        // Check entry-level fields: label, subject, id, target, kind.
        for (field_name, field_value) in &[
            ("label", entry.label.as_str()),
            ("subject", entry.subject.as_str()),
            ("id", entry.id.as_str()),
            ("target", entry.target.as_str()),
            ("kind", entry.kind.as_str()),
        ] {
            let lower = field_value.to_lowercase();
            let tier = classify_match(&lower, &query_lower);
            if tier > MatchTier::None {
                matched_fields.push(field_name);
                if tier > best_tier {
                    best_tier = tier;
                }
            }
        }

        // Check evidence_links subject fields.
        for link in &entry.evidence_links {
            let lower = link.subject.to_lowercase();
            let tier = classify_match(&lower, &query_lower);
            if tier > MatchTier::None {
                matched_fields.push("evidence.subject");
                if tier > best_tier {
                    best_tier = tier;
                }
            }
        }

        if best_tier > MatchTier::None {
            matches.push((i, best_tier, matched_fields));
        }
    }

    // Stable sort: tier descending, then manifest order (index) ascending.
    matches.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Build result hints from matching entries.
    let hints: Vec<RouteHintItem> = matches
        .into_iter()
        .map(|(_idx, _tier, fields)| {
            let mut hint = base.hints[_idx].clone();
            let match_suffix = format!("; query match on {}", join_matched_fields(&fields));
            hint.reason.push_str(&match_suffix);
            hint
        })
        .collect();

    RouteHintDocument {
        schema_version: HINT_SCHEMA_VERSION.to_string(),
        hints,
    }
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
    use scryrs_types::{EvidenceLink, EvidenceSourceKind, GraphMetadata, RouteEntry};

    fn make_evidence_link(
        source_kind: EvidenceSourceKind,
        subject: &str,
        row_ids: Vec<u64>,
    ) -> EvidenceLink {
        EvidenceLink {
            source_kind,
            subject: subject.into(),
            row_ids,
            doc_ref: None,
            description: None,
            score: None,
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
            kind: subject_kind.into(),
            evidence_links: evidence,
            grouping: None,
            metadata: None,
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
            "Route 'auth' (search:auth): 3 evidence link(s), subject kind search"
        );
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
            "Route 'auth' (search:auth): 3 evidence link(s), subject kind search"
        );
        // Ensure no query-match suffix leaked into hints_from_manifest.
        assert!(!doc.hints[0].reason.contains("query match on"));
    }
}
