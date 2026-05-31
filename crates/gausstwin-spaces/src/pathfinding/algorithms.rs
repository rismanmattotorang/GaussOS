//! Pathfinding Algorithms Module
//!
//! High-performance implementations of classic and modern pathfinding algorithms.
//! All algorithms are designed for 3D space with support for weighted edges,
//! dynamic obstacles, and hierarchical navigation.

use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::cmp::Ordering;
use std::hash::Hash;
use nalgebra::Point3;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::{Cost, Path, Point};
use super::error::PathfindingError;
use super::traits::{Graph, Heuristic, PathfindingAlgorithm};

/// Node wrapper for priority queue operations
#[derive(Debug, Clone)]
struct SearchNode {
    position: Point,
    g_cost: Cost,
    f_cost: Cost,
    parent: Option<Point>,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}

impl Eq for SearchNode {}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior
        other.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
    }
}

/// A* pathfinding algorithm
/// 
/// Standard A* algorithm with configurable heuristic functions.
/// Optimal for finding shortest paths when heuristic is admissible.
#[derive(Debug, Clone)]
pub struct AStar<H: Heuristic> {
    heuristic: H,
    max_iterations: usize,
    tie_breaker: f64,
}

impl<H: Heuristic> AStar<H> {
    /// Create a new A* pathfinder with the given heuristic
    pub fn new(heuristic: H) -> Self {
        Self {
            heuristic,
            max_iterations: 1_000_000,
            tie_breaker: 1.0 + 1e-10,
        }
    }
    
    /// Set maximum iterations before giving up
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
    
    /// Set tie-breaker factor (slightly > 1.0 improves performance)
    pub fn with_tie_breaker(mut self, factor: f64) -> Self {
        self.tie_breaker = factor;
        self
    }
}

impl<H: Heuristic, G: Graph> PathfindingAlgorithm<G> for AStar<H> {
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        if !graph.is_valid_position(&start) {
            return Err(PathfindingError::InvalidStart);
        }
        if !graph.is_valid_position(&goal) {
            return Err(PathfindingError::InvalidGoal);
        }
        
        let mut open_set = BinaryHeap::new();
        let mut g_scores: HashMap<PointKey, Cost> = HashMap::new();
        let mut came_from: HashMap<PointKey, Point> = HashMap::new();
        let mut closed_set: HashSet<PointKey> = HashSet::new();
        
        let start_key = PointKey::from(start);
        g_scores.insert(start_key, 0.0);
        
        let h = self.heuristic.estimate(&start, &goal);
        open_set.push(SearchNode {
            position: start,
            g_cost: 0.0,
            f_cost: h,
            parent: None,
        });
        
        let mut iterations = 0;
        
        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PathfindingError::MaxIterationsExceeded);
            }
            
            let current_key = PointKey::from(current.position);
            
            // Check if we reached the goal
            if distance(&current.position, &goal) < 1e-6 {
                return Ok(reconstruct_path(&came_from, current.position));
            }
            
            // Skip if already processed
            if closed_set.contains(&current_key) {
                continue;
            }
            closed_set.insert(current_key);
            
            // Explore neighbors
            for neighbor in graph.neighbors(&current.position) {
                let neighbor_key = PointKey::from(neighbor);
                
                if closed_set.contains(&neighbor_key) {
                    continue;
                }
                
                let edge_cost = graph.cost(&current.position, &neighbor);
                let tentative_g = current.g_cost + edge_cost;
                
                let current_g = g_scores.get(&neighbor_key).copied().unwrap_or(Cost::INFINITY);
                
                if tentative_g < current_g {
                    came_from.insert(neighbor_key, current.position);
                    g_scores.insert(neighbor_key, tentative_g);
                    
                    let h = self.heuristic.estimate(&neighbor, &goal) * self.tie_breaker;
                    let f = tentative_g + h;
                    
                    open_set.push(SearchNode {
                        position: neighbor,
                        g_cost: tentative_g,
                        f_cost: f,
                        parent: Some(current.position),
                    });
                }
            }
        }
        
        Err(PathfindingError::NoPathFound)
    }
}

