use pyo3::prelude::*;
use pyo3_asyncio::tokio::future_into_py;
use numpy::{IntoPyArray, PyArray2, PyArray1};
use gausstwin_core::{
    agent::{Agent, DefaultAgentState, StandardBehavior, BasicAgent, AgentState},
    model::{StandardModel, ModelConfig},
    space::grid::GridSpace,
};
use gausstwin_ai::Metrics;
use gausstwin_ai::ml::models::GNNModel;
use tch::Device;
use std::sync::Arc;

/// GaussTwin Python Module
#[pymodule]
fn gausstwin(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    // Register core types
    m.add_class::<PyAgent>()?;
    m.add_class::<PyModel>()?;
    m.add_class::<PySpace>()?;
    m.add_class::<PyGrid>()?;
    m.add_class::<PyGNNModel>()?;
    m.add_class::<PyMetrics>()?;
    m.add_class::<PyBehavior>()?;
    m.add_class::<PySimulationConfig>()?;
    
    Ok(())
}

#[pyclass]
struct PySimulationConfig {
    time_step: f64,
    max_agents: usize,
    space_bounds: Vec<f64>,
}

#[pymethods]
impl PySimulationConfig {
    #[new]
    fn new(time_step: f64, max_agents: usize, space_bounds: Vec<f64>) -> Self {
        Self {
            time_step,
            max_agents,
            space_bounds,
        }
    }
}

/// Python wrapper for GaussTwin Agent
#[pyclass]
struct PyAgent {
    inner: Arc<BasicAgent<DefaultAgentState>>,
}

#[pymethods]
impl PyAgent {
    #[new]
    fn new(_id: String) -> Self {
        Self {
            inner: Arc::new(BasicAgent::new(DefaultAgentState::default(), StandardBehavior::Stationary))
        }
    }

    /// Get agent ID
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Update agent state asynchronously
    #[pyo3(text_signature = "($self, state)")]
    fn update_state<'p>(&self, py: Python<'p>, _state: PyObject) -> PyResult<&'p PyAny> {
        let _inner = self.inner.clone();
        future_into_py(py, async move {
            // TODO: Implement state update
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    /// Get agent position
    fn get_position<'py>(&self, py: Python<'py>) -> &'py PyArray1<f64> {
        // Convert VecN to Vec<f64> for numpy compatibility
        let pos_vec = match self.inner.state().position() {
            Some(pos) => vec![pos.x, pos.y, pos.z],
            None => vec![0.0, 0.0, 0.0],
        };
        pos_vec.into_pyarray(py)
    }

    /// Set agent behavior
    fn set_behavior(&self, _behavior: &PyBehavior) -> PyResult<()> {
        // TODO: Implement behavior setting
        Ok(())
    }

    /// Get agent neighbors within radius
    fn get_neighbors(&self, _radius: f64) -> Vec<PyAgent> {
        // TODO: Implement neighbor query
        vec![]
    }
}

/// Python wrapper for GaussTwin Model
#[pyclass]
struct PyModel {
    inner: Arc<StandardModel<DefaultAgentState>>,
}

#[pymethods]
impl PyModel {
    #[new]
    fn new() -> Self {
        let config = ModelConfig {
            name: "python_model".to_string(),
            ..Default::default()
        };
        Self {
            inner: Arc::new(StandardModel::new(config).unwrap())
        }
    }

    /// Run model simulation asynchronously
    #[pyo3(text_signature = "($self, steps, config)")]
    fn run<'p>(&self, py: Python<'p>, _steps: u64, _config: Option<&PySimulationConfig>) -> PyResult<&'p PyAny> {
        let _inner = self.inner.clone();
        future_into_py(py, async move {
            // TODO: Implement model simulation
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    /// Add agent to model
    fn add_agent(&self, _agent: &PyAgent) -> PyResult<()> {
        // TODO: Implement agent addition
        Ok(())
    }

    /// Get model metrics
    fn get_metrics(&self) -> PyMetrics {
        PyMetrics {
            inner: Arc::new(Metrics::default())
        }
    }
}

/// Python wrapper for GaussTwin Space
#[pyclass]
struct PySpace {
    inner: Arc<GridSpace>,
}

#[pymethods]
impl PySpace {
    #[new]
    fn new(dimensions: Vec<f64>) -> Self {
        // Use the first dimension as cell size, or default to 1.0
        let cell_size = dimensions.first().copied().unwrap_or(1.0);
        Self {
            inner: Arc::new(GridSpace::new(cell_size))
        }
    }

    /// Get space dimensions as numpy array
    fn dimensions<'py>(&self, py: Python<'py>) -> &'py PyArray2<f64> {
        // TODO: Implement dimensions
        let dims = vec![vec![100.0, 100.0]];
        let array = numpy::ndarray::Array2::from_shape_vec((1, 2), dims.into_iter().flatten().collect()).unwrap();
        array.into_pyarray(py)
    }

    /// Query points in region
    fn query_region(&self, _min_bound: Vec<f64>, _max_bound: Vec<f64>) -> Vec<PyAgent> {
        // TODO: Implement region query
        vec![]
    }

    /// Get nearest neighbors
    fn get_nearest_neighbors(&self, _point: Vec<f64>, _k: usize) -> Vec<PyAgent> {
        // TODO: Implement nearest neighbors
        vec![]
    }
}

