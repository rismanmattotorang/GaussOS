//! High-Performance Agent Processing System
//! 
//! Provides optimized agent processing with SIMD acceleration and lock-free updates.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use rayon::prelude::*;
use crossbeam::epoch;
use serde::{Deserialize, Serialize};
use nalgebra as na;

use crate::spatial::{Point3, Vector3, Transform3, BoundingBox};

/// Agent identifier
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub usize);

/// Agent state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentState {
    /// Agent position
    pub position: Point3,
    /// Agent velocity
    pub velocity: Vector3,
    /// Agent orientation
    pub orientation: Transform3,
    /// Agent scale
    pub scale: Vector3,
    /// Agent type
    pub agent_type: AgentType,
    /// Agent properties
    pub properties: AgentProperties,
    /// Agent behavior
    pub behavior: AgentBehavior,
}

/// Agent type
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AgentType {
    Physical,
    Virtual,
    Hybrid,
    Custom(u32),
}

/// Agent properties
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentProperties {
    /// Mass in kg
    pub mass: f64,
    /// Maximum speed in m/s
    pub max_speed: f64,
    /// Maximum force in N
    pub max_force: f64,
    /// Maximum torque in N⋅m
    pub max_torque: f64,
    /// Radius in meters
    pub radius: f64,
    /// Custom properties
    pub custom: serde_json::Value,
}

/// Agent behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AgentBehavior {
    /// Seek target
    Seek {
        target: Point3,
        arrival_distance: f64,
    },
    /// Flee from target
    Flee {
        target: Point3,
        panic_distance: f64,
    },
    /// Follow path
    Path {
        waypoints: Vec<Point3>,
        loop_path: bool,
    },
    /// Flock with neighbors
    Flock {
        separation_weight: f64,
        alignment_weight: f64,
        cohesion_weight: f64,
    },
    /// Custom behavior
    Custom(serde_json::Value),
}

/// Agent data
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentData {
    /// Agent state
    pub state: AgentState,
    /// Agent statistics
    pub stats: AgentStats,
    /// Agent memory
    pub memory: AgentMemory,
}

/// Agent statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentStats {
    /// Time alive in seconds
    pub time_alive: f64,
    /// Distance traveled in meters
    pub distance_traveled: f64,
    /// Average speed in m/s
    pub average_speed: f64,
    /// Number of interactions
    pub interaction_count: u64,
    /// Custom statistics
    pub custom_stats: serde_json::Value,
}

/// Agent memory
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentMemory {
    /// Known agents
    pub known_agents: Vec<AgentId>,
    /// Visited locations
    pub visited_locations: Vec<Point3>,
    /// Event history
    pub events: Vec<AgentEvent>,
    /// Custom memory
    pub custom_memory: serde_json::Value,
}

/// Agent event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Event timestamp
    pub timestamp: f64,
    /// Event type
    pub event_type: String,
    /// Event data
    pub data: serde_json::Value,
}

/// High-performance agent manager
pub struct AgentManager {
    /// Spatial index
    spatial_index: Arc<crate::spatial::SpatialHashGrid>,
    /// Agent data
    agents: Arc<RwLock<Vec<Option<Arc<RwLock<AgentData>>>>>>,
    /// Free agent IDs
    free_ids: Arc<crossbeam::queue::SegQueue<AgentId>>,
    /// Next agent ID
    next_id: AtomicU64,
    /// Agent count
    agent_count: AtomicU64,
}

impl AgentManager {
    pub fn new(bounds: &BoundingBox, cell_size: f64) -> Self {
        Self {
            spatial_index: Arc::new(crate::spatial::SpatialHashGrid::new(bounds, cell_size)),
            agents: Arc::new(RwLock::new(Vec::new())),
            free_ids: Arc::new(crossbeam::queue::SegQueue::new()),
            next_id: AtomicU64::new(0),
            agent_count: AtomicU64::new(0),
        }
    }
    
    /// Create new agent
    pub fn create_agent(&self, state: AgentState) -> AgentId {
        // Get agent ID
        let id = if let Some(id) = self.free_ids.pop() {
            id
        } else {
            AgentId(self.next_id.fetch_add(1, Ordering::SeqCst) as usize)
        };
        
        // Create agent data
        let data = AgentData {
            state: state.clone(),
            stats: AgentStats::default(),
            memory: AgentMemory::default(),
        };
        
        // Insert into spatial index
        self.spatial_index.insert(id, state.position);
        
        // Store agent data
        let mut agents = self.agents.write();
        if id.0 >= agents.len() {
            agents.resize_with(id.0 + 1, || None);
        }
        agents[id.0] = Some(Arc::new(RwLock::new(data)));
        
        self.agent_count.fetch_add(1, Ordering::SeqCst);
        id
    }
    