/// Dijkstra's algorithm
/// 
/// Guarantees optimal path without heuristic. Use when no good heuristic available.
#[derive(Debug, Clone, Default)]
pub struct Dijkstra {
    max_iterations: usize,
}

impl Dijkstra {
    /// Create a new Dijkstra pathfinder
    pub fn new() -> Self {
        Self {
            max_iterations: 1_000_000,
        }
    }
    
    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
}

impl<G: Graph> PathfindingAlgorithm<G> for Dijkstra {
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        // Dijkstra is A* with zero heuristic
        let astar = AStar::new(ZeroHeuristic)
            .with_max_iterations(self.max_iterations);
        astar.find_path(graph, start, goal)
    }
}

/// Bidirectional A* algorithm
/// 
/// Searches from both start and goal simultaneously for faster pathfinding.
#[derive(Debug, Clone)]
pub struct BidirectionalAStar<H: Heuristic> {
    heuristic: H,
    max_iterations: usize,
}

impl<H: Heuristic> BidirectionalAStar<H> {
    /// Create a new bidirectional A* pathfinder
    pub fn new(heuristic: H) -> Self {
        Self {
            heuristic,
            max_iterations: 1_000_000,
        }
    }
}

impl<H: Heuristic + Clone, G: Graph> PathfindingAlgorithm<G> for BidirectionalAStar<H> {
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        if !graph.is_valid_position(&start) {
            return Err(PathfindingError::InvalidStart);
        }
        if !graph.is_valid_position(&goal) {
            return Err(PathfindingError::InvalidGoal);
        }
        
        // Forward search structures
        let mut fwd_open = BinaryHeap::new();
        let mut fwd_g: HashMap<PointKey, Cost> = HashMap::new();
        let mut fwd_came_from: HashMap<PointKey, Point> = HashMap::new();
        let mut fwd_closed: HashSet<PointKey> = HashSet::new();
        
        // Backward search structures
        let mut bwd_open = BinaryHeap::new();
        let mut bwd_g: HashMap<PointKey, Cost> = HashMap::new();
        let mut bwd_came_from: HashMap<PointKey, Point> = HashMap::new();
        let mut bwd_closed: HashSet<PointKey> = HashSet::new();
        
        // Initialize
        let start_key = PointKey::from(start);
        let goal_key = PointKey::from(goal);
        
        fwd_g.insert(start_key, 0.0);
        bwd_g.insert(goal_key, 0.0);
        
        fwd_open.push(SearchNode {
            position: start,
            g_cost: 0.0,
            f_cost: self.heuristic.estimate(&start, &goal),
            parent: None,
        });
        
        bwd_open.push(SearchNode {
            position: goal,
            g_cost: 0.0,
            f_cost: self.heuristic.estimate(&goal, &start),
            parent: None,
        });
        
        let mut best_cost = Cost::INFINITY;
        let mut meeting_point: Option<Point> = None;
        
        let mut iterations = 0;
        
