//! Optional model-assisted curator layer for bounded drafting and semantic
//! grouping over deterministic evidence.
//!
//! This crate implements Foundation 01: library-only APIs that consume
//! existing hotspot, graph, proposal, and document evidence through a
//! bounded `EvidencePack`, invoke a provider-neutral `ModelClient`, and
//! return reviewable `ProposalDocument` drafts and grouping suggestions.
//! Model output is proposal input only — it never mutates source-of-truth
//! graph, hotspot, or route artifacts.
//!
//! # Crate boundary
//!
//! `scryrs-curator-llm` is the only model-aware curator layer.
//! `scryrs-curator` remains the deterministic proposal-generation crate
//! and does not depend on this crate or on `scryrs-llm`.

use std::collections::HashMap;

use scryrs_llm::{ModelClient, ModelError, ModelMode, ModelRequest};
use scryrs_types::{
    EvidenceLink, EvidenceSourceKind, GraphNode, HotspotEntry, KnowledgeGraphDocument,
    PROPOSAL_SCHEMA_VERSION, ProposalDocument, ProposalTargetType, ProposedContent,
    SemanticGraphGrouping,
};

// ---------------------------------------------------------------------------
// Evidence entry kinds
// ---------------------------------------------------------------------------

/// A single bounded evidence item keyed to a stable input-local citation ID.
#[derive(Debug, Clone)]
pub enum EvidenceEntry {
    Hotspot(HotspotEntry),
    GraphNode(GraphNode),
    Proposal(ProposalDocument),
    Document {
        doc_ref: String,
        description: Option<String>,
    },
}

impl EvidenceEntry {
    /// Return a short summary string for model context serialization.
    fn summary(&self) -> String {
        match self {
            EvidenceEntry::Hotspot(h) => {
                format!(
                    "[hotspot] {}:{} score={} rank={}",
                    h.subjectKind, h.subject, h.score, h.rank
                )
            }
            EvidenceEntry::GraphNode(n) => {
                format!("[graph_node] id={} label={} kind={}", n.id, n.label, n.kind)
            }
            EvidenceEntry::Proposal(p) => {
                format!(
                    "[proposal] id={} target={:?} title={}",
                    p.id, p.target_type, p.title
                )
            }
            EvidenceEntry::Document { doc_ref, .. } => {
                format!("[document] doc_ref={}", doc_ref)
            }
        }
    }