    /// Remove agent
    pub fn remove_agent(&self, id: AgentId) {
        // Remove from spatial index
        self.spatial_index.remove(id);
        
        // Remove agent data
        let mut agents = self.agents.write();
        if id.0 < agents.len() {
            agents[id.0] = None;
        }
        
        // Add ID to free list
        self.free_ids.push(id);
        
        self.agent_count.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// Update agent state
    pub fn update_agent(&self, id: AgentId, state: AgentState) {
        if let Some(Some(agent)) = self.agents.read().get(id.0) {
            // Update spatial index
            self.spatial_index.update(id, state.position);
            
            // Update agent data
            let mut agent = agent.write();
            agent.state = state;
        }
    }
    
    /// Get agent data
    pub fn get_agent(&self, id: AgentId) -> Option<Arc<RwLock<AgentData>>> {
        self.agents.read().get(id.0)?.clone()
    }
    
    /// Get nearby agents
    pub fn get_nearby_agents(&self, position: &Point3, radius: f64) -> Vec<AgentId> {
        self.spatial_index.get_neighbors(position, radius)
    }
    
    /// Update all agents
    pub fn update_all(&self, dt: f64) {
        let agents = self.agents.read();
        
        // Process agents in parallel
        agents.par_iter().enumerate().for_each(|(i, agent)| {
            if let Some(agent) = agent {
                let mut agent = agent.write();
                self.update_agent_state(&mut agent, dt);
            }
        });
    }
    
    /// Update agent state with SIMD acceleration
    fn update_agent_state(&self, agent: &mut AgentData, dt: f64) {
        use std::arch::x86_64::*;
        
        unsafe {
            // Load agent state
            let pos = _mm256_loadu_pd(&agent.state.position[0]);
            let vel = _mm256_loadu_pd(&agent.state.velocity[0]);
            
            // Update position
            let dt_vec = _mm256_set1_pd(dt);
            let new_pos = _mm256_add_pd(pos, _mm256_mul_pd(vel, dt_vec));
            
            // Apply behavior
            match &agent.state.behavior {
                AgentBehavior::Seek { target, arrival_distance } => {
                    let target_pos = _mm256_loadu_pd(&target[0]);
                    let to_target = _mm256_sub_pd(target_pos, new_pos);
                    let dist = _mm256_dp_pd(to_target, to_target, 0x71);
                    
                    if _mm256_cvtsd_f64(dist) > arrival_distance * arrival_distance {
                        let speed = _mm256_set1_pd(agent.state.properties.max_speed);
                        let new_vel = _mm256_mul_pd(
                            _mm256_div_pd(to_target, _mm256_sqrt_pd(dist)),
                            speed
                        );
                        _mm256_storeu_pd(&mut agent.state.velocity[0], new_vel);
                    }
                }
                AgentBehavior::Flee { target, panic_distance } => {
                    let target_pos = _mm256_loadu_pd(&target[0]);
                    let from_target = _mm256_sub_pd(new_pos, target_pos);
                    let dist = _mm256_dp_pd(from_target, from_target, 0x71);
                    
                    if _mm256_cvtsd_f64(dist) < panic_distance * panic_distance {
                        let speed = _mm256_set1_pd(agent.state.properties.max_speed);
                        let new_vel = _mm256_mul_pd(
                            _mm256_div_pd(from_target, _mm256_sqrt_pd(dist)),
                            speed
                        );
                        _mm256_storeu_pd(&mut agent.state.velocity[0], new_vel);
                    }
                }
                AgentBehavior::Flock { separation_weight, alignment_weight, cohesion_weight } => {
                    // Get nearby agents
                    let neighbors = self.get_nearby_agents(&Point3::from(new_pos), 10.0);
                    
                    if !neighbors.is_empty() {
                        let mut separation = Vector3::zeros();
                        let mut alignment = Vector3::zeros();
                        let mut cohesion = Point3::origin();
                        let mut count = 0;
                        
                        for &neighbor_id in &neighbors {
                            if let Some(Some(neighbor)) = self.agents.read().get(neighbor_id.0) {
                                let neighbor = neighbor.read();
                                let to_neighbor = neighbor.state.position - Point3::from(new_pos);
                                let dist = to_neighbor.magnitude();
                                
                                if dist > 0.0 {
                                    // Separation
                                    separation -= to_neighbor.normalize() / dist;
                                    
                                    // Alignment
                                    alignment += neighbor.state.velocity;
                                    
                                    // Cohesion
                                    cohesion += neighbor.state.position.coords;
                                    
                                    count += 1;
                                }
                            }
                        }
                        
                        if count > 0 {
                            let count_f = count as f64;
                            
                            // Average and weight forces
                            separation *= *separation_weight / count_f;
                            alignment = (alignment / count_f - agent.state.velocity) * *alignment_weight;
                            cohesion = ((cohesion / count_f) - agent.state.position.coords) * *cohesion_weight;
                            
                            // Combine forces
                            let force = separation + alignment + cohesion;
                            
                            // Apply force
                            let acceleration = force / agent.state.properties.mass;
                            let new_vel = agent.state.velocity + acceleration * dt;
                            
                            // Limit speed
                            let speed = new_vel.magnitude();
                            if speed > agent.state.properties.max_speed {
                                agent.state.velocity = new_vel * (agent.state.properties.max_speed / speed);
                            } else {
                                agent.state.velocity = new_vel;
                            }
                        }
                    }
                }
                _ => {}
            }
            
            // Store updated position
            _mm256_storeu_pd(&mut agent.state.position[0], new_pos);
            
            // Update stats
            agent.stats.time_alive += dt;
            agent.stats.distance_traveled += agent.state.velocity.magnitude() * dt;
            agent.stats.average_speed = agent.stats.distance_traveled / agent.stats.time_alive;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_agent_creation_and_removal() {
        let bounds = BoundingBox::new(
            Point3::new(-100.0, -100.0, -100.0),
            Point3::new(100.0, 100.0, 100.0),
        );
        let manager = AgentManager::new(&bounds, 10.0);
        
        // Create agents
        let mut rng = rand::thread_rng();
        let mut agent_ids = Vec::new();
        
        for _ in 0..1000 {
            let state = AgentState {
                position: Point3::new(
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                    rng.gen_range(-100.0..100.0),
                ),
                velocity: Vector3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.0..1.0),
                ),
                orientation: Transform3::identity(),
                scale: Vector3::new(1.0, 1.0, 1.0),
                agent_type: AgentType::Physical,
                properties: AgentProperties {
                    mass: 1.0,
                    max_speed: 10.0,
                    max_force: 100.0,
                    max_torque: 10.0,
                    radius: 1.0,
                    custom: serde_json::Value::Null,
                },
                behavior: AgentBehavior::Flock {
                    separation_weight: 1.0,
                    alignment_weight: 1.0,
                    cohesion_weight: 1.0,
                },
            };
            
            let id = manager.create_agent(state);
            agent_ids.push(id);
        }
        
        assert_eq!(manager.agent_count.load(Ordering::SeqCst), 1000);
        
        // Test neighbor queries
        for &id in &agent_ids {
            if let Some(agent) = manager.get_agent(id) {
                let agent = agent.read();
                let neighbors = manager.get_nearby_agents(&agent.state.position, 10.0);
                assert!(!neighbors.is_empty());
            }
        }
        
        // Remove agents
        for id in agent_ids {
            manager.remove_agent(id);
        }
        
        assert_eq!(manager.agent_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_agent_behavior() {
        let bounds = BoundingBox::new(
            Point3::new(-100.0, -100.0, -100.0),
            Point3::new(100.0, 100.0, 100.0),
        );
        let manager = AgentManager::new(&bounds, 10.0);
        
        // Create seeking agent
        let state = AgentState {
            position: Point3::origin(),
            velocity: Vector3::zeros(),
            orientation: Transform3::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            agent_type: AgentType::Physical,
            properties: AgentProperties {
                mass: 1.0,
                max_speed: 10.0,
                max_force: 100.0,
                max_torque: 10.0,
                radius: 1.0,
                custom: serde_json::Value::Null,
            },
            behavior: AgentBehavior::Seek {
                target: Point3::new(10.0, 10.0, 10.0),
                arrival_distance: 1.0,
            },
        };
        
        let id = manager.create_agent(state);
        
        // Update for several steps
        for _ in 0..100 {
            manager.update_all(0.016);
        }
        
        // Check if agent reached target
        if let Some(agent) = manager.get_agent(id) {
            let agent = agent.read();
            let dist = (agent.state.position - Point3::new(10.0, 10.0, 10.0)).magnitude();
            assert!(dist < 1.0);
        }
    }
} 