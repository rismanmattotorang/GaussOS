use crate::{
    AgentId, Space, SpaceResult,
    common::{DistanceMetric, MemoryPool, Point},
    error::helpers::*,
};
use nalgebra::Point3;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Grid cell type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Empty,
    Occupied(AgentId),
    Wall,
    Custom(u8),
}

/// Grid space configuration
#[derive(Debug, Clone)]
pub struct GridSpaceConfig {
    /// Grid dimensions
    pub dimensions: (usize, usize, usize),
    /// Whether grid wraps around at boundaries
    pub periodic: bool,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Whether to allow multiple agents per cell
    pub multi_agent: bool,
    /// Whether to use property layers
    pub use_property_layers: bool,
}

impl Default for GridSpaceConfig {
    fn default() -> Self {
        Self {
            dimensions: (100, 100, 100),
            periodic: false,
            metric: DistanceMetric::Manhattan,
            multi_agent: false,
            use_property_layers: false,
        }
    }
}

/// High-performance grid space implementation
pub struct GridSpace {
    config: GridSpaceConfig,
    cells: RwLock<Vec<CellType>>,
    multi_cells: RwLock<Vec<HashSet<AgentId>>>,
    property_layers: RwLock<HashMap<String, Box<dyn PropertyLayer>>>,
    path_cache: RwLock<lru::LruCache<PathKey, Vec<GridPosition>>>,
    memory_pool: Arc<MemoryPool<Vec<AgentId>>>,
}

/// Grid position type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

impl GridPosition {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        Self { x, y, z }
    }
    
    pub fn to_index(&self, dimensions: (usize, usize, usize)) -> usize {
        self.x + self.y * dimensions.0 + self.z * dimensions.0 * dimensions.1
    }
}

/// Property layer trait for grid space
pub trait PropertyLayer: Send + Sync {
    /// Get property value at a position
    fn get_value(&self, position: GridPosition) -> f64;
    
    /// Set property value at a position
    fn set_value(&self, position: GridPosition, value: f64);
    
    /// Apply function to all values in parallel
    fn par_apply<F>(&self, f: F)
    where
        F: Fn(f64) -> f64 + Send + Sync;
}

#[derive(Hash, Eq, PartialEq)]
struct PathKey {
    start: GridPosition,
    end: GridPosition,
}

impl GridSpace {
    /// Create a new grid space
    pub fn new(config: GridSpaceConfig) -> Self {
        let cell_count = config.dimensions.0 * config.dimensions.1 * config.dimensions.2;
        let cells = vec![CellType::Empty; cell_count];
        let multi_cells = vec![HashSet::new(); cell_count];
        
        Self {
            config,
            cells: RwLock::new(cells),
            multi_cells: RwLock::new(multi_cells),
            property_layers: RwLock::new(HashMap::new()),
            path_cache: RwLock::new(lru::LruCache::new(1000)),
            memory_pool: Arc::new(MemoryPool::new(1000)),
        }
    }
    
    /// Add a property layer
    pub fn add_property_layer(&self, name: &str, layer: Box<dyn PropertyLayer>) {
        self.property_layers.write().insert(name.to_string(), layer);
    }
    
    /// Get property value from a layer
    pub fn get_property(&self, layer_name: &str, position: GridPosition) -> Option<f64> {
        if !self.config.use_property_layers {
            return None;
        }
        
        self.property_layers.read()
            .get(layer_name)
            .map(|layer| layer.get_value(position))
    }
    
    /// Set property value in a layer
    pub fn set_property(&self, layer_name: &str, position: GridPosition, value: f64) -> SpaceResult<()> {
        if !self.config.use_property_layers {
            return Err(invalid_operation("Property layers are disabled"));
        }
        
        self.property_layers.read()
            .get(layer_name)
            .ok_or_else(|| invalid_operation("Layer not found"))
            .map(|layer| layer.set_value(position, value))
    }
    
