//! Pathfinding Traits Module
//!
//! Core traits and interfaces for pathfinding algorithms.

use super::{Cost, Path, Point};
use super::error::PathfindingError;

/// Trait for graph representations used in pathfinding
pub trait Graph: Send + Sync {
    /// Get neighboring nodes from a given position
    fn neighbors(&self, point: &Point) -> Vec<Point>;
    
    /// Get the cost of traversing from one node to another
    fn cost(&self, from: &Point, to: &Point) -> Cost;
    
    /// Check if a position is valid (within bounds and not blocked)
    fn is_valid_position(&self, point: &Point) -> bool;
    
    /// Get successors with costs (for optimization)
    fn successors(&self, point: &Point) -> Vec<(Point, Cost)> {
        self.neighbors(point)
            .into_iter()
            .map(|n| {
                let c = self.cost(point, &n);
                (n, c)
            })
            .collect()
    }
    
    /// Estimate if a path exists (for quick rejection)
    fn path_might_exist(&self, _start: &Point, _goal: &Point) -> bool {
        true
    }
}

/// Trait for heuristic functions used in informed search
pub trait Heuristic: Send + Sync {
    /// Estimate the cost from a point to the goal
    /// 
    /// For A* to find optimal paths, this must be admissible (never overestimate).
    /// For better performance, it should also be consistent (monotonic).
    fn estimate(&self, from: &Point, to: &Point) -> Cost;
    
    /// Check if heuristic is admissible
    fn is_admissible(&self) -> bool {
        true
    }
    
    /// Check if heuristic is consistent (monotonic)
    fn is_consistent(&self) -> bool {
        true
    }
}

/// Trait for pathfinding algorithms
pub trait PathfindingAlgorithm<G: Graph>: Send + Sync {
    /// Find a path from start to goal
    fn find_path(&self, graph: &G, start: Point, goal: Point) -> Result<Path, PathfindingError>;
    
    /// Find a path with intermediate waypoints
    fn find_path_via(&self, graph: &G, waypoints: &[Point]) -> Result<Path, PathfindingError> {
        if waypoints.len() < 2 {
            return Err(PathfindingError::InvalidWaypoints);
        }
        
        let mut full_path = Vec::new();
        
        for window in waypoints.windows(2) {
            let segment = self.find_path(graph, window[0], window[1])?;
            
            // Avoid duplicating waypoints
            if full_path.is_empty() {
                full_path.extend(segment);
            } else {
                full_path.extend(segment.into_iter().skip(1));
            }
        }
        
        Ok(full_path)
    }
    
    /// Check if a path exists without computing it
    fn path_exists(&self, graph: &G, start: Point, goal: Point) -> bool {
        self.find_path(graph, start, goal).is_ok()
    }
}

/// Trait for hierarchical pathfinding
pub trait HierarchicalGraph: Graph {
    /// Get the abstract level of a node
    fn node_level(&self, point: &Point) -> usize;
    
    /// Get the abstract representation of a point
    fn abstract_point(&self, point: &Point) -> Option<Point>;
    
    /// Refine an abstract path to concrete path
    fn refine_path(&self, abstract_path: &Path) -> Result<Path, PathfindingError>;
    
    /// Get boundaries for a region
    fn region_boundaries(&self, region: &Point) -> Vec<Point>;
}

/// Trait for dynamic graphs that change over time
pub trait DynamicGraph: Graph {
    /// Update the cost of an edge
    fn update_edge_cost(&mut self, from: &Point, to: &Point, cost: Cost);
    
    /// Block a node (make it impassable)
    fn block_node(&mut self, point: &Point);
    
    /// Unblock a node
    fn unblock_node(&mut self, point: &Point);
    
    /// Get version number for change tracking
    fn version(&self) -> u64;
    
    /// Get changed nodes since a version
    fn changes_since(&self, version: u64) -> Vec<Point>;
}

/// Trait for graphs supporting incremental updates
pub trait IncrementalGraph: DynamicGraph {
    /// Notify of an edge cost increase
    fn notify_cost_increase(&mut self, from: &Point, to: &Point, old_cost: Cost, new_cost: Cost);
    
    /// Notify of an edge cost decrease
    fn notify_cost_decrease(&mut self, from: &Point, to: &Point, old_cost: Cost, new_cost: Cost);
}

/// Trait for weighted graphs with edge attributes
pub trait WeightedGraph: Graph {
    /// Get all edge attributes
    fn edge_attributes(&self, from: &Point, to: &Point) -> EdgeAttributes;
    
    /// Check if an edge is bidirectional
    fn is_bidirectional(&self, from: &Point, to: &Point) -> bool;
}

