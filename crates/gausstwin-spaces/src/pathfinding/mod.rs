//! High-performance pathfinding module with multiple algorithms and optimizations.

pub mod algorithms;
pub mod cache;
pub mod enhanced;
pub mod error;
pub mod traits;

use nalgebra::Point3;

/// Re-exports of common types
pub type Point = Point3<f64>;
pub type Path = Vec<Point>;
pub type Cost = f64;

pub use algorithms::*;
pub use cache::*;
pub use enhanced::*;
pub use error::*;
pub use traits::*; 