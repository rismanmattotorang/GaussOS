use std::sync::Arc;

use dashmap::DashMap;
use nalgebra::Point3;
use rayon::prelude::*;
use thiserror::Error;

#[cfg(feature = "simd")]
use std::simd::{f64x4, mask64x4};

use crate::memory::MemoryPool;
use crate::Point;

#[derive(Debug, Error)]
pub enum GridError {
    #[error("Invalid grid dimensions")]
    InvalidDimensions,
    #[error("Point out of bounds")]
    OutOfBounds,
    #[error("Memory allocation failed")]
    MemoryError,
}

/// A high-performance spatial grid with SIMD acceleration
pub struct SpatialGrid<T> {
    /// Grid dimensions
    dimensions: Point3<usize>,
    /// Cell size
    cell_size: f64,
    /// Grid cells
    cells: Arc<DashMap<usize, Vec<(Point, T)>>>,
    /// Memory pool for cell allocations
    memory_pool: Arc<MemoryPool<Vec<(Point, T)>>>,
    /// Grid bounds
    bounds: (Point, Point),
}

impl<T: Send + Sync + Clone> SpatialGrid<T> {
    /// Create a new spatial grid
    pub fn new(
        dimensions: Point3<usize>,
        cell_size: f64,
        bounds: (Point, Point),
    ) -> Result<Self, GridError> {
        if dimensions.x == 0 || dimensions.y == 0 || dimensions.z == 0 {
            return Err(GridError::InvalidDimensions);
        }

        let total_cells = dimensions.x * dimensions.y * dimensions.z;
        let memory_pool = Arc::new(MemoryPool::new(total_cells)
            .map_err(|_| GridError::MemoryError)?);

        Ok(Self {
            dimensions,
            cell_size,
            cells: Arc::new(DashMap::new()),
            memory_pool,
            bounds,
        })
    }

    /// Insert a point with associated data
    pub fn insert(&self, point: Point, data: T) -> Result<(), GridError> {
        let cell_index = self.get_cell_index(&point)?;
        
        self.cells
            .entry(cell_index)
            .or_insert_with(|| Vec::new())
            .push((point, data));

        Ok(())
    }

    /// Remove a point
    pub fn remove(&self, point: &Point) -> Result<Option<T>, GridError> {
        let cell_index = self.get_cell_index(point)?;
        
        if let Some(mut cell) = self.cells.get_mut(&cell_index) {
            if let Some(pos) = cell.iter().position(|(p, _)| p == point) {
                let (_, data) = cell.swap_remove(pos);
                return Ok(Some(data));
            }
        }
        
        Ok(None)
    }

    /// Query points within radius
    #[cfg(feature = "simd")]
    pub fn query_radius(&self, center: &Point, radius: f64) -> Vec<(Point, T)> {
        let radius_squared = radius * radius;
        let cell_radius = (radius / self.cell_size).ceil() as isize;
        let mut result = Vec::new();

        let center_index = self.get_cell_index(center).unwrap_or(0);
        let (cx, cy, cz) = self.index_to_coords(center_index);

        // Pre-compute SIMD vectors
        let center_x = f64x4::splat(center.x);
        let center_y = f64x4::splat(center.y);
        let center_z = f64x4::splat(center.z);
        let radius_squared_simd = f64x4::splat(radius_squared);

        // Parallel iteration over neighboring cells
        (-cell_radius..=cell_radius).into_par_iter().for_each(|dx| {
            (-cell_radius..=cell_radius).for_each(|dy| {
                (-cell_radius..=cell_radius).for_each(|dz| {
                    let nx = cx.wrapping_add(dx as usize);
                    let ny = cy.wrapping_add(dy as usize);
                    let nz = cz.wrapping_add(dz as usize);

                    if nx < self.dimensions.x && ny < self.dimensions.y && nz < self.dimensions.z {
                        let neighbor_index = self.coords_to_index(nx, ny, nz);
                        if let Some(cell) = self.cells.get(&neighbor_index) {
                            let points: Vec<_> = cell.iter().collect();
                            
                            // Process points in chunks of 4 using SIMD
                            for chunk in points.chunks(4) {
                                let mut x_coords = [0.0; 4];
                                let mut y_coords = [0.0; 4];
                                let mut z_coords = [0.0; 4];

                                for (i, &(point, _)) in chunk.iter().enumerate() {
                                    x_coords[i] = point.x;
                                    y_coords[i] = point.y;
                                    z_coords[i] = point.z;
                                }

                                let x_simd = f64x4::from_array(x_coords);
                                let y_simd = f64x4::from_array(y_coords);
                                let z_simd = f64x4::from_array(z_coords);

                                let dx_simd = x_simd - center_x;
                                let dy_simd = y_simd - center_y;
                                let dz_simd = z_simd - center_z;

                                let dist_squared_simd = dx_simd * dx_simd + dy_simd * dy_simd + dz_simd * dz_simd;
                                let mask = dist_squared_simd.simd_le(radius_squared_simd);

                                for (i, &(point, ref data)) in chunk.iter().enumerate() {
                                    if mask.test(i) {
                                        result.push((point.clone(), data.clone()));
                                    }
                                }
                            }

                            // Handle remaining points
                            let remainder = points.len() % 4;
                            if remainder > 0 {
                                for &(point, ref data) in points.iter().skip(points.len() - remainder) {
                                    let dx = point.x - center.x;
                                    let dy = point.y - center.y;
                                    let dz = point.z - center.z;
                                    let dist_squared = dx * dx + dy * dy + dz * dz;
                                    
                                    if dist_squared <= radius_squared {
                                        result.push((point.clone(), data.clone()));
                                    }
                                }
                            }
                        }
                    }
                });
            });
        });

        result
    }

