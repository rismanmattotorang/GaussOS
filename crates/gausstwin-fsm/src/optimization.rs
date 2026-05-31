use std::collections::HashMap;
use std::sync::Arc;

use nalgebra::{DMatrix, DVector};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::{DynamicsError, Flow, Variable};

#[derive(Error, Debug)]
pub enum OptimizationError {
    #[error("Invalid objective function")]
    InvalidObjective,
    #[error("Optimization failed: {0}")]
    Failed(String),
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub method: OptimizationMethod,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub population_size: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
}

#[derive(Debug, Clone)]
pub enum OptimizationMethod {
    GradientDescent {
        learning_rate: f64,
        momentum: f64,
    },
    ParticleSwarm {
        inertia: f64,
        cognitive: f64,
        social: f64,
    },
    GeneticAlgorithm {
        elite_size: usize,
        tournament_size: usize,
    },
    SimulatedAnnealing {
        initial_temp: f64,
        cooling_rate: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub optimal_values: HashMap<String, f64>,
    pub objective_value: f64,
    pub iterations: usize,
    pub convergence_history: Vec<f64>,
    pub constraints_satisfied: bool,
}

pub struct StateFlowOptimizer {
    config: OptimizationConfig,
    rng: StdRng,
    objective_fn: Arc<dyn Fn(&HashMap<String, f64>) -> f64 + Send + Sync>,
    constraints: Vec<Arc<dyn Fn(&HashMap<String, f64>) -> bool + Send + Sync>>,
}

impl StateFlowOptimizer {
    pub fn new(config: OptimizationConfig) -> Self {
        Self {
            config,
            rng: StdRng::from_entropy(),
            objective_fn: Arc::new(|_| 0.0),
            constraints: Vec::new(),
        }
    }

    pub fn set_objective<F>(&mut self, objective: F)
    where
        F: Fn(&HashMap<String, f64>) -> f64 + Send + Sync + 'static,
    {
        self.objective_fn = Arc::new(objective);
    }

    pub fn add_constraint<F>(&mut self, constraint: F)
    where
        F: Fn(&HashMap<String, f64>) -> bool + Send + Sync + 'static,
    {
        self.constraints.push(Arc::new(constraint));
    }

    pub fn optimize(
        &mut self,
        variables: &HashMap<String, Variable>,
        flows: &[Flow],
    ) -> Result<OptimizationResult, OptimizationError> {
        match self.config.method {
            OptimizationMethod::GradientDescent { .. } => self.gradient_descent(variables),
            OptimizationMethod::ParticleSwarm { .. } => self.particle_swarm(variables),
            OptimizationMethod::GeneticAlgorithm { .. } => self.genetic_algorithm(variables),
            OptimizationMethod::SimulatedAnnealing { .. } => self.simulated_annealing(variables),
        }
    }