    /// Find path between two positions using A*
    pub fn find_path(&self, start: GridPosition, end: GridPosition) -> Option<Vec<GridPosition>> {
        let key = PathKey { start, end };
        
        // Check cache first
        if let Some(path) = self.path_cache.write().get(&key) {
            return Some(path.clone());
        }
        
        // A* implementation
        use std::collections::BinaryHeap;
        use std::cmp::Ordering;
        
        #[derive(Copy, Clone, Eq, PartialEq)]
        struct Node {
            position: GridPosition,
            f_score: i32,
        }
        
        impl Ord for Node {
            fn cmp(&self, other: &Self) -> Ordering {
                other.f_score.cmp(&self.f_score)
            }
        }
        
        impl PartialOrd for Node {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        
        let mut open_set = BinaryHeap::new();
        let mut came_from = HashMap::new();
        let mut g_score = HashMap::new();
        
        open_set.push(Node {
            position: start,
            f_score: 0,
        });
        g_score.insert(start, 0);
        
        while let Some(current) = open_set.pop() {
            if current.position == end {
                // Reconstruct path
                let mut path = vec![current.position];
                let mut pos = current.position;
                while let Some(&prev) = came_from.get(&pos) {
                    path.push(prev);
                    pos = prev;
                }
                path.reverse();
                
                // Cache the result
                self.path_cache.write().put(key, path.clone());
                
                return Some(path);
            }
            
            // Get neighbors
            let neighbors = self.get_neighbors(current.position);
            
            for neighbor in neighbors {
                let tentative_g_score = g_score[&current.position] + 1;
                
                if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current.position);
                    g_score.insert(neighbor, tentative_g_score);
                    
                    let h_score = manhattan_distance(neighbor, end) as i32;
                    let f_score = tentative_g_score + h_score;
                    
                    open_set.push(Node {
                        position: neighbor,
                        f_score,
                    });
                }
            }
        }
        
        None
    }
    
    /// Get valid neighbors of a position
    fn get_neighbors(&self, pos: GridPosition) -> Vec<GridPosition> {
        let mut neighbors = Vec::with_capacity(6);
        let cells = self.cells.read();
        
        let offsets = [
            (-1, 0, 0), (1, 0, 0),
            (0, -1, 0), (0, 1, 0),
            (0, 0, -1), (0, 0, 1),
        ];
        
        for (dx, dy, dz) in offsets.iter() {
            let new_x = pos.x as i32 + dx;
            let new_y = pos.y as i32 + dy;
            let new_z = pos.z as i32 + dz;
            
            if self.is_valid_position(new_x, new_y, new_z) {
                let new_pos = GridPosition::new(new_x as usize, new_y as usize, new_z as usize);
                let idx = new_pos.to_index(self.config.dimensions);
                
                if cells[idx] != CellType::Wall {
                    neighbors.push(new_pos);
                }
            }
        }
        
        neighbors
    }
    
    /// Check if position is valid
    fn is_valid_position(&self, x: i32, y: i32, z: i32) -> bool {
        if self.config.periodic {
            true
        } else {
            x >= 0 && x < self.config.dimensions.0 as i32 &&
            y >= 0 && y < self.config.dimensions.1 as i32 &&
            z >= 0 && z < self.config.dimensions.2 as i32
        }
    }
    
    /// Adjust position for periodic boundaries
    fn adjust_periodic(&self, mut pos: GridPosition) -> GridPosition {
        if !self.config.periodic {
            return pos;
        }
        
        pos.x = pos.x % self.config.dimensions.0;
        pos.y = pos.y % self.config.dimensions.1;
        pos.z = pos.z % self.config.dimensions.2;
        
        pos
    }
}

fn manhattan_distance(a: GridPosition, b: GridPosition) -> usize {
    ((a.x as i32 - b.x as i32).abs() +
     (a.y as i32 - b.y as i32).abs() +
     (a.z as i32 - b.z as i32).abs()) as usize
}

impl Space for GridSpace {
    type Position = GridPosition;
    
    fn add_agent(&self, id: AgentId, position: Self::Position) {
        let position = self.adjust_periodic(position);
        if !self.is_valid_position(position.x as i32, position.y as i32, position.z as i32) {
            return;
        }
        
        let idx = position.to_index(self.config.dimensions);
        
        if self.config.multi_agent {
            self.multi_cells.write()[idx].insert(id);
        } else {
            let mut cells = self.cells.write();
            if cells[idx] == CellType::Empty {
                cells[idx] = CellType::Occupied(id);
            }
        }
    }
    
    fn remove_agent(&self, id: AgentId) {
        if self.config.multi_agent {
            let mut multi_cells = self.multi_cells.write();
            for cell in multi_cells.iter_mut() {
                cell.remove(&id);
            }
        } else {
            let mut cells = self.cells.write();
            for cell in cells.iter_mut() {
                if let CellType::Occupied(agent_id) = cell {
                    if *agent_id == id {
                        *cell = CellType::Empty;
                    }
                }
            }
        }
    }
    
    fn move_agent(&self, id: AgentId, new_position: Self::Position) {
        self.remove_agent(id);
        self.add_agent(id, new_position);
    }
    
