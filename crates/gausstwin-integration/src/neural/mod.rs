//! Neural Network-based Optimization System
//! 
//! Provides advanced neural network capabilities for optimization and prediction.

use std::sync::Arc;
use parking_lot::RwLock;
use tch::{Device, Tensor, nn};
use serde::{Deserialize, Serialize};

/// Neural network types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkType {
    /// Feedforward neural network
    Feedforward {
        layers: Vec<usize>,
        activation: Activation,
    },
    /// Long short-term memory
    LSTM {
        input_size: usize,
        hidden_size: usize,
        num_layers: usize,
    },
    /// Transformer
    Transformer {
        num_layers: usize,
        num_heads: usize,
        d_model: usize,
        d_ff: usize,
    },
    /// Graph neural network
    GNN {
        node_features: usize,
        edge_features: usize,
        hidden_size: usize,
        num_layers: usize,
    },
}

/// Activation functions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Activation {
    ReLU,
    LeakyReLU,
    Sigmoid,
    Tanh,
    GELU,
}

/// Neural optimizer types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizerType {
    Adam {
        learning_rate: f64,
        beta1: f64,
        beta2: f64,
    },
    AdamW {
        learning_rate: f64,
        weight_decay: f64,
    },
    RMSprop {
        learning_rate: f64,
        alpha: f64,
    },
}

/// Training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub batch_size: usize,
    pub num_epochs: usize,
    pub optimizer: OptimizerType,
    pub scheduler: Option<SchedulerType>,
    pub early_stopping: Option<EarlyStoppingConfig>,
}

/// Learning rate scheduler types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulerType {
    StepLR {
        step_size: usize,
        gamma: f64,
    },
    CosineAnnealing {
        t_max: usize,
        eta_min: f64,
    },
    ReduceLROnPlateau {
        patience: usize,
        factor: f64,
        min_lr: f64,
    },
}

/// Early stopping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EarlyStoppingConfig {
    pub patience: usize,
    pub min_delta: f64,
    pub mode: EarlyStoppingMode,
}

/// Early stopping modes
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum EarlyStoppingMode {
    Min,
    Max,
}

/// Neural network manager
pub struct NeuralManager {
    network_type: NetworkType,
    model: Arc<RwLock<nn::Sequential>>,
    optimizer: Arc<RwLock<Box<dyn nn::Optimizer>>>,
    device: Device,
    config: TrainingConfig,
}

impl NeuralManager {
    pub fn new(network_type: NetworkType, config: TrainingConfig) -> tch::Result<Self> {
        let device = Device::cuda_if_available();
        let vs = nn::VarStore::new(device);
        let model = Self::create_network(&network_type, vs.root())?;
        let optimizer = Self::create_optimizer(&config.optimizer, &vs)?;
        
        Ok(Self {
            network_type,
            model: Arc::new(RwLock::new(model)),
            optimizer: Arc::new(RwLock::new(optimizer)),
            device,
            config,
        })
    }
    
    /// Create neural network
    fn create_network(network_type: &NetworkType, vs: nn::Path) -> tch::Result<nn::Sequential> {
        match network_type {
            NetworkType::Feedforward { layers, activation } => {
                let mut seq = nn::Sequential::new();
                
                for i in 0..layers.len() - 1 {
                    seq = seq.add(nn::linear(&vs, layers[i] as i64, layers[i+1] as i64, Default::default()));
                    
                    if i < layers.len() - 2 {
                        seq = seq.add(Self::create_activation(*activation));
                    }
                }
                
                Ok(seq)
            }
            NetworkType::LSTM { input_size, hidden_size, num_layers } => {
                let lstm = nn::lstm(&vs, *input_size as i64, *hidden_size as i64, Default::default());
                let mut seq = nn::Sequential::new();
                
                for _ in 0..*num_layers {
                    seq = seq.add(lstm.clone());
                }
                
                Ok(seq)
            }
            NetworkType::Transformer { num_layers, num_heads, d_model, d_ff } => {
                let mut seq = nn::Sequential::new();
                
                for _ in 0..*num_layers {
                    // Multi-head attention
                    let mha = nn::attention(&vs, *d_model as i64, *num_heads as i64, Default::default());
                    seq = seq.add(mha);
                    
                    // Feed-forward network
                    seq = seq.add(nn::linear(&vs, *d_model as i64, *d_ff as i64, Default::default()));
                    seq = seq.add(nn::func(|xs| xs.gelu()));
                    seq = seq.add(nn::linear(&vs, *d_ff as i64, *d_model as i64, Default::default()));
                    
                    // Layer normalization
                    seq = seq.add(nn::layer_norm(&vs, vec![*d_model as i64], Default::default()));
                }
                
                Ok(seq)
            }
            NetworkType::GNN { node_features, edge_features, hidden_size, num_layers } => {
                let mut seq = nn::Sequential::new();
                
                // Node embedding
                seq = seq.add(nn::linear(&vs, *node_features as i64, *hidden_size as i64, Default::default()));
                
                // Edge embedding
                let edge_embed = nn::linear(&vs, *edge_features as i64, *hidden_size as i64, Default::default());
                
                // Message passing layers
                for _ in 0..*num_layers {
                    seq = seq.add_fn(move |xs| {
                        // Message function
                        let messages = xs.matmul(&edge_embed.forward(&xs.transpose(0, 1)));
                        
                        // Update function
                        let updates = messages.mean_dim(&[1], false, tch::Kind::Float);
                        
                        xs + updates
                    });
                    
                    seq = seq.add(nn::layer_norm(&vs, vec![*hidden_size as i64], Default::default()));
                    seq = seq.add(nn::func(|xs| xs.relu()));
                }
                
                Ok(seq)
            }
        }
    }
    