    /// Query points within radius (non-SIMD version)
    #[cfg(not(feature = "simd"))]
    pub fn query_radius(&self, center: &Point, radius: f64) -> Vec<(Point, T)> {
        let radius_squared = radius * radius;
        let cell_radius = (radius / self.cell_size).ceil() as isize;
        let mut result = Vec::new();

        let center_index = self.get_cell_index(center).unwrap_or(0);
        let (cx, cy, cz) = self.index_to_coords(center_index);

        // Parallel iteration over neighboring cells
        (-cell_radius..=cell_radius).into_par_iter().for_each(|dx| {
            (-cell_radius..=cell_radius).for_each(|dy| {
                (-cell_radius..=cell_radius).for_each(|dz| {
                    let nx = cx.wrapping_add(dx as usize);
                    let ny = cy.wrapping_add(dy as usize);
                    let nz = cz.wrapping_add(dz as usize);

                    if nx < self.dimensions.x && ny < self.dimensions.y && nz < self.dimensions.z {
                        let neighbor_index = self.coords_to_index(nx, ny, nz);
                        if let Some(cell) = self.cells.get(&neighbor_index) {
                            for &(point, ref data) in cell.iter() {
                                let dx = point.x - center.x;
                                let dy = point.y - center.y;
                                let dz = point.z - center.z;
                                let dist_squared = dx * dx + dy * dy + dz * dz;
                                
                                if dist_squared <= radius_squared {
                                    result.push((point.clone(), data.clone()));
                                }
                            }
                        }
                    }
                });
            });
        });

        result
    }

    /// Get the cell index for a point
    fn get_cell_index(&self, point: &Point) -> Result<usize, GridError> {
        if !self.is_point_in_bounds(point) {
            return Err(GridError::OutOfBounds);
        }

        let x = ((point.x - self.bounds.0.x) / self.cell_size) as usize;
        let y = ((point.y - self.bounds.0.y) / self.cell_size) as usize;
        let z = ((point.z - self.bounds.0.z) / self.cell_size) as usize;

        Ok(self.coords_to_index(x, y, z))
    }

    /// Convert cell coordinates to index
    #[inline]
    fn coords_to_index(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.dimensions.x + z * self.dimensions.x * self.dimensions.y
    }

    /// Convert cell index to coordinates
    #[inline]
    fn index_to_coords(&self, index: usize) -> (usize, usize, usize) {
        let x = index % self.dimensions.x;
        let y = (index / self.dimensions.x) % self.dimensions.y;
        let z = index / (self.dimensions.x * self.dimensions.y);
        (x, y, z)
    }

    /// Check if a point is within bounds
    #[inline]
    fn is_point_in_bounds(&self, point: &Point) -> bool {
        point.x >= self.bounds.0.x && point.x <= self.bounds.1.x &&
        point.y >= self.bounds.0.y && point.y <= self.bounds.1.y &&
        point.z >= self.bounds.0.z && point.z <= self.bounds.1.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_grid_basic() {
        let grid = SpatialGrid::new(
            Point3::new(10, 10, 10),
            1.0,
            (Point::new(0.0, 0.0, 0.0), Point::new(10.0, 10.0, 10.0)),
        ).unwrap();

        // Test insertion
        grid.insert(Point::new(1.0, 1.0, 1.0), "A").unwrap();
        grid.insert(Point::new(2.0, 2.0, 2.0), "B").unwrap();

        // Test radius query
        let results = grid.query_radius(&Point::new(1.5, 1.5, 1.5), 1.0);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_grid_bounds() {
        let grid = SpatialGrid::new(
            Point3::new(10, 10, 10),
            1.0,
            (Point::new(0.0, 0.0, 0.0), Point::new(10.0, 10.0, 10.0)),
        ).unwrap();

        // Test out of bounds
        assert!(grid.insert(Point::new(-1.0, 0.0, 0.0), "A").is_err());
        assert!(grid.insert(Point::new(11.0, 0.0, 0.0), "B").is_err());
    }

    #[test]
    fn test_grid_parallel() {
        let grid = Arc::new(SpatialGrid::new(
            Point3::new(100, 100, 100),
            1.0,
            (Point::new(0.0, 0.0, 0.0), Point::new(100.0, 100.0, 100.0)),
        ).unwrap());

        // Insert points in parallel
        (0..1000).into_par_iter().for_each(|i| {
            let x = (i % 10) as f64;
            let y = ((i / 10) % 10) as f64;
            let z = (i / 100) as f64;
            grid.insert(Point::new(x, y, z), i).unwrap();
        });

        // Query in parallel
        let results: Vec<_> = (0..10).into_par_iter().map(|i| {
            let center = Point::new(i as f64, i as f64, i as f64);
            grid.query_radius(&center, 2.0)
        }).collect();

        assert!(!results.is_empty());
    }
} 