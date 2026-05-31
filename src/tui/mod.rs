//! GaussOS TUI Module
//! Comprehensive terminal-based administration interface
//!
//! This module provides a full-featured TUI for managing GaussOS:
//! - Real-time dashboard with system metrics
//! - Memory browser and search
//! - Agent management and monitoring
//! - Configuration editor
//! - Log viewer
//! - Query interface

#[cfg(feature = "tui")]
mod app;
#[cfg(feature = "tui")]
mod dashboard;
#[cfg(feature = "tui")]
mod memory_browser;
#[cfg(feature = "tui")]
mod agent_manager;
#[cfg(feature = "tui")]
mod log_viewer;
#[cfg(feature = "tui")]
mod config_editor;
#[cfg(feature = "tui")]
mod query_repl;
#[cfg(feature = "tui")]
mod widgets;
#[cfg(feature = "tui")]
mod theme;

#[cfg(feature = "tui")]
pub use app::{App, run_app};
#[cfg(feature = "tui")]
pub use theme::Theme;