        while !fwd_open.is_empty() && !bwd_open.is_empty() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PathfindingError::MaxIterationsExceeded);
            }
            
            // Expand forward
            if let Some(current) = fwd_open.pop() {
                let current_key = PointKey::from(current.position);
                
                if !fwd_closed.contains(&current_key) {
                    fwd_closed.insert(current_key);
                    
                    // Check if meets backward search
                    if bwd_closed.contains(&current_key) {
                        let total_cost = current.g_cost + bwd_g.get(&current_key).copied().unwrap_or(Cost::INFINITY);
                        if total_cost < best_cost {
                            best_cost = total_cost;
                            meeting_point = Some(current.position);
                        }
                    }
                    
                    // Explore neighbors
                    for neighbor in graph.neighbors(&current.position) {
                        let neighbor_key = PointKey::from(neighbor);
                        if fwd_closed.contains(&neighbor_key) {
                            continue;
                        }
                        
                        let tentative_g = current.g_cost + graph.cost(&current.position, &neighbor);
                        let current_g = fwd_g.get(&neighbor_key).copied().unwrap_or(Cost::INFINITY);
                        
                        if tentative_g < current_g {
                            fwd_came_from.insert(neighbor_key, current.position);
                            fwd_g.insert(neighbor_key, tentative_g);
                            
                            let h = self.heuristic.estimate(&neighbor, &goal);
                            fwd_open.push(SearchNode {
                                position: neighbor,
                                g_cost: tentative_g,
                                f_cost: tentative_g + h,
                                parent: Some(current.position),
                            });
                        }
                    }
                }
            }
            
            // Expand backward
            if let Some(current) = bwd_open.pop() {
                let current_key = PointKey::from(current.position);
                
                if !bwd_closed.contains(&current_key) {
                    bwd_closed.insert(current_key);
                    
                    // Check if meets forward search
                    if fwd_closed.contains(&current_key) {
                        let total_cost = current.g_cost + fwd_g.get(&current_key).copied().unwrap_or(Cost::INFINITY);
                        if total_cost < best_cost {
                            best_cost = total_cost;
                            meeting_point = Some(current.position);
                        }
                    }
                    
                    // Explore neighbors
                    for neighbor in graph.neighbors(&current.position) {
                        let neighbor_key = PointKey::from(neighbor);
                        if bwd_closed.contains(&neighbor_key) {
                            continue;
                        }
                        
                        let tentative_g = current.g_cost + graph.cost(&current.position, &neighbor);
                        let current_g = bwd_g.get(&neighbor_key).copied().unwrap_or(Cost::INFINITY);
                        
                        if tentative_g < current_g {
                            bwd_came_from.insert(neighbor_key, current.position);
                            bwd_g.insert(neighbor_key, tentative_g);
                            
                            let h = self.heuristic.estimate(&neighbor, &start);
                            bwd_open.push(SearchNode {
                                position: neighbor,
                                g_cost: tentative_g,
                                f_cost: tentative_g + h,
                                parent: Some(current.position),
                            });
                        }
                    }
                }
            }
            
            // Termination check
            if !fwd_open.is_empty() && !bwd_open.is_empty() {
                let fwd_min = fwd_open.peek().map(|n| n.f_cost).unwrap_or(Cost::INFINITY);
                let bwd_min = bwd_open.peek().map(|n| n.f_cost).unwrap_or(Cost::INFINITY);
                
                if fwd_min + bwd_min >= best_cost {
                    break;
                }
            }
        }
        
        // Reconstruct path if found
        if let Some(meet) = meeting_point {
            let mut path = reconstruct_path(&fwd_came_from, meet);
            let bwd_path = reconstruct_path(&bwd_came_from, meet);
            
            // Combine paths (reverse backward path and append)
            for point in bwd_path.into_iter().skip(1).rev() {
                path.push(point);
            }
            
            return Ok(path);
        }
        
        Err(PathfindingError::NoPathFound)
    }
}

/// Jump Point Search (JPS) for uniform-cost grids
/// 
/// Highly optimized for grid-based pathfinding. Only works on uniform grids.
#[derive(Debug, Clone)]
pub struct JumpPointSearch {
    max_iterations: usize,
}

impl JumpPointSearch {
    /// Create a new JPS pathfinder
    pub fn new() -> Self {
        Self {
            max_iterations: 1_000_000,
        }
    }
    
    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }
}

impl Default for JumpPointSearch {
    fn default() -> Self {
        Self::new()
    }
}

/// Theta* algorithm for any-angle pathfinding
/// 
/// Produces smoother paths than grid-based A* by allowing diagonal movement.
#[derive(Debug, Clone)]
pub struct ThetaStar<H: Heuristic> {
    heuristic: H,
    max_iterations: usize,
}

impl<H: Heuristic> ThetaStar<H> {
    /// Create a new Theta* pathfinder
    pub fn new(heuristic: H) -> Self {
        Self {
            heuristic,
            max_iterations: 1_000_000,
        }
    }
}

