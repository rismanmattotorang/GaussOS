//! GaussOS TUI Admin Application
//! A comprehensive terminal-based administration tool built with ratatui
//!
//! Features:
//! - Real-time system monitoring dashboard
//! - Memory browser and manager
//! - Agent orchestration control
//! - Configuration management
//! - Log viewer with filtering
//! - Query REPL interface

use gaussos::tui::{App, run_app};
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to file (not terminal)
    let log_file = std::fs::File::create("logs/tui.log")?;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_ansi(false)
        .init();

    // Setup terminal
    let terminal = ratatui::init();
    
    // Create application
    let app = App::new().await?;
    
    // Run the application
    let result = run_app(terminal, app).await;
    
    // Restore terminal
    ratatui::restore();
    
    if let Err(e) = result {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}