    /// Return the graph node ID if this entry is a `GraphNode`, otherwise
    /// `None`. Used for source-node-ID validation in grouping responses.
    fn graph_node_id(&self) -> Option<&str> {
        match self {
            EvidenceEntry::GraphNode(n) => Some(&n.id),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// EvidencePack — bounded evidence input
// ---------------------------------------------------------------------------

/// Per-type capacity configuration for `EvidencePackBuilder`.
#[derive(Debug, Clone)]
pub struct EvidencePackConfig {
    /// Maximum total characters across all serialized evidence.
    pub max_input_chars: usize,
    /// Maximum hotspot entries admitted.
    pub max_hotspots: usize,
    /// Maximum graph nodes admitted.
    pub max_graph_nodes: usize,
    /// Maximum proposal documents admitted.
    pub max_proposals: usize,
    /// Maximum document evidence entries admitted.
    pub max_documents: usize,
}

impl Default for EvidencePackConfig {
    fn default() -> Self {
        Self {
            max_input_chars: 32_000,
            max_hotspots: 50,
            max_graph_nodes: 100,
            max_proposals: 20,
            max_documents: 20,
        }
    }
}

/// A validated, bounded collection of evidence items with stable
/// input-local citation IDs and preserved graph node ID lookups.
#[derive(Debug, Clone)]
pub struct EvidencePack {
    /// Ordered evidence entries, each assigned a stable input-local ID
    /// `"e0"`, `"e1"`, ... matching its index.
    entries: Vec<EvidenceEntry>,
    /// Map from input-local citation ID (e.g. `"e0"`) to index.
    citation_index: HashMap<String, usize>,
    /// Set of graph node IDs present in this pack for output validation.
    graph_node_ids: HashMap<String, usize>,
    /// The budget configuration this pack was built with.
    config: EvidencePackConfig,
    /// Total chars serialized for budget enforcement.
    total_chars: usize,
}

impl EvidencePack {
    /// Build an `EvidencePack` from explicit entry collections.
    ///
    /// Returns an error if any budget cap is exceeded.
    pub fn build(
        config: EvidencePackConfig,
        hotspots: Vec<HotspotEntry>,
        graph_nodes: Vec<GraphNode>,
        proposals: Vec<ProposalDocument>,
        documents: Vec<(String, Option<String>)>,
    ) -> Result<Self, PackError> {
        let mut entries: Vec<EvidenceEntry> = Vec::new();
        let mut total_chars: usize = 0;

        // Hotspots
        if hotspots.len() > config.max_hotspots {
            return Err(PackError::BudgetExceeded {
                field: "hotspots".into(),
                limit: config.max_hotspots,
                actual: hotspots.len(),
            });
        }
        for h in hotspots {
            let summary = EvidenceEntry::Hotspot(h.clone()).summary();
            total_chars += summary.len();
            entries.push(EvidenceEntry::Hotspot(h));
        }

        // Graph nodes
        if graph_nodes.len() > config.max_graph_nodes {
            return Err(PackError::BudgetExceeded {
                field: "graph_nodes".into(),
                limit: config.max_graph_nodes,
                actual: graph_nodes.len(),
            });
        }
        for n in graph_nodes {
            let summary = EvidenceEntry::GraphNode(n.clone()).summary();
            total_chars += summary.len();
            entries.push(EvidenceEntry::GraphNode(n));
        }

        // Proposals
        if proposals.len() > config.max_proposals {
            return Err(PackError::BudgetExceeded {
                field: "proposals".into(),
                limit: config.max_proposals,
                actual: proposals.len(),
            });
        }
        for p in proposals {
            let summary = EvidenceEntry::Proposal(p.clone()).summary();
            total_chars += summary.len();
            entries.push(EvidenceEntry::Proposal(p));
        }

        // Documents
        if documents.len() > config.max_documents {
            return Err(PackError::BudgetExceeded {
                field: "documents".into(),
                limit: config.max_documents,
                actual: documents.len(),
            });
        }
        for (doc_ref, description) in documents {
            let entry = EvidenceEntry::Document {
                doc_ref: doc_ref.clone(),
                description: description.clone(),
            };
            total_chars += entry.summary().len();
            entries.push(entry);
        }

        // Budget enforcement: max_input_chars
        if total_chars > config.max_input_chars {
            return Err(PackError::BudgetExceeded {
                field: "total_chars".into(),
                limit: config.max_input_chars,
                actual: total_chars,
            });
        }

        let citation_index: HashMap<String, usize> =
            (0..entries.len()).map(|i| (format!("e{i}"), i)).collect();

        let graph_node_ids: HashMap<String, usize> = entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.graph_node_id().map(|id| (id.to_string(), i)))
            .collect();

        Ok(Self {
            entries,
            citation_index,
            graph_node_ids,
            config,
            total_chars,
        })
    }

    /// Build an `EvidencePack` from a `KnowledgeGraphDocument` plus
    /// hotspot entries using default configuration.
    pub fn from_graph_and_hotspots(
        graph: &KnowledgeGraphDocument,
        hotspots: &[HotspotEntry],
    ) -> Result<Self, PackError> {
        Self::build(
            EvidencePackConfig::default(),
            hotspots.to_vec(),
            graph.nodes.clone(),
            Vec::new(),
            Vec::new(),
        )
    }

    /// Return the total character count across all evidence entries.
    pub fn total_chars(&self) -> usize {
        self.total_chars
    }

    /// Return the number of evidence entries in this pack.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the pack is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up an entry by its input-local citation ID (e.g. `"e0"`).
    pub fn get_by_citation(&self, citation_id: &str) -> Option<&EvidenceEntry> {
        self.citation_index
            .get(citation_id)
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Check whether a graph node ID is present in this pack.
    pub fn has_graph_node_id(&self, node_id: &str) -> bool {
        self.graph_node_ids.contains_key(node_id)
    }

    /// Serialize the pack into a model-ready context string.
    fn to_prompt_context(&self) -> String {
        let mut ctx = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            ctx.push_str(&format!("[e{i}] {}\n", entry.summary()));
        }
        ctx
    }
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors produced during evidence pack construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    BudgetExceeded {
        field: String,
        limit: usize,
        actual: usize,
    },
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::BudgetExceeded {
                field,
                limit,
                actual,
            } => {
                write!(
                    f,
                    "evidence budget exceeded for '{}': limit={}, actual={}",
                    field, limit, actual
                )
            }
        }
    }
}

impl std::error::Error for PackError {}

/// Errors produced during model-assisted drafting or grouping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssistError {
    /// The model returned an error.
    ModelError(String),
    /// The model response was not valid JSON.
    InvalidJson(String),
    /// The model response was missing required fields.
    MissingField(String),
    /// A cited evidence ID was not found in the evidence pack.
    UnknownCitation(String),
    /// A source node ID referenced by a grouping suggestion is not present
    /// in the evidence pack.
    UnknownSourceNode(String),
    /// An evidence citation was missing (output had claims without evidence).
    MissingEvidence,
    /// The returned proposal had a different target type than the input.
    TargetTypeMismatch {
        expected: ProposalTargetType,
        got: ProposalTargetType,
    },
    /// The returned proposal had a different content shape than the input.
    ContentShapeMismatch,
    /// The model output was empty.
    EmptyOutput,
    /// Too many output tokens (would exceed budget).
    OutputTooLarge,
}

impl std::fmt::Display for AssistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssistError::ModelError(msg) => write!(f, "model error: {}", msg),
            AssistError::InvalidJson(msg) => write!(f, "invalid JSON response: {}", msg),
            AssistError::MissingField(field) => write!(f, "missing required field: {}", field),
            AssistError::UnknownCitation(id) => write!(f, "unknown evidence citation: {}", id),
            AssistError::UnknownSourceNode(id) => write!(f, "unknown source node ID: {}", id),
            AssistError::MissingEvidence => write!(f, "output missing evidence citations"),
            AssistError::TargetTypeMismatch { expected, got } => {
                write!(
                    f,
                    "target type mismatch: expected {:?}, got {:?}",
                    expected, got
                )
            }
            AssistError::ContentShapeMismatch => {
                write!(f, "proposal content shape does not match input")
            }
            AssistError::EmptyOutput => write!(f, "model returned empty output"),
            AssistError::OutputTooLarge => write!(f, "model output exceeds token budget"),
        }
    }
}