/// Python wrapper for GaussTwin Grid
#[pyclass]
struct PyGrid {
    inner: Arc<GridSpace>,
}

#[pymethods]
impl PyGrid {
    #[new]
    fn new(size: (usize, usize)) -> Self {
        // Use the smaller dimension as cell size
        let cell_size = size.0.min(size.1) as f64;
        Self {
            inner: Arc::new(GridSpace::new(cell_size))
        }
    }

    /// Get cell occupancy
    fn get_cell_occupancy(&self, _x: usize, _y: usize) -> usize {
        // TODO: Implement cell occupancy
        0
    }

    /// Get agents in cell
    fn get_agents_in_cell(&self, _x: usize, _y: usize) -> Vec<PyAgent> {
        // TODO: Implement agents in cell
        vec![]
    }
}

/// Python wrapper for GNN Model
#[pyclass]
struct PyGNNModel {
    inner: Arc<GNNModel>,
}

#[pymethods]
impl PyGNNModel {
    #[new]
    fn new() -> Self {
        // Create a simple config without tch dependency
        let config = gausstwin_ai::ml::ModelConfig {
            name: "gnn_model".to_string(),
            architecture: gausstwin_ai::ml::ModelArchitecture::GNN {
                gnn_type: gausstwin_ai::ml::GNNType::GCN,
                aggregation: gausstwin_ai::ml::GraphAggregation::Mean,
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
            device: Device::Cpu,
            custom_params: std::collections::HashMap::new(),
        };
        Self {
            inner: Arc::new(GNNModel::new(config).unwrap())
        }
    }

    /// Train the GNN model asynchronously
    #[pyo3(text_signature = "($self, data)")]
    fn train<'p>(&self, py: Python<'p>, _data: PyObject) -> PyResult<&'p PyAny> {
        let _inner = self.inner.clone();
        future_into_py(py, async move {
            // inner.train(data).await;
            Ok(Python::with_gil(|py| py.None()))
        })
    }

    /// Predict using the trained model
    fn predict<'p>(&self, py: Python<'p>, _input: PyObject) -> PyResult<&'p PyAny> {
        let _inner = self.inner.clone();
        future_into_py(py, async move {
            // let result = inner.predict(input).await;
            let result = "prediction_result".to_string();
            Ok(Python::with_gil(|py| result.into_py(py)))
        })
    }
}

/// Python wrapper for Metrics
#[pyclass]
struct PyMetrics {
    inner: Arc<Metrics>,
}

#[pymethods]
impl PyMetrics {
    /// Get agent count
    fn agent_count(&self) -> usize {
        // self.inner.agent_count()
        0
    }

    /// Get average density
    fn average_density(&self) -> f64 {
        // self.inner.average_density()
        0.0
    }

    /// Get performance statistics
    fn get_performance_stats(&self) -> PyObject {
        Python::with_gil(|py| {
            let stats = py.None();
            stats.into()
        })
    }
}

/// Python wrapper for Behavior
#[pyclass]
struct PyBehavior {
    inner: Arc<StandardBehavior>,
}

#[pymethods]
impl PyBehavior {
    #[new]
    fn new(behavior_type: &str, _params: Option<PyObject>) -> PyResult<Self> {
        let behavior = match behavior_type {
            "stationary" => StandardBehavior::Stationary,
            "random_walk" => StandardBehavior::RandomWalk { speed: 1.0 },
            "follow_target" => StandardBehavior::FollowTarget { 
                target: gausstwin_core::space::VecN::new(0.0, 0.0, 0.0), 
                speed: 1.0 
            },
            _ => StandardBehavior::Stationary,
        };
        Ok(Self {
            inner: Arc::new(behavior)
        })
    }
}
