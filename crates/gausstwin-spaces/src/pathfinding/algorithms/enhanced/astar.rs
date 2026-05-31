use crate::{
    pathfinding::{
        cache::HighPerformanceCache,
        error::PathfindingError,
        traits::{CollisionChecker, CostFunction},
    },
    Point,
};
use dashmap::DashMap;
use parking_lot::RwLock;
use priority_queue::PriorityQueue;
use rayon::prelude::*;
use smallvec::{SmallVec, smallvec};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};
use std::simd::{f64x4, mask64x4};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use nalgebra::Point3;
use crate::memory::MemoryPool;
use crate::pathfinding::{Cost, Path, Point};
use crate::spatial::SpatialGrid;

/// Configuration for enhanced A* algorithm
#[derive(Debug, Clone)]
pub struct AStarConfig {
    /// Maximum number of iterations
    pub max_iterations: usize,
    /// Heuristic weight (1.0 = standard A*)
    pub heuristic_weight: f64,
    /// Whether to use beam search
    pub use_beam_search: bool,
    /// Initial beam width
    pub initial_beam_width: usize,
    /// Maximum beam width
    pub max_beam_width: usize,
    /// Whether to use parallel neighbor expansion
    pub parallel_expansion: bool,
    /// Whether to use SIMD acceleration
    pub use_simd: bool,
    /// Cache size for path segments
    pub cache_size: usize,
}

impl Default for AStarConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10000,
            heuristic_weight: 1.0,
            use_beam_search: true,
            initial_beam_width: 8,
            max_beam_width: 64,
            parallel_expansion: true,
            use_simd: true,
            cache_size: 1000,
        }
    }
}

/// Node in the A* search
#[derive(Debug, Clone)]
struct Node {
    point: Point,
    g_score: f64,
    f_score: f64,
}

/// SIMD-accelerated distance calculator
struct SimdDistanceCalculator {
    chunk_size: usize,
}

impl SimdDistanceCalculator {
    fn new() -> Self {
        Self { chunk_size: 4 }
    }
    
    /// Calculate distances using SIMD instructions
    unsafe fn calculate_distances_simd(&self, points: &[Point], target: &Point) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points.len());
        let target_x = f64x4::splat(target.x);
        let target_y = f64x4::splat(target.y);
        let target_z = f64x4::splat(target.z);
        
        for chunk in points.chunks(self.chunk_size) {
            let mut x_vals = [0.0; 4];
            let mut y_vals = [0.0; 4];
            let mut z_vals = [0.0; 4];
            
            for (i, point) in chunk.iter().enumerate() {
                x_vals[i] = point.x;
                y_vals[i] = point.y;
                z_vals[i] = point.z;
            }
            
            let x = f64x4::from_array(x_vals);
            let y = f64x4::from_array(y_vals);
            let z = f64x4::from_array(z_vals);
            
            let dx = x - target_x;
            let dy = y - target_y;
            let dz = z - target_z;
            
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let mask = mask64x4::from_array([
                chunk.len() > 0,
                chunk.len() > 1,
                chunk.len() > 2,
                chunk.len() > 3,
            ]);
            
            let dist_arr = dist_sq.to_array();
            distances.extend(dist_arr.iter()
                .zip(mask.to_array())
                .filter(|&(_, m)| m)
                .map(|(&d, _)| d.sqrt()));
        }
        
        distances
    }
}

/// Parallel neighbor expansion
struct ParallelNeighborExpander {
    chunk_size: usize,
}

impl ParallelNeighborExpander {
    fn new(chunk_size: usize) -> Self {
        Self { chunk_size }
    }
    
    fn expand_parallel(
        &self,
        neighbors: &[Point],
        goal: &Point,
        cost_function: &dyn CostFunction,
    ) -> Vec<(Point, f64)> {
        if neighbors.len() <= self.chunk_size {
            // Sequential processing for small sets
            neighbors.iter()
                .map(|&p| (p, cost_function.calculate_cost(p, *goal)))
                .collect()
        } else {
            // Parallel processing for large sets
            neighbors.par_chunks(self.chunk_size)
                .flat_map(|chunk| {
                    chunk.iter()
                        .map(|&p| (p, cost_function.calculate_cost(p, *goal)))
                        .collect::<Vec<_>>()
                })
                .collect()
        }
    }
}

