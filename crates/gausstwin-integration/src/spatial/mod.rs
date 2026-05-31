//! High-Performance Spatial Data Structures
//! 
//! Provides optimized spatial data structures for agent-based simulations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use nalgebra as na;

/// Spatial point in 3D space
pub type Point3 = na::Point3<f64>;
/// Vector in 3D space
pub type Vector3 = na::Vector3<f64>;
/// 3D transformation matrix
pub type Transform3 = na::Isometry3<f64>;

/// Spatial hash grid for fast neighbor queries
pub struct SpatialHashGrid {
    /// Cell size for spatial hashing
    cell_size: f64,
    /// Grid dimensions
    dimensions: Vector3,
    /// Grid cells containing agent IDs
    cells: Arc<RwLock<Vec<Vec<AgentId>>>>,
    /// Agent positions
    positions: Arc<RwLock<Vec<Point3>>>,
    /// Number of agents
    agent_count: AtomicU64,
}

impl SpatialHashGrid {
    pub fn new(bounds: &BoundingBox, cell_size: f64) -> Self {
        let dimensions = Vector3::new(
            (bounds.max.x - bounds.min.x) / cell_size,
            (bounds.max.y - bounds.min.y) / cell_size,
            (bounds.max.z - bounds.min.z) / cell_size,
        ).ceil();
        
        let cell_count = (dimensions.x * dimensions.y * dimensions.z) as usize;
        let mut cells = Vec::with_capacity(cell_count);
        cells.resize_with(cell_count, Vec::new);
        
        Self {
            cell_size,
            dimensions,
            cells: Arc::new(RwLock::new(cells)),
            positions: Arc::new(RwLock::new(Vec::new())),
            agent_count: AtomicU64::new(0),
        }
    }
    
    /// Insert agent into grid
    pub fn insert(&self, agent_id: AgentId, position: Point3) {
        let cell_idx = self.get_cell_index(&position);
        
        // Update positions
        let mut positions = self.positions.write();
        if agent_id.0 >= positions.len() {
            positions.resize(agent_id.0 + 1, Point3::origin());
        }
        positions[agent_id.0] = position;
        
        // Update cells
        let mut cells = self.cells.write();
        cells[cell_idx].push(agent_id);
        
        self.agent_count.fetch_add(1, Ordering::SeqCst);
    }
    
    /// Remove agent from grid
    pub fn remove(&self, agent_id: AgentId) {
        let position = {
            let positions = self.positions.read();
            positions[agent_id.0]
        };
        
        let cell_idx = self.get_cell_index(&position);
        
        let mut cells = self.cells.write();
        if let Some(pos) = cells[cell_idx].iter().position(|id| *id == agent_id) {
            cells[cell_idx].swap_remove(pos);
        }
        
        self.agent_count.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Update agent position
    pub fn update(&self, agent_id: AgentId, new_position: Point3) {
        let old_position = {
            let positions = self.positions.read();
            positions[agent_id.0]
        };
        
        let old_cell = self.get_cell_index(&old_position);
        let new_cell = self.get_cell_index(&new_position);
        
        if old_cell != new_cell {
            let mut cells = self.cells.write();
            // Remove from old cell
            if let Some(pos) = cells[old_cell].iter().position(|id| *id == agent_id) {
                cells[old_cell].swap_remove(pos);
            }
            // Add to new cell
            cells[new_cell].push(agent_id);
        }
        
        // Update position
        let mut positions = self.positions.write();
        positions[agent_id.0] = new_position;
    }
    
    /// Get neighbors within radius
    pub fn get_neighbors(&self, position: &Point3, radius: f64) -> Vec<AgentId> {
        let cell_radius = (radius / self.cell_size).ceil() as i32;
        let center_cell = self.get_cell_coords(position);
        let mut neighbors = Vec::new();
        
        // Iterate over neighboring cells
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                for dz in -cell_radius..=cell_radius {
                    let x = center_cell.x + dx;
                    let y = center_cell.y + dy;
                    let z = center_cell.z + dz;
                    
                    // Skip out of bounds cells
                    if x < 0 || y < 0 || z < 0 || 
                       x >= self.dimensions.x as i32 || 
                       y >= self.dimensions.y as i32 || 
                       z >= self.dimensions.z as i32 {
                        continue;
                    }
                    
                    let cell_idx = self.get_cell_index_from_coords(x as usize, y as usize, z as usize);
                    let cells = self.cells.read();
                    
                    // Check each agent in cell
                    for &agent_id in &cells[cell_idx] {
                        let positions = self.positions.read();
                        let agent_pos = positions[agent_id.0];
                        if (agent_pos - position).magnitude() <= radius {
                            neighbors.push(agent_id);
                        }
                    }
                }
            }
        }
        
