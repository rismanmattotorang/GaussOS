use crate::{
    AgentId, Space, SpaceResult,
    common::{DistanceMetric, MemoryPool, Point, SpatialCache},
    error::helpers::*,
    spatial_index::{SpatialIndex, KdTreeIndex, GridIndex, RTreeIndex},
};
use nalgebra::{Point3, Vector3};
use parking_lot::RwLock;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

/// Configuration for continuous space
#[derive(Debug, Clone)]
pub struct ContinuousSpaceConfig {
    /// Bounds of the space
    pub bounds: (Point3<f64>, Point3<f64>),
    /// Whether space wraps around at boundaries
    pub periodic: bool,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Cell size for grid-based acceleration
    pub cell_size: Option<f64>,
    /// Whether to use force fields
    pub use_force_fields: bool,
    /// Whether to use property layers
    pub use_property_layers: bool,
    /// Cache TTL for spatial queries
    pub cache_ttl: Duration,
}

impl Default for ContinuousSpaceConfig {
    fn default() -> Self {
        Self {
            bounds: (
                Point3::new(-100.0, -100.0, -100.0),
                Point3::new(100.0, 100.0, 100.0),
            ),
            periodic: false,
            metric: DistanceMetric::Euclidean,
            cell_size: Some(10.0),
            use_force_fields: false,
            use_property_layers: false,
            cache_ttl: Duration::from_millis(100),
        }
    }
}

/// High-performance continuous space implementation
pub struct ContinuousSpace {
    config: ContinuousSpaceConfig,
    spatial_index: Box<dyn SpatialIndex<Position = Point3<f64>>>,
    force_fields: RwLock<Vec<Box<dyn ForceField>>>,
    property_layers: RwLock<HashMap<String, Box<dyn PropertyLayer>>>,
    query_cache: Arc<SpatialCache<QueryKey, Vec<AgentId>>>,
    memory_pool: Arc<MemoryPool<Vec<AgentId>>>,
}

#[derive(Hash, Eq, PartialEq)]
struct QueryKey {
    query_type: QueryType,
    center: (i64, i64, i64), // Discretized position
    param: u64, // radius or k
}

#[derive(Hash, Eq, PartialEq)]
enum QueryType {
    Radius,
    KNearest,
}

/// Force field trait for continuous space
pub trait ForceField: Send + Sync {
    /// Calculate force at a point
    fn calculate_force(&self, position: Point3<f64>) -> Vector3<f64>;
}

/// Property layer trait for continuous space
pub trait PropertyLayer: Send + Sync {
    /// Get property value at a point
    fn get_value(&self, position: Point3<f64>) -> f64;
    
    /// Set property value at a point
    fn set_value(&self, position: Point3<f64>, value: f64);
    
    /// Apply function to all values in parallel
    fn par_apply<F>(&self, f: F)
    where
        F: Fn(f64) -> f64 + Send + Sync;
}

impl ContinuousSpace {
    /// Create a new continuous space
    pub fn new(config: ContinuousSpaceConfig) -> Self {
        let spatial_index: Box<dyn SpatialIndex<Position = Point3<f64>>> = if let Some(cell_size) = config.cell_size {
            Box::new(GridIndex::new(cell_size))
        } else {
            Box::new(KdTreeIndex::new())
        };
        
        Self {
            config,
            spatial_index,
            force_fields: RwLock::new(Vec::new()),
            property_layers: RwLock::new(HashMap::new()),
            query_cache: Arc::new(SpatialCache::new(Duration::from_millis(100))),
            memory_pool: Arc::new(MemoryPool::new(1000)),
        }
    }
    
    /// Add a force field
    pub fn add_force_field(&self, field: Box<dyn ForceField>) {
        self.force_fields.write().push(field);
    }
    
    /// Add a property layer
    pub fn add_property_layer(&self, name: &str, layer: Box<dyn PropertyLayer>) {
        self.property_layers.write().insert(name.to_string(), layer);
    }
    
    /// Calculate total force at a point
    pub fn calculate_total_force(&self, position: Point3<f64>) -> Vector3<f64> {
        if !self.config.use_force_fields {
            return Vector3::zeros();
        }
        
        self.force_fields.read()
            .par_iter()
            .map(|field| field.calculate_force(position))
            .reduce(Vector3::zeros, |a, b| a + b)
    }
    
    /// Get property value from a layer
    pub fn get_property(&self, layer_name: &str, position: Point3<f64>) -> Option<f64> {
        if !self.config.use_property_layers {
            return None;
        }
        
        self.property_layers.read()
            .get(layer_name)
            .map(|layer| layer.get_value(position))
    }
    
    /// Set property value in a layer
    pub fn set_property(&self, layer_name: &str, position: Point3<f64>, value: f64) -> SpaceResult<()> {
        if !self.config.use_property_layers {
            return Err(invalid_operation("Property layers are disabled"));
        }
        
        self.property_layers.read()
            .get(layer_name)
            .ok_or_else(|| invalid_operation("Layer not found"))
            .map(|layer| layer.set_value(position, value))
    }
    
    /// Check if position is within bounds
    fn check_bounds(&self, position: &Point3<f64>) -> bool {
        let (min, max) = &self.config.bounds;
        position.x >= min.x && position.x <= max.x &&
        position.y >= min.y && position.y <= max.y &&
        position.z >= min.z && position.z <= max.z
    }
    
