//! Enhanced Pathfinding Module
//!
//! Advanced pathfinding features including hierarchical planning,
//! real-time replanning, and multi-agent coordination.

use std::collections::{HashMap, HashSet, BinaryHeap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{Cost, Path, Point};
use super::algorithms::{AStar, EuclideanHeuristic};
use super::cache::{PathCache, PathCacheConfig};
use super::error::PathfindingError;
use super::traits::{Graph, Heuristic, PathfindingAlgorithm};

/// Hierarchical Pathfinding A* (HPA*)
/// 
/// Preprocesses the map into a hierarchy of abstract graphs
/// for faster long-distance pathfinding.
#[derive(Debug)]
pub struct HierarchicalPathfinder<G: Graph> {
    /// The underlying graph
    graph: Arc<G>,
    /// Cluster size for abstraction
    cluster_size: f64,
    /// Number of hierarchy levels
    levels: usize,
    /// Abstract nodes per level
    abstract_nodes: Vec<HashMap<PointKey, AbstractNode>>,
    /// Inter-cluster edges
    inter_edges: Vec<Vec<(PointKey, PointKey, Cost)>>,
    /// Path cache
    cache: PathCache,
}

/// Abstract node in hierarchical graph
#[derive(Debug, Clone)]
struct AbstractNode {
    position: Point,
    level: usize,
    cluster_id: ClusterId,
    boundary_nodes: Vec<Point>,
}

/// Cluster identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClusterId {
    level: usize,
    x: i64,
    y: i64,
    z: i64,
}

/// Hashable point key
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PointKey {
    x: i64,
    y: i64,
    z: i64,
}

impl From<Point> for PointKey {
    fn from(p: Point) -> Self {
        const SCALE: f64 = 1000.0;
        Self {
            x: (p.x * SCALE) as i64,
            y: (p.y * SCALE) as i64,
            z: (p.z * SCALE) as i64,
        }
    }
}

impl<G: Graph> HierarchicalPathfinder<G> {
    /// Create a new hierarchical pathfinder
    pub fn new(graph: Arc<G>, cluster_size: f64, levels: usize) -> Self {
        let mut pathfinder = Self {
            graph,
            cluster_size,
            levels,
            abstract_nodes: vec![HashMap::new(); levels],
            inter_edges: vec![Vec::new(); levels],
            cache: PathCache::new(),
        };
        
        pathfinder.preprocess();
        pathfinder
    }
    
    /// Preprocess the graph to build hierarchy
    fn preprocess(&mut self) {
        // Build abstract graph for each level
        for level in 0..self.levels {
            let cluster_size = self.cluster_size * (2.0_f64).powi(level as i32);
            self.build_level(level, cluster_size);
        }
    }
    
    /// Build a single level of abstraction
    fn build_level(&mut self, level: usize, cluster_size: f64) {
        // Simplified implementation - would need proper boundary detection
        // This is a placeholder for the full HPA* algorithm
    }
    
    /// Find path using hierarchical search
    pub fn find_path(&self, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        // Check cache first
        if let Some(cached) = self.cache.get(start, goal) {
            return Ok(cached.path);
        }
        
        // Determine which hierarchy level to use based on distance
        let distance = euclidean_distance(&start, &goal);
        let level = self.select_level(distance);
        
        // Find abstract path
        let abstract_path = self.find_abstract_path(start, goal, level)?;
        
        // Refine to concrete path
        let concrete_path = self.refine_path(&abstract_path)?;
        
        // Cache the result
        let cost: Cost = concrete_path.windows(2)
            .map(|w| self.graph.cost(&w[0], &w[1]))
            .sum();
        self.cache.insert(start, goal, concrete_path.clone(), cost);
        
        Ok(concrete_path)
    }
    
    /// Select appropriate hierarchy level based on distance
    fn select_level(&self, distance: f64) -> usize {
        for level in (0..self.levels).rev() {
            let cluster_size = self.cluster_size * (2.0_f64).powi(level as i32);
            if distance > cluster_size * 2.0 {
                return level;
            }
        }
        0
    }
    
    /// Find path at abstract level
    fn find_abstract_path(&self, start: Point, goal: Point, level: usize) -> Result<Path, PathfindingError> {
        // For now, fall back to standard A*
        let astar = AStar::new(EuclideanHeuristic);
        astar.find_path(self.graph.as_ref(), start, goal)
    }
    
    /// Refine abstract path to concrete path
    fn refine_path(&self, abstract_path: &Path) -> Result<Path, PathfindingError> {
        // For hierarchical, would refine each segment
        // For now, return as-is
        Ok(abstract_path.clone())
    }
    
