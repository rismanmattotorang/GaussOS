//! Quantum-inspired Optimization System
//! 
//! Provides quantum-inspired algorithms for optimization problems.

use std::sync::Arc;
use parking_lot::RwLock;
use rand::prelude::*;
use rand_distr::{Distribution, Normal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use nalgebra as na;

/// Quantum state
#[derive(Clone, Debug)]
pub struct QuantumState {
    /// Amplitude vector
    amplitudes: na::DVector<num_complex::Complex64>,
    /// Number of qubits
    num_qubits: usize,
}

impl QuantumState {
    pub fn new(num_qubits: usize) -> Self {
        let dim = 1 << num_qubits;
        let mut amplitudes = na::DVector::zeros(dim);
        amplitudes[0] = num_complex::Complex64::new(1.0, 0.0);
        
        Self {
            amplitudes,
            num_qubits,
        }
    }
    
    /// Apply quantum gate
    pub fn apply_gate(&mut self, gate: &QuantumGate) {
        match gate {
            QuantumGate::Hadamard { target } => {
                let h = na::Matrix2::new(
                    1.0 / 2.0_f64.sqrt(), 1.0 / 2.0_f64.sqrt(),
                    1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(),
                );
                self.apply_single_qubit_gate(*target, &h);
            }
            QuantumGate::PauliX { target } => {
                let x = na::Matrix2::new(
                    0.0, 1.0,
                    1.0, 0.0,
                );
                self.apply_single_qubit_gate(*target, &x);
            }
            QuantumGate::PauliY { target } => {
                let y = na::Matrix2::new(
                    0.0, -num_complex::Complex64::i(),
                    num_complex::Complex64::i(), 0.0,
                );
                self.apply_single_qubit_gate(*target, &y);
            }
            QuantumGate::PauliZ { target } => {
                let z = na::Matrix2::new(
                    1.0, 0.0,
                    0.0, -1.0,
                );
                self.apply_single_qubit_gate(*target, &z);
            }
            QuantumGate::CNOT { control, target } => {
                self.apply_controlled_gate(*control, *target, &na::Matrix2::new(
                    0.0, 1.0,
                    1.0, 0.0,
                ));
            }
            QuantumGate::Phase { target, phase } => {
                let p = na::Matrix2::new(
                    1.0, 0.0,
                    0.0, num_complex::Complex64::from_polar(1.0, *phase),
                );
                self.apply_single_qubit_gate(*target, &p);
            }
        }
    }
    
    /// Apply single-qubit gate
    fn apply_single_qubit_gate(&mut self, target: usize, gate: &na::Matrix2<num_complex::Complex64>) {
        let mask = 1 << target;
        
        for i in 0..self.amplitudes.len() {
            if i & mask == 0 {
                let i1 = i | mask;
                let a0 = self.amplitudes[i];
                let a1 = self.amplitudes[i1];
                
                self.amplitudes[i] = gate[(0, 0)] * a0 + gate[(0, 1)] * a1;
                self.amplitudes[i1] = gate[(1, 0)] * a0 + gate[(1, 1)] * a1;
            }
        }
    }
    
    /// Apply controlled gate
    fn apply_controlled_gate(
        &mut self,
        control: usize,
        target: usize,
        gate: &na::Matrix2<num_complex::Complex64>,
    ) {
        let control_mask = 1 << control;
        let target_mask = 1 << target;
        
        for i in 0..self.amplitudes.len() {
            if (i & control_mask != 0) && (i & target_mask == 0) {
                let i1 = i | target_mask;
                let a0 = self.amplitudes[i];
                let a1 = self.amplitudes[i1];
                
                self.amplitudes[i] = gate[(0, 0)] * a0 + gate[(0, 1)] * a1;
                self.amplitudes[i1] = gate[(1, 0)] * a0 + gate[(1, 1)] * a1;
            }
        }
    }
    
    /// Measure quantum state
    pub fn measure(&self) -> Vec<bool> {
        let mut rng = thread_rng();
        let mut result = vec![false; self.num_qubits];
        let mut prob_sum = 0.0;
        let r = rng.gen::<f64>();
        
        for (i, &amplitude) in self.amplitudes.iter().enumerate() {
            prob_sum += amplitude.norm_sqr();
            if r <= prob_sum {
                for j in 0..self.num_qubits {
                    result[j] = (i & (1 << j)) != 0;
                }
                break;
            }
        }
        
        result
    }
}

/// Quantum gates
#[derive(Clone, Debug)]
pub enum QuantumGate {
    Hadamard { target: usize },
    PauliX { target: usize },
    PauliY { target: usize },
    PauliZ { target: usize },
    CNOT { control: usize, target: usize },
    Phase { target: usize, phase: f64 },
}

/// Quantum-inspired optimization algorithm
pub struct QuantumOptimizer {
    /// Number of qubits
    num_qubits: usize,
    /// Population size
    population_size: usize,
    /// Quantum states
    states: Vec<QuantumState>,
    /// Best solution
    best_solution: Arc<RwLock<Solution>>,
    /// Optimization parameters
    params: OptimizationParams,
}

impl QuantumOptimizer {
    pub fn new(num_qubits: usize, population_size: usize, params: OptimizationParams) -> Self {
        let states = (0..population_size)
            .map(|_| QuantumState::new(num_qubits))
            .collect();
        
        Self {
            num_qubits,
            population_size,
            states,
            best_solution: Arc::new(RwLock::new(Solution::default())),
            params,
        }
    }
    
    /// Run optimization
    pub fn optimize<F>(&mut self, objective: F, max_iterations: usize) -> OptimizationResult
    where
        F: Fn(&[bool]) -> f64 + Send + Sync,
    {
        let mut rng = thread_rng();
        let mut iteration = 0;
        let mut no_improvement = 0;
        
        while iteration < max_iterations && no_improvement < self.params.max_no_improvement {
            // Update quantum states
            self.update_states();
            
            // Measure states and evaluate solutions
            let solutions: Vec<_> = self.states
                .par_iter()
                .map(|state| {
                    let bits = state.measure();
                    let fitness = objective(&bits);
                    Solution { bits, fitness }
                })
                .collect();
            
            // Update best solution
            let mut best = self.best_solution.write();
            let current_best = solutions
                .iter()
                .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap())
                .unwrap();
            
            if current_best.fitness > best.fitness {
                *best = current_best.clone();
                no_improvement = 0;
            } else {
                no_improvement += 1;
            }
            
            // Apply quantum gates
            for state in &mut self.states {
                // Hadamard gates
                for i in 0..self.num_qubits {
                    if rng.gen::<f64>() < self.params.hadamard_prob {
                        state.apply_gate(&QuantumGate::Hadamard { target: i });
                    }
                }
                
                // CNOT gates
                for i in 0..self.num_qubits {
                    for j in 0..self.num_qubits {
                        if i != j && rng.gen::<f64>() < self.params.cnot_prob {
                            state.apply_gate(&QuantumGate::CNOT {
                                control: i,
                                target: j,
                            });
                        }
                    }
                }
                
                // Phase gates
                for i in 0..self.num_qubits {
                    if rng.gen::<f64>() < self.params.phase_prob {
                        let phase = rng.gen::<f64>() * 2.0 * std::f64::consts::PI;
                        state.apply_gate(&QuantumGate::Phase {
                            target: i,
                            phase,
                        });
                    }
                }
            }
            
            iteration += 1;
        }
        
        OptimizationResult {
            best_solution: self.best_solution.read().clone(),
            iterations: iteration,
            no_improvement_count: no_improvement,
        }
    }
    
    /// Update quantum states
    fn update_states(&mut self) {
        let best = self.best_solution.read();
        
        for state in &mut self.states {
            // Apply rotation gates based on best solution
            for (i, &bit) in best.bits.iter().enumerate() {
                let angle = if bit {
                    self.params.rotation_angle
                } else {
                    -self.params.rotation_angle
                };
                
                state.apply_gate(&QuantumGate::Phase {
                    target: i,
                    phase: angle,
                });
            }
            
            // Apply mutation
            if thread_rng().gen::<f64>() < self.params.mutation_prob {
                let target = thread_rng().gen_range(0..self.num_qubits);
                state.apply_gate(&QuantumGate::PauliX { target });
            }
        }
    }
}

