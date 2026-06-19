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
}