        neighbors
    }
    
    /// Get cell index from position
    fn get_cell_index(&self, position: &Point3) -> usize {
        let coords = self.get_cell_coords(position);
        self.get_cell_index_from_coords(
            coords.x as usize,
            coords.y as usize,
            coords.z as usize,
        )
    }
    
    /// Get cell coordinates from position
    fn get_cell_coords(&self, position: &Point3) -> na::Point3<i32> {
        na::Point3::new(
            (position.x / self.cell_size).floor() as i32,
            (position.y / self.cell_size).floor() as i32,
            (position.z / self.cell_size).floor() as i32,
        )
    }
    
    /// Get cell index from coordinates
    fn get_cell_index_from_coords(&self, x: usize, y: usize, z: usize) -> usize {
        x + y * self.dimensions.x as usize + z * self.dimensions.x as usize * self.dimensions.y as usize
    }
}

/// R*-tree for efficient spatial queries
pub struct RStarTree {
    root: Arc<RwLock<Node>>,
    max_entries: usize,
    min_entries: usize,
}

impl RStarTree {
    pub fn new(max_entries: usize) -> Self {
        let min_entries = max_entries / 2;
        Self {
            root: Arc::new(RwLock::new(Node::Leaf {
                entries: Vec::new(),
                bbox: BoundingBox::empty(),
            })),
            max_entries,
            min_entries,
        }
    }
    
    /// Insert entry into tree
    pub fn insert(&self, entry: Entry) {
        let mut root = self.root.write();
        if root.is_full(self.max_entries) {
            // Split root
            let new_root = Node::Internal {
                children: vec![std::mem::replace(&mut *root, Node::empty())],
                bbox: root.bbox().clone(),
            };
            *root = new_root;
        }
        root.insert(entry, self.max_entries, self.min_entries);
    }
    
    /// Remove entry from tree
    pub fn remove(&self, entry: &Entry) -> bool {
        self.root.write().remove(entry)
    }
    
    /// Search entries within bounding box
    pub fn search(&self, bbox: &BoundingBox) -> Vec<Entry> {
        let mut results = Vec::new();
        self.root.read().search(bbox, &mut results);
        results
    }
    
    /// Nearest neighbor search
    pub fn nearest(&self, point: &Point3, k: usize) -> Vec<Entry> {
        let mut heap = BinaryHeap::new();
        self.root.read().nearest(point, k, &mut heap);
        heap.into_sorted_vec()
    }
}

/// R*-tree node
#[derive(Clone)]
enum Node {
    Internal {
        children: Vec<Node>,
        bbox: BoundingBox,
    },
    Leaf {
        entries: Vec<Entry>,
        bbox: BoundingBox,
    },
}

impl Node {
    fn empty() -> Self {
        Node::Leaf {
            entries: Vec::new(),
            bbox: BoundingBox::empty(),
        }
    }
    
    fn is_full(&self, max_entries: usize) -> bool {
        match self {
            Node::Internal { children, .. } => children.len() >= max_entries,
            Node::Leaf { entries, .. } => entries.len() >= max_entries,
        }
    }
    
    fn bbox(&self) -> &BoundingBox {
        match self {
            Node::Internal { bbox, .. } => bbox,
            Node::Leaf { bbox, .. } => bbox,
        }
    }
    