/// Optimization parameters
#[derive(Clone, Debug)]
pub struct OptimizationParams {
    /// Rotation angle
    pub rotation_angle: f64,
    /// Hadamard gate probability
    pub hadamard_prob: f64,
    /// CNOT gate probability
    pub cnot_prob: f64,
    /// Phase gate probability
    pub phase_prob: f64,
    /// Mutation probability
    pub mutation_prob: f64,
    /// Maximum iterations without improvement
    pub max_no_improvement: usize,
}

impl Default for OptimizationParams {
    fn default() -> Self {
        Self {
            rotation_angle: 0.1,
            hadamard_prob: 0.1,
            cnot_prob: 0.05,
            phase_prob: 0.05,
            mutation_prob: 0.01,
            max_no_improvement: 100,
        }
    }
}

/// Optimization solution
#[derive(Clone, Debug, Default)]
pub struct Solution {
    /// Bit string
    pub bits: Vec<bool>,
    /// Fitness value
    pub fitness: f64,
}

/// Optimization result
#[derive(Clone, Debug)]
pub struct OptimizationResult {
    /// Best solution found
    pub best_solution: Solution,
    /// Number of iterations
    pub iterations: usize,
    /// Number of iterations without improvement
    pub no_improvement_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_state() {
        let mut state = QuantumState::new(2);
        
        // Apply Hadamard to both qubits
        state.apply_gate(&QuantumGate::Hadamard { target: 0 });
        state.apply_gate(&QuantumGate::Hadamard { target: 1 });
        
        // Measure multiple times
        let mut counts = vec![0; 4];
        for _ in 0..1000 {
            let measurement = state.measure();
            let idx = measurement[0] as usize + (measurement[1] as usize << 1);
            counts[idx] += 1;
        }
        
        // Check if distribution is roughly uniform
        for count in counts {
            assert!(count > 200 && count < 300); // Should be around 250
        }
    }

    #[test]
    fn test_quantum_optimization() {
        // Define optimization problem (maximize number of 1s)
        let objective = |bits: &[bool]| -> f64 {
            bits.iter().filter(|&&b| b).count() as f64
        };
        
        let mut optimizer = QuantumOptimizer::new(10, 50, OptimizationParams::default());
        let result = optimizer.optimize(objective, 1000);
        
        // Check if solution is good
        assert!(result.best_solution.fitness >= 8.0); // Should find at least 8 ones
        assert!(result.iterations > 0);
        assert!(result.no_improvement_count < 100);
    }
} 