use crate::pathfinding::{
    cache::HighPerformanceCache,
    error::{PathfindingError, PathfindingResult},
    traits::{CollisionChecker, CostFunction, PathFinder},
    Cost, Path, Point,
};
use dashmap::DashMap;
use nalgebra::Vector3;
use parking_lot::RwLock;
use rand::prelude::*;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    f64::INFINITY,
    sync::{atomic::{AtomicUsize, Ordering}, Arc},
    time::{Duration, Instant},
};
use crate::pathfinding::algorithms::enhanced::SpatialGrid;

/// Spatial grid for fast nearest neighbor lookup
pub struct AdaptiveGrid {
    cells: DashMap<(i32, i32, i32), Vec<Point>>,
    cell_size: f64,
}

impl AdaptiveGrid {
    pub fn new(cell_size: f64) -> Self {
        Self {
            cells: DashMap::new(),
            cell_size,
        }
    }

    fn get_cell_coords(&self, point: &Point) -> (i32, i32, i32) {
        (
            (point.coords.x / self.cell_size).floor() as i32,
            (point.coords.y / self.cell_size).floor() as i32,
            (point.coords.z / self.cell_size).floor() as i32,
        )
    }

    pub fn insert(&self, point: Point) {
        let coords = self.get_cell_coords(&point);
        self.cells.entry(coords).or_default().push(point);
    }

    pub fn get_nearby_points(&self, point: &Point, k: usize) -> Vec<Point> {
        let center = self.get_cell_coords(point);
        let mut nearby = Vec::new();

        // Search in current and neighboring cells
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let cell_coords = (
                        center.0 + dx,
                        center.1 + dy,
                        center.2 + dz,
                    );
                    if let Some(points) = self.cells.get(&cell_coords) {
                        nearby.extend(points.iter().cloned());
                    }
                }
            }
        }

        // Sort by distance and take k nearest
        nearby.par_sort_unstable_by(|a, b| {
            let da = (a.coords - point.coords).norm();
            let db = (b.coords - point.coords).norm();
            da.partial_cmp(&db).unwrap()
        });

        nearby.truncate(k);
        nearby
    }
}

/// Parallel point sampler
pub struct ParallelSampler {
    batch_size: usize,
    rng: ThreadRng,
}

impl ParallelSampler {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            rng: thread_rng(),
        }
    }

    pub fn sample_batch(&mut self, bounds: &[(f64, f64); 3], goal_bias: f64, goal: &Point) -> Vec<Point> {
        (0..self.batch_size)
            .into_par_iter()
            .map(|_| {
                if random::<f64>() < goal_bias {
                    *goal
                } else {
                    Point::new(
                        self.rng.gen_range(bounds[0].0..bounds[0].1),
                        self.rng.gen_range(bounds[1].0..bounds[1].1),
                        self.rng.gen_range(bounds[2].0..bounds[2].1),
                    )
                }
            })
            .collect()
    }
}