    fn get_position(&self, id: AgentId) -> Option<Self::Position> {
        if self.config.multi_agent {
            let multi_cells = self.multi_cells.read();
            for (idx, cell) in multi_cells.iter().enumerate() {
                if cell.contains(&id) {
                    let x = idx % self.config.dimensions.0;
                    let y = (idx / self.config.dimensions.0) % self.config.dimensions.1;
                    let z = idx / (self.config.dimensions.0 * self.config.dimensions.1);
                    return Some(GridPosition::new(x, y, z));
                }
            }
        } else {
            let cells = self.cells.read();
            for (idx, cell) in cells.iter().enumerate() {
                if let CellType::Occupied(agent_id) = cell {
                    if *agent_id == id {
                        let x = idx % self.config.dimensions.0;
                        let y = (idx / self.config.dimensions.0) % self.config.dimensions.1;
                        let z = idx / (self.config.dimensions.0 * self.config.dimensions.1);
                        return Some(GridPosition::new(x, y, z));
                    }
                }
            }
        }
        None
    }
    
    fn query_radius(&self, center: Self::Position, radius: f64) -> Vec<AgentId> {
        let radius = radius as usize;
        let mut result = Vec::new();
        
        for x in center.x.saturating_sub(radius)..=center.x.saturating_add(radius) {
            for y in center.y.saturating_sub(radius)..=center.y.saturating_add(radius) {
                for z in center.z.saturating_sub(radius)..=center.z.saturating_add(radius) {
                    if !self.is_valid_position(x as i32, y as i32, z as i32) {
                        continue;
                    }
                    
                    let pos = GridPosition::new(x, y, z);
                    let dist = manhattan_distance(center, pos);
                    
                    if dist <= radius {
                        let idx = pos.to_index(self.config.dimensions);
                        if self.config.multi_agent {
                            result.extend(self.multi_cells.read()[idx].iter().copied());
                        } else if let CellType::Occupied(id) = self.cells.read()[idx] {
                            result.push(id);
                        }
                    }
                }
            }
        }
        
        result
    }
    
    fn query_k_nearest(&self, center: Self::Position, k: usize) -> Vec<AgentId> {
        let mut agents = Vec::new();
        let mut radius = 1;
        
        while agents.len() < k {
            let nearby = self.query_radius(center, radius as f64);
            agents.extend(nearby);
            radius += 1;
            
            if radius > self.config.dimensions.0.max(self.config.dimensions.1).max(self.config.dimensions.2) {
                break;
            }
        }
        
        agents.truncate(k);
        agents
    }
    
    fn agent_count(&self) -> usize {
        if self.config.multi_agent {
            self.multi_cells.read()
                .iter()
                .map(|cell| cell.len())
                .sum()
        } else {
            self.cells.read()
                .iter()
                .filter(|&&cell| matches!(cell, CellType::Occupied(_)))
                .count()
        }
    }
    
    fn clear(&self) {
        if self.config.multi_agent {
            for cell in self.multi_cells.write().iter_mut() {
                cell.clear();
            }
        } else {
            for cell in self.cells.write().iter_mut() {
                *cell = CellType::Empty;
            }
        }
        self.property_layers.write().clear();
        self.path_cache.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestLayer {
        values: RwLock<HashMap<GridPosition, f64>>,
    }
    
    impl TestLayer {
        fn new() -> Self {
            Self {
                values: RwLock::new(HashMap::new()),
            }
        }
    }
    
    impl PropertyLayer for TestLayer {
        fn get_value(&self, position: GridPosition) -> f64 {
            self.values.read().get(&position).copied().unwrap_or(0.0)
        }
        
        fn set_value(&self, position: GridPosition, value: f64) {
            self.values.write().insert(position, value);
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
    fn test_grid_space() {
        let mut config = GridSpaceConfig::default();
        config.dimensions = (10, 10, 10);
        config.use_property_layers = true;
        
        let space = GridSpace::new(config);
        
        // Test basic operations
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        let pos1 = GridPosition::new(0, 0, 0);
        let pos2 = GridPosition::new(1, 1, 1);
        
        space.add_agent(id1, pos1);
        space.add_agent(id2, pos2);
        assert_eq!(space.agent_count(), 2);
        
        let nearby = space.query_radius(pos1, 1.0);
        assert_eq!(nearby.len(), 1);
        assert!(nearby.contains(&id1));
        
        // Test property layers
        let layer = Box::new(TestLayer::new());
        space.add_property_layer("test", layer);
        
        space.set_property("test", pos1, 1.0).unwrap();
        assert_eq!(space.get_property("test", pos1).unwrap(), 1.0);
        
        // Test pathfinding
        let path = space.find_path(pos1, pos2).unwrap();
        assert_eq!(path.len(), 4); // Should be [0,0,0] -> [1,0,0] -> [1,1,0] -> [1,1,1]
    }
} 