    /// Create activation function
    fn create_activation(activation: Activation) -> nn::Func {
        match activation {
            Activation::ReLU => nn::func(|xs| xs.relu()),
            Activation::LeakyReLU => nn::func(|xs| xs.leaky_relu()),
            Activation::Sigmoid => nn::func(|xs| xs.sigmoid()),
            Activation::Tanh => nn::func(|xs| xs.tanh()),
            Activation::GELU => nn::func(|xs| xs.gelu()),
        }
    }
    
    /// Create optimizer
    fn create_optimizer(optimizer_type: &OptimizerType, vs: &nn::VarStore) -> tch::Result<Box<dyn nn::Optimizer>> {
        match optimizer_type {
            OptimizerType::Adam { learning_rate, beta1, beta2 } => {
                Ok(Box::new(nn::Adam::new(vs.trainable_variables(), *learning_rate, *beta1, *beta2)?))
            }
            OptimizerType::AdamW { learning_rate, weight_decay } => {
                Ok(Box::new(nn::AdamW::new(vs.trainable_variables(), *learning_rate, *weight_decay)?))
            }
            OptimizerType::RMSprop { learning_rate, alpha } => {
                Ok(Box::new(nn::RmsProp::new(vs.trainable_variables(), *learning_rate, *alpha)?))
            }
        }
    }
    
    /// Train network
    pub fn train(&self, data: &DataLoader) -> tch::Result<TrainingMetrics> {
        let mut metrics = TrainingMetrics::default();
        let mut early_stopper = self.config.early_stopping.map(EarlyStopper::new);
        
        for epoch in 0..self.config.num_epochs {
            let mut epoch_loss = 0.0;
            let mut num_batches = 0;
            
            // Training loop
            for (inputs, targets) in data.train_batches(self.config.batch_size) {
                let inputs = inputs.to_device(self.device);
                let targets = targets.to_device(self.device);
                
                // Forward pass
                let outputs = {
                    let model = self.model.read();
                    model.forward(&inputs)
                };
                
                // Compute loss
                let loss = outputs.mse_loss(&targets, tch::Reduction::Mean);
                epoch_loss += f64::from(&loss);
                
                // Backward pass
                {
                    let mut optimizer = self.optimizer.write();
                    optimizer.zero_grad();
                    loss.backward();
                    optimizer.step();
                }
                
                num_batches += 1;
            }
            
            // Compute epoch metrics
            epoch_loss /= num_batches as f64;
            metrics.train_losses.push(epoch_loss);
            
            // Validation
            let val_loss = self.validate(data)?;
            metrics.val_losses.push(val_loss);
            
            // Learning rate scheduling
            if let Some(scheduler) = &self.config.scheduler {
                self.update_learning_rate(scheduler, val_loss);
            }
            
            // Early stopping
            if let Some(stopper) = &mut early_stopper {
                if stopper.should_stop(val_loss) {
                    break;
                }
            }
        }
        
        Ok(metrics)
    }
    
    /// Validate network
    fn validate(&self, data: &DataLoader) -> tch::Result<f64> {
        let mut val_loss = 0.0;
        let mut num_batches = 0;
        
        tch::no_grad(|| {
            for (inputs, targets) in data.val_batches(self.config.batch_size) {
                let inputs = inputs.to_device(self.device);
                let targets = targets.to_device(self.device);
                
                let outputs = {
                    let model = self.model.read();
                    model.forward(&inputs)
                };
                
                let loss = outputs.mse_loss(&targets, tch::Reduction::Mean);
                val_loss += f64::from(&loss);
                num_batches += 1;
            }
        });
        
        Ok(val_loss / num_batches as f64)
    }
    
