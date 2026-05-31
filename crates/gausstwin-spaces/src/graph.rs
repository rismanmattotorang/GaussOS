use crate::{
    AgentId, Space, SpaceResult,
    common::{DistanceMetric, MemoryPool},
    error::helpers::*,
};
use nalgebra::Point3;
use parking_lot::RwLock;
use petgraph::{
    graph::{DiGraph, NodeIndex, UnGraph},
    visit::Bfs,
    algo::{dijkstra, bellman_ford, astar},
};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Graph type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphType {
    Directed,
    Undirected,
}

/// Edge weight type
#[derive(Debug, Clone)]
pub struct EdgeWeight {
    /// Distance/cost of traversing this edge
    pub distance: f64,
    /// Additional properties
    pub properties: HashMap<String, f64>,
}

impl Default for EdgeWeight {
    fn default() -> Self {
        Self {
            distance: 1.0,
            properties: HashMap::new(),
        }
    }
}

/// Graph space configuration
#[derive(Debug, Clone)]
pub struct GraphSpaceConfig {
    /// Type of graph
    pub graph_type: GraphType,
    /// Whether to use edge weights
    pub weighted: bool,
    /// Whether to use community detection
    pub use_communities: bool,
    /// Whether to use property layers
    pub use_property_layers: bool,
}

impl Default for GraphSpaceConfig {
    fn default() -> Self {
        Self {
            graph_type: GraphType::Undirected,
            weighted: true,
            use_communities: false,
            use_property_layers: false,
        }
    }
}

/// High-performance graph space implementation
pub struct GraphSpace {
    config: GraphSpaceConfig,
    directed_graph: RwLock<Option<DiGraph<AgentData, EdgeWeight>>>,
    undirected_graph: RwLock<Option<UnGraph<AgentData, EdgeWeight>>>,
    node_indices: RwLock<HashMap<AgentId, NodeIndex>>,
    communities: RwLock<HashMap<AgentId, usize>>,
    property_layers: RwLock<HashMap<String, Box<dyn PropertyLayer>>>,
    path_cache: RwLock<lru::LruCache<PathKey, Vec<AgentId>>>,
    memory_pool: Arc<MemoryPool<Vec<AgentId>>>,
}

/// Data stored in graph nodes
#[derive(Debug, Clone)]
struct AgentData {
    id: AgentId,
    position: Point3<f64>,
    properties: HashMap<String, f64>,
}

/// Property layer trait for graph space
pub trait PropertyLayer: Send + Sync {
    /// Get property value for an agent
    fn get_value(&self, agent: AgentId) -> f64;
    
    /// Set property value for an agent
    fn set_value(&self, agent: AgentId, value: f64);
    
    /// Apply function to all values in parallel
    fn par_apply<F>(&self, f: F)
    where
        F: Fn(f64) -> f64 + Send + Sync;
}

#[derive(Hash, Eq, PartialEq)]
struct PathKey {
    start: AgentId,
    end: AgentId,
}

impl GraphSpace {
    /// Create a new graph space
    pub fn new(config: GraphSpaceConfig) -> Self {
        let (directed_graph, undirected_graph) = match config.graph_type {
            GraphType::Directed => (Some(DiGraph::new()), None),
            GraphType::Undirected => (None, Some(UnGraph::new())),
        };
        
        Self {
            config,
            directed_graph: RwLock::new(directed_graph),
            undirected_graph: RwLock::new(undirected_graph),
            node_indices: RwLock::new(HashMap::new()),
            communities: RwLock::new(HashMap::new()),
            property_layers: RwLock::new(HashMap::new()),
            path_cache: RwLock::new(lru::LruCache::new(1000)),
            memory_pool: Arc::new(MemoryPool::new(1000)),
        }
    }
    
    /// Add an edge between agents
    pub fn add_edge(&self, from: AgentId, to: AgentId, weight: EdgeWeight) -> SpaceResult<()> {
        let node_indices = self.node_indices.read();
        let from_idx = node_indices.get(&from).ok_or_else(|| agent_not_found(from))?;
        let to_idx = node_indices.get(&to).ok_or_else(|| agent_not_found(to))?;
        
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &mut *self.directed_graph.write() {
                    graph.add_edge(*from_idx, *to_idx, weight);
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &mut *self.undirected_graph.write() {
                    graph.add_edge(*from_idx, *to_idx, weight);
                }
            }
        }
        