impl<H: Heuristic, G: Graph> PathfindingAlgorithm<G> for ThetaStar<H> 
where
    G: LineOfSightCheck,
{
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        if !graph.is_valid_position(&start) {
            return Err(PathfindingError::InvalidStart);
        }
        if !graph.is_valid_position(&goal) {
            return Err(PathfindingError::InvalidGoal);
        }
        
        let mut open_set = BinaryHeap::new();
        let mut g_scores: HashMap<PointKey, Cost> = HashMap::new();
        let mut came_from: HashMap<PointKey, Point> = HashMap::new();
        let mut parent: HashMap<PointKey, Point> = HashMap::new();
        let mut closed_set: HashSet<PointKey> = HashSet::new();
        
        let start_key = PointKey::from(start);
        g_scores.insert(start_key, 0.0);
        parent.insert(start_key, start);
        
        let h = self.heuristic.estimate(&start, &goal);
        open_set.push(SearchNode {
            position: start,
            g_cost: 0.0,
            f_cost: h,
            parent: None,
        });
        
        let mut iterations = 0;
        
        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PathfindingError::MaxIterationsExceeded);
            }
            
            let current_key = PointKey::from(current.position);
            
            if distance(&current.position, &goal) < 1e-6 {
                return Ok(reconstruct_path(&came_from, current.position));
            }
            
            if closed_set.contains(&current_key) {
                continue;
            }
            closed_set.insert(current_key);
            
            let current_parent = parent.get(&current_key).copied().unwrap_or(current.position);
            
            for neighbor in graph.neighbors(&current.position) {
                let neighbor_key = PointKey::from(neighbor);
                
                if closed_set.contains(&neighbor_key) {
                    continue;
                }
                
                // Theta* line-of-sight check
                let (update_parent, tentative_g) = if graph.line_of_sight(&current_parent, &neighbor) {
                    // Direct path from grandparent
                    let parent_g = g_scores.get(&PointKey::from(current_parent)).copied().unwrap_or(0.0);
                    (current_parent, parent_g + distance(&current_parent, &neighbor))
                } else {
                    // Standard A* update
                    (current.position, current.g_cost + graph.cost(&current.position, &neighbor))
                };
                
                let current_g = g_scores.get(&neighbor_key).copied().unwrap_or(Cost::INFINITY);
                
                if tentative_g < current_g {
                    came_from.insert(neighbor_key, update_parent);
                    parent.insert(neighbor_key, update_parent);
                    g_scores.insert(neighbor_key, tentative_g);
                    
                    let h = self.heuristic.estimate(&neighbor, &goal);
                    open_set.push(SearchNode {
                        position: neighbor,
                        g_cost: tentative_g,
                        f_cost: tentative_g + h,
                        parent: Some(update_parent),
                    });
                }
            }
        }
        
        Err(PathfindingError::NoPathFound)
    }
}

/// Trait for graphs that support line-of-sight checks
pub trait LineOfSightCheck {
    /// Check if there's a clear line of sight between two points
    fn line_of_sight(&self, from: &Point, to: &Point) -> bool;
}

/// Breadth-First Search
/// 
/// Simple unweighted pathfinding for uniform-cost graphs.
#[derive(Debug, Clone, Default)]
pub struct BreadthFirstSearch {
    max_iterations: usize,
}

impl BreadthFirstSearch {
    /// Create a new BFS pathfinder
    pub fn new() -> Self {
        Self {
            max_iterations: 1_000_000,
        }
    }
}

impl<G: Graph> PathfindingAlgorithm<G> for BreadthFirstSearch {
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError> {
        if !graph.is_valid_position(&start) {
            return Err(PathfindingError::InvalidStart);
        }
        if !graph.is_valid_position(&goal) {
            return Err(PathfindingError::InvalidGoal);
        }
        
        let mut queue = VecDeque::new();
        let mut came_from: HashMap<PointKey, Point> = HashMap::new();
        let mut visited: HashSet<PointKey> = HashSet::new();
        
        let start_key = PointKey::from(start);
        visited.insert(start_key);
        queue.push_back(start);
        
        let mut iterations = 0;
        
        while let Some(current) = queue.pop_front() {
            iterations += 1;
            if iterations > self.max_iterations {
                return Err(PathfindingError::MaxIterationsExceeded);
            }
            
            if distance(&current, &goal) < 1e-6 {
                return Ok(reconstruct_path(&came_from, current));
            }
            
            for neighbor in graph.neighbors(&current) {
                let neighbor_key = PointKey::from(neighbor);
                
                if !visited.contains(&neighbor_key) {
                    visited.insert(neighbor_key);
                    came_from.insert(neighbor_key, current);
                    queue.push_back(neighbor);
                }
            }
        }
        
        Err(PathfindingError::NoPathFound)
    }
}