    fn gradient_descent(
        &mut self,
        variables: &HashMap<String, Variable>,
    ) -> Result<OptimizationResult, OptimizationError> {
        let OptimizationMethod::GradientDescent {
            learning_rate,
            momentum,
        } = self.config.method
        else {
            return Err(OptimizationError::Failed("Invalid method".into()));
        };

        let mut current_values = HashMap::new();
        let mut velocities = HashMap::new();
        let mut convergence_history = Vec::new();

        // Initialize with current values
        for (name, var) in variables {
            current_values.insert(name.clone(), var.value);
            velocities.insert(name.clone(), 0.0);
        }

        for iteration in 0..self.config.max_iterations {
            let mut gradients = HashMap::new();
            
            // Compute numerical gradients
            for (name, _) in variables {
                let h = 1e-6; // Small perturbation
                let mut plus_h = current_values.clone();
                let mut minus_h = current_values.clone();
                
                *plus_h.get_mut(name).unwrap() += h;
                *minus_h.get_mut(name).unwrap() -= h;

                let gradient = ((self.objective_fn)(&plus_h) - (self.objective_fn)(&minus_h)) / (2.0 * h);
                gradients.insert(name.clone(), gradient);
            }

            // Update values with momentum
            let mut max_change = 0.0;
            for (name, var) in variables {
                let velocity = velocities.get_mut(name).unwrap();
                *velocity = momentum * *velocity - learning_rate * gradients[name];
                
                let value = current_values.get_mut(name).unwrap();
                let old_value = *value;
                *value += *velocity;

                // Apply bounds
                if let Some(min) = var.min {
                    *value = value.max(min);
                }
                if let Some(max) = var.max {
                    *value = value.min(max);
                }

                max_change = max_change.max((*value - old_value).abs());
            }

            // Check constraints
            let constraints_satisfied = self.constraints.iter()
                .all(|constraint| (constraint)(&current_values));

            let objective_value = (self.objective_fn)(&current_values);
            convergence_history.push(objective_value);

            if max_change < self.config.tolerance && constraints_satisfied {
                return Ok(OptimizationResult {
                    optimal_values: current_values,
                    objective_value,
                    iterations: iteration + 1,
                    convergence_history,
                    constraints_satisfied,
                });
            }
        }

        Err(OptimizationError::Failed("Max iterations reached".into()))
    }

    fn particle_swarm(
        &mut self,
        variables: &HashMap<String, Variable>,
    ) -> Result<OptimizationResult, OptimizationError> {
        let OptimizationMethod::ParticleSwarm {
            inertia,
            cognitive,
            social,
        } = self.config.method
        else {
            return Err(OptimizationError::Failed("Invalid method".into()));
        };

        // Initialize particles
        let mut particles = Vec::new();
        let mut velocities = Vec::new();
        let mut personal_best_positions = Vec::new();
        let mut personal_best_values = Vec::new();
        let mut global_best_position = HashMap::new();
        let mut global_best_value = f64::INFINITY;
        let mut convergence_history = Vec::new();

        for _ in 0..self.config.population_size {
            let mut position = HashMap::new();
            let mut velocity = HashMap::new();

            for (name, var) in variables {
                let range = var.max.unwrap_or(1.0) - var.min.unwrap_or(0.0);
                position.insert(name.clone(), self.rng.gen::<f64>() * range + var.min.unwrap_or(0.0));
                velocity.insert(name.clone(), (self.rng.gen::<f64>() - 0.5) * range * 0.1);
            }

            let value = (self.objective_fn)(&position);
            if value < global_best_value {
                global_best_value = value;
                global_best_position = position.clone();
            }

            particles.push(position.clone());
            velocities.push(velocity);
            personal_best_positions.push(position);
            personal_best_values.push(value);
        }

        // Main PSO loop
        for iteration in 0..self.config.max_iterations {
            for i in 0..self.config.population_size {
                let mut new_position = HashMap::new();
                let mut new_velocity = HashMap::new();

                for (name, var) in variables {
                    let r1 = self.rng.gen::<f64>();
                    let r2 = self.rng.gen::<f64>();

                    let vel = velocities[i][name];
                    let pos = particles[i][name];
                    let p_best = personal_best_positions[i][name];
                    let g_best = global_best_position[name];

                    let new_vel = inertia * vel
                        + cognitive * r1 * (p_best - pos)
                        + social * r2 * (g_best - pos);

                    let mut new_pos = pos + new_vel;

                    // Apply bounds
                    if let Some(min) = var.min {
                        new_pos = new_pos.max(min);
                    }
                    if let Some(max) = var.max {
                        new_pos = new_pos.min(max);
                    }

                    new_velocity.insert(name.clone(), new_vel);
                    new_position.insert(name.clone(), new_pos);
                }

                let new_value = (self.objective_fn)(&new_position);

                // Update personal best
                if new_value < personal_best_values[i] {
                    personal_best_values[i] = new_value;
                    personal_best_positions[i] = new_position.clone();

                    // Update global best
                    if new_value < global_best_value {
                        global_best_value = new_value;
                        global_best_position = new_position.clone();
                    }
                }

                particles[i] = new_position;
                velocities[i] = new_velocity;
            }

            convergence_history.push(global_best_value);

            // Check convergence
            if iteration > 0 && (convergence_history[iteration] - convergence_history[iteration - 1]).abs() < self.config.tolerance {
                let constraints_satisfied = self.constraints.iter()
                    .all(|constraint| (constraint)(&global_best_position));

                return Ok(OptimizationResult {
                    optimal_values: global_best_position,
                    objective_value: global_best_value,
                    iterations: iteration + 1,
                    convergence_history,
                    constraints_satisfied,
                });
            }
        }

        Err(OptimizationError::Failed("Max iterations reached".into()))
    }

