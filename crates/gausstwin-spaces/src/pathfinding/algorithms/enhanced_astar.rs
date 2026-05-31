use crate::pathfinding::{
    cache::HighPerformanceCache,
    error::{PathfindingError, PathfindingResult},
    traits::{CollisionChecker, CostFunction, PathFinder},
    Cost, Path, Point,
};
use dashmap::DashMap;
use nalgebra::Vector3;
use parking_lot::RwLock;
use priority_queue::PriorityQueue;
use rayon::prelude::*;
use smallvec::SmallVec;
use std::{
    arch::x86_64::*,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    f64::INFINITY,
    sync::{atomic::AtomicUsize, Arc},
    time::{Duration, Instant},
};

/// SIMD-accelerated distance calculator
pub struct SimdDistanceCalculator {
    chunk_size: usize,
}

impl SimdDistanceCalculator {
    pub fn new() -> Self {
        Self { chunk_size: 4 }
    }

    #[target_feature(enable = "avx2")]
    unsafe fn calculate_distances_simd(&self, points: &[Point], target: &Point) -> Vec<f64> {
        let mut distances = Vec::with_capacity(points.len());
        
        for chunk in points.chunks(4) {
            let mut xs = _mm256_setzero_pd();
            let mut ys = _mm256_setzero_pd();
            let mut zs = _mm256_setzero_pd();
            
            // Load coordinates
            for (i, point) in chunk.iter().enumerate() {
                xs = _mm256_insertf64x2(xs, _mm_set_pd(point.x, 0.0), i as i32);
                ys = _mm256_insertf64x2(ys, _mm_set_pd(point.y, 0.0), i as i32);
                zs = _mm256_insertf64x2(zs, _mm_set_pd(point.z, 0.0), i as i32);
            }
            
            // Calculate differences
            let dx = _mm256_sub_pd(xs, _mm256_set1_pd(target.x));
            let dy = _mm256_sub_pd(ys, _mm256_set1_pd(target.y));
            let dz = _mm256_sub_pd(zs, _mm256_set1_pd(target.z));
            
            // Calculate squared distances
            let dx2 = _mm256_mul_pd(dx, dx);
            let dy2 = _mm256_mul_pd(dy, dy);
            let dz2 = _mm256_mul_pd(dz, dz);
            
            // Sum components
            let sum = _mm256_add_pd(_mm256_add_pd(dx2, dy2), dz2);
            
            // Extract distances
            let mut result = [0.0; 4];
            _mm256_storeu_pd(result.as_mut_ptr(), _mm256_sqrt_pd(sum));
            
            distances.extend_from_slice(&result[..chunk.len()]);
        }
        
        distances
    }
}

/// Parallel neighbor expander
pub struct ParallelNeighborExpander {
    batch_size: usize,
}

impl ParallelNeighborExpander {
    pub fn new(batch_size: usize) -> Self {
        Self { batch_size }
    }

    pub fn expand_parallel(&self, neighbors: &[Point], goal: &Point, cost_fn: &dyn CostFunction) -> Vec<(Point, Cost)> {
        neighbors.par_chunks(self.batch_size)
            .flat_map(|chunk| {
                chunk.iter().map(|n| {
                    let cost = cost_fn.cost(n, goal);
                    (*n, cost)
                }).collect::<Vec<_>>()
            })
            .collect()
    }
}

/// Adaptive beam width controller
pub struct AdaptiveBeamWidth {
    min_width: usize,
    max_width: usize,
    current_width: AtomicUsize,
}

impl AdaptiveBeamWidth {
    pub fn new(min_width: usize, max_width: usize) -> Self {
        Self {
            min_width,
            max_width,
            current_width: AtomicUsize::new(min_width),
        }
    }

    pub fn adjust(&self, expansion_time: Duration, target_time: Duration) {
        let current = self.current_width.load(Ordering::Relaxed);
        if expansion_time > target_time {
            self.current_width.store(
                (current * 9 / 10).max(self.min_width),
                Ordering::Relaxed,
            );
        } else {
            self.current_width.store(
                (current * 11 / 10).min(self.max_width),
                Ordering::Relaxed,
            );
        }
    }

    pub fn get_width(&self) -> usize {
        self.current_width.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug)]
struct Node {
    point: Point,
    g_cost: Cost,
    f_cost: Cost,
    parent: Option<Point>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost.eq(&other.f_cost)
    }
}