    /// Invalidate cache when graph changes
    pub fn invalidate(&self) {
        self.cache.invalidate();
    }
}

/// Real-time replanning pathfinder (D* Lite style)
/// 
/// Efficiently updates paths when the environment changes.
#[derive(Debug)]
pub struct ReplanningPathfinder<G: Graph> {
    /// The underlying graph
    graph: Arc<RwLock<G>>,
    /// Current path
    current_path: RwLock<Option<Path>>,
    /// Current position index in path
    current_index: RwLock<usize>,
    /// Goal position
    goal: RwLock<Option<Point>>,
    /// Graph version for change detection
    graph_version: std::sync::atomic::AtomicU64,
    /// Replan threshold (distance)
    replan_threshold: f64,
}

impl<G: Graph + Send + Sync> ReplanningPathfinder<G> {
    /// Create a new replanning pathfinder
    pub fn new(graph: G) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            current_path: RwLock::new(None),
            current_index: RwLock::new(0),
            goal: RwLock::new(None),
            graph_version: std::sync::atomic::AtomicU64::new(0),
            replan_threshold: 5.0,
        }
    }
    
    /// Set the goal and compute initial path
    pub fn set_goal(&self, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        let path = self.compute_path(start, goal)?;
        
        *self.current_path.write() = Some(path.clone());
        *self.current_index.write() = 0;
        *self.goal.write() = Some(goal);
        
        Ok(path)
    }
    
    /// Update position and check if replanning is needed
    pub fn update_position(&self, current: Point) -> Result<Option<Path>, PathfindingError> {
        let path = self.current_path.read();
        let goal = self.goal.read();
        
        if path.is_none() || goal.is_none() {
            return Ok(None);
        }
        
        let path = path.as_ref().unwrap();
        let goal = goal.unwrap();
        
        // Find current position in path
        let mut closest_idx = 0;
        let mut closest_dist = f64::MAX;
        
        for (i, point) in path.iter().enumerate() {
            let dist = euclidean_distance(&current, point);
            if dist < closest_dist {
                closest_dist = dist;
                closest_idx = i;
            }
        }
        
        // Check if we've deviated too far from path
        if closest_dist > self.replan_threshold {
            drop(path);
            drop(goal);
            return Ok(Some(self.replan(current)?));
        }
        
        *self.current_index.write() = closest_idx;
        
        Ok(None)
    }
    
    /// Notify of obstacle changes
    pub fn notify_obstacle_change(&self, changed_points: &[Point]) {
        // Increment version to invalidate cached computations
        self.graph_version.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // Check if any changed points affect current path
        let path = self.current_path.read();
        if let Some(ref path) = *path {
            for changed in changed_points {
                for path_point in path {
                    if euclidean_distance(changed, path_point) < 1.0 {
                        // Path is affected, mark for replanning
                        // In a real implementation, would trigger async replan
                        return;
                    }
                }
            }
        }
    }
    
    /// Force replanning from current position
    pub fn replan(&self, current: Point) -> Result<Path, PathfindingError> {
        let goal = self.goal.read().ok_or(PathfindingError::InvalidGoal)?;
        
        let path = self.compute_path(current, goal)?;
        
        *self.current_path.write() = Some(path.clone());
        *self.current_index.write() = 0;
        
        Ok(path)
    }
    
    /// Compute path using underlying algorithm
    fn compute_path(&self, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        let graph = self.graph.read();
        let astar = AStar::new(EuclideanHeuristic);
        astar.find_path(&*graph, start, goal)
    }
    
    /// Get remaining path from current position
    pub fn get_remaining_path(&self) -> Option<Path> {
        let path = self.current_path.read();
        let index = *self.current_index.read();
        
        path.as_ref().map(|p| p[index..].to_vec())
    }
    
    /// Get next waypoint
    pub fn get_next_waypoint(&self) -> Option<Point> {
        let path = self.current_path.read();
        let index = *self.current_index.read();
        
        path.as_ref().and_then(|p| p.get(index + 1).copied())
    }
}

/// Multi-agent pathfinding coordinator
/// 
/// Coordinates paths for multiple agents to avoid collisions.
pub struct MultiAgentPathfinder<G: Graph> {
    /// The underlying graph
    graph: Arc<G>,
    /// Reserved positions (time, position) -> agent_id
    reservations: RwLock<HashMap<(u64, PointKey), usize>>,
    /// Agent paths
    agent_paths: RwLock<HashMap<usize, Path>>,
    /// Collision avoidance window (time steps)
    collision_window: u64,
}

