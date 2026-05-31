//! GaussTwin Terminal User Interface
//!
//! A high-performance terminal UI for managing and monitoring GaussTwin simulations.

mod api;
mod app;
mod handlers;
mod ui;
mod utils;
mod views;
mod widgets;

pub use api::ApiClient;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use app::{App, AppConfig};

/// GaussTwin Terminal User Interface
#[derive(Parser, Debug)]
#[command(name = "gausstwin-tui")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// API server URL
    #[arg(short, long, env = "GAUSSTWIN_API_URL", default_value = "http://localhost:8080")]
    api_url: String,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, env = "GAUSSTWIN_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Enable mouse support
    #[arg(short, long, default_value = "true")]
    mouse: bool,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,

    /// Theme (dark, light, gruvbox, nord, tokyo-night)
    #[arg(short, long, default_value = "tokyo-night")]
    theme: String,

    /// Tick rate in milliseconds
    #[arg(long, default_value = "250")]
    tick_rate: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli.log_level)?;
    info!("Starting GaussTwin TUI v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = AppConfig {
        api_url: cli.api_url,
        theme: cli.theme,
        mouse_enabled: cli.mouse,
        tick_rate: cli.tick_rate,
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run application
    let mut app = App::new(config).await?;
    let result = app.run(&mut terminal).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(e) = result {
        eprintln!("Application error: {}", e);
        std::process::exit(1);
    }

    info!("GaussTwin TUI shutdown complete");
    Ok(())
}

fn init_logging(level: &str) -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("gausstwin_tui={}", level).into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tui_logger::tracing_subscriber_layer())
        .init();

    tui_logger::init_logger(log::LevelFilter::Trace)?;
    tui_logger::set_default_level(match level {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    });

    Ok(())
}
