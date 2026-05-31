// src/graph/builder.rs
//! Graph builder for constructing execution graphs

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NodeSpec {
    pub id: String,
    pub node_type: String,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

#[derive(Debug)]
pub struct GraphBuilder {
    pub nodes: Vec<NodeSpec>,
    pub edges: Vec<Edge>,
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, spec: NodeSpec) -> &mut Self {
        self.nodes.push(spec);
        self
    }

    pub fn add_edge(&mut self, edge: Edge) -> &mut Self {
        self.edges.push(edge);
        self
    }
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
