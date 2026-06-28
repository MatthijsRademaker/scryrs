//! Generic Markdown adapter foundation.
//!
//! ## Migration note (proposal-contract)
//!
//! The placeholder `KnowledgeProposal` type has been replaced by the
//! versioned `ProposalDocument` contract in `scryrs-types`. This crate
//! renders proposal markdown from the new contract but does not add
//! any publishing, file I/O, or review-workflow behavior.

use scryrs_types::{FeatureDescriptor, ProposalDocument, ProposedContent};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "adapter-markdown",
        title: "scryrs-adapter-markdown",
        summary: "generic Markdown publishing surface foundation",
    }
}

/// Render the title plus markdown content from a proposal document.
///
/// Panics if `proposed_content` is not a `ProposedContent::Markdown`
/// variant — callers should guard with `validate()` or pattern match first.
pub fn render_proposal(proposal: &ProposalDocument) -> String {
    let body = match &proposal.proposed_content {
        ProposedContent::Markdown(text) => text.as_str(),
        other => panic!("render_proposal expects Markdown proposed content, got {other:?}"),
    };
    format!("# {}\n\n{}\n", proposal.title, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::{
        EvidenceLink, EvidenceSourceKind, PROPOSAL_SCHEMA_VERSION, ProposalDocument,
        ProposalTargetType, ProposedContent,
    };

    fn make_proposal(title: &str, rationale: &str, markdown: &str) -> ProposalDocument {
        let content = ProposedContent::Markdown(markdown.into());
        let id = ProposalDocument::compute_id(&ProposalTargetType::DocsNote, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type: ProposalTargetType::DocsNote,
            title: title.into(),
            rationale: rationale.into(),
            proposed_content: content,
            evidence: vec![EvidenceLink {
                source_kind: EvidenceSourceKind::HotspotSubject,
                subject: "test".into(),
                row_ids: vec![1],
                doc_ref: None,
                description: None,
                score: None,
                metadata: None,
            }],
            created_at: "2026-06-27T12:00:00Z".into(),
        }
    }

    #[test]
    fn renders_markdown_heading() {
        let markdown = render_proposal(&make_proposal(
            "Document routing",
            "Repeated lookup.",
            "## Routing details\n",
        ));

        assert!(markdown.starts_with("# Document routing"));
    }

    #[test]
    fn render_includes_proposed_content_after_title() {
        let markdown = render_proposal(&make_proposal(
            "Fix cache",
            "Cache misses are high.",
            "## Cache strategy\n",
        ));
        assert!(markdown.contains("## Cache strategy"));
    }

    #[test]
    fn render_produces_trailing_newline() {
        let markdown = render_proposal(&make_proposal("T", "R", "body\n"));
        assert!(markdown.ends_with('\n'));
    }

    #[test]
    fn empty_proposed_content_still_renders() {
        // Empty markdown renders but the contract validate() would reject it.
        let markdown = render_proposal(&make_proposal("Just title", "reason", ""));
        assert!(markdown.starts_with("# Just title"));
        assert!(markdown.contains("\n\n"));
    }
}
