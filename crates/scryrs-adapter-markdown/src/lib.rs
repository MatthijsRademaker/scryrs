//! Generic Markdown adapter foundation.

use scryrs_types::{FeatureDescriptor, KnowledgeProposal};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "adapter-markdown",
        title: "scryrs-adapter-markdown",
        summary: "generic Markdown publishing surface foundation",
    }
}

pub fn render_proposal(proposal: &KnowledgeProposal) -> String {
    format!("# {}\n\n{}\n", proposal.title, proposal.rationale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown_heading() {
        let markdown = render_proposal(&KnowledgeProposal {
            title: "Document routing".to_string(),
            rationale: "Repeated lookup.".to_string(),
        });

        assert!(markdown.starts_with("# Document routing"));
    }
}
