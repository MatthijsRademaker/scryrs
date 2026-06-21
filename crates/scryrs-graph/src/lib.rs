//! Knowledge graph and routing manifest foundation.

use scryrs_types::{FeatureDescriptor, GraphNode};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "graph",
        title: "scryrs-graph",
        summary: "knowledge graph and routing manifest foundation",
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeGraph {
    nodes: Vec<GraphNode>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[GraphNode] {
        &self.nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_empty() {
        assert!(KnowledgeGraph::new().nodes().is_empty());
    }

    #[test]
    fn add_node_grows_graph() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "n1".to_string(),
            title: "Node One".to_string(),
        });
        assert_eq!(graph.nodes().len(), 1);
        assert_eq!(graph.nodes()[0].id, "n1");
    }

    #[test]
    fn multiple_nodes_preserved_in_order() {
        let mut graph = KnowledgeGraph::new();
        graph.add_node(GraphNode {
            id: "a".to_string(),
            title: "First".to_string(),
        });
        graph.add_node(GraphNode {
            id: "b".to_string(),
            title: "Second".to_string(),
        });
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
}