    /// Adjust position for periodic boundaries
    fn adjust_periodic(&self, mut position: Point3<f64>) -> Point3<f64> {
        if !self.config.periodic {
            return position;
        }
        
        let (min, max) = &self.config.bounds;
        let width = max.x - min.x;
        let height = max.y - min.y;
        let depth = max.z - min.z;
        
        position.x = (position.x - min.x).rem_euclid(width) + min.x;
        position.y = (position.y - min.y).rem_euclid(height) + min.y;
        position.z = (position.z - min.z).rem_euclid(depth) + min.z;
        
        position
    }
}

impl Space for ContinuousSpace {
    type Position = Point3<f64>;
    
    fn add_agent(&self, id: AgentId, position: Self::Position) {
        let position = self.adjust_periodic(position);
        if !self.check_bounds(&position) {
            return;
        }
        self.spatial_index.insert(id, position).unwrap_or_default();
    }
    
    fn remove_agent(&self, id: AgentId) {
        self.spatial_index.remove(id).unwrap_or_default();
    }
    
    fn move_agent(&self, id: AgentId, new_position: Self::Position) {
        let new_position = self.adjust_periodic(new_position);
        if !self.check_bounds(&new_position) {
            return;
        }
        self.spatial_index.update(id, new_position).unwrap_or_default();
    }
    
    fn get_position(&self, id: AgentId) -> Option<Self::Position> {
        // This would require additional storage in spatial index implementations
        None
    }
    
    fn query_radius(&self, center: Self::Position, radius: f64) -> Vec<AgentId> {
        let center = self.adjust_periodic(center);
        if !self.check_bounds(&center) {
            return Vec::new();
        }
        
        // Try cache first
        let key = QueryKey {
            query_type: QueryType::Radius,
            center: (
                (center.x * 100.0) as i64,
                (center.y * 100.0) as i64,
                (center.z * 100.0) as i64,
            ),
            param: (radius * 100.0) as u64,
        };
        
        if let Some(cached) = self.query_cache.get(&key) {
            return cached;
        }
        
        // Perform query
        let result = self.spatial_index.query_radius(center, radius);
        
        // Cache result
        self.query_cache.insert(key, result.clone());
        
        result
    }
    
    fn query_k_nearest(&self, center: Self::Position, k: usize) -> Vec<AgentId> {
        let center = self.adjust_periodic(center);
        if !self.check_bounds(&center) {
            return Vec::new();
        }
        
        // Try cache first
        let key = QueryKey {
            query_type: QueryType::KNearest,
            center: (
                (center.x * 100.0) as i64,
                (center.y * 100.0) as i64,
                (center.z * 100.0) as i64,
            ),
            param: k as u64,
        };
        
        if let Some(cached) = self.query_cache.get(&key) {
            return cached;
        }
        
        // Perform query
        let result = self.spatial_index.query_k_nearest(center, k);
        
        // Cache result
        self.query_cache.insert(key, result.clone());
        
        result
    }
    
    fn agent_count(&self) -> usize {
        self.spatial_index.size()
    }
    
    fn clear(&self) {
        self.spatial_index.clear();
        self.force_fields.write().clear();
        self.property_layers.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;
    
    struct RadialForceField {
        center: Point3<f64>,
        strength: f64,
    }
    
    impl ForceField for RadialForceField {
        fn calculate_force(&self, position: Point3<f64>) -> Vector3<f64> {
            let diff = position - self.center;
            let dist = diff.magnitude();
            if dist < 1e-10 {
                Vector3::zeros()
            } else {
                diff.normalize() * self.strength / (dist * dist)
            }
        }
    }
    
    struct ScalarField {
        values: RwLock<HashMap<(i64, i64, i64), f64>>,
        resolution: f64,
    }
    
    impl ScalarField {
        fn new(resolution: f64) -> Self {
            Self {
                values: RwLock::new(HashMap::new()),
                resolution,
            }
        }
        
        fn discretize(&self, position: Point3<f64>) -> (i64, i64, i64) {
            (
                (position.x / self.resolution).floor() as i64,
                (position.y / self.resolution).floor() as i64,
                (position.z / self.resolution).floor() as i64,
            )
        }
    }
    
    impl PropertyLayer for ScalarField {
        fn get_value(&self, position: Point3<f64>) -> f64 {
            let key = self.discretize(position);
            self.values.read().get(&key).copied().unwrap_or(0.0)
        }
        
        fn set_value(&self, position: Point3<f64>, value: f64) {
            let key = self.discretize(position);
            self.values.write().insert(key, value);
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
    fn test_continuous_space() {
        let mut config = ContinuousSpaceConfig::default();
        config.use_force_fields = true;
        config.use_property_layers = true;
        
        let space = ContinuousSpace::new(config);
        
        // Test basic operations
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        let pos1 = Point3::new(0.0, 0.0, 0.0);
        let pos2 = Point3::new(1.0, 1.0, 1.0);
        
        space.add_agent(id1, pos1);
        space.add_agent(id2, pos2);
        assert_eq!(space.agent_count(), 2);
        
        let nearby = space.query_radius(pos1, 0.5);
        assert_eq!(nearby.len(), 1);
        assert!(nearby.contains(&id1));
        
        // Test force fields
        let field = Box::new(RadialForceField {
            center: Point3::origin(),
            strength: 1.0,
        });
        space.add_force_field(field);
        
        let force = space.calculate_total_force(Point3::new(1.0, 0.0, 0.0));
        assert!((force.x + 1.0).abs() < 1e-10); // Should point towards center
        
        // Test property layers
        let layer = Box::new(ScalarField::new(0.1));
        space.add_property_layer("test", layer);
        
        space.set_property("test", pos1, 1.0).unwrap();
        assert_eq!(space.get_property("test", pos1).unwrap(), 1.0);
    }
} 