    fn genetic_algorithm(
        &mut self,
        variables: &HashMap<String, Variable>,
    ) -> Result<OptimizationResult, OptimizationError> {
        let OptimizationMethod::GeneticAlgorithm {
            elite_size,
            tournament_size,
        } = self.config.method
        else {
            return Err(OptimizationError::Failed("Invalid method".into()));
        };

        // Initialize population
        let mut population = Vec::new();
        let mut fitness_values = Vec::new();
        let mut best_individual = HashMap::new();
        let mut best_fitness = f64::INFINITY;
        let mut convergence_history = Vec::new();

        for _ in 0..self.config.population_size {
            let mut individual = HashMap::new();
            for (name, var) in variables {
                let range = var.max.unwrap_or(1.0) - var.min.unwrap_or(0.0);
                individual.insert(name.clone(), self.rng.gen::<f64>() * range + var.min.unwrap_or(0.0));
            }
            population.push(individual);
        }

        // Main GA loop
        for iteration in 0..self.config.max_iterations {
            // Evaluate fitness
            fitness_values.clear();
            for individual in &population {
                let fitness = (self.objective_fn)(individual);
                fitness_values.push(fitness);

                if fitness < best_fitness {
                    best_fitness = fitness;
                    best_individual = individual.clone();
                }
            }

            convergence_history.push(best_fitness);

            // Check convergence
            if iteration > 0 && (convergence_history[iteration] - convergence_history[iteration - 1]).abs() < self.config.tolerance {
                let constraints_satisfied = self.constraints.iter()
                    .all(|constraint| (constraint)(&best_individual));

                return Ok(OptimizationResult {
                    optimal_values: best_individual,
                    objective_value: best_fitness,
                    iterations: iteration + 1,
                    convergence_history,
                    constraints_satisfied,
                });
            }

            // Create new population
            let mut new_population = Vec::new();

            // Elitism
            let mut indexed_fitness: Vec<(usize, f64)> = fitness_values.iter().enumerate()
                .map(|(i, &f)| (i, f))
                .collect();
            indexed_fitness.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            for i in 0..elite_size {
                new_population.push(population[indexed_fitness[i].0].clone());
            }

            // Crossover and mutation
            while new_population.len() < self.config.population_size {
                // Tournament selection
                let parent1 = self.tournament_select(&population, &fitness_values, tournament_size);
                let parent2 = self.tournament_select(&population, &fitness_values, tournament_size);

                // Crossover
                let mut child = if self.rng.gen::<f64>() < self.config.crossover_rate {
                    self.crossover(&parent1, &parent2)
                } else {
                    parent1.clone()
                };

                // Mutation
                self.mutate(&mut child, variables);

                new_population.push(child);
            }

            population = new_population;
        }

        Err(OptimizationError::Failed("Max iterations reached".into()))
    }