    fn insert(&mut self, entry: Entry, max_entries: usize, min_entries: usize) {
        match self {
            Node::Internal { children, bbox } => {
                // Choose subtree
                let idx = Self::choose_subtree(children, &entry.bbox);
                children[idx].insert(entry, max_entries, min_entries);
                
                // Update bounding box
                *bbox = BoundingBox::union_all(children.iter().map(|child| child.bbox()));
                
                // Handle overflow
                if children.len() > max_entries {
                    Self::split_node(self, max_entries, min_entries);
                }
            }
            Node::Leaf { entries, bbox } => {
                entries.push(entry);
                *bbox = BoundingBox::union_all(entries.iter().map(|e| &e.bbox));
                
                if entries.len() > max_entries {
                    Self::split_node(self, max_entries, min_entries);
                }
            }
        }
    }
    
    fn remove(&mut self, entry: &Entry) -> bool {
        match self {
            Node::Internal { children, bbox } => {
                let mut removed = false;
                for child in children {
                    if child.bbox().intersects(&entry.bbox) {
                        removed |= child.remove(entry);
                    }
                }
                if removed {
                    *bbox = BoundingBox::union_all(children.iter().map(|child| child.bbox()));
                }
                removed
            }
            Node::Leaf { entries, bbox } => {
                if let Some(pos) = entries.iter().position(|e| e == entry) {
                    entries.swap_remove(pos);
                    *bbox = BoundingBox::union_all(entries.iter().map(|e| &e.bbox));
                    true
                } else {
                    false
                }
            }
        }
    }
    
    fn search(&self, query: &BoundingBox, results: &mut Vec<Entry>) {
        match self {
            Node::Internal { children, bbox } => {
                if bbox.intersects(query) {
                    for child in children {
                        child.search(query, results);
                    }
                }
            }
            Node::Leaf { entries, bbox } => {
                if bbox.intersects(query) {
                    for entry in entries {
                        if entry.bbox.intersects(query) {
                            results.push(entry.clone());
                        }
                    }
                }
            }
        }
    }
    
    fn nearest(&self, point: &Point3, k: usize, heap: &mut BinaryHeap<Entry>) {
        match self {
            Node::Internal { children, .. } => {
                // Sort children by distance to query point
                let mut distances: Vec<_> = children
                    .iter()
                    .map(|child| (child.bbox().distance_to_point(point), child))
                    .collect();
                distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                
                // Visit children in order of distance
                for (_, child) in distances {
                    child.nearest(point, k, heap);
                    if heap.len() >= k {
                        let max_dist = heap.peek().unwrap().distance_to_point(point);
                        if max_dist < child.bbox().distance_to_point(point) {
                            break;
                        }
                    }
                }
            }
            Node::Leaf { entries, .. } => {
                for entry in entries {
                    let dist = entry.distance_to_point(point);
                    if heap.len() < k || dist < heap.peek().unwrap().distance_to_point(point) {
                        heap.push(entry.clone());
                        if heap.len() > k {
                            heap.pop();
                        }
                    }
                }
            }
        }
    }
    
    fn choose_subtree(children: &[Node], bbox: &BoundingBox) -> usize {
        children
            .iter()
            .enumerate()
            .min_by_key(|(_, child)| {
                let enlarged = child.bbox().union(bbox);
                (enlarged.volume() - child.bbox().volume()) as i64
            })
            .map(|(i, _)| i)
            .unwrap()
    }
    
    fn split_node(node: &mut Node, max_entries: usize, min_entries: usize) {
        match node {
            Node::Internal { children, .. } => {
                let (left, right) = Self::split_entries(children, max_entries, min_entries);
                *node = Node::Internal {
                    children: left,
                    bbox: BoundingBox::empty(),
                };
            }
            Node::Leaf { entries, .. } => {
                let (left, right) = Self::split_entries(entries, max_entries, min_entries);
                *node = Node::Leaf {
                    entries: left,
                    bbox: BoundingBox::empty(),
                };
            }
        }
    }
    
    fn split_entries<T>(entries: &mut Vec<T>, max_entries: usize, min_entries: usize) -> (Vec<T>, Vec<T>)
    where
        T: Clone + HasBBox,
    {
        // R*-tree split algorithm
        let axis = Self::choose_split_axis(entries, max_entries, min_entries);
        let index = Self::choose_split_index(entries, axis, max_entries, min_entries);
        
        // Sort entries along chosen axis
        entries.sort_by(|a, b| {
            let a_center = a.bbox().center()[axis];
            let b_center = b.bbox().center()[axis];
            a_center.partial_cmp(&b_center).unwrap()
        });
        
        // Split at chosen index
        let right = entries.split_off(index);
        (entries.clone(), right)
    }
    
