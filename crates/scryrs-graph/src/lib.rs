//! Knowledge graph and routing manifest foundation.

use std::collections::HashSet;

use scryrs_types::{
    EvidenceLink, FeatureDescriptor, GRAPH_SCHEMA_VERSION, GraphEdge, GraphMetadata, GraphNode,
    KnowledgeGraphDocument,
};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "graph",
        title: "scryrs-graph",
        summary: "knowledge graph and routing manifest foundation",
    }
}

/// Container that holds graph nodes and directed edges under the shared
/// `scryrs-types` contract. Validates structural references, preserves
/// evidence links, and materializes a deterministic `KnowledgeGraphDocument`.
///
/// This container does not implement graph build, CLI commands,
/// route-manifest generation, docs crawling, adapter integration, server
/// endpoints, or runtime retrieval.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeGraph {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    /// Add a directed edge to the graph. Validation happens at
    /// materialization time rather than insertion time so the consumer
    /// can add nodes and edges in any order.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Return a snapshot of the current nodes.
    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }

    /// Return a snapshot of the current edges.
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Validate structural invariants: every edge `source_node_id` and
    /// `target_node_id` must reference an existing node ID.
    pub fn validate(&self) -> Result<(), String> {
        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        for edge in &self.edges {
            if !node_ids.contains(edge.source_node_id.as_str()) {
                return Err(format!(
                    "edge '{}' source_node_id '{}' does not reference an existing node",
                    edge.id, edge.source_node_id
                ));
            }
            if !node_ids.contains(edge.target_node_id.as_str()) {
                return Err(format!(
                    "edge '{}' target_node_id '{}' does not reference an existing node",
                    edge.id, edge.target_node_id
                ));
            }
        }

        Ok(())
    }

    /// Materialize a deterministically ordered `KnowledgeGraphDocument`.
    ///
    /// Ordering rules:
    /// - `nodes` sort by `id` ascending.
    /// - `edges` sort by `id` ascending.
    /// - Node `tags`, node `aliases`, and edge `tags` sort lexicographically.
    /// - `evidence_links` sort by `(sourceKind, subject, docRef, description,
    ///   rowIds, score)` ascending, treating missing string fields as empty
    ///   values and comparing `rowIds` lexicographically as ordered lists.
    ///
    /// Returns an error if structural validation fails.
    pub fn to_document(
        &self,
        repository_id: Option<String>,
    ) -> Result<KnowledgeGraphDocument, String> {
        self.validate()?;

        let mut nodes: Vec<GraphNode> = self.nodes.clone();
        let mut edges: Vec<GraphEdge> = self.edges.clone();

        // Deterministic ordering for nodes and edges by id.
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        // Sort tags, aliases, and evidence_links within each node/edge.
        for node in &mut nodes {
            sort_node_collections(node);
        }
        for edge in &mut edges {
            sort_edge_collections(edge);
        }

        Ok(KnowledgeGraphDocument {
            schema_version: GRAPH_SCHEMA_VERSION.to_string(),
            metadata: GraphMetadata {
                repository_id,
                metadata: None,
            },
            nodes,
            edges,
        })
    }
}

/// Sort the collections within a `GraphNode` for deterministic output.
fn sort_node_collections(node: &mut GraphNode) {
    node.tags.sort();
    node.aliases.sort();
    sort_evidence_links(&mut node.evidence_links);
}

/// Sort the collections within a `GraphEdge` for deterministic output.
fn sort_edge_collections(edge: &mut GraphEdge) {
    edge.tags.sort();
    sort_evidence_links(&mut edge.evidence_links);
}

