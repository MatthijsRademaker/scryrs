//! Reviewable knowledge proposal foundation.
//!
//! ## Migration note (proposal-contract)
//!
//! The placeholder `KnowledgeProposal` type has been replaced by the
//! versioned `ProposalDocument` contract in `scryrs-types`. This crate
//! constructs a minimal proposal document from a hotspot entry for
//! compatibility only. It does NOT add proposal file generation, CLI
//! registration, or any auto-apply behavior.

use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, FeatureDescriptor, HotspotEntry, PROPOSAL_SCHEMA_VERSION,
    ProposalDocument, ProposalTargetType, ProposedContent,
};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "curator",
        title: "scryrs-curator",
        summary: "reviewable docs, skills, decisions, and memory proposal foundation",
    }
}

/// Build a minimal proposal document from a hotspot entry for
/// compatibility migration. The resulting document targets `docs_note`
/// with markdown content derived from the hotspot subject and score.
/// This is a placeholder surface; real proposal generation is a
/// follow-up concern.
pub fn propose_from_hotspot(hotspot: &HotspotEntry) -> ProposalDocument {
    let title = format!("Document {}", hotspot.subject);
    let rationale = format!("{} appeared with score {}", hotspot.subject, hotspot.score);
    let markdown = format!("# {title}\n\n{hotspot:?}\n");
    let content = ProposedContent::Markdown(markdown);
    let id = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
        .unwrap_or_else(|e| panic!("compute_id: {e}"));

    ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type: ProposalTargetType::DocsNote,
        title,
        rationale,
        proposed_content: content,
        evidence: vec![EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: hotspot.subject.clone(),
            row_ids: hotspot.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(hotspot.score),
            metadata: None,
        }],
        created_at: "1970-01-01T00:00:00Z".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::{HotspotCounts, HotspotEvidence, ProposalTargetType};
    use std::collections::HashMap;

    fn make_hotspot(subject: &str, score: u32) -> HotspotEntry {
        HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: subject.to_string(),
            score,
            counts: HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![1] },
        }
    }

    #[test]
    fn proposal_mentions_hotspot_subject() {
        let proposal = propose_from_hotspot(&make_hotspot("routing", 3));

        assert!(proposal.title.contains("routing"));
    }

    #[test]
    fn proposal_rationale_includes_subject_and_score() {
        let proposal = propose_from_hotspot(&make_hotspot("routing", 5));

        assert!(
            proposal.rationale.contains("routing"),
            "rationale should contain the subject 'routing', got: {}",
            proposal.rationale
        );
        assert!(
            proposal.rationale.contains("5"),
            "rationale should contain the score '5', got: {}",
            proposal.rationale
        );
    }

    #[test]
    fn proposal_validates_against_contract() {
        let hotspot = HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: "routing".to_string(),
            score: 0,
            counts: HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 0,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![42] },
        };

        let proposal = propose_from_hotspot(&hotspot);

        assert!(!proposal.title.is_empty());
        assert!(!proposal.rationale.is_empty());
        assert!(!proposal.evidence.is_empty());
        assert!(proposal.validate().is_ok());
    }

    #[test]
    fn proposal_target_type_is_docs_note() {
        let proposal = propose_from_hotspot(&make_hotspot("routing", 3));
        assert_eq!(proposal.target_type, ProposalTargetType::DocsNote);
    }

    #[test]
    fn proposal_has_deterministic_id() {
        let proposal = propose_from_hotspot(&make_hotspot("routing", 3));
        assert_eq!(proposal.id.len(), 64);
        assert!(proposal.id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn proposal_created_at_is_non_empty() {
        let proposal = propose_from_hotspot(&make_hotspot("routing", 3));
        assert!(
            !proposal.created_at.is_empty(),
            "created_at must not be empty"
        );
    }
}
