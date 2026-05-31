mod astar;
mod rrt;
mod cache;

use nalgebra::Vector3;
use std::collections::HashMap;
use rayon::prelude::*;

/// A cell in the spatial grid containing points
#[derive(Default)]
struct Cell {
    points: Vec<(usize, Vector3<f64>)>, // (index, position)
}

/// Cache-aware spatial grid for efficient nearest neighbor searches
pub struct SpatialGrid {
    cell_size: f64,
    cells: HashMap<(i32, i32, i32), Cell>,
    bounds: (Vector3<f64>, Vector3<f64>), // (min, max)
}

impl SpatialGrid {
    /// Create a new spatial grid with the given cell size
    pub fn new(cell_size: f64) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            bounds: (Vector3::zeros(), Vector3::zeros()),
        }
    }

    /// Insert a point into the grid
    pub fn insert(&mut self, index: usize, position: Vector3<f64>) {
        let cell_coords = self.get_cell_coords(&position);
        let cell = self.cells.entry(cell_coords).or_default();
        cell.points.push((index, position));

        // Update bounds
        if self.cells.len() == 1 {
            self.bounds = (position, position);
        } else {
            self.bounds.0 = self.bounds.0.zip_map(&position, f64::min);
            self.bounds.1 = self.bounds.1.zip_map(&position, f64::max);
        }
    }

    /// Insert multiple points in parallel
    pub fn insert_batch(&mut self, points: Vec<(usize, Vector3<f64>)>) {
        // Group points by cell in parallel
        let cell_points: HashMap<_, Vec<_>> = points
            .into_par_iter()
            .fold(
                || HashMap::new(),
                |mut acc, (index, position)| {
                    let cell_coords = self.get_cell_coords(&position);
                    acc.entry(cell_coords)
                        .or_default()
                        .push((index, position));
                    acc
                },
            )
            .reduce(
                || HashMap::new(),
                |mut a, b| {
                    for (coords, points) in b {
                        a.entry(coords)
                            .or_default()
                            .extend(points);
                    }
                    a
                },
            );

        // Update cells and bounds
        for (coords, points) in cell_points {
            let cell = self.cells.entry(coords).or_default();
            cell.points.extend(points);
            
            // Update bounds
            for &(_, pos) in &cell.points {
                self.bounds.0 = self.bounds.0.zip_map(&pos, f64::min);
                self.bounds.1 = self.bounds.1.zip_map(&pos, f64::max);
            }
        }
    }

    /// Find the k nearest neighbors to multiple query points in parallel
    pub fn find_nearest_k_batch(&self, queries: &[Vector3<f64>], k: usize) -> Vec<Vec<(usize, f64)>> {
        queries
            .par_iter()
            .map(|query| self.find_nearest_k(query, k))
            .collect()
    }

    /// Find the k nearest neighbors to a query point
    pub fn find_nearest_k(&self, query: &Vector3<f64>, k: usize) -> Vec<(usize, f64)> {
        let mut neighbors = Vec::new();
        let cell_coords = self.get_cell_coords(query);
        
        // Search in expanding rings of cells until we have enough points
        let mut ring = 0;
        while neighbors.len() < k {
            let cells_to_search = self.get_cells_in_ring(cell_coords, ring);
            if cells_to_search.is_empty() {
                break;
            }

            // Process cells in parallel
            let mut ring_neighbors: Vec<_> = cells_to_search
                .par_iter()
                .filter_map(|coords| self.cells.get(coords))
                .flat_map(|cell| {
                    cell.points
                        .iter()
                        .map(|&(idx, pos)| {
                            let dist = (pos - query).norm();
                            (idx, dist)
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            neighbors.extend(ring_neighbors);
            
            // Sort and keep only k nearest
            neighbors.par_sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            if neighbors.len() > k {
                neighbors.truncate(k);
            }

            ring += 1;
        }

        neighbors
    }

    /// Get the cell coordinates for a position
    fn get_cell_coords(&self, position: &Vector3<f64>) -> (i32, i32, i32) {
        (
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }

    /// Get all cell coordinates in a ring at the given distance from the center
    fn get_cells_in_ring(&self, center: (i32, i32, i32), ring: i32) -> Vec<(i32, i32, i32)> {
        if ring == 0 {
            return vec![center];
        }

        let mut cells = Vec::new();
        for dx in -ring..=ring {
            for dy in -ring..=ring {
                for dz in -ring..=ring {
                    // Only include cells on the surface of the cube
                    if dx.abs() == ring || dy.abs() == ring || dz.abs() == ring {
                        cells.push((
                            center.0 + dx,
                            center.1 + dy,
                            center.2 + dz,
                        ));
                    }
                }
            }
        }
        cells
    }

    /// Clear all points from the grid
    pub fn clear(&mut self) {
        self.cells.clear();
        self.bounds = (Vector3::zeros(), Vector3::zeros());
    }
}

pub use astar::*;
pub use rrt::*;
pub use cache::*;

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_parallel_batch_operations() {
        let mut grid = SpatialGrid::new(1.0);
        let mut rng = rand::thread_rng();

        // Generate test points
        let points: Vec<(usize, Vector3<f64>)> = (0..1000)
            .map(|i| {
                (i, Vector3::new(
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                ))
            })
            .collect();

        // Test batch insertion
        grid.insert_batch(points.clone());

        // Test batch queries
        let queries: Vec<Vector3<f64>> = (0..10)
            .map(|_| {
                Vector3::new(
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                    rng.gen_range(-5.0..5.0),
                )
            })
            .collect();

        let results = grid.find_nearest_k_batch(&queries, 5);
        assert_eq!(results.len(), queries.len());
        for neighbors in results {
            assert_eq!(neighbors.len(), 5);
            // Verify distances are sorted
            for i in 1..neighbors.len() {
                assert!(neighbors[i].1 >= neighbors[i-1].1);
            }
        }
    }

    #[test]
    fn test_cell_assignment() {
        let mut grid = SpatialGrid::new(1.0);
        let point = Vector3::new(1.5, -2.3, 3.7);
        grid.insert(0, point);

        let cell_coords = grid.get_cell_coords(&point);
        assert_eq!(cell_coords, (1, -3, 3));
    }
} 