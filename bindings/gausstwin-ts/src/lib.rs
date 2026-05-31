use wasm_bindgen::prelude::*;
use gausstwin_core::{
    agent::{Agent, DefaultAgentState, BasicAgent, StandardBehavior, AgentState as CoreAgentState},
    model::{StandardModel, ModelConfig as CoreModelConfig},
    space::{Space, Position, VecN, grid::GridSpace}
};
use gausstwin_ai::Metrics;
use gausstwin_ai::ml::{models::GNNModel, ModelConfig as AIModelConfig, ModelArchitecture, GNNType, GraphAggregation};
use js_sys::{Promise, Array, Float64Array};
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use wasm_bindgen_futures::future_to_promise;
use web_sys::console;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
pub fn start() {
    console::log_1(&"GaussTwin TypeScript bindings initialized".into());
}

#[derive(Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub position: Vec<f64>,
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct SimulationConfig {
    pub time_step: f64,
    pub max_agents: usize,
    pub space_bounds: Vec<f64>,
}

#[wasm_bindgen]
pub struct TSAgent {
    inner: Arc<BasicAgent<DefaultAgentState>>,
}

#[wasm_bindgen]
impl TSAgent {
    #[wasm_bindgen(constructor)]
    pub fn new(_id: String) -> Self {
        let state = DefaultAgentState {
            position: Some(VecN::new(0.0, 0.0, 0.0)),
            properties: std::collections::HashMap::new(),
        };
        let behavior = StandardBehavior::Stationary;
        Self {
            inner: Arc::new(BasicAgent::new(state, behavior))
        }
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    pub fn update_state(&self, _state: JsValue) -> Promise {
        let _inner = self.inner.clone();
        future_to_promise(async move {
            // TODO: Implement state update logic
            Ok(JsValue::NULL)
        })
    }

    pub fn get_position(&self) -> Float64Array {
        if let Some(pos) = self.inner.state().position {
            let coords = [pos.x, pos.y, pos.z];
            Float64Array::from(&coords[..])
        } else {
            let coords = [0.0, 0.0, 0.0];
            Float64Array::from(&coords[..])
        }
    }

    pub fn set_behavior(&self, _behavior: &TSBehavior) -> Result<(), JsValue> {
        // TODO: Implement behavior setting
        Ok(())
    }

    pub fn get_neighbors(&self, _radius: f64) -> Array {
        // TODO: Implement neighbor querying
        Array::new()
    }
}

#[wasm_bindgen]
pub struct TSModel {
    inner: Arc<StandardModel<DefaultAgentState>>,
}

#[wasm_bindgen]
impl TSModel {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let config = CoreModelConfig::new("default_model".to_string());
        Self {
            inner: Arc::new(StandardModel::new(config).unwrap())
        }
    }

    pub fn run(&self, _steps: u64, _config: JsValue) -> Promise {
        let _inner = self.inner.clone();
        future_to_promise(async move {
            // TODO: Configure model with config
            // TODO: Implement model running logic
            Ok(JsValue::NULL)
        })
    }

    pub fn add_agent(&self, _agent: &TSAgent) -> Result<(), JsValue> {
        // TODO: Implement agent addition
        Ok(())
    }

    pub fn get_metrics(&self) -> TSMetrics {
        TSMetrics {
            inner: Arc::new(Metrics::default())
        }
    }
}

#[wasm_bindgen]
pub struct TSSpace {
    inner: Arc<GridSpace>,
}

#[wasm_bindgen]
impl TSSpace {
    #[wasm_bindgen(constructor)]
    pub fn new(dimensions: Array) -> Self {
        let _dims: Vec<f64> = dimensions.to_vec().into_iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect();
        Self {
            inner: Arc::new(GridSpace::new(1.0))
        }
    }

    pub fn dimensions(&self) -> Array {
        let bounds = self.inner.bounds();
        let array = Array::new();
        array.push(&JsValue::from_f64(bounds.max.x - bounds.min.x));
        array.push(&JsValue::from_f64(bounds.max.y - bounds.min.y));
        array.push(&JsValue::from_f64(bounds.max.z - bounds.min.z));
        array
    }

    pub fn query_region(&self, min_bound: Array, max_bound: Array) -> Array {
        let _min: Vec<f64> = min_bound.to_vec().into_iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect();
        let _max: Vec<f64> = max_bound.to_vec().into_iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect();
        
        // TODO: Implement region querying
        Array::new()
    }

    pub fn get_nearest_neighbors(&self, point: Array, _k: usize) -> Array {
        let _p: Vec<f64> = point.to_vec().into_iter()
            .map(|v| v.as_f64().unwrap_or(0.0))
            .collect();
        
        // TODO: Implement nearest neighbor querying
        Array::new()
    }
}