    /// Update learning rate
    fn update_learning_rate(&self, scheduler: &SchedulerType, val_loss: f64) {
        let mut optimizer = self.optimizer.write();
        
        match scheduler {
            SchedulerType::StepLR { step_size, gamma } => {
                if optimizer.step_count() % step_size == 0 {
                    optimizer.set_learning_rate(optimizer.learning_rate() * gamma);
                }
            }
            SchedulerType::CosineAnnealing { t_max, eta_min } => {
                let progress = (optimizer.step_count() % t_max) as f64 / *t_max as f64;
                let lr = eta_min + (optimizer.learning_rate() - eta_min) * 
                        (1.0 + std::f64::consts::PI * progress).cos() / 2.0;
                optimizer.set_learning_rate(lr);
            }
            SchedulerType::ReduceLROnPlateau { patience, factor, min_lr } => {
                if optimizer.step_count() > *patience && val_loss > optimizer.best_loss() {
                    let new_lr = (optimizer.learning_rate() * factor).max(*min_lr);
                    optimizer.set_learning_rate(new_lr);
                }
            }
        }
    }
    
    /// Predict using network
    pub fn predict(&self, inputs: &Tensor) -> tch::Result<Tensor> {
        let inputs = inputs.to_device(self.device);
        
        tch::no_grad(|| {
            let model = self.model.read();
            Ok(model.forward(&inputs))
        })
    }
}

/// Training metrics
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub train_losses: Vec<f64>,
    pub val_losses: Vec<f64>,
}

/// Early stopping
struct EarlyStopper {
    patience: usize,
    min_delta: f64,
    mode: EarlyStoppingMode,
    counter: usize,
    best_value: f64,
}

impl EarlyStopper {
    fn new(config: EarlyStoppingConfig) -> Self {
        Self {
            patience: config.patience,
            min_delta: config.min_delta,
            mode: config.mode,
            counter: 0,
            best_value: match config.mode {
                EarlyStoppingMode::Min => f64::INFINITY,
                EarlyStoppingMode::Max => f64::NEG_INFINITY,
            },
        }
    }
    
    fn should_stop(&mut self, value: f64) -> bool {
        let improved = match self.mode {
            EarlyStoppingMode::Min => value < self.best_value - self.min_delta,
            EarlyStoppingMode::Max => value > self.best_value + self.min_delta,
        };
        
        if improved {
            self.best_value = value;
            self.counter = 0;
            false
        } else {
            self.counter += 1;
            self.counter >= self.patience
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tch::Kind;

    #[test]
    fn test_feedforward_network() -> tch::Result<()> {
        let network_type = NetworkType::Feedforward {
            layers: vec![10, 64, 32, 1],
            activation: Activation::ReLU,
        };
        
        let config = TrainingConfig {
            batch_size: 32,
            num_epochs: 10,
            optimizer: OptimizerType::Adam {
                learning_rate: 0.001,
                beta1: 0.9,
                beta2: 0.999,
            },
            scheduler: Some(SchedulerType::StepLR {
                step_size: 5,
                gamma: 0.1,
            }),
            early_stopping: Some(EarlyStoppingConfig {
                patience: 3,
                min_delta: 1e-4,
                mode: EarlyStoppingMode::Min,
            }),
        };
        
        let manager = NeuralManager::new(network_type, config)?;
        
        // Create dummy data
        let inputs = Tensor::rand(&[100, 10], (Kind::Float, Device::Cpu));
        let targets = Tensor::rand(&[100, 1], (Kind::Float, Device::Cpu));
        
        // Test prediction
        let outputs = manager.predict(&inputs)?;
        assert_eq!(outputs.size(), &[100, 1]);
        
        Ok(())
    }

    #[test]
    fn test_lstm_network() -> tch::Result<()> {
        let network_type = NetworkType::LSTM {
            input_size: 10,
            hidden_size: 32,
            num_layers: 2,
        };
        
        let config = TrainingConfig {
            batch_size: 32,
            num_epochs: 10,
            optimizer: OptimizerType::AdamW {
                learning_rate: 0.001,
                weight_decay: 0.01,
            },
            scheduler: None,
            early_stopping: None,
        };
        
        let manager = NeuralManager::new(network_type, config)?;
        
        // Create dummy sequential data
        let inputs = Tensor::rand(&[100, 20, 10], (Kind::Float, Device::Cpu));
        let outputs = manager.predict(&inputs)?;
        assert_eq!(outputs.size()[1..], [20, 32]);
        
        Ok(())
    }
} 