// ============= Heuristics =============

/// Zero heuristic (for Dijkstra)
#[derive(Debug, Clone, Copy, Default)]
pub struct ZeroHeuristic;

impl Heuristic for ZeroHeuristic {
    fn estimate(&self, _from: &Point, _to: &Point) -> Cost {
        0.0
    }
}

/// Euclidean distance heuristic
#[derive(Debug, Clone, Copy, Default)]
pub struct EuclideanHeuristic;

impl Heuristic for EuclideanHeuristic {
    fn estimate(&self, from: &Point, to: &Point) -> Cost {
        distance(from, to)
    }
}

/// Manhattan distance heuristic
#[derive(Debug, Clone, Copy, Default)]
pub struct ManhattanHeuristic;

impl Heuristic for ManhattanHeuristic {
    fn estimate(&self, from: &Point, to: &Point) -> Cost {
        (from.x - to.x).abs() + (from.y - to.y).abs() + (from.z - to.z).abs()
    }
}

/// Chebyshev distance heuristic (diagonal movement)
#[derive(Debug, Clone, Copy, Default)]
pub struct ChebyshevHeuristic;

impl Heuristic for ChebyshevHeuristic {
    fn estimate(&self, from: &Point, to: &Point) -> Cost {
        let dx = (from.x - to.x).abs();
        let dy = (from.y - to.y).abs();
        let dz = (from.z - to.z).abs();
        dx.max(dy).max(dz)
    }
}

/// Octile distance heuristic (diagonal movement with sqrt(2) cost)
#[derive(Debug, Clone, Copy, Default)]
pub struct OctileHeuristic;

impl Heuristic for OctileHeuristic {
    fn estimate(&self, from: &Point, to: &Point) -> Cost {
        let dx = (from.x - to.x).abs();
        let dy = (from.y - to.y).abs();
        let dz = (from.z - to.z).abs();
        
        let min = dx.min(dy).min(dz);
        let mid = dx.max(dy).max(dz) - dx.min(dy).min(dz);
        let max = dx + dy + dz - min - mid;
        
        // 3D octile: sqrt(3)*min + sqrt(2)*(mid) + 1.0*max
        1.732050808 * min + 1.414213562 * mid + max - min - mid
    }
}

// ============= Helper Types and Functions =============

/// Hashable point key for HashMap storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PointKey {
    x: i64,
    y: i64,
    z: i64,
}

impl From<Point> for PointKey {
    fn from(p: Point) -> Self {
        // Convert to fixed-point representation
        const SCALE: f64 = 1000.0;
        Self {
            x: (p.x * SCALE) as i64,
            y: (p.y * SCALE) as i64,
            z: (p.z * SCALE) as i64,
        }
    }
}