/// Sort `EvidenceLink`s by the documented tie-break chain:
/// `(sourceKind, subject, docRef, description, rowIds, score)` ascending.
fn sort_evidence_links(links: &mut [EvidenceLink]) {
    links.sort_by(|a, b| {
        a.source_kind
            .cmp(&b.source_kind)
            .then_with(|| a.subject.cmp(&b.subject))
            .then_with(|| {
                a.doc_ref
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.doc_ref.as_deref().unwrap_or(""))
            })
            .then_with(|| {
                a.description
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.description.as_deref().unwrap_or(""))
            })
            .then_with(|| a.row_ids.cmp(&b.row_ids))
            .then_with(|| a.score.cmp(&b.score))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, label: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            label: label.to_string(),
            description: None,
            kind: "concept".to_string(),
            tags: vec![],
            aliases: vec![],
            evidence_links: vec![],
            metadata: None,
        }
    }

    fn make_edge(id: &str, source: &str, target: &str, relationship: &str) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source_node_id: source.to_string(),
            target_node_id: target.to_string(),
            relationship: relationship.to_string(),
            label: None,
            tags: vec![],
            evidence_links: vec![],
            metadata: None,
        }
    }

    #[test]
    fn starts_empty() {
        let graph = KnowledgeGraph::new();
        assert!(graph.nodes().is_empty());
        assert!(graph.edges().is_empty());
    }

    #[test]
    fn add_node_grows_graph() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "Node One"));
        assert_eq!(graph.nodes().len(), 1);
        assert_eq!(graph.nodes()[0].id, "n1");
    }

    #[test]
    fn add_edge_grows_graph() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("a", "A"));
        graph.add_node(make_node("b", "B"));
        graph.add_edge(make_edge("e1", "a", "b", "depends_on"));
        assert_eq!(graph.edges().len(), 1);
        assert_eq!(graph.edges()[0].id, "e1");
    }

    #[test]
    fn multiple_nodes_preserved_in_order() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("a", "First"));
        graph.add_node(make_node("b", "Second"));
        assert_eq!(graph.nodes().len(), 2);
        assert_eq!(graph.nodes()[0].id, "a");
        assert_eq!(graph.nodes()[1].id, "b");
    }

    #[test]
    fn new_and_default_are_equivalent() {
        let a = KnowledgeGraph::new();
        let b = KnowledgeGraph::default();
        assert_eq!(a, b);
    }

    // --- Structural validation ---

    #[test]
    fn validate_accepts_valid_graph() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_node(make_node("n2", "Two"));
        graph.add_edge(make_edge("e1", "n1", "n2", "relates_to"));
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn validate_accepts_graph_with_no_edges() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn validate_accepts_empty_graph() {
        let graph = KnowledgeGraph::new();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn validate_rejects_dangling_source_node() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_edge(make_edge("e1", "n1", "nonexistent", "relates_to"));
        let err = match graph.validate() {
            Err(e) => e,
            Ok(()) => panic!("validate should reject dangling target_node_id"),
        };
        assert!(err.contains("target_node_id"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn validate_rejects_dangling_target_node() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_edge(make_edge("e1", "nonexistent", "n1", "relates_to"));
        let err = match graph.validate() {
            Err(e) => e,
            Ok(()) => panic!("validate should reject dangling source_node_id"),
        };
        assert!(err.contains("source_node_id"));
        assert!(err.contains("nonexistent"));
    }

    #[test]
    fn validate_rejects_both_ends_dangling() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_edge(make_edge("e1", "ghost_a", "ghost_b", "relates_to"));
        let err = match graph.validate() {
            Err(e) => e,
            Ok(()) => panic!("validate should reject both ends dangling"),
        };
        // Should catch the first problem (source_node_id).
        assert!(err.contains("source_node_id"));
        assert!(err.contains("ghost_a"));
    }

    // --- Deterministic materialization ---

    #[test]
    fn to_document_sorts_nodes_by_id() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("c", "C"));
        graph.add_node(make_node("a", "A"));
        graph.add_node(make_node("b", "B"));
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(doc.nodes[0].id, "a");
        assert_eq!(doc.nodes[1].id, "b");
        assert_eq!(doc.nodes[2].id, "c");
    }

    #[test]
    fn to_document_sorts_edges_by_id() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_node(make_node("n2", "Two"));
        graph.add_node(make_node("n3", "Three"));
        graph.add_edge(make_edge("e3", "n1", "n2", "r"));
        graph.add_edge(make_edge("e1", "n2", "n3", "r"));
        graph.add_edge(make_edge("e2", "n3", "n1", "r"));
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(doc.edges[0].id, "e1");
        assert_eq!(doc.edges[1].id, "e2");
        assert_eq!(doc.edges[2].id, "e3");
    }

    #[test]
    fn to_document_sorts_tags_and_aliases() {
        let mut graph = KnowledgeGraph::new();
        let mut node = make_node("n1", "One");
        node.tags = vec!["zebra".into(), "alpha".into(), "mike".into()];
        node.aliases = vec!["gamma".into(), "beta".into(), "delta".into()];
        graph.add_node(node);
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(doc.nodes[0].tags, vec!["alpha", "mike", "zebra"]);
        assert_eq!(doc.nodes[0].aliases, vec!["beta", "delta", "gamma"]);
    }

    #[test]
    fn to_document_sorts_edge_tags() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_node(make_node("n2", "Two"));
        let mut edge = make_edge("e1", "n1", "n2", "r");
        edge.tags = vec!["ccc".into(), "aaa".into(), "bbb".into()];
        graph.add_edge(edge);
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(doc.edges[0].tags, vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn to_document_sorts_evidence_links() {
        let mut graph = KnowledgeGraph::new();
        let mut node = make_node("n1", "One");

        // Construct links in intentionally unsorted order.
        use scryrs_types::EvidenceSourceKind;
        let link_a = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: "s1".into(),
            row_ids: vec![2, 1],
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        };
        let link_b = EvidenceLink {
            source_kind: EvidenceSourceKind::HotspotSubject,
            subject: "s1".into(),
            row_ids: vec![],
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        };
        let link_c = EvidenceLink {
            source_kind: EvidenceSourceKind::LocalTraceRow,
            subject: "s1".into(),
            row_ids: vec![1, 1],
            doc_ref: None,
            description: None,
            score: None,
            metadata: None,
        };
        node.evidence_links = vec![link_a, link_b, link_c];
        graph.add_node(node);

        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        let links = &doc.nodes[0].evidence_links;
        assert_eq!(links[0].source_kind, EvidenceSourceKind::HotspotSubject);
        assert_eq!(links[1].source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(links[1].row_ids, vec![1, 1]);
        assert_eq!(links[2].source_kind, EvidenceSourceKind::LocalTraceRow);
        assert_eq!(links[2].row_ids, vec![2, 1]);
    }

    #[test]
    fn to_document_includes_schema_version() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(doc.schema_version, GRAPH_SCHEMA_VERSION);
    }

    #[test]
    fn to_document_preserves_repository_id() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        let doc = match graph.to_document(Some("github.com/example/repo".into())) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert_eq!(
            doc.metadata.repository_id.as_deref(),
            Some("github.com/example/repo")
        );
    }

    #[test]
    fn to_document_without_repository_id() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        let doc = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        assert!(doc.metadata.repository_id.is_none());
    }

    #[test]
    fn to_document_fails_on_dangling_edge() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("n1", "One"));
        graph.add_edge(make_edge("e1", "n1", "ghost", "r"));
        assert!(graph.to_document(None).is_err());
    }

    #[test]
    fn repeated_materialization_is_idempotent() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(make_node("c", "C"));
        graph.add_node(make_node("a", "A"));
        graph.add_node(make_node("b", "B"));
        graph.add_node(make_node("d", "D"));

        graph.add_edge(make_edge("e_zz", "a", "b", "r"));
        graph.add_edge(make_edge("e_aa", "b", "c", "r"));

        let doc1 = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize: {e}"),
        };
        let doc2 = match graph.to_document(None) {
            Ok(d) => d,
            Err(e) => panic!("valid graph must materialize (second call): {e}"),
        };
        assert_eq!(doc1, doc2);
    }
}