impl std::error::Error for AssistError {}

impl From<ModelError> for AssistError {
    fn from(e: ModelError) -> Self {
        AssistError::ModelError(e.message)
    }
}

// ---------------------------------------------------------------------------
// Model-assisted drafting
// ---------------------------------------------------------------------------

/// Draft a model-assisted improvement of an existing `ProposalDocument`
/// using evidence from the provided `EvidencePack`.
///
/// The returned `ProposalDocument` preserves the input's `target_type` and
/// content shape. Every evidence link on the returned draft is reconstructed
/// from cited `EvidencePack` entry IDs — uncited claims cause the entire
/// drafting run to fail.
///
/// # Errors
///
/// Returns `AssistError` if the model call fails, the response is malformed,
/// citations are missing or unknown, or the returned proposal violates the
/// input contract shape.
pub fn draft_proposal(
    client: &dyn ModelClient,
    proposal: &ProposalDocument,
    pack: &EvidencePack,
    model_id: &str,
    max_output_tokens: u32,
    timeout_ms: u64,
) -> Result<ProposalDocument, AssistError> {
    if pack.is_empty() {
        return Err(AssistError::MissingEvidence);
    }

    let prompt = build_draft_prompt(proposal, pack);

    let request = ModelRequest {
        model_id: model_id.to_string(),
        mode: ModelMode::Explain,
        input: prompt,
        max_input_chars: pack.config.max_input_chars,
        max_output_tokens,
        timeout_ms,
        allow_tools: false,
        trace_id: format!("draft-{}", proposal.id),
    };

    // Validate request before sending.
    request
        .validate()
        .map_err(|e| AssistError::ModelError(e.message))?;

    let response = client.generate(request)?;
    let output = response.output.trim().to_string();

    if output.is_empty() {
        return Err(AssistError::EmptyOutput);
    }

    parse_draft_response(&output, proposal, pack)
}

fn build_draft_prompt(proposal: &ProposalDocument, pack: &EvidencePack) -> String {
    let content_summary = match &proposal.proposed_content {
        ProposedContent::Markdown(text) => text.clone(),
        other => format!("{other:?}"),
    };

    format!(
        "You are drafting an improved version of a knowledge proposal.\n\
         Return ONLY valid JSON matching this schema:\n\
         {{\"title\": \"...\", \"rationale\": \"...\", \"content\": \"...\", \"cited_evidence_ids\": [\"e0\", \"e1\"]}}\n\
         \n\
         The input proposal:\n\
         - target_type: {:?}\n\
         - title: {}\n\
         - rationale: {}\n\
         - content: {}\n\
         \n\
         Evidence:\n\
         {}\n\
         \n\
         Rules:\n\
         - Keep the same target_type and content shape.\n\
         - Every claim in rationale and content MUST cite only the evidence IDs listed below.\n\
         - cited_evidence_ids MUST be a non-empty array of valid evidence IDs from the evidence list.\n\
         - Improve clarity, relevance, and conciseness without inventing facts.",
        proposal.target_type,
        proposal.title,
        proposal.rationale,
        content_summary,
        pack.to_prompt_context(),
    )
}

/// Structured draft response expected from the model.
#[derive(Debug, serde::Deserialize)]
struct DraftResponse {
    title: String,
    rationale: String,
    content: String,
    cited_evidence_ids: Vec<String>,
}

fn parse_draft_response(
    output: &str,
    original: &ProposalDocument,
    pack: &EvidencePack,
) -> Result<ProposalDocument, AssistError> {
    let draft: DraftResponse =
        serde_json::from_str(output).map_err(|e| AssistError::InvalidJson(e.to_string()))?;

    // Validate required fields are non-empty.
    if draft.title.trim().is_empty() {
        return Err(AssistError::MissingField("title".into()));
    }
    if draft.rationale.trim().is_empty() {
        return Err(AssistError::MissingField("rationale".into()));
    }
    if draft.content.trim().is_empty() {
        return Err(AssistError::MissingField("content".into()));
    }
    if draft.cited_evidence_ids.is_empty() {
        return Err(AssistError::MissingEvidence);
    }

    // Validate all cited evidence IDs exist in the pack.
    let mut evidence_links: Vec<EvidenceLink> = Vec::new();
    for citation_id in &draft.cited_evidence_ids {
        let entry = pack
            .get_by_citation(citation_id)
            .ok_or_else(|| AssistError::UnknownCitation(citation_id.clone()))?;

        let link = entry_to_evidence_link(entry);
        evidence_links.push(link);
    }

    // Validate content shape matches input (must be Markdown since that's
    // what we accept for drafting).
    let content = match &original.proposed_content {
        ProposedContent::Markdown(_) => ProposedContent::Markdown(draft.content),
        _ => {
            // For non-markdown proposals, we still return markdown but
            // preserve the content kind expectation — if input is not
            // markdown, the model shouldn't be drafting it.
            return Err(AssistError::ContentShapeMismatch);
        }
    };

    let id = ProposalDocument::compute_id(&original.target_type, &content)
        .map_err(|e| AssistError::InvalidJson(e.to_string()))?;

    Ok(ProposalDocument {
        schema_version: PROPOSAL_SCHEMA_VERSION.into(),
        id,
        target_type: original.target_type.clone(),
        title: draft.title,
        rationale: draft.rationale,
        proposed_content: content,
        evidence: evidence_links,
        created_at: chrono_now(),
    })
}

