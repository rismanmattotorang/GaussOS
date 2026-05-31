//! Pathfinding Error Types
//!
//! Comprehensive error handling for pathfinding operations.

use std::fmt;
use thiserror::Error;

/// Errors that can occur during pathfinding operations
#[derive(Debug, Clone, Error)]
pub enum PathfindingError {
    /// No path exists between start and goal
    #[error("No path found between start and goal")]
    NoPathFound,
    
    /// Start position is invalid (out of bounds or blocked)
    #[error("Invalid start position")]
    InvalidStart,
    
    /// Goal position is invalid (out of bounds or blocked)
    #[error("Invalid goal position")]
    InvalidGoal,
    
    /// Invalid waypoints provided
    #[error("Invalid waypoints for path planning")]
    InvalidWaypoints,
    
    /// Invalid path detected
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    /// Maximum iterations exceeded without finding a path
    #[error("Maximum iterations exceeded")]
    MaxIterationsExceeded,
    
    /// Memory allocation failed
    #[error("Memory allocation failed")]
    OutOfMemory,
    
    /// Graph is not connected
    #[error("Graph is not connected")]
    DisconnectedGraph,
    
    /// Invalid graph configuration
    #[error("Invalid graph configuration: {0}")]
    InvalidGraph(String),
    
    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),
    
    /// Algorithm not suitable for this graph type
    #[error("Algorithm not suitable: {0}")]
    UnsuitableAlgorithm(String),
    
    /// Timeout during pathfinding
    #[error("Pathfinding timed out after {0:?}")]
    Timeout(std::time::Duration),
    
    /// Path smoothing failed
    #[error("Path smoothing failed: {0}")]
    SmoothingFailed(String),
    
    /// Hierarchical pathfinding error
    #[error("Hierarchical pathfinding error: {0}")]
    HierarchicalError(String),
    
    /// Internal error
    #[error("Internal pathfinding error: {0}")]
    Internal(String),
}

impl PathfindingError {
    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            PathfindingError::NoPathFound
                | PathfindingError::MaxIterationsExceeded
                | PathfindingError::Timeout(_)
                | PathfindingError::CacheError(_)
        )
    }
    
    /// Get error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            PathfindingError::NoPathFound => "path_not_found",
            PathfindingError::InvalidStart | PathfindingError::InvalidGoal => "invalid_position",
            PathfindingError::InvalidWaypoints | PathfindingError::InvalidPath(_) => "invalid_input",
            PathfindingError::MaxIterationsExceeded | PathfindingError::Timeout(_) => "resource_limit",
            PathfindingError::OutOfMemory => "memory",
            PathfindingError::DisconnectedGraph | PathfindingError::InvalidGraph(_) => "graph_error",
            PathfindingError::CacheError(_) => "cache",
            PathfindingError::UnsuitableAlgorithm(_) => "algorithm",
            PathfindingError::SmoothingFailed(_) => "smoothing",
            PathfindingError::HierarchicalError(_) => "hierarchical",
            PathfindingError::Internal(_) => "internal",
        }
    }
}

/// Result type for pathfinding operations
pub type PathfindingResult<T> = Result<T, PathfindingError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_display() {
        let err = PathfindingError::NoPathFound;
        assert!(err.to_string().contains("No path found"));
    }
    
    #[test]
    fn test_error_categories() {
        assert_eq!(PathfindingError::NoPathFound.category(), "path_not_found");
        assert_eq!(PathfindingError::InvalidStart.category(), "invalid_position");
        assert_eq!(PathfindingError::OutOfMemory.category(), "memory");
    }
    
    #[test]
    fn test_recoverable() {
        assert!(PathfindingError::NoPathFound.is_recoverable());
        assert!(!PathfindingError::InvalidStart.is_recoverable());
        assert!(!PathfindingError::OutOfMemory.is_recoverable());
    }
}