/// Nearest neighbor cache
pub struct NearestNeighborCache {
    cache: DashMap<Point, Vec<Point>>,
    max_size: usize,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl NearestNeighborCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_size,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, point: &Point) -> Option<Vec<Point>> {
        if let Some(neighbors) = self.cache.get(point) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(neighbors.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub fn insert(&self, point: Point, neighbors: Vec<Point>) {
        if self.cache.len() >= self.max_size {
            // Simple eviction strategy: remove random entry
            if let Some(random_key) = self.cache.iter().next() {
                self.cache.remove(random_key.key());
            }
        }
        self.cache.insert(point, neighbors);
    }
}

/// Optimized RRT implementation
pub struct OptimizedRRT {
    collision_checker: Arc<dyn CollisionChecker>,
    cost_function: Arc<dyn CostFunction>,
    cache: RwLock<HighPerformanceCache>,
    config: super::RRTConfig,
    spatial_grid: AdaptiveGrid,
    parallel_sampler: ParallelSampler,
    nearest_neighbor_cache: NearestNeighborCache,
}

impl OptimizedRRT {
    pub fn new(
        collision_checker: Arc<dyn CollisionChecker>,
        cost_function: Arc<dyn CostFunction>,
        config: super::RRTConfig,
    ) -> Self {
        Self {
            collision_checker,
            cost_function,
            cache: RwLock::new(HighPerformanceCache::new(Default::default())),
            config: config.clone(),
            spatial_grid: AdaptiveGrid::new(config.step_size * 5.0),
            parallel_sampler: ParallelSampler::new(32),
            nearest_neighbor_cache: NearestNeighborCache::new(1000),
        }
    }

    fn extend(&self, from: &Point, to: &Point) -> Option<Point> {
        let direction = to.coords - from.coords;
        let distance = direction.norm();

        if distance < self.config.step_size {
            if self.is_valid(to) {
                Some(*to)
            } else {
                None
            }
        } else {
            let normalized = direction / distance;
            let new_point = Point::from(from.coords + normalized * self.config.step_size);
            if self.is_valid(&new_point) {
                Some(new_point)
            } else {
                None
            }
        }
    }

    fn find_nearest_neighbors(&self, point: &Point, k: usize) -> Vec<Point> {
        if let Some(cached) = self.nearest_neighbor_cache.get(point) {
            return cached;
        }

        let neighbors = self.spatial_grid.get_nearby_points(point, k);
        self.nearest_neighbor_cache.insert(*point, neighbors.clone());
        neighbors
    }

    fn is_valid(&self, point: &Point) -> bool {
        !self.collision_checker.is_in_collision(point)
    }
}

impl PathFinder for OptimizedRRT {
    fn find_path(&self, start: Point, goal: Point) -> PathfindingResult<Path> {
        // Check cache first
        if let Some(cached_path) = self.cache.read().retrieve(&start, &goal) {
            return Ok(cached_path);
        }

        // Validate input points
        if !self.is_valid(&start) {
            return Err(PathfindingError::invalid_start(start, "Start point is invalid"));
        }
        if !self.is_valid(&goal) {
            return Err(PathfindingError::invalid_goal(goal, "Goal point is invalid"));
        }

        let mut points = Vec::with_capacity(self.config.preallocate_nodes);
        let mut parent_map = HashMap::with_capacity(self.config.preallocate_nodes);
        points.push(start);
        self.spatial_grid.insert(start);

        let bounds = [
            (start.coords.x.min(goal.coords.x) - 10.0, start.coords.x.max(goal.coords.x) + 10.0),
            (start.coords.y.min(goal.coords.y) - 10.0, start.coords.y.max(goal.coords.y) + 10.0),
            (start.coords.z.min(goal.coords.z) - 10.0, start.coords.z.max(goal.coords.z) + 10.0),
        ];

        let mut iterations = 0;
        let mut best_distance = INFINITY;

        while iterations < self.config.max_iterations {
            // Generate random points in parallel
            let random_points = self.parallel_sampler.sample_batch(&bounds, self.config.goal_bias, &goal);

            // Process random points in parallel
            let results: Vec<_> = random_points.par_iter().filter_map(|random_point| {
                let nearest = self.find_nearest_neighbors(random_point, 1)[0];
                self.extend(&nearest, random_point).map(|new_point| (nearest, new_point))
            }).collect();

            // Update tree with valid extensions
            for (nearest, new_point) in results {
                points.push(new_point);
                parent_map.insert(new_point, nearest);
                self.spatial_grid.insert(new_point);

                let distance = self.get_distance(&new_point, &goal);
                if distance < best_distance {
                    best_distance = distance;
                }

                if distance <= self.config.goal_tolerance {
                    // Found a path to goal
                    let mut path = vec![goal];
                    let mut current = new_point;

                    while let Some(parent) = parent_map.get(&current) {
                        path.push(*parent);
                        current = *parent;
                    }

                    path.reverse();
                    self.cache.write().store(start, goal, path.clone());
                    return Ok(path);
                }
            }

            iterations += random_points.len();
        }

        Err(PathfindingError::max_iterations_reached(iterations, best_distance))
    }

    fn is_valid(&self, point: &Point) -> bool {
        !self.collision_checker.is_in_collision(point)
    }

    fn get_cost(&self, from: &Point, to: &Point) -> Cost {
        self.cost_function.cost(from, to)
    }

    fn get_heuristic(&self, from: &Point, to: &Point) -> Cost {
        self.cost_function.heuristic(from, to)
    }

    fn get_distance(&self, from: &Point, to: &Point) -> f64 {
        (to.coords - from.coords).norm()
    }
}

pub struct EnhancedRRT {
    grid: SpatialGrid,
    step_size: f64,
    max_iterations: usize,
}

impl EnhancedRRT {
    pub fn new(step_size: f64, max_iterations: usize) -> Self {
        Self {
            grid: SpatialGrid::new(step_size), // Use step size as cell size
            step_size,
            max_iterations,
        }
    }

    pub fn find_path(
        &mut self,
        start: Vector3<f64>,
        goal: Vector3<f64>,
        obstacles: &[Vector3<f64>],
    ) -> Option<Vec<Vector3<f64>>> {
        let mut rng = rand::thread_rng();
        let mut vertices = vec![start];
        let mut parents = vec![0]; // Index of parent vertex
        
        // Insert start point into grid
        self.grid.insert(0, start);

        for i in 0..self.max_iterations {
            // Sample random point
            let random_point = if rng.gen_bool(0.1) {
                goal // Bias towards goal
            } else {
                Vector3::new(
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                )
            };

            // Find nearest vertex using spatial grid
            let nearest = self.grid.find_nearest_k(&random_point, 1);
            if nearest.is_empty() {
                continue;
            }
            let nearest_idx = nearest[0].0;
            let nearest_vertex = vertices[nearest_idx];

            // Extend towards random point
            let direction = random_point - nearest_vertex;
            let distance = direction.norm();
            let new_vertex = if distance > self.step_size {
                nearest_vertex + direction * (self.step_size / distance)
            } else {
                random_point
            };

            // Check if path is collision-free
            if self.is_path_clear(&nearest_vertex, &new_vertex, obstacles) {
                vertices.push(new_vertex);
                parents.push(nearest_idx);
                self.grid.insert(vertices.len() - 1, new_vertex);

                // Check if we can connect to goal
                if (new_vertex - goal).norm() < self.step_size
                    && self.is_path_clear(&new_vertex, &goal, obstacles)
                {
                    // Reconstruct path
                    let mut path = vec![goal, new_vertex];
                    let mut current_idx = vertices.len() - 1;
                    while current_idx != 0 {
                        current_idx = parents[current_idx];
                        path.push(vertices[current_idx]);
                    }
                    path.reverse();
                    return Some(path);
                }
            }
        }

        None
    }

    fn is_path_clear(
        &self,
        start: &Vector3<f64>,
        end: &Vector3<f64>,
        obstacles: &[Vector3<f64>],
    ) -> bool {
        let direction = end - start;
        let distance = direction.norm();
        let steps = (distance / (self.step_size * 0.1)).ceil() as usize;
        
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let point = start + direction * t;
            
            // Check collision with obstacles
            for obstacle in obstacles {
                if (point - obstacle).norm() < self.step_size {
                    return false;
                }
            }
        }
        
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_path() {
        let mut rrt = EnhancedRRT::new(1.0, 1000);
        let start = Vector3::new(0.0, 0.0, 0.0);
        let goal = Vector3::new(5.0, 5.0, 0.0);
        let obstacles = vec![Vector3::new(2.5, 2.5, 0.0)];

        let path = rrt.find_path(start, goal, &obstacles);
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path[0], start);
        assert_eq!(*path.last().unwrap(), goal);
    }

    #[test]
    fn test_no_path() {
        let mut rrt = EnhancedRRT::new(1.0, 100);
        let start = Vector3::new(0.0, 0.0, 0.0);
        let goal = Vector3::new(5.0, 5.0, 0.0);
        let obstacles = vec![
            Vector3::new(2.0, 2.0, 0.0),
            Vector3::new(2.0, 3.0, 0.0),
            Vector3::new(3.0, 2.0, 0.0),
            Vector3::new(3.0, 3.0, 0.0),
        ];

        let path = rrt.find_path(start, goal, &obstacles);
        assert!(path.is_none());
    }
} 