        Ok(())
    }
    
    /// Remove an edge between agents
    pub fn remove_edge(&self, from: AgentId, to: AgentId) -> SpaceResult<()> {
        let node_indices = self.node_indices.read();
        let from_idx = node_indices.get(&from).ok_or_else(|| agent_not_found(from))?;
        let to_idx = node_indices.get(&to).ok_or_else(|| agent_not_found(to))?;
        
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &mut *self.directed_graph.write() {
                    graph.remove_edge(graph.find_edge(*from_idx, *to_idx).unwrap());
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &mut *self.undirected_graph.write() {
                    graph.remove_edge(graph.find_edge(*from_idx, *to_idx).unwrap());
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a property layer
    pub fn add_property_layer(&self, name: &str, layer: Box<dyn PropertyLayer>) {
        self.property_layers.write().insert(name.to_string(), layer);
    }
    
    /// Get property value from a layer
    pub fn get_property(&self, layer_name: &str, agent: AgentId) -> Option<f64> {
        if !self.config.use_property_layers {
            return None;
        }
        
        self.property_layers.read()
            .get(layer_name)
            .map(|layer| layer.get_value(agent))
    }
    
    /// Set property value in a layer
    pub fn set_property(&self, layer_name: &str, agent: AgentId, value: f64) -> SpaceResult<()> {
        if !self.config.use_property_layers {
            return Err(invalid_operation("Property layers are disabled"));
        }
        
        self.property_layers.read()
            .get(layer_name)
            .ok_or_else(|| invalid_operation("Layer not found"))
            .map(|layer| layer.set_value(agent, value))
    }
    
    /// Find shortest path between agents
    pub fn find_path(&self, start: AgentId, end: AgentId) -> Option<Vec<AgentId>> {
        let key = PathKey { start, end };
        
        // Check cache first
        if let Some(path) = self.path_cache.write().get(&key) {
            return Some(path.clone());
        }
        
        let node_indices = self.node_indices.read();
        let start_idx = node_indices.get(&start)?;
        let end_idx = node_indices.get(&end)?;
        
        let path = match self.config.graph_type {
            GraphType::Directed => {
                let graph = self.directed_graph.read().as_ref()?;
                if self.config.weighted {
                    astar(
                        graph,
                        *start_idx,
                        |finish| finish == *end_idx,
                        |e| e.weight().distance,
                        |_| 0.0,
                    ).map(|(_, path)| path)
                } else {
                    let mut bfs = Bfs::new(graph, *start_idx);
                    let mut path = Vec::new();
                    while let Some(nx) = bfs.next(graph) {
                        path.push(graph[nx].id);
                        if nx == *end_idx {
                            break;
                        }
                    }
                    Some(path)
                }
            }
            GraphType::Undirected => {
                let graph = self.undirected_graph.read().as_ref()?;
                if self.config.weighted {
                    astar(
                        graph,
                        *start_idx,
                        |finish| finish == *end_idx,
                        |e| e.weight().distance,
                        |_| 0.0,
                    ).map(|(_, path)| path)
                } else {
                    let mut bfs = Bfs::new(graph, *start_idx);
                    let mut path = Vec::new();
                    while let Some(nx) = bfs.next(graph) {
                        path.push(graph[nx].id);
                        if nx == *end_idx {
                            break;
                        }
                    }
                    Some(path)
                }
            }
        }?;
        
        // Cache the result
        self.path_cache.write().put(key, path.clone());
        
        Some(path)
    }
    
    /// Detect communities using the Louvain method
    pub fn detect_communities(&self) -> SpaceResult<()> {
        if !self.config.use_communities {
            return Err(invalid_operation("Community detection is disabled"));
        }
        
        // Implementation of Louvain method for community detection
        // This is a simplified version - a full implementation would be more complex
        
        let mut communities = self.communities.write();
        communities.clear();
        
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &*self.directed_graph.read() {
                    // Initialize each node to its own community
                    for node in graph.node_indices() {
                        communities.insert(graph[node].id, node.index());
                    }
                    
                    // TODO: Implement full Louvain method
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &*self.undirected_graph.read() {
                    // Initialize each node to its own community
                    for node in graph.node_indices() {
                        communities.insert(graph[node].id, node.index());
                    }
                    
                    // TODO: Implement full Louvain method
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the community of an agent
    pub fn get_community(&self, agent: AgentId) -> Option<usize> {
        self.communities.read().get(&agent).copied()
    }
}

impl Space for GraphSpace {
    type Position = Point3<f64>;
    
    fn add_agent(&self, id: AgentId, position: Self::Position) {
        let data = AgentData {
            id,
            position,
            properties: HashMap::new(),
        };
        
        let node_idx = match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &mut *self.directed_graph.write() {
                    graph.add_node(data)
                } else {
                    return;
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &mut *self.undirected_graph.write() {
                    graph.add_node(data)
                } else {
                    return;
                }
            }
        };
        
        self.node_indices.write().insert(id, node_idx);
    }
    
    fn remove_agent(&self, id: AgentId) {
        let mut node_indices = self.node_indices.write();
        if let Some(node_idx) = node_indices.remove(&id) {
            match self.config.graph_type {
                GraphType::Directed => {
                    if let Some(graph) = &mut *self.directed_graph.write() {
                        graph.remove_node(node_idx);
                    }
                }
                GraphType::Undirected => {
                    if let Some(graph) = &mut *self.undirected_graph.write() {
                        graph.remove_node(node_idx);
                    }
                }
            }
        }
    }
    
    fn move_agent(&self, id: AgentId, new_position: Self::Position) {
        let node_indices = self.node_indices.read();
        if let Some(&node_idx) = node_indices.get(&id) {
            match self.config.graph_type {
                GraphType::Directed => {
                    if let Some(graph) = &mut *self.directed_graph.write() {
                        if let Some(node_data) = graph.node_weight_mut(node_idx) {
                            node_data.position = new_position;
                        }
                    }
                }
                GraphType::Undirected => {
                    if let Some(graph) = &mut *self.undirected_graph.write() {
                        if let Some(node_data) = graph.node_weight_mut(node_idx) {
                            node_data.position = new_position;
                        }
                    }
                }
            }
        }
    }
    
    fn get_position(&self, id: AgentId) -> Option<Self::Position> {
        let node_indices = self.node_indices.read();
        let &node_idx = node_indices.get(&id)?;
        
        match self.config.graph_type {
            GraphType::Directed => {
                self.directed_graph.read()
                    .as_ref()?
                    .node_weight(node_idx)
                    .map(|data| data.position)
            }
            GraphType::Undirected => {
                self.undirected_graph.read()
                    .as_ref()?
                    .node_weight(node_idx)
                    .map(|data| data.position)
            }
        }
    }
    
    fn query_radius(&self, center: Self::Position, radius: f64) -> Vec<AgentId> {
        let mut result = Vec::new();
        let node_indices = self.node_indices.read();
        
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &*self.directed_graph.read() {
                    for node in graph.node_indices() {
                        let data = &graph[node];
                        if (data.position - center).magnitude() <= radius {
                            result.push(data.id);
                        }
                    }
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &*self.undirected_graph.read() {
                    for node in graph.node_indices() {
                        let data = &graph[node];
                        if (data.position - center).magnitude() <= radius {
                            result.push(data.id);
                        }
                    }
                }
            }
        }
        
        result
    }
    
    fn query_k_nearest(&self, center: Self::Position, k: usize) -> Vec<AgentId> {
        let mut agents = Vec::new();
        let node_indices = self.node_indices.read();
        
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &*self.directed_graph.read() {
                    let mut distances: Vec<_> = graph.node_indices()
                        .map(|node| {
                            let data = &graph[node];
                            (data.id, (data.position - center).magnitude())
                        })
                        .collect();
                    
                    distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    agents.extend(distances.into_iter().take(k).map(|(id, _)| id));
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &*self.undirected_graph.read() {
                    let mut distances: Vec<_> = graph.node_indices()
                        .map(|node| {
                            let data = &graph[node];
                            (data.id, (data.position - center).magnitude())
                        })
                        .collect();
                    
                    distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    agents.extend(distances.into_iter().take(k).map(|(id, _)| id));
                }
            }
        }
        
        agents
    }
    
    fn agent_count(&self) -> usize {
        self.node_indices.read().len()
    }
    
    fn clear(&self) {
        match self.config.graph_type {
            GraphType::Directed => {
                if let Some(graph) = &mut *self.directed_graph.write() {
                    graph.clear();
                }
            }
            GraphType::Undirected => {
                if let Some(graph) = &mut *self.undirected_graph.write() {
                    graph.clear();
                }
            }
        }
        
        self.node_indices.write().clear();
        self.communities.write().clear();
        self.property_layers.write().clear();
        self.path_cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestLayer {
        values: RwLock<HashMap<AgentId, f64>>,
    }
    
    impl TestLayer {
        fn new() -> Self {
            Self {
                values: RwLock::new(HashMap::new()),
            }
        }
    }
    
    impl PropertyLayer for TestLayer {
        fn get_value(&self, agent: AgentId) -> f64 {
            self.values.read().get(&agent).copied().unwrap_or(0.0)
        }
        
        fn set_value(&self, agent: AgentId, value: f64) {
            self.values.write().insert(agent, value);
        }
        
        fn par_apply<F>(&self, f: F)
        where
            F: Fn(f64) -> f64 + Send + Sync,
        {
            let mut values = self.values.write();
            values.par_iter_mut().for_each(|(_, value)| {
                *value = f(*value);
            });
        }
    }
    
    #[test]
    fn test_graph_space() {
        let mut config = GraphSpaceConfig::default();
        config.use_property_layers = true;
        config.use_communities = true;
        
        let space = GraphSpace::new(config);
        
        // Test basic operations
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        let pos1 = Point3::new(0.0, 0.0, 0.0);
        let pos2 = Point3::new(1.0, 1.0, 1.0);
        
        space.add_agent(id1, pos1);
        space.add_agent(id2, pos2);
        assert_eq!(space.agent_count(), 2);
        
        // Test edge operations
        let weight = EdgeWeight::default();
        space.add_edge(id1, id2, weight).unwrap();
        
        // Test property layers
        let layer = Box::new(TestLayer::new());
        space.add_property_layer("test", layer);
        
        space.set_property("test", id1, 1.0).unwrap();
        assert_eq!(space.get_property("test", id1).unwrap(), 1.0);
        
        // Test pathfinding
        let path = space.find_path(id1, id2).unwrap();
        assert_eq!(path.len(), 2); // Should be [id1, id2]
        
        // Test community detection
        space.detect_communities().unwrap();
        assert!(space.get_community(id1).is_some());
    }
} 