/// Edge attributes for weighted graphs
#[derive(Debug, Clone, Default)]
pub struct EdgeAttributes {
    /// Base cost
    pub cost: Cost,
    /// Time-dependent cost factor
    pub time_factor: f64,
    /// Risk/danger level
    pub risk: f64,
    /// Terrain type identifier
    pub terrain_type: u32,
    /// Custom attributes
    pub custom: std::collections::HashMap<String, f64>,
}

/// Trait for multi-objective pathfinding
pub trait MultiObjectiveGraph: Graph {
    /// Get all objectives for an edge
    fn objectives(&self, from: &Point, to: &Point) -> Vec<Cost>;
    
    /// Number of objectives
    fn num_objectives(&self) -> usize;
    
    /// Objective names
    fn objective_names(&self) -> Vec<String>;
}

/// Result of multi-objective pathfinding
#[derive(Debug, Clone)]
pub struct ParetoPath {
    /// The path
    pub path: Path,
    /// Costs for each objective
    pub costs: Vec<Cost>,
}

/// Trait for path smoothing
pub trait PathSmoother: Send + Sync {
    /// Smooth a path to reduce unnecessary turns
    fn smooth(&self, path: Path, graph: &dyn Graph) -> Path;
    
    /// Smooth with quality parameter (0.0-1.0)
    fn smooth_with_quality(&self, path: Path, graph: &dyn Graph, quality: f64) -> Path {
        let _ = quality;
        self.smooth(path, graph)
    }
}

/// Trait for path validators
pub trait PathValidator: Send + Sync {
    /// Validate that a path is traversable
    fn validate(&self, path: &Path, graph: &dyn Graph) -> Result<(), PathfindingError>;
    
    /// Get the actual cost of a path
    fn calculate_cost(&self, path: &Path, graph: &dyn Graph) -> Cost;
}

/// Basic path smoother using string-pulling
#[derive(Debug, Clone, Default)]
pub struct StringPullingPathSmoother;

impl PathSmoother for StringPullingPathSmoother {
    fn smooth(&self, path: Path, _graph: &dyn Graph) -> Path {
        if path.len() <= 2 {
            return path;
        }
        
        // Simple string-pulling algorithm
        let mut smoothed = vec![path[0]];
        let mut current_idx = 0;
        
        while current_idx < path.len() - 1 {
            // Try to skip to the furthest visible point
            let mut furthest = current_idx + 1;
            
            for i in (current_idx + 2)..path.len() {
                // Simplified visibility check - would need proper line-of-sight
                furthest = i;
            }
            
            smoothed.push(path[furthest]);
            current_idx = furthest;
        }
        
        smoothed
    }
}

/// Basic path validator
#[derive(Debug, Clone, Default)]
pub struct BasicPathValidator;

impl PathValidator for BasicPathValidator {
    fn validate(&self, path: &Path, graph: &dyn Graph) -> Result<(), PathfindingError> {
        if path.is_empty() {
            return Err(PathfindingError::InvalidPath("Empty path".to_string()));
        }
        
        // Check all points are valid
        for point in path {
            if !graph.is_valid_position(point) {
                return Err(PathfindingError::InvalidPath(
                    format!("Invalid position: {:?}", point)
                ));
            }
        }
        
        // Check consecutive points are neighbors
        for window in path.windows(2) {
            let neighbors = graph.neighbors(&window[0]);
            if !neighbors.iter().any(|n| {
                let dx = (n.x - window[1].x).abs();
                let dy = (n.y - window[1].y).abs();
                let dz = (n.z - window[1].z).abs();
                dx < 1e-6 && dy < 1e-6 && dz < 1e-6
            }) {
                return Err(PathfindingError::InvalidPath(
                    format!("Discontinuous path at {:?}", window[0])
                ));
            }
        }
        
        Ok(())
    }
    
    fn calculate_cost(&self, path: &Path, graph: &dyn Graph) -> Cost {
        path.windows(2)
            .map(|w| graph.cost(&w[0], &w[1]))
            .sum()
    }
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
            point.x >= 0.0 && point.x <= 10.0
                && point.y >= 0.0 && point.y <= 10.0
                && point.z >= 0.0 && point.z <= 10.0
        }
    }
    
    #[test]
    fn test_basic_validator() {
        let graph = SimpleGraph;
        let validator = BasicPathValidator;
        
        let valid_path = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(2.0, 0.0, 0.0),
        ];
        
        assert!(validator.validate(&valid_path, &graph).is_ok());
        
        let cost = validator.calculate_cost(&valid_path, &graph);
        assert_eq!(cost, 2.0);
    }
    
    #[test]
    fn test_path_smoother() {
        let graph = SimpleGraph;
        let smoother = StringPullingPathSmoother;
        
        let path = vec![
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
            Point::new(2.0, 0.0, 0.0),
            Point::new(3.0, 0.0, 0.0),
        ];
        
        let smoothed = smoother.smooth(path.clone(), &graph);
        
        // Should reduce path length if possible
        assert!(!smoothed.is_empty());
    }
}