impl Eq for Node {}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.f_cost.partial_cmp(&self.f_cost)
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
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
            cache: RwLock::new(HighPerformanceCache::new(Default::default())),
            config,
            neighbor_cache: DashMap::new(),
            distance_cache: DashMap::new(),
            visited_regions: DashMap::new(),
            parallel_expander: ParallelNeighborExpander::new(32),
            simd_calculator: SimdDistanceCalculator::new(),
            beam_width: AdaptiveBeamWidth::new(8, 64),
        }
    }

    fn expand_neighbors_enhanced(
        &self,
        current: &Node,
        goal: &Point,
        open_set: &mut PriorityQueue<Point, Node>,
        came_from: &mut HashMap<Point, Point>,
        g_score: &mut HashMap<Point, Cost>,
    ) {
        let start_time = Instant::now();
        let neighbors = self.get_neighbors(&current.point);
        
        // Use SIMD for distance calculations
        let distances = unsafe {
            self.simd_calculator.calculate_distances_simd(&neighbors, goal)
        };
        
        // Parallel neighbor processing
        let expanded = self.parallel_expander.expand_parallel(&neighbors, goal, &*self.cost_function);
        
        // Apply beam search
        let beam_width = self.beam_width.get_width();
        let mut best_neighbors: Vec<_> = expanded.into_iter()
            .take(beam_width)
            .collect();
        best_neighbors.par_sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        
        for (neighbor, cost) in best_neighbors {
            let tentative_g_score = current.g_cost + cost;
            
            let mut should_update = false;
            g_score
                .entry(neighbor)
                .and_modify(|e| {
                    if tentative_g_score < *e {
                        *e = tentative_g_score;
                        should_update = true;
                    }
                })
                .or_insert_with(|| {
                    should_update = true;
                    tentative_g_score
                });
            
            if should_update {
                let h_score = self.cost_function.heuristic(&neighbor, goal)
                    * self.config.heuristic_weight;
                let f_score = tentative_g_score + h_score;
                
                let neighbor_node = Node {
                    point: neighbor,
                    g_cost: tentative_g_score,
                    f_cost: f_score,
                    parent: Some(current.point),
                };
                
                open_set.push(neighbor, neighbor_node);
                came_from.insert(neighbor, current.point);
            }
        }
        
        // Adjust beam width based on performance
        self.beam_width.adjust(start_time.elapsed(), Duration::from_micros(100));
    }

    fn get_neighbors(&self, point: &Point) -> SmallVec<[Point; 8]> {
        if let Some(cached) = self.neighbor_cache.get(point) {
            return cached.clone();
        }

        let mut neighbors = SmallVec::new();
        let step = self.config.step_size;

        // Generate neighbors in 26 directions
        for dx in [-step, 0.0, step].iter() {
            for dy in [-step, 0.0, step].iter() {
                for dz in [-step, 0.0, step].iter() {
                    if dx == &0.0 && dy == &0.0 && dz == &0.0 {
                        continue;
                    }

                    let neighbor = Point::new(
                        point.x + dx,
                        point.y + dy,
                        point.z + dz,
                    );

                    if self.is_valid(&neighbor) {
                        neighbors.push(neighbor);
                    }
                }
            }
        }

        self.neighbor_cache.insert(*point, neighbors.clone());
        neighbors
    }

    fn is_valid(&self, point: &Point) -> bool {
        !self.collision_checker.is_in_collision(point)
    }
}

impl PathFinder for EnhancedAStar {
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

        let mut open_set = PriorityQueue::with_capacity(self.config.preallocate_nodes);
        let mut came_from = HashMap::with_capacity(self.config.preallocate_nodes);
        let mut g_score = HashMap::with_capacity(self.config.preallocate_nodes);
        let mut closed_set = HashSet::with_capacity(self.config.preallocate_nodes);

        // Initialize start node
        let start_node = Node {
            point: start,
            g_cost: 0.0,
            f_cost: self.cost_function.heuristic(&start, &goal),
            parent: None,
        };

        open_set.push(start, start_node);
        g_score.insert(start, 0.0);

        let mut iterations = 0;
        let mut best_distance = INFINITY;

        while let Some((current_point, current_node)) = open_set.pop() {
            if iterations >= self.config.max_iterations {
                return Err(PathfindingError::max_iterations_reached(iterations, best_distance));
            }

            if self.get_distance(&current_point, &goal) <= self.config.goal_tolerance {
                let mut path = vec![goal];
                let mut current = current_point;

                while let Some(parent) = came_from.get(&current) {
                    path.push(*parent);
                    current = *parent;
                }

                path.reverse();
                self.cache.write().store(start, goal, path.clone());
                return Ok(path);
            }

            closed_set.insert(current_point);

            self.expand_neighbors_enhanced(
                &current_node,
                &goal,
                &mut open_set,
                &mut came_from,
                &mut g_score,
            );

            iterations += 1;
        }

        Err(PathfindingError::no_path_found())
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
        if let Some(distance) = self.distance_cache.get(&(*from, *to)) {
            return *distance;
        }

        let distance = (to.coords - from.coords).norm();
        self.distance_cache.insert((*from, *to), distance);
        distance
    }
} 