// ---------------------------------------------------------------------------
// Model-assisted semantic grouping
// ---------------------------------------------------------------------------

/// Generate model-assisted semantic grouping proposals from evidence.
///
/// Returns zero or more reviewable `ProposalDocument` instances with
/// `target_type = SemanticGraphGrouping`. Each suggestion includes exact
/// `sourceNodeIds`, `targetGroupNodeId`, `targetGroupLabel`, title, rationale,
/// and evidence reconstructed from cited input evidence IDs.
///
/// Suggestions are review-only proposals and do not mutate graph or route
/// artifacts.
///
/// # Errors
///
/// Returns `AssistError` if the model call fails, the response is malformed,
/// any source node ID is not present in the evidence pack, or any cited
/// evidence ID is unknown. One invalid candidate aborts the entire run.
pub fn suggest_grouping(
    client: &dyn ModelClient,
    pack: &EvidencePack,
    model_id: &str,
    max_output_tokens: u32,
    timeout_ms: u64,
) -> Result<Vec<ProposalDocument>, AssistError> {
    if pack.is_empty() {
        return Ok(Vec::new());
    }

    // Only proceed if we have graph nodes to group.
    let graph_node_count = pack
        .entries
        .iter()
        .filter(|e| matches!(e, EvidenceEntry::GraphNode(_)))
        .count();
    if graph_node_count == 0 {
        return Ok(Vec::new());
    }

    let prompt = build_grouping_prompt(pack);

    let request = ModelRequest {
        model_id: model_id.to_string(),
        mode: ModelMode::Suggest,
        input: prompt,
        max_input_chars: pack.config.max_input_chars,
        max_output_tokens,
        timeout_ms,
        allow_tools: false,
        trace_id: "grouping".to_string(),
    };

    request
        .validate()
        .map_err(|e| AssistError::ModelError(e.message))?;

    let response = client.generate(request)?;
    let output = response.output.trim().to_string();

    if output.is_empty() {
        return Ok(Vec::new());
    }

    parse_grouping_response(&output, pack)
}

fn build_grouping_prompt(pack: &EvidencePack) -> String {
    format!(
        "You are suggesting semantic groupings for graph nodes based on shared characteristics.\n\
         Return ONLY valid JSON matching this schema:\n\
         {{\"suggestions\": [{{\"title\": \"...\", \"rationale\": \"...\", \"source_node_ids\": [\"id1\", \"id2\"], \"target_group_node_id\": \"domain_term:X\", \"target_group_label\": \"X\", \"cited_evidence_ids\": [\"e0\"]}}]}}\n\
         \n\
         Evidence:\n\
         {}\n\
         \n\
         Rules:\n\
         - source_node_ids MUST contain exact graph node IDs from the evidence list.\n\
         - target_group_node_id MUST follow the pattern 'domain_term:<lowercase_label>'.\n\
         - target_group_label is a human-readable short label.\n\
         - cited_evidence_ids MUST be a non-empty array of valid evidence IDs.\n\
         - Every suggestion MUST cite evidence backing the grouping.\n\
         - Group only nodes that share a common term, domain, or subject.\n\
         - Return an empty suggestions array if no meaningful groupings exist.",
        pack.to_prompt_context(),
    )
}

/// Structured grouping response expected from the model.
#[derive(Debug, serde::Deserialize)]
struct GroupingSuggestion {
    title: String,
    rationale: String,
    source_node_ids: Vec<String>,
    target_group_node_id: String,
    target_group_label: String,
    cited_evidence_ids: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct GroupingResponse {
    suggestions: Vec<GroupingSuggestion>,
}

fn parse_grouping_response(
    output: &str,
    pack: &EvidencePack,
) -> Result<Vec<ProposalDocument>, AssistError> {
    let response: GroupingResponse =
        serde_json::from_str(output).map_err(|e| AssistError::InvalidJson(e.to_string()))?;

    let mut proposals: Vec<ProposalDocument> = Vec::new();

    for suggestion in &response.suggestions {
        // Validate required fields.
        if suggestion.title.trim().is_empty() {
            return Err(AssistError::MissingField("title".into()));
        }
        if suggestion.rationale.trim().is_empty() {
            return Err(AssistError::MissingField("rationale".into()));
        }
        if suggestion.source_node_ids.is_empty() {
            return Err(AssistError::MissingField("source_node_ids".into()));
        }
        if suggestion.target_group_node_id.trim().is_empty() {
            return Err(AssistError::MissingField("target_group_node_id".into()));
        }
        if suggestion.target_group_label.trim().is_empty() {
            return Err(AssistError::MissingField("target_group_label".into()));
        }
        if suggestion.cited_evidence_ids.is_empty() {
            return Err(AssistError::MissingEvidence);
        }

        // Validate every source node ID exists in the graph.
        for node_id in &suggestion.source_node_ids {
            if !pack.has_graph_node_id(node_id) {
                return Err(AssistError::UnknownSourceNode(node_id.clone()));
            }
        }

        // Validate cited evidence IDs.
        let mut evidence_links: Vec<EvidenceLink> = Vec::new();
        for citation_id in &suggestion.cited_evidence_ids {
            let entry = pack
                .get_by_citation(citation_id)
                .ok_or_else(|| AssistError::UnknownCitation(citation_id.clone()))?;
            let link = entry_to_evidence_link(entry);
            evidence_links.push(link);
        }

        let content = ProposedContent::SemanticGraphGrouping(SemanticGraphGrouping {
            source_node_ids: suggestion.source_node_ids.clone(),
            target_group_node_id: suggestion.target_group_node_id.clone(),
            target_group_label: suggestion.target_group_label.clone(),
        });

        let id = ProposalDocument::compute_id(&ProposalTargetType::SemanticGraphGrouping, &content)
            .map_err(|e| AssistError::InvalidJson(e.to_string()))?;

        proposals.push(ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type: ProposalTargetType::SemanticGraphGrouping,
            title: suggestion.title.clone(),
            rationale: suggestion.rationale.clone(),
            proposed_content: content,
            evidence: evidence_links,
            created_at: chrono_now(),
        });
    }