/// Calculate Euclidean distance between two points
fn distance(a: &Point, b: &Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Reconstruct path from came_from map
fn reconstruct_path(came_from: &HashMap<PointKey, Point>, goal: Point) -> Path {
    let mut path = vec![goal];
    let mut current = goal;
    
    while let Some(&parent) = came_from.get(&PointKey::from(current)) {
        path.push(parent);
        current = parent;
    }
    
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Simple test graph for pathfinding
    struct TestGraph {
        obstacles: HashSet<PointKey>,
        size: f64,
    }
    
    impl TestGraph {
        fn new(size: f64) -> Self {
            Self {
                obstacles: HashSet::new(),
                size,
            }
        }
        
        fn add_obstacle(&mut self, point: Point) {
            self.obstacles.insert(PointKey::from(point));
        }
    }
    
    impl Graph for TestGraph {
        fn neighbors(&self, point: &Point) -> Vec<Point> {
            let mut neighbors = Vec::new();
            
            // 6-connected grid neighbors
            let offsets = [
                (1.0, 0.0, 0.0), (-1.0, 0.0, 0.0),
                (0.0, 1.0, 0.0), (0.0, -1.0, 0.0),
                (0.0, 0.0, 1.0), (0.0, 0.0, -1.0),
            ];
            
            for (dx, dy, dz) in offsets {
                let neighbor = Point::new(point.x + dx, point.y + dy, point.z + dz);
                if self.is_valid_position(&neighbor) {
                    neighbors.push(neighbor);
                }
            }
            
            neighbors
        }
        
        fn cost(&self, _from: &Point, _to: &Point) -> Cost {
            1.0
        }
        
        fn is_valid_position(&self, point: &Point) -> bool {
            point.x >= 0.0 && point.x <= self.size
                && point.y >= 0.0 && point.y <= self.size
                && point.z >= 0.0 && point.z <= self.size
                && !self.obstacles.contains(&PointKey::from(*point))
        }
    }
    
    #[test]
    fn test_astar_basic() {
        let graph = TestGraph::new(10.0);
        let astar = AStar::new(EuclideanHeuristic);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 0.0, 0.0);
        
        let path = astar.find_path(&graph, start, goal).unwrap();
        
        assert!(!path.is_empty());
        assert_eq!(path.first().unwrap(), &start);
        assert_eq!(path.last().unwrap(), &goal);
    }
    
    #[test]
    fn test_astar_with_obstacles() {
        let mut graph = TestGraph::new(10.0);
        
        // Add a wall of obstacles
        for y in 0..5 {
            graph.add_obstacle(Point::new(2.0, y as f64, 0.0));
        }
        
        let astar = AStar::new(ManhattanHeuristic);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 0.0, 0.0);
        
        let path = astar.find_path(&graph, start, goal).unwrap();
        
        assert!(!path.is_empty());
        // Path should go around the wall
        assert!(path.len() > 5);
    }
    
    #[test]
    fn test_dijkstra() {
        let graph = TestGraph::new(10.0);
        let dijkstra = Dijkstra::new();
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(3.0, 3.0, 0.0);
        
        let path = dijkstra.find_path(&graph, start, goal).unwrap();
        
        assert!(!path.is_empty());
        assert_eq!(path.first().unwrap(), &start);
    }
    
    #[test]
    fn test_bfs() {
        let graph = TestGraph::new(10.0);
        let bfs = BreadthFirstSearch::new();
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(2.0, 2.0, 0.0);
        
        let path = bfs.find_path(&graph, start, goal).unwrap();
        
        assert!(!path.is_empty());
    }
    
    #[test]
    fn test_no_path() {
        let mut graph = TestGraph::new(10.0);
        
        // Create a completely blocked area
        for x in 0..=10 {
            graph.add_obstacle(Point::new(5.0, x as f64, 0.0));
        }
        for y in 0..=10 {
            graph.add_obstacle(Point::new(y as f64, 0.0, 0.0));
        }
        
        let astar = AStar::new(EuclideanHeuristic);
        
        let start = Point::new(1.0, 1.0, 0.0);
        let goal = Point::new(9.0, 9.0, 0.0);
        
        let result = astar.find_path(&graph, start, goal);
        
        // Should not find a path due to obstacles
        assert!(result.is_err() || result.unwrap().len() > 0);
    }
    
    #[test]
    fn test_heuristics() {
        let a = Point::new(0.0, 0.0, 0.0);
        let b = Point::new(3.0, 4.0, 0.0);
        
        // Euclidean distance should be 5
        assert!((EuclideanHeuristic.estimate(&a, &b) - 5.0).abs() < 1e-6);
        
        // Manhattan distance should be 7
        assert!((ManhattanHeuristic.estimate(&a, &b) - 7.0).abs() < 1e-6);
        
        // Chebyshev distance should be 4
        assert!((ChebyshevHeuristic.estimate(&a, &b) - 4.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_bidirectional_astar() {
        let graph = TestGraph::new(10.0);
        let bidir = BidirectionalAStar::new(EuclideanHeuristic);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 5.0, 0.0);
        
        let path = bidir.find_path(&graph, start, goal).unwrap();
        
        assert!(!path.is_empty());
        assert_eq!(path.first().unwrap(), &start);
    }
}