/// Adaptive beam width controller
struct AdaptiveBeamWidth {
    current_width: RwLock<usize>,
    min_width: usize,
    max_width: usize,
}

impl AdaptiveBeamWidth {
    fn new(min_width: usize, max_width: usize) -> Self {
        Self {
            current_width: RwLock::new(min_width),
            min_width,
            max_width,
        }
    }
    
    fn adjust(&self, path_found: bool, iteration_count: usize) {
        let mut width = self.current_width.write();
        if path_found {
            // Path found - reduce beam width
            *width = (*width * 2 / 3).max(self.min_width);
        } else if iteration_count >= 1000 {
            // Search taking too long - increase beam width
            *width = (*width * 3 / 2).min(self.max_width);
        }
    }
    
    fn get_width(&self) -> usize {
        *self.current_width.read()
    }
}

/// Enhanced A* implementation with optimizations
pub struct EnhancedAStar {
    collision_checker: Arc<dyn CollisionChecker>,
    cost_function: Arc<dyn CostFunction>,
    cache: RwLock<HighPerformanceCache>,
    config: AStarConfig,
    neighbor_cache: DashMap<Point, SmallVec<[Point; 8]>>,
    distance_cache: DashMap<(Point, Point), f64>,
    visited_regions: DashMap<Point, usize>,
    parallel_expander: ParallelNeighborExpander,
    simd_calculator: SimdDistanceCalculator,
    beam_width: AdaptiveBeamWidth,
}

impl EnhancedAStar {
    pub fn new(
        collision_checker: Arc<dyn CollisionChecker>,
        cost_function: Arc<dyn CostFunction>,
        config: AStarConfig,
    ) -> Self {
        Self {
            collision_checker,
            cost_function,
            cache: RwLock::new(HighPerformanceCache::new(config.cache_size)),
            config,
            neighbor_cache: DashMap::new(),
            distance_cache: DashMap::new(),
            visited_regions: DashMap::new(),
            parallel_expander: ParallelNeighborExpander::new(32),
            simd_calculator: SimdDistanceCalculator::new(),
            beam_width: AdaptiveBeamWidth::new(config.initial_beam_width, config.max_beam_width),
        }
    }
    
    pub fn find_path(&self, start: Point, goal: Point) -> Result<Vec<Point>, PathfindingError> {
        // Check cache first
        if let Some(path) = self.cache.read().get(&(start, goal)) {
            return Ok(path.clone());
        }
        
        let mut open_set = PriorityQueue::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();
        let mut iteration_count = 0;
        
        // Initialize start node
        let start_node = Node {
            point: start,
            g_score: 0.0,
            f_score: self.config.heuristic_weight * self.heuristic(start, goal),
        };
        
        open_set.push(start, std::cmp::Reverse(start_node.f_score));
        g_score.insert(start, 0.0);
        
        while let Some((current, _)) = open_set.pop() {
            iteration_count += 1;
            
            if iteration_count > self.config.max_iterations {
                return Err(PathfindingError::MaxIterationsReached);
            }
            
            if current == goal {
                let path = self.reconstruct_path(&came_from, current);
                self.cache.write().insert((start, goal), path.clone());
                self.beam_width.adjust(true, iteration_count);
                return Ok(path);
            }
            
            self.expand_neighbors_enhanced(
                &Node {
                    point: current,
                    g_score: *g_score.get(&current).unwrap(),
                    f_score: 0.0, // Not needed for expansion
                },
                &goal,
                &mut open_set,
                &mut came_from,
                &mut g_score,
            );
        }
        
        self.beam_width.adjust(false, iteration_count);
        Err(PathfindingError::NoPathFound)
    }
    