    fn choose_split_axis<T>(entries: &[T], max_entries: usize, min_entries: usize) -> usize
    where
        T: HasBBox,
    {
        (0..3)
            .min_by_key(|&axis| {
                let mut sorted = entries.to_vec();
                sorted.sort_by(|a, b| {
                    let a_center = a.bbox().center()[axis];
                    let b_center = b.bbox().center()[axis];
                    a_center.partial_cmp(&b_center).unwrap()
                });
                
                let distributions = min_entries..=max_entries - min_entries;
                distributions
                    .map(|k| {
                        let (left, right) = sorted.split_at(k);
                        let left_bbox = BoundingBox::union_all(left.iter().map(|e| e.bbox()));
                        let right_bbox = BoundingBox::union_all(right.iter().map(|e| e.bbox()));
                        left_bbox.perimeter() + right_bbox.perimeter()
                    })
                    .sum::<f64>() as i64
            })
            .unwrap()
    }
    
    fn choose_split_index<T>(entries: &[T], axis: usize, max_entries: usize, min_entries: usize) -> usize
    where
        T: HasBBox,
    {
        let mut min_overlap = f64::INFINITY;
        let mut best_index = min_entries;
        
        for k in min_entries..=max_entries - min_entries {
            let (left, right) = entries.split_at(k);
            let left_bbox = BoundingBox::union_all(left.iter().map(|e| e.bbox()));
            let right_bbox = BoundingBox::union_all(right.iter().map(|e| e.bbox()));
            
            let overlap = left_bbox.intersection(&right_bbox).volume();
            if overlap < min_overlap {
                min_overlap = overlap;
                best_index = k;
            }
        }
        
        best_index
    }
}

/// Trait for types with bounding boxes
pub trait HasBBox {
    fn bbox(&self) -> &BoundingBox;
}

/// Bounding box
#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub min: Point3,
    pub max: Point3,
}

impl BoundingBox {
    pub fn empty() -> Self {
        Self {
            min: Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY),
            max: Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
    
    pub fn new(min: Point3, max: Point3) -> Self {
        Self { min, max }
    }
    
    pub fn union(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: Point3::new(
                self.min.x.min(other.min.x),
                self.min.y.min(other.min.y),
                self.min.z.min(other.min.z),
            ),
            max: Point3::new(
                self.max.x.max(other.max.x),
                self.max.y.max(other.max.y),
                self.max.z.max(other.max.z),
            ),
        }
    }
    
    pub fn union_all<'a, I>(boxes: I) -> BoundingBox
    where
        I: Iterator<Item = &'a BoundingBox>,
    {
        boxes.fold(BoundingBox::empty(), |acc, bbox| acc.union(bbox))
    }
    
    pub fn intersection(&self, other: &BoundingBox) -> BoundingBox {
        BoundingBox {
            min: Point3::new(
                self.min.x.max(other.min.x),
                self.min.y.max(other.min.y),
                self.min.z.max(other.min.z),
            ),
            max: Point3::new(
                self.max.x.min(other.max.x),
                self.max.y.min(other.max.y),
                self.max.z.min(other.max.z),
            ),
        }
    }
    
