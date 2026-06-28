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
}