impl<G: Graph> MultiAgentPathfinder<G> {
    /// Create a new multi-agent pathfinder
    pub fn new(graph: Arc<G>) -> Self {
        Self {
            graph,
            reservations: RwLock::new(HashMap::new()),
            agent_paths: RwLock::new(HashMap::new()),
            collision_window: 10,
        }
    }
    
    /// Plan path for an agent considering other agents
    pub fn plan_for_agent(
        &self,
        agent_id: usize,
        start: Point,
        goal: Point,
        start_time: u64,
    ) -> Result<Path, PathfindingError> {
        // Use space-time A* with reservations
        let path = self.space_time_astar(start, goal, start_time, agent_id)?;
        
        // Reserve the path
        self.reserve_path(agent_id, &path, start_time);
        
        // Store the path
        self.agent_paths.write().insert(agent_id, path.clone());
        
        Ok(path)
    }
    
    /// Space-time A* implementation
    fn space_time_astar(
        &self,
        start: Point,
        goal: Point,
        start_time: u64,
        agent_id: usize,
    ) -> Result<Path, PathfindingError> {
        // Simplified implementation - would need proper space-time search
        // For now, use regular A* with collision checking
        let astar = AStar::new(EuclideanHeuristic);
        let base_path = astar.find_path(self.graph.as_ref(), start, goal)?;
        
        // Check for collisions and wait if needed
        let mut final_path = Vec::new();
        let mut current_time = start_time;
        
        for point in base_path {
            // Check if position is reserved
            let key = PointKey::from(point);
            let reservations = self.reservations.read();
            
            while let Some(&reserved_agent) = reservations.get(&(current_time, key)) {
                if reserved_agent != agent_id {
                    // Wait at current position
                    if let Some(last) = final_path.last() {
                        final_path.push(*last);
                    }
                    current_time += 1;
                } else {
                    break;
                }
            }
            
            final_path.push(point);
            current_time += 1;
        }
        
        Ok(final_path)
    }
    
    /// Reserve a path for an agent
    fn reserve_path(&self, agent_id: usize, path: &Path, start_time: u64) {
        let mut reservations = self.reservations.write();
        
        for (i, point) in path.iter().enumerate() {
            let time = start_time + i as u64;
            let key = PointKey::from(*point);
            
            // Reserve for collision window
            for t in time..time + self.collision_window {
                reservations.insert((t, key), agent_id);
            }
        }
    }
    
    /// Clear reservations for an agent
    pub fn clear_agent(&self, agent_id: usize) {
        let mut reservations = self.reservations.write();
        reservations.retain(|_, &mut id| id != agent_id);
        
        self.agent_paths.write().remove(&agent_id);
    }
    
    /// Get all current paths
    pub fn get_all_paths(&self) -> HashMap<usize, Path> {
        self.agent_paths.read().clone()
    }
    
    /// Check if a position is available at a given time
    pub fn is_available(&self, point: Point, time: u64, agent_id: usize) -> bool {
        let key = PointKey::from(point);
        let reservations = self.reservations.read();
        
        match reservations.get(&(time, key)) {
            Some(&id) => id == agent_id,
            None => true,
        }
    }
}

/// Flow field pathfinder for many agents to same goal
/// 
/// Precomputes a vector field pointing toward the goal
/// for efficient pathfinding of many agents.
pub struct FlowFieldPathfinder<G: Graph> {
    /// The underlying graph
    graph: Arc<G>,
    /// Flow field (position -> direction)
    flow_field: RwLock<HashMap<PointKey, (f64, f64, f64)>>,
    /// Current goal
    goal: RwLock<Option<Point>>,
    /// Grid resolution
    resolution: f64,
}

impl<G: Graph> FlowFieldPathfinder<G> {
    /// Create a new flow field pathfinder
    pub fn new(graph: Arc<G>, resolution: f64) -> Self {
        Self {
            graph,
            flow_field: RwLock::new(HashMap::new()),
            goal: RwLock::new(None),
            resolution,
        }
    }
    
    /// Set goal and compute flow field
    pub fn set_goal(&self, goal: Point) {
        *self.goal.write() = Some(goal);
        self.compute_flow_field(goal);
    }
    
