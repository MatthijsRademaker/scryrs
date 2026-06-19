//! Reviewable knowledge proposal foundation.

use scryrs_types::{FeatureDescriptor, Hotspot, KnowledgeProposal};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "curator",
        title: "scryrs-curator",
        summary: "reviewable docs, skills, decisions, and memory proposal foundation",
    }
}

pub fn propose_from_hotspot(hotspot: &Hotspot) -> KnowledgeProposal {
    KnowledgeProposal {
        title: format!("Document {}", hotspot.subject),
        rationale: format!("{} appeared with score {}", hotspot.subject, hotspot.score),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_mentions_hotspot_subject() {
        let proposal = propose_from_hotspot(&Hotspot {
            subject: "routing".to_string(),
            score: 3,
        });

        assert!(proposal.title.contains("routing"));
    }
}
