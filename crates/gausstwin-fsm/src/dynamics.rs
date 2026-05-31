use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use nalgebra::{DMatrix, DVector};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum DynamicsError {
    #[error("Variable {0} not found")]
    VariableNotFound(String),
    #[error("Invalid equation: {0}")]
    InvalidEquation(String),
    #[error("Solver error: {0}")]
    SolverError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub unit: Option<String>,
    pub description: Option<String>,
    pub tags: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum FlowType {
    Linear(f64),
    Exponential(f64),
    Logistic { k: f64, r: f64, capacity: f64 },
    Custom(Arc<dyn Fn(f64) -> f64 + Send + Sync>),
}

#[derive(Debug, Clone)]
pub struct Flow {
    pub from: String,
    pub to: String,
    pub flow_type: FlowType,
    pub constraints: Vec<Box<dyn Fn(f64) -> bool + Send + Sync>>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub time: f64,
    pub variables: HashMap<String, f64>,
    pub flows: Vec<(String, String, f64)>,
}

pub struct AdvancedSystemDynamics {
    variables: DashMap<String, Variable>,
    flows: Vec<Flow>,
    time: f64,
    dt: f64,
    history: Arc<RwLock<Vec<Snapshot>>>,
    metrics: Arc<DashMap<String, f64>>,
    solver_config: SolverConfig,
    observers: tokio::sync::broadcast::Sender<Snapshot>,
}

#[derive(Debug, Clone)]
pub struct SolverConfig {
    pub method: SolverMethod,
    pub tolerance: f64,
    pub max_iterations: usize,
    pub step_size: f64,
}

#[derive(Debug, Clone)]
pub enum SolverMethod {
    Euler,
    RK4,
    AdaptiveRK45,
}

impl AdvancedSystemDynamics {
    pub fn new(dt: f64) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(1024);
        
        Self {
            variables: DashMap::new(),
            flows: Vec::new(),
            time: 0.0,
            dt,
            history: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(DashMap::new()),
            solver_config: SolverConfig {
                method: SolverMethod::RK4,
                tolerance: 1e-6,
                max_iterations: 1000,
                step_size: dt,
            },
            observers: tx,
        }
    }

    pub fn add_variable(&mut self, variable: Variable) -> Result<(), DynamicsError> {
        // Validate variable
        if let Some(min) = variable.min {
            if variable.value < min {
                return Err(DynamicsError::InvalidEquation(
                    format!("Initial value {} below minimum {}", variable.value, min)
                ));
            }
        }
        if let Some(max) = variable.max {
            if variable.value > max {
                return Err(DynamicsError::InvalidEquation(
                    format!("Initial value {} above maximum {}", variable.value, max)
                ));
            }
        }

        self.variables.insert(variable.name.clone(), variable);
        Ok(())
    }

    pub fn add_flow(&mut self, flow: Flow) -> Result<(), DynamicsError> {
        // Validate flow
        if !self.variables.contains_key(&flow.from) {
            return Err(DynamicsError::VariableNotFound(flow.from));
        }
        if !self.variables.contains_key(&flow.to) {
            return Err(DynamicsError::VariableNotFound(flow.to));
        }

        self.flows.push(flow);
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), DynamicsError> {
        match self.solver_config.method {
            SolverMethod::Euler => self.solve_euler(),
            SolverMethod::RK4 => self.solve_rk4(),
            SolverMethod::AdaptiveRK45 => self.solve_adaptive_rk45(),
        }
    }

    fn solve_euler(&mut self) -> Result<(), DynamicsError> {
        let mut changes = HashMap::new();
        
        // Calculate all flows
        for flow in &self.flows {
            let from_value = self.variables.get(&flow.from)
                .ok_or_else(|| DynamicsError::VariableNotFound(flow.from.clone()))?
                .value;

            // Check constraints
            if !flow.constraints.iter().all(|c| c(from_value)) {
                continue;
            }

            let flow_value = match &flow.flow_type {
                FlowType::Linear(rate) => rate * from_value,
                FlowType::Exponential(rate) => from_value * rate.exp(),
                FlowType::Logistic { k, r, capacity } => {
                    k * from_value * (1.0 - from_value / capacity) * r
                },
                FlowType::Custom(f) => (f)(from_value),
            } * self.dt;

            *changes.entry(flow.from.clone()).or_insert(0.0) -= flow_value;
            *changes.entry(flow.to.clone()).or_insert(0.0) += flow_value;
        }

        // Apply changes with bounds checking
        for (var_name, change) in changes {
            if let Some(mut var) = self.variables.get_mut(&var_name) {
                let new_value = var.value + change;
                
                // Apply bounds
                var.value = match (var.min, var.max) {
                    (Some(min), Some(max)) => new_value.clamp(min, max),
                    (Some(min), None) => new_value.max(min),
                    (None, Some(max)) => new_value.min(max),
                    (None, None) => new_value,
                };
            }
        }

        self.time += self.dt;
        self.record_snapshot()?;
        Ok(())
    }

    fn solve_rk4(&mut self) -> Result<(), DynamicsError> {
        let n = self.variables.len();
        let mut state = DVector::zeros(n);
        let mut i = 0;
        
        // Build state vector
        for var in self.variables.iter() {
            state[i] = var.value;
            i += 1;
        }

        // RK4 implementation
        let k1 = self.evaluate_derivatives(&state)?;
        let k2 = self.evaluate_derivatives(&(state + &k1 * self.dt / 2.0))?;
        let k3 = self.evaluate_derivatives(&(state + &k2 * self.dt / 2.0))?;
        let k4 = self.evaluate_derivatives(&(state + &k3 * self.dt))?;

        let new_state = state + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * self.dt / 6.0;

        // Update variables
        i = 0;
        for mut var in self.variables.iter_mut() {
            var.value = new_state[i];
            i += 1;
        }

        self.time += self.dt;
        self.record_snapshot()?;
        Ok(())
    }

    fn solve_adaptive_rk45(&mut self) -> Result<(), DynamicsError> {
        let mut h = self.solver_config.step_size;
        let mut remaining_time = self.dt;

        while remaining_time > 0.0 {
            // Compute both RK4 and RK5 solutions
            let rk4_solution = self.solve_rk4_step(h)?;
            let rk5_solution = self.solve_rk5_step(h)?;

            // Estimate error
            let error = (&rk5_solution - &rk4_solution).abs().max();

            // Adjust step size based on error
            let new_h = if error > self.solver_config.tolerance {
                h * 0.9 * (self.solver_config.tolerance / error).powf(0.2)
            } else {
                h * 1.1
            };

            // Accept or reject step
            if error <= self.solver_config.tolerance {
                // Update state
                let mut i = 0;
                for mut var in self.variables.iter_mut() {
                    var.value = rk5_solution[i];
                    i += 1;
                }
                
                remaining_time -= h;
                self.time += h;
            }

            h = new_h.min(remaining_time);
        }

        self.record_snapshot()?;
        Ok(())
    }

    fn evaluate_derivatives(&self, state: &DVector<f64>) -> Result<DVector<f64>, DynamicsError> {
        let n = state.len();
        let mut derivatives = DVector::zeros(n);
        
        // Calculate derivatives based on flows
        for flow in &self.flows {
            let from_idx = self.get_variable_index(&flow.from)?;
            let to_idx = self.get_variable_index(&flow.to)?;
            
            let from_value = state[from_idx];
            
            let flow_value = match &flow.flow_type {
                FlowType::Linear(rate) => rate * from_value,
                FlowType::Exponential(rate) => from_value * rate.exp(),
                FlowType::Logistic { k, r, capacity } => {
                    k * from_value * (1.0 - from_value / capacity) * r
                },
                FlowType::Custom(f) => (f)(from_value),
            };

            derivatives[from_idx] -= flow_value;
            derivatives[to_idx] += flow_value;
        }

        Ok(derivatives)
    }

    fn get_variable_index(&self, name: &str) -> Result<usize, DynamicsError> {
        self.variables.iter()
            .position(|r| r.key() == name)
            .ok_or_else(|| DynamicsError::VariableNotFound(name.to_string()))
    }

    fn record_snapshot(&mut self) -> Result<(), DynamicsError> {
        let mut variables = HashMap::new();
        for var in self.variables.iter() {
            variables.insert(var.key().clone(), var.value);
        }

        let flows: Vec<(String, String, f64)> = self.flows.iter()
            .map(|f| {
                let from_value = self.variables.get(&f.from).unwrap().value;
                let flow_value = match &f.flow_type {
                    FlowType::Linear(rate) => rate * from_value,
                    FlowType::Exponential(rate) => from_value * rate.exp(),
                    FlowType::Logistic { k, r, capacity } => {
                        k * from_value * (1.0 - from_value / capacity) * r
                    },
                    FlowType::Custom(func) => (func)(from_value),
                };
                (f.from.clone(), f.to.clone(), flow_value)
            })
            .collect();

        let snapshot = Snapshot {
            time: self.time,
            variables,
            flows,
        };

        self.history.write().push(snapshot.clone());
        let _ = self.observers.send(snapshot);

        // Update metrics
        self.metrics.insert("updates_total".into(),
            self.metrics.get("updates_total").map_or(1.0, |v| v + 1.0));

        Ok(())
    }

    pub fn get_history(&self) -> Arc<RwLock<Vec<Snapshot>>> {
        self.history.clone()
    }

    pub fn get_metrics(&self) -> Arc<DashMap<String, f64>> {
        self.metrics.clone()
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Snapshot> {
        self.observers.subscribe()
    }

    pub fn set_solver_config(&mut self, config: SolverConfig) {
        self.solver_config = config;
    }
} 