    fn expand_neighbors_enhanced(
        &self,
        current: &Node,
        goal: &Point,
        open_set: &mut PriorityQueue<Point, std::cmp::Reverse<f64>>,
        came_from: &mut HashMap<Point, Point>,
        g_score: &mut HashMap<Point, f64>,
    ) {
        let start_time = Instant::now();
        let neighbors = self.get_neighbors(&current.point);
        
        // Use SIMD for distance calculations if enabled
        let distances = if self.config.use_simd {
            unsafe {
                self.simd_calculator.calculate_distances_simd(&neighbors, goal)
            }
        } else {
            neighbors.iter()
                .map(|p| self.euclidean_distance(p, goal))
                .collect()
        };
        
        // Parallel neighbor processing if enabled
        let expanded = if self.config.parallel_expansion {
            self.parallel_expander.expand_parallel(&neighbors, goal, &*self.cost_function)
        } else {
            neighbors.iter()
                .map(|&p| (p, self.cost_function.calculate_cost(p, *goal)))
                .collect()
        };
        
        // Apply beam search if enabled
        let beam_width = if self.config.use_beam_search {
            self.beam_width.get_width()
        } else {
            expanded.len()
        };
        
        let mut best_neighbors: Vec<_> = expanded.into_iter()
            .zip(distances)
            .filter(|((point, _), _)| self.is_valid_neighbor(&current.point, point))
            .map(|((point, cost), dist)| {
                let tentative_g = current.g_score + cost;
                let h = self.config.heuristic_weight * dist;
                (point, tentative_g, tentative_g + h)
            })
            .collect();
        
        best_neighbors.sort_unstable_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        best_neighbors.truncate(beam_width);
        
        for (neighbor, tentative_g, f_score) in best_neighbors {
            if let Some(&current_g) = g_score.get(&neighbor) {
                if tentative_g >= current_g {
                    continue;
                }
            }
            
            came_from.insert(neighbor, current.point);
            g_score.insert(neighbor, tentative_g);
            open_set.push(neighbor, std::cmp::Reverse(f_score));
            
            // Update region visit count for adaptive beam width
            let region = self.get_region(neighbor);
            self.visited_regions.entry(region)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }
    
    fn get_neighbors(&self, point: &Point) -> Vec<Point> {
        if let Some(cached) = self.neighbor_cache.get(point) {
            return cached.clone();
        }
        
        let mut neighbors = SmallVec::new();
        // Generate neighbors based on movement constraints
        // This is a simplified version - implement your own neighbor generation logic
        for dx in [-1.0, 0.0, 1.0].iter() {
            for dy in [-1.0, 0.0, 1.0].iter() {
                for dz in [-1.0, 0.0, 1.0].iter() {
                    if *dx == 0.0 && *dy == 0.0 && *dz == 0.0 {
                        continue;
                    }
                    let neighbor = Point::new(
                        point.x + dx,
                        point.y + dy,
                        point.z + dz,
                    );
                    if self.collision_checker.is_valid_position(&neighbor) {
                        neighbors.push(neighbor);
                    }
                }
            }
        }
        
        self.neighbor_cache.insert(*point, neighbors.clone());
        neighbors.into_vec()
    }
    
    fn is_valid_neighbor(&self, from: &Point, to: &Point) -> bool {
        self.collision_checker.check_line_of_sight(from, to)
    }
    
    fn heuristic(&self, from: Point, to: Point) -> f64 {
        if let Some(&dist) = self.distance_cache.get(&(from, to)) {
            return dist;
        }
        
        let dist = self.euclidean_distance(&from, &to);
        self.distance_cache.insert((from, to), dist);
        dist
    }
    
    fn euclidean_distance(&self, a: &Point, b: &Point) -> f64 {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let dz = b.z - a.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    fn get_region(&self, point: Point) -> Point {
        // Discretize space into regions for visit tracking
        let region_size = 10.0;
        Point::new(
            (point.x / region_size).floor() * region_size,
            (point.y / region_size).floor() * region_size,
            (point.z / region_size).floor() * region_size,
        )
    }
    
    fn reconstruct_path(&self, came_from: &HashMap<Point, Point>, current: Point) -> Vec<Point> {
        let mut path = vec![current];
        let mut current = current;
        
        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }
        
        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    
    struct TestCollisionChecker;
    
    impl CollisionChecker for TestCollisionChecker {
        fn is_valid_position(&self, point: &Point) -> bool {
            point.x >= -10.0 && point.x <= 10.0 &&
            point.y >= -10.0 && point.y <= 10.0 &&
            point.z >= -10.0 && point.z <= 10.0
        }
        
        fn check_line_of_sight(&self, from: &Point, to: &Point) -> bool {
            // Simple line of sight check
            self.is_valid_position(from) && self.is_valid_position(to)
        }
    }
    
    struct TestCostFunction;
    
    impl CostFunction for TestCostFunction {
        fn calculate_cost(&self, from: Point, to: Point) -> f64 {
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let dz = to.z - from.z;
            (dx * dx + dy * dy + dz * dz).sqrt()
        }
    }
    
    #[test]
    fn test_path_finding() {
        let collision_checker = Arc::new(TestCollisionChecker);
        let cost_function = Arc::new(TestCostFunction);
        let config = AStarConfig::default();
        
        let astar = EnhancedAStar::new(collision_checker, cost_function, config);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 5.0, 5.0);
        
        let path = astar.find_path(start, goal).unwrap();
        
        assert!(!path.is_empty());
        assert_eq!(path[0], start);
        assert_eq!(path[path.len() - 1], goal);
        
        // Verify path continuity
        for i in 1..path.len() {
            let dist = astar.euclidean_distance(&path[i-1], &path[i]);
            assert!(dist <= 2.0); // Maximum step size for diagonal movement
        }
    }
    
    #[test]
    fn test_invalid_path() {
        let collision_checker = Arc::new(TestCollisionChecker);
        let cost_function = Arc::new(TestCostFunction);
        let config = AStarConfig {
            max_iterations: 10, // Very low to force failure
            ..Default::default()
        };
        
        let astar = EnhancedAStar::new(collision_checker, cost_function, config);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(100.0, 100.0, 100.0); // Outside valid range
        
        let result = astar.find_path(start, goal);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_path_caching() {
        let collision_checker = Arc::new(TestCollisionChecker);
        let cost_function = Arc::new(TestCostFunction);
        let config = AStarConfig::default();
        
        let astar = EnhancedAStar::new(collision_checker, cost_function, config);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(3.0, 3.0, 3.0);
        
        // First path finding
        let start_time = Instant::now();
        let path1 = astar.find_path(start, goal).unwrap();
        let first_duration = start_time.elapsed();
        
        // Second path finding (should use cache)
        let start_time = Instant::now();
        let path2 = astar.find_path(start, goal).unwrap();
        let second_duration = start_time.elapsed();
        
        assert_eq!(path1, path2);
        assert!(second_duration < first_duration);
    }
    
    #[test]
    fn test_beam_search() {
        let collision_checker = Arc::new(TestCollisionChecker);
        let cost_function = Arc::new(TestCostFunction);
        let config = AStarConfig {
            use_beam_search: true,
            initial_beam_width: 4,
            max_beam_width: 8,
            ..Default::default()
        };
        
        let astar = EnhancedAStar::new(collision_checker, cost_function, config);
        
        let start = Point::new(0.0, 0.0, 0.0);
        let goal = Point::new(5.0, 5.0, 5.0);
        
        let path = astar.find_path(start, goal).unwrap();
        assert!(!path.is_empty());
        
        // Verify beam width adaptation
        let initial_width = astar.beam_width.get_width();
        
        // Force some failed searches
        for _ in 0..5 {
            let _ = astar.find_path(start, Point::new(100.0, 100.0, 100.0));
        }
        
        let adapted_width = astar.beam_width.get_width();
        assert!(adapted_width > initial_width);
    }
} 