    pub fn volume(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            (self.max.x - self.min.x) * 
            (self.max.y - self.min.y) * 
            (self.max.z - self.min.z)
        }
    }
    
    pub fn perimeter(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            2.0 * ((self.max.x - self.min.x) + 
                   (self.max.y - self.min.y) + 
                   (self.max.z - self.min.z))
        }
    }
    
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || 
        self.min.y > self.max.y || 
        self.min.z > self.max.z
    }
    
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        !(self.min.x > other.max.x || 
          self.max.x < other.min.x || 
          self.min.y > other.max.y || 
          self.max.y < other.min.y || 
          self.min.z > other.max.z || 
          self.max.z < other.min.z)
    }
    
    pub fn contains_point(&self, point: &Point3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
    
    pub fn distance_to_point(&self, point: &Point3) -> f64 {
        let dx = if point.x < self.min.x {
            self.min.x - point.x
        } else if point.x > self.max.x {
            point.x - self.max.x
        } else {
            0.0
        };
        
        let dy = if point.y < self.min.y {
            self.min.y - point.y
        } else if point.y > self.max.y {
            point.y - self.max.y
        } else {
            0.0
        };
        
        let dz = if point.z < self.min.z {
            self.min.z - point.z
        } else if point.z > self.max.z {
            point.z - self.max.z
        } else {
            0.0
        };
        
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
    
    pub fn center(&self) -> Point3 {
        Point3::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
}

/// Spatial entry
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: AgentId,
    pub bbox: BoundingBox,
    pub data: Arc<RwLock<AgentData>>,
}

impl Entry {
    pub fn new(id: AgentId, bbox: BoundingBox, data: AgentData) -> Self {
        Self {
            id,
            bbox,
            data: Arc::new(RwLock::new(data)),
        }
    }
    
    pub fn distance_to_point(&self, point: &Point3) -> f64 {
        self.bbox.distance_to_point(point)
    }
}

impl HasBBox for Entry {
    fn bbox(&self) -> &BoundingBox {
        &self.bbox
    }
}

impl HasBBox for Node {
    fn bbox(&self) -> &BoundingBox {
        match self {
            Node::Internal { bbox, .. } => bbox,
            Node::Leaf { bbox, .. } => bbox,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_spatial_hash_grid() {
        let bounds = BoundingBox::new(
            Point3::new(-100.0, -100.0, -100.0),
            Point3::new(100.0, 100.0, 100.0),
        );
        let grid = SpatialHashGrid::new(&bounds, 10.0);
        
        // Insert agents
        let mut rng = rand::thread_rng();
        let mut positions = Vec::new();
        
        for i in 0..1000 {
            let position = Point3::new(
                rng.gen_range(-100.0..100.0),
                rng.gen_range(-100.0..100.0),
                rng.gen_range(-100.0..100.0),
            );
            grid.insert(AgentId(i), position);
            positions.push(position);
        }
        
        // Test neighbor queries
        for position in positions {
            let neighbors = grid.get_neighbors(&position, 20.0);
            assert!(!neighbors.is_empty());
            
            // Verify distances
            for &neighbor in &neighbors {
                let neighbor_pos = {
                    let positions = grid.positions.read();
                    positions[neighbor.0]
                };
                assert!((neighbor_pos - position).magnitude() <= 20.0);
            }
        }
    }

    #[test]
    fn test_r_star_tree() {
        let tree = RStarTree::new(16);
        let mut rng = rand::thread_rng();
        
        // Insert entries
        let mut entries = Vec::new();
        for i in 0..1000 {
            let min = Point3::new(
                rng.gen_range(-100.0..90.0),
                rng.gen_range(-100.0..90.0),
                rng.gen_range(-100.0..90.0),
            );
            let max = Point3::new(
                min.x + rng.gen_range(0.0..10.0),
                min.y + rng.gen_range(0.0..10.0),
                min.z + rng.gen_range(0.0..10.0),
            );
            let entry = Entry::new(
                AgentId(i),
                BoundingBox::new(min, max),
                AgentData::default(),
            );
            tree.insert(entry.clone());
            entries.push(entry);
        }
        
        // Test range queries
        let query_bbox = BoundingBox::new(
            Point3::new(-50.0, -50.0, -50.0),
            Point3::new(50.0, 50.0, 50.0),
        );
        let results = tree.search(&query_bbox);
        
        for entry in &results {
            assert!(entry.bbox.intersects(&query_bbox));
        }
        
        // Test nearest neighbor queries
        let query_point = Point3::new(0.0, 0.0, 0.0);
        let k = 10;
        let nearest = tree.nearest(&query_point, k);
        
        assert_eq!(nearest.len(), k);
        for i in 1..k {
            assert!(nearest[i-1].distance_to_point(&query_point) <= 
                   nearest[i].distance_to_point(&query_point));
        }
    }
} 