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

    #[test]
    fn proposal_mentions_hotspot_subject() {
        let proposal = propose_from_hotspot(&HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: "routing".to_string(),
            score: 3,
            counts: scryrs_types::HotspotCounts {
                eventType: HashMap::new(),
                outcome: HashMap::new(),
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![] },
        });

        assert!(proposal.title.contains("routing"));
    }
}