    fn tournament_select(
        &mut self,
        population: &[HashMap<String, f64>],
        fitness_values: &[f64],
        tournament_size: usize,
    ) -> HashMap<String, f64> {
        let mut best_idx = self.rng.gen_range(0..population.len());
        let mut best_fitness = fitness_values[best_idx];

        for _ in 1..tournament_size {
            let idx = self.rng.gen_range(0..population.len());
            if fitness_values[idx] < best_fitness {
                best_idx = idx;
                best_fitness = fitness_values[idx];
            }
        }

        population[best_idx].clone()
    }

    fn crossover(
        &mut self,
        parent1: &HashMap<String, f64>,
        parent2: &HashMap<String, f64>,
    ) -> HashMap<String, f64> {
        let mut child = HashMap::new();
        
        for (name, &value1) in parent1 {
            let value2 = parent2[name];
            let alpha = self.rng.gen::<f64>();
            child.insert(name.clone(), alpha * value1 + (1.0 - alpha) * value2);
        }

        child
    }

    fn mutate(&mut self, individual: &mut HashMap<String, f64>, variables: &HashMap<String, Variable>) {
        for (name, value) in individual.iter_mut() {
            if self.rng.gen::<f64>() < self.config.mutation_rate {
                let var = &variables[name];
                let range = var.max.unwrap_or(1.0) - var.min.unwrap_or(0.0);
                let mutation = (self.rng.gen::<f64>() - 0.5) * range * 0.1;
                *value = (*value + mutation)
                    .max(var.min.unwrap_or(f64::NEG_INFINITY))
                    .min(var.max.unwrap_or(f64::INFINITY));
            }
        }
    }

    fn simulated_annealing(
        &mut self,
        variables: &HashMap<String, Variable>,
    ) -> Result<OptimizationResult, OptimizationError> {
        let OptimizationMethod::SimulatedAnnealing {
            initial_temp,
            cooling_rate,
        } = self.config.method
        else {
            return Err(OptimizationError::Failed("Invalid method".into()));
        };

        let mut current_solution = HashMap::new();
        let mut best_solution = HashMap::new();
        let mut convergence_history = Vec::new();

        // Initialize with current values
        for (name, var) in variables {
            current_solution.insert(name.clone(), var.value);
        }

        let mut current_value = (self.objective_fn)(&current_solution);
        let mut best_value = current_value;
        best_solution = current_solution.clone();

        let mut temperature = initial_temp;

        for iteration in 0..self.config.max_iterations {
            // Generate neighbor
            let mut neighbor = current_solution.clone();
            for (name, var) in variables {
                if self.rng.gen::<f64>() < 0.5 {
                    let range = var.max.unwrap_or(1.0) - var.min.unwrap_or(0.0);
                    let perturbation = (self.rng.gen::<f64>() - 0.5) * range * temperature / initial_temp;
                    *neighbor.get_mut(name).unwrap() = (neighbor[name] + perturbation)
                        .max(var.min.unwrap_or(f64::NEG_INFINITY))
                        .min(var.max.unwrap_or(f64::INFINITY));
                }
            }

            let neighbor_value = (self.objective_fn)(&neighbor);

            // Accept or reject new solution
            let accept = if neighbor_value <= current_value {
                true
            } else {
                let probability = ((current_value - neighbor_value) / temperature).exp();
                self.rng.gen::<f64>() < probability
            };

            if accept {
                current_solution = neighbor;
                current_value = neighbor_value;

                if current_value < best_value {
                    best_value = current_value;
                    best_solution = current_solution.clone();
                }
            }

            convergence_history.push(best_value);
            temperature *= cooling_rate;

            // Check convergence
            if temperature < self.config.tolerance {
                let constraints_satisfied = self.constraints.iter()
                    .all(|constraint| (constraint)(&best_solution));

                return Ok(OptimizationResult {
                    optimal_values: best_solution,
                    objective_value: best_value,
                    iterations: iteration + 1,
                    convergence_history,
                    constraints_satisfied,
                });
            }
        }

        Err(OptimizationError::Failed("Max iterations reached".into()))
    }
} 