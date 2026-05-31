//! Sustainability Error Types
//! 
//! Defines error types for sustainability modeling

use std::{error::Error, fmt};

/// Sustainability error type
#[derive(Debug)]
pub enum SustainabilityError {
    /// Invalid configuration
    InvalidConfig(String),
    
    /// Data error
    DataError(String),
    
    /// Calculation error
    CalculationError(String),
    
    /// Optimization error
    OptimizationError(String),
    
    /// Validation error
    ValidationError(String),
    
    /// Integration error
    IntegrationError(String),
    
    /// Resource error
    ResourceError(String),
    
    /// System error
    SystemError(String),
}

impl fmt::Display for SustainabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::DataError(msg) => write!(f, "Data error: {}", msg),
            Self::CalculationError(msg) => write!(f, "Calculation error: {}", msg),
            Self::OptimizationError(msg) => write!(f, "Optimization error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::IntegrationError(msg) => write!(f, "Integration error: {}", msg),
            Self::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            Self::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl Error for SustainabilityError {} 