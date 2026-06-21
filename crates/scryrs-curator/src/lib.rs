//! Reviewable knowledge proposal foundation.

use scryrs_types::{FeatureDescriptor, HotspotEntry, KnowledgeProposal};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "curator",
        title: "scryrs-curator",
        summary: "reviewable docs, skills, decisions, and memory proposal foundation",
    }
}

pub fn propose_from_hotspot(hotspot: &HotspotEntry) -> KnowledgeProposal {
    KnowledgeProposal {
        title: format!("Document {}", hotspot.subject),
        rationale: format!("{} appeared with score {}", hotspot.subject, hotspot.score),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::HotspotEvidence;
    use std::collections::HashMap;

    fn make_hotspot(subject: &str, score: u32) -> HotspotEntry {
        HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: subject.to_string(),
            score,
            counts: scryrs_types::HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![] },
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
    fn empty_counts_and_evidence_no_panic() {
        let hotspot = HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: "routing".to_string(),
            score: 0,
            counts: scryrs_types::HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 0,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![] },
        };

        let proposal = propose_from_hotspot(&hotspot);

        assert!(!proposal.title.is_empty());
        assert!(!proposal.rationale.is_empty());
    }
}