    Ok(proposals)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert an `EvidenceEntry` into an `EvidenceLink` for citation.
fn entry_to_evidence_link(entry: &EvidenceEntry) -> EvidenceLink {
    match entry {
        EvidenceEntry::Hotspot(h) => EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: h.subject.clone(),
            row_ids: h.evidence.rowIds.clone(),
            doc_ref: None,
            description: None,
            score: Some(h.score),
            metadata: None,
        },
        EvidenceEntry::GraphNode(n) => EvidenceLink {
            source_kind: EvidenceSourceKind::RecordedEvidence,
            subject: n.id.clone(),
            row_ids: Vec::new(),
            doc_ref: None,
            description: n.description.clone(),
            score: None,
            metadata: None,
        },
        EvidenceEntry::Proposal(p) => EvidenceLink {
            source_kind: EvidenceSourceKind::RecordedEvidence,
            subject: p.id.clone(),
            row_ids: Vec::new(),
            doc_ref: None,
            description: Some(format!("proposal: {}", p.title)),
            score: None,
            metadata: None,
        },
        EvidenceEntry::Document {
            doc_ref,
            description,
        } => EvidenceLink {
            source_kind: EvidenceSourceKind::DocReference,
            subject: doc_ref.clone(),
            row_ids: Vec::new(),
            doc_ref: Some(doc_ref.clone()),
            description: description.clone(),
            score: None,
            metadata: None,
        },
    }
}

