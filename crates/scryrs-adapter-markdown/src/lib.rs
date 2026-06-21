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

    #[test]
    fn render_includes_rationale_after_title() {
        let markdown = render_proposal(&KnowledgeProposal {
            title: "Fix cache".to_string(),
            rationale: "Cache misses are high.".to_string(),
        });
        assert!(markdown.contains("Cache misses are high."));
    }

    #[test]
    fn render_produces_trailing_newline() {
        let markdown = render_proposal(&KnowledgeProposal {
            title: "T".to_string(),
            rationale: "R".to_string(),
        });
        assert!(markdown.ends_with('\n'));
    }

    #[test]
    fn empty_rationale_still_renders() {
        let markdown = render_proposal(&KnowledgeProposal {
            title: "Just title".to_string(),
            rationale: String::new(),
        });
        assert!(markdown.starts_with("# Just title"));
        // Should not panic; empty rationale is valid input.
        assert!(markdown.contains("\n\n"));
    }
}