#[wasm_bindgen]
pub struct TSGrid {
    inner: Arc<GridSpace>,
}

#[wasm_bindgen]
impl TSGrid {
    #[wasm_bindgen(constructor)]
    pub fn new(_width: usize, _height: usize) -> Self {
        Self {
            inner: Arc::new(GridSpace::new(1.0))
        }
    }

    pub fn get_cell_occupancy(&self, x: usize, y: usize) -> usize {
        let pos = Position::Grid(VecN::new(x as f64, y as f64, 0.0));
        self.inner.get_agents_at(&pos).len()
    }

    pub fn get_agents_in_cell(&self, x: usize, y: usize) -> Array {
        let pos = Position::Grid(VecN::new(x as f64, y as f64, 0.0));
        let agent_ids = self.inner.get_agents_at(&pos);
        
        // Convert agent IDs to TSAgent instances
        // Note: This is a simplified implementation
        agent_ids.into_iter()
            .map(|_id| {
                let state = DefaultAgentState {
                    position: Some(VecN::new(x as f64, y as f64, 0.0)),
                    properties: std::collections::HashMap::new(),
                };
                let behavior = StandardBehavior::Stationary;
                TSAgent { inner: Arc::new(BasicAgent::new(state, behavior)) }
            })
            .map(JsValue::from)
            .collect()
    }
}

#[wasm_bindgen]
pub struct TSGNNModel {
    inner: Arc<GNNModel>,
}

#[wasm_bindgen]
impl TSGNNModel {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Create a proper GNNModel instance with default config
        let config = AIModelConfig {
            architecture: ModelArchitecture::GNN {
                gnn_type: GNNType::GCN,
                aggregation: GraphAggregation::Mean,
                num_layers: 2,
                hidden_dims: vec![64, 32],
            },
            input_dims: vec![10],
            output_dims: vec![1],
            hidden_layers: vec![],
            activations: vec![],
            dropout_rates: vec![],
            normalizations: vec![],
            learning_rate: 0.001,
            batch_size: 32,
            device: tch::Device::Cpu,
            custom_params: std::collections::HashMap::new(),
            name: "default_gnn".to_string(),
        };
        Self {
            inner: Arc::new(GNNModel::new(config).unwrap())
        }
    }

    pub fn train(&self, _data: JsValue) -> Promise {
        let _inner = self.inner.clone();
        future_to_promise(async move {
            // TODO: Implement training logic
            Ok(JsValue::NULL)
        })
    }

    pub fn predict(&self, _input: JsValue) -> Promise {
        let _inner = self.inner.clone();
        future_to_promise(async move {
            let result = "prediction_result".to_string();
            Ok(JsValue::from_serde(&result).unwrap())
        })
    }
}

#[wasm_bindgen]
pub struct TSMetrics {
    inner: Arc<Metrics>,
}

#[wasm_bindgen]
impl TSMetrics {
    pub fn agent_count(&self) -> usize {
        // TODO: Implement actual metrics
        0
    }

    pub fn average_density(&self) -> f64 {
        // TODO: Implement actual metrics
        0.0
    }

    pub fn get_performance_stats(&self) -> Result<JsValue, JsValue> {
        let stats = "performance_stats".to_string();
        Ok(JsValue::from_serde(&stats).unwrap())
    }
}

#[wasm_bindgen]
pub struct TSBehavior {
    inner: Arc<StandardBehavior>,
}

#[wasm_bindgen]
impl TSBehavior {
    #[wasm_bindgen(constructor)]
    pub fn new(behavior_type: &str, params: JsValue) -> Result<TSBehavior, JsValue> {
        let behavior = match behavior_type {
            "random_walk" => {
                let speed = if params.is_null() || params.is_undefined() {
                    1.0
                } else {
                    params.as_f64().unwrap_or(1.0)
                };
                StandardBehavior::RandomWalk { speed }
            }
            "follow_target" => {
                let target = if params.is_null() || params.is_undefined() {
                    VecN::new(0.0, 0.0, 0.0)
                } else {
                    // TODO: Parse target from params
                    VecN::new(0.0, 0.0, 0.0)
                };
                let speed = 1.0;
                StandardBehavior::FollowTarget { target, speed }
            }
            "stationary" => StandardBehavior::Stationary,
            _ => return Err(JsValue::from_str("Unknown behavior type")),
        };
        
        Ok(Self { inner: Arc::new(behavior) })
    }
}

#[cfg(feature = "deno")]
pub extern "C" fn deno_plugin_init() -> *mut std::ffi::c_void {
    Box::into_raw(Box::new(())) as *mut std::ffi::c_void
}