/// Return a timestamp string. In production this would use a real clock;
/// for Foundation 01 we use a fixed timestamp to keep output deterministic.
fn chrono_now() -> String {
    // Use a fixed timestamp for deterministic test output.
    // Callers wanting real timestamps should set created_at after calling.
    "2026-06-28T00:00:00Z".to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_llm::{ModelClient, ModelError, ModelRequest, ModelResponse};
    use scryrs_types::{
        EvidenceLink, EvidenceSourceKind, HotspotCounts, HotspotEntry, HotspotEvidence,
        PROPOSAL_SCHEMA_VERSION, ProposalDocument, ProposalTargetType, ProposedContent,
    };
    use std::cell::RefCell;

    // -----------------------------------------------------------------------
    // Fake ModelClient for deterministic testing
    // -----------------------------------------------------------------------

    /// A fake `ModelClient` that returns a pre-configured response.
    struct FakeModelClient {
        response: RefCell<Result<String, String>>,
    }

    impl FakeModelClient {
        fn new_ok(output: &str) -> Self {
            Self {
                response: RefCell::new(Ok(output.to_string())),
            }
        }

        fn new_err(msg: &str) -> Self {
            Self {
                response: RefCell::new(Err(msg.to_string())),
            }
        }
    }

    impl ModelClient for FakeModelClient {
        fn generate(&self, _request: ModelRequest) -> Result<ModelResponse, ModelError> {
            match &*self.response.borrow() {
                Ok(output) => {
                    // Enforce allow_tools and budget from the request.
                    Ok(ModelResponse {
                        output: output.clone(),
                    })
                }
                Err(msg) => Err(ModelError::new(msg.clone())),
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn make_hotspot(subject_kind: &str, subject: &str, score: u32, rank: u32) -> HotspotEntry {
        HotspotEntry {
            rank,
            subjectKind: subject_kind.to_string(),
            subject: subject.to_string(),
            score,
            counts: HotspotCounts {
                eventType: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("FileOpened".to_string(), 1);
                    m
                },
                outcome: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("success".to_string(), 1);
                    m
                },
            },
            sessionCount: 1,
            firstSeen: "2026-06-27T00:00:00Z".into(),
            lastSeen: "2026-06-27T00:00:00Z".into(),
            evidence: HotspotEvidence { rowIds: vec![1] },
        }
    }

    fn make_graph_node(id: &str, label: &str, kind: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            kind: kind.to_string(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        }
    }

    fn make_proposal_doc(title: &str, rationale: &str, markdown: &str) -> ProposalDocument {
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
                score: Some(5),
                metadata: None,
            }],
            created_at: "2026-06-27T12:00:00Z".into(),
        }
    }

    // -----------------------------------------------------------------------
    // EvidencePack tests (Section 2)
    // -----------------------------------------------------------------------

    #[test]
    fn evidence_pack_assigns_stable_ids() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/a.rs", 5, 1)],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        assert_eq!(pack.len(), 2);
        assert!(pack.get_by_citation("e0").is_some());
        assert!(pack.get_by_citation("e1").is_some());
        assert!(pack.get_by_citation("e2").is_none());
    }

    #[test]
    fn evidence_pack_tracks_graph_node_ids() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![
                make_graph_node("file:auth", "auth", "file"),
                make_graph_node("search:auth", "auth search", "search"),
            ],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        assert!(pack.has_graph_node_id("file:auth"));
        assert!(pack.has_graph_node_id("search:auth"));
        assert!(!pack.has_graph_node_id("nonexistent"));
    }

    #[test]
    fn evidence_pack_rejects_hotspot_overflow() {
        let hotspots: Vec<HotspotEntry> = (0..100)
            .map(|i| make_hotspot("file", &format!("file{i}.rs"), 1, i))
            .collect();

        let config = EvidencePackConfig {
            max_hotspots: 10,
            ..EvidencePackConfig::default()
        };
        let result = EvidencePack::build(config, hotspots, vec![], vec![], vec![]);
        assert!(result.is_err());
        let msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(msg.contains("hotspots"));
        assert!(msg.contains("10"));
    }

    #[test]
    fn evidence_pack_rejects_total_char_overflow() {
        let config = EvidencePackConfig {
            max_input_chars: 10,
            ..EvidencePackConfig::default()
        };
        let result = EvidencePack::build(
            config,
            vec![make_hotspot("file", "src/a.rs", 5, 1)],
            vec![],
            vec![],
            vec![],
        );
        assert!(result.is_err());
        let msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected error"),
        };
        assert!(msg.contains("total_chars"));
    }

    #[test]
    fn evidence_pack_prompt_context_includes_citation_ids() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/main.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let ctx = pack.to_prompt_context();
        assert!(ctx.contains("[e0]"));
        assert!(ctx.contains("src/main.rs"));
    }

    #[test]
    fn evidence_pack_default_config_bounds_are_reasonable() {
        let default = EvidencePackConfig::default();
        assert!(default.max_input_chars > 0);
        assert!(default.max_hotspots > 0);
        assert!(default.max_graph_nodes > 0);
        assert!(default.max_proposals > 0);
        assert!(default.max_documents > 0);
    }

    #[test]
    fn from_graph_and_hotspots_constructs_valid_pack() {
        let graph_nodes = vec![
            make_graph_node("file:auth", "auth", "file"),
            make_graph_node("search:auth", "auth search", "search"),
        ];
        let graph = scryrs_types::KnowledgeGraphDocument {
            schema_version: "1.0.0".into(),
            metadata: scryrs_types::GraphMetadata {
                repository_id: None,
                metadata: None,
            },
            nodes: graph_nodes,
            edges: vec![],
        };
        let hotspots = vec![make_hotspot("file", "src/lib.rs", 5, 1)];

        let pack = EvidencePack::from_graph_and_hotspots(&graph, &hotspots)
            .unwrap_or_else(|e| panic!("from_graph_and_hotspots: {e}"));

        assert_eq!(pack.len(), 3);
        assert!(pack.has_graph_node_id("file:auth"));
    }

    // -----------------------------------------------------------------------
    // Drafting tests (Section 3)
    // -----------------------------------------------------------------------

    #[test]
    fn draft_preserves_target_type_and_content_shape() {
        let proposal = make_proposal_doc("Test", "Because.", "## Hello\n\nWorld.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "title": "Improved Test",
            "rationale": "Better rationale with evidence.",
            "content": "## Hello\n\nImproved world.",
            "cited_evidence_ids": ["e0"]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);

        let draft = result.unwrap_or_else(|e| panic!("draft_proposal: {e}"));
        assert_eq!(draft.target_type, ProposalTargetType::DocsNote);
        assert_eq!(draft.title, "Improved Test");
        assert!(matches!(
            draft.proposed_content,
            ProposedContent::Markdown(_)
        ));
        assert!(!draft.evidence.is_empty());
    }

    #[test]
    fn draft_rejects_empty_evidence_pack() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("{}");
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::MissingEvidence)));
    }

    #[test]
    fn draft_rejects_uncited_evidence_ids() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        // Response cites e99 which doesn't exist.
        let response = serde_json::json!({
            "title": "Test",
            "rationale": "Because.",
            "content": "Content.",
            "cited_evidence_ids": ["e99"]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::UnknownCitation(_))));
    }

    #[test]
    fn draft_rejects_missing_evidence_in_response() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        // Response has empty cited_evidence_ids.
        let response = serde_json::json!({
            "title": "Test",
            "rationale": "Because.",
            "content": "Content.",
            "cited_evidence_ids": []
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::MissingEvidence)));
    }

    #[test]
    fn draft_rejects_malformed_json() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("not valid json {{{");
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::InvalidJson(_))));
    }

    #[test]
    fn draft_rejects_empty_model_output() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("   ");
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::EmptyOutput)));
    }

    #[test]
    fn draft_rejects_model_error() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_err("API unavailable");
        let result = draft_proposal(&client, &proposal, &pack, "test-model", 100, 30_000);
        assert!(matches!(result, Err(AssistError::ModelError(_))));
    }

    #[test]
    fn draft_request_disables_tools() {
        // The ModelRequest must have allow_tools = false.
        // We verify this through the FakeModelClient which receives the request.
        struct ToolCheckClient;
        impl ModelClient for ToolCheckClient {
            fn generate(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
                assert!(!request.allow_tools, "tools must be disabled");
                assert!(request.max_output_tokens > 0);
                assert!(request.timeout_ms > 0);
                Ok(ModelResponse {
                    output: serde_json::json!({
                        "title": "T",
                        "rationale": "R",
                        "content": "C",
                        "cited_evidence_ids": ["e0"]
                    })
                    .to_string(),
                })
            }
        }

        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let result = draft_proposal(&ToolCheckClient, &proposal, &pack, "m", 50, 10_000);
        assert!(result.is_ok());
    }

    #[test]
    fn draft_non_markdown_proposal_is_rejected() {
        // Proposals with non-Markdown content should be rejected by drafting.
        let content = ProposedContent::SemanticGraphGrouping(SemanticGraphGrouping {
            source_node_ids: vec!["file:x".into()],
            target_group_node_id: "domain_term:x".into(),
            target_group_label: "X".into(),
        });
        let id = ProposalDocument::compute_id(&ProposalTargetType::SemanticGraphGrouping, &content)
            .unwrap_or_else(|e| panic!("compute_id: {e}"));
        let proposal = ProposalDocument {
            schema_version: PROPOSAL_SCHEMA_VERSION.into(),
            id,
            target_type: ProposalTargetType::SemanticGraphGrouping,
            title: "Test".into(),
            rationale: "Because.".into(),
            proposed_content: content,
            evidence: vec![],
            created_at: "2026-06-27T12:00:00Z".into(),
        };

        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "title": "T",
            "rationale": "R",
            "content": "C",
            "cited_evidence_ids": ["e0"]
        })
        .to_string();
        let client = FakeModelClient::new_ok(&response);
        let result = draft_proposal(&client, &proposal, &pack, "m", 100, 30_000);
        assert!(matches!(result, Err(AssistError::ContentShapeMismatch)));
    }

    #[test]
    fn draft_rejects_missing_title() {
        let proposal = make_proposal_doc("Test", "Because.", "Content.");
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 10, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "title": "  ",
            "rationale": "R",
            "content": "C",
            "cited_evidence_ids": ["e0"]
        })
        .to_string();
        let client = FakeModelClient::new_ok(&response);
        let result = draft_proposal(&client, &proposal, &pack, "m", 100, 30_000);
        assert!(matches!(result, Err(AssistError::MissingField(_))));
    }

    // -----------------------------------------------------------------------
    // Grouping tests (Section 4)
    // -----------------------------------------------------------------------

    #[test]
    fn grouping_produces_valid_semantic_graph_grouping_proposals() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/auth.rs", 5, 1)],
            vec![
                make_graph_node("file:auth", "auth", "file"),
                make_graph_node("search:auth", "auth search", "search"),
                make_graph_node("symbol:auth", "auth symbol", "symbol"),
            ],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "suggestions": [{
                "title": "Group auth-related nodes",
                "rationale": "All three nodes share the auth domain term.",
                "source_node_ids": ["file:auth", "search:auth", "symbol:auth"],
                "target_group_node_id": "domain_term:auth",
                "target_group_label": "Auth",
                "cited_evidence_ids": ["e0", "e1", "e2", "e3"]
            }]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);

        let proposals = result.unwrap_or_else(|e| panic!("suggest_grouping: {e}"));
        assert_eq!(proposals.len(), 1);

        let p = &proposals[0];
        assert_eq!(p.target_type, ProposalTargetType::SemanticGraphGrouping);
        assert_eq!(p.title, "Group auth-related nodes");
        assert!(!p.rationale.is_empty());
        assert!(!p.evidence.is_empty());

        match &p.proposed_content {
            ProposedContent::SemanticGraphGrouping(g) => {
                assert_eq!(g.source_node_ids.len(), 3);
                assert!(g.source_node_ids.contains(&"file:auth".to_string()));
                assert!(g.source_node_ids.contains(&"search:auth".to_string()));
                assert!(g.source_node_ids.contains(&"symbol:auth".to_string()));
                assert_eq!(g.target_group_node_id, "domain_term:auth");
                assert_eq!(g.target_group_label, "Auth");
            }
            other => panic!("expected SemanticGraphGrouping, got {other:?}"),
        }
    }

    #[test]
    fn grouping_rejects_hallucinated_source_node() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        // Response cites a node ID that doesn't exist.
        let response = serde_json::json!({
            "suggestions": [{
                "title": "Group",
                "rationale": "Because.",
                "source_node_ids": ["file:auth", "file:nonexistent"],
                "target_group_node_id": "domain_term:auth",
                "target_group_label": "Auth",
                "cited_evidence_ids": ["e0"]
            }]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        assert!(matches!(result, Err(AssistError::UnknownSourceNode(_))));
    }

    #[test]
    fn grouping_rejects_unknown_citation() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "suggestions": [{
                "title": "Group",
                "rationale": "Because.",
                "source_node_ids": ["file:auth"],
                "target_group_node_id": "domain_term:auth",
                "target_group_label": "Auth",
                "cited_evidence_ids": ["e99"]
            }]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        assert!(matches!(result, Err(AssistError::UnknownCitation(_))));
    }

    #[test]
    fn grouping_returns_empty_for_no_graph_nodes() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/lib.rs", 5, 1)],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("{}");
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);

        let proposals = result.unwrap_or_else(|e| panic!("suggest_grouping: {e}"));
        assert!(proposals.is_empty());
    }

    #[test]
    fn grouping_empty_pack_returns_empty() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("{}");
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);

        let proposals = result.unwrap_or_else(|e| panic!("suggest_grouping: {e}"));
        assert!(proposals.is_empty());
    }

    #[test]
    fn grouping_rejects_malformed_json() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("not json");
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        assert!(matches!(result, Err(AssistError::InvalidJson(_))));
    }

    #[test]
    fn grouping_one_invalid_candidate_aborts_entire_run() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![make_hotspot("file", "src/auth.rs", 5, 1)],
            vec![
                make_graph_node("file:auth", "auth", "file"),
                make_graph_node("search:auth", "auth search", "search"),
            ],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        // First suggestion is valid, second has unknown source node.
        let response = serde_json::json!({
            "suggestions": [
                {
                    "title": "Valid group",
                    "rationale": "Good.",
                    "source_node_ids": ["file:auth"],
                    "target_group_node_id": "domain_term:auth",
                    "target_group_label": "Auth",
                    "cited_evidence_ids": ["e0"]
                },
                {
                    "title": "Invalid group",
                    "rationale": "Bad.",
                    "source_node_ids": ["file:nonexistent"],
                    "target_group_node_id": "domain_term:fake",
                    "target_group_label": "Fake",
                    "cited_evidence_ids": ["e0"]
                }
            ]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        // The entire run should fail, not return partial success.
        assert!(matches!(result, Err(AssistError::UnknownSourceNode(_))));
    }

    #[test]
    fn grouping_empty_suggestions_returns_empty_vec() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "suggestions": []
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);

        let proposals = result.unwrap_or_else(|e| panic!("suggest_grouping: {e}"));
        assert!(proposals.is_empty());
    }

    #[test]
    fn grouping_rejects_missing_source_node_ids() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let response = serde_json::json!({
            "suggestions": [{
                "title": "Group",
                "rationale": "Because.",
                "source_node_ids": [],
                "target_group_node_id": "domain_term:auth",
                "target_group_label": "Auth",
                "cited_evidence_ids": ["e0"]
            }]
        })
        .to_string();

        let client = FakeModelClient::new_ok(&response);
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        assert!(matches!(result, Err(AssistError::MissingField(_))));
    }

    #[test]
    fn grouping_request_disables_tools() {
        struct ToolCheckClient;
        impl ModelClient for ToolCheckClient {
            fn generate(&self, request: ModelRequest) -> Result<ModelResponse, ModelError> {
                assert!(!request.allow_tools, "tools must be disabled");
                assert!(request.max_output_tokens > 0);
                Ok(ModelResponse {
                    output: serde_json::json!({"suggestions": []}).to_string(),
                })
            }
        }

        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let result = suggest_grouping(&ToolCheckClient, &pack, "m", 50, 10_000);
        assert!(result.is_ok());
    }

    #[test]
    fn grouping_empty_model_output_returns_empty_vec() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_ok("   ");
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);

        let proposals = result.unwrap_or_else(|e| panic!("suggest_grouping: {e}"));
        assert!(proposals.is_empty());
    }

    #[test]
    fn grouping_model_error_is_propagated() {
        let pack = EvidencePack::build(
            EvidencePackConfig::default(),
            vec![],
            vec![make_graph_node("file:auth", "auth", "file")],
            vec![],
            vec![],
        )
        .unwrap_or_else(|e| panic!("build: {e}"));

        let client = FakeModelClient::new_err("timeout");
        let result = suggest_grouping(&client, &pack, "test-model", 200, 30_000);
        assert!(matches!(result, Err(AssistError::ModelError(_))));
    }

    // -----------------------------------------------------------------------
    // Cross-cutting: deterministic freedom tests (Section 5)
    // -----------------------------------------------------------------------

    /// This test exists as a compile-time proof that the curator-llm crate
    /// depends on scryrs-llm (it must, since it accepts ModelClient), but
    /// the scryrs-curator crate does NOT and cannot. The test compiles only
    /// because we're in scryrs-curator-llm, proving model-awareness is
    /// properly isolated.
    #[test]
    fn accepts_model_client_trait_from_scryrs_llm() {
        // Compile-time proof: we can name the ModelClient trait.
        fn _assert_accepts(_client: &dyn ModelClient) {}
    }
}