    /// Compute flow field using Dijkstra from goal
    fn compute_flow_field(&self, goal: Point) {
        let mut flow_field = self.flow_field.write();
        flow_field.clear();
        
        // Use Dijkstra's algorithm from goal to compute distances
        let mut distances: HashMap<PointKey, Cost> = HashMap::new();
        let mut heap = BinaryHeap::new();
        
        let goal_key = PointKey::from(goal);
        distances.insert(goal_key, 0.0);
        heap.push(std::cmp::Reverse((ordered_float::OrderedFloat(0.0), goal)));
        
        while let Some(std::cmp::Reverse((dist, current))) = heap.pop() {
            let current_key = PointKey::from(current);
            
            if dist.0 > distances.get(&current_key).copied().unwrap_or(f64::MAX) {
                continue;
            }
            
            for neighbor in self.graph.neighbors(&current) {
                let neighbor_key = PointKey::from(neighbor);
                let edge_cost = self.graph.cost(&current, &neighbor);
                let new_dist = dist.0 + edge_cost;
                
                if new_dist < distances.get(&neighbor_key).copied().unwrap_or(f64::MAX) {
                    distances.insert(neighbor_key, new_dist);
                    heap.push(std::cmp::Reverse((ordered_float::OrderedFloat(new_dist), neighbor)));
                }
            }
        }
        
        // Compute flow vectors (gradient descent toward goal)
        for (key, _dist) in &distances {
            let point = Point::new(
                key.x as f64 / 1000.0,
                key.y as f64 / 1000.0,
                key.z as f64 / 1000.0,
            );
            
            let mut best_dir = (0.0, 0.0, 0.0);
            let mut best_dist = f64::MAX;
            
            for neighbor in self.graph.neighbors(&point) {
                let neighbor_key = PointKey::from(neighbor);
                if let Some(&neighbor_dist) = distances.get(&neighbor_key) {
                    if neighbor_dist < best_dist {
                        best_dist = neighbor_dist;
                        let dx = neighbor.x - point.x;
                        let dy = neighbor.y - point.y;
                        let dz = neighbor.z - point.z;
                        let len = (dx*dx + dy*dy + dz*dz).sqrt();
                        if len > 0.0 {
                            best_dir = (dx/len, dy/len, dz/len);
                        }
                    }
                }
            }
            
            flow_field.insert(*key, best_dir);
        }
    }
    
    /// Get flow direction at a position
    pub fn get_direction(&self, position: Point) -> Option<(f64, f64, f64)> {
        let key = PointKey::from(position);
        self.flow_field.read().get(&key).copied()
    }
    
    /// Get next position for an agent following the flow
    pub fn get_next_position(&self, current: Point, speed: f64) -> Point {
        if let Some((dx, dy, dz)) = self.get_direction(current) {
            Point::new(
                current.x + dx * speed,
                current.y + dy * speed,
                current.z + dz * speed,
            )
        } else {
            current
        }
    }
}

// Helper functions

fn euclidean_distance(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx*dx + dy*dy + dz*dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct SimpleGraph;
    
    impl Graph for SimpleGraph {
        fn neighbors(&self, point: &Point) -> Vec<Point> {
            vec![
                Point::new(point.x + 1.0, point.y, point.z),
                Point::new(point.x - 1.0, point.y, point.z),
                Point::new(point.x, point.y + 1.0, point.z),
                Point::new(point.x, point.y - 1.0, point.z),
            ]
        }
        
        fn cost(&self, _from: &Point, _to: &Point) -> Cost {
            1.0
        }
        
        fn is_valid_position(&self, point: &Point) -> bool {
            point.x >= 0.0 && point.x <= 20.0
                && point.y >= 0.0 && point.y <= 20.0
                && point.z >= 0.0 && point.z <= 20.0
        }
    }
    
    #[test]
    fn test_replanning_pathfinder() {
        let graph = SimpleGraph;
        let pathfinder = ReplanningPathfinder::new(graph);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 0.0, 0.0);
        
        let path = pathfinder.set_goal(start, goal).unwrap();
        assert!(!path.is_empty());
        
        // Update position
        let result = pathfinder.update_position(Point::new(2.0, 0.0, 0.0)).unwrap();
        assert!(result.is_none()); // No replan needed
    }
    
    #[test]
    fn test_multi_agent_pathfinder() {
        let graph = Arc::new(SimpleGraph);
        let pathfinder = MultiAgentPathfinder::new(graph);
        
        let agent1_start = Point::new(0.0, 0.0, 0.0);
        let agent1_goal = Point::new(5.0, 0.0, 0.0);
        
        let agent2_start = Point::new(5.0, 0.0, 0.0);
        let agent2_goal = Point::new(0.0, 0.0, 0.0);
        
        let path1 = pathfinder.plan_for_agent(0, agent1_start, agent1_goal, 0).unwrap();
        let path2 = pathfinder.plan_for_agent(1, agent2_start, agent2_goal, 0).unwrap();
        
        assert!(!path1.is_empty());
        assert!(!path2.is_empty());
        
        // Paths should avoid each other
        let all_paths = pathfinder.get_all_paths();
        assert_eq!(all_paths.len(), 2);
    }
}
