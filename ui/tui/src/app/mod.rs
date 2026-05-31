//! Application state and main loop

mod state;
mod config;
mod actions;

pub use state::*;
pub use config::*;
pub use actions::*;

use crate::handlers::EventHandler;
use crate::ui;
use crate::views::View;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::prelude::*;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Main application state
pub struct App {
    /// Application configuration
    pub config: AppConfig,
    /// Current view
    pub current_view: View,
    /// Application state
    pub state: AppState,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Command palette state
    pub command_palette: CommandPaletteState,
    /// Help overlay visible
    pub show_help: bool,
    /// Last tick timestamp
    last_tick: Instant,
    /// Event handler
    event_handler: EventHandler,
    /// Action sender
    action_tx: mpsc::UnboundedSender<Action>,
    /// Action receiver
    action_rx: mpsc::UnboundedReceiver<Action>,
}

impl App {
    /// Create a new application instance
    pub async fn new(config: AppConfig) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let event_handler = EventHandler::new(config.tick_rate, action_tx.clone());

        let state = AppState::new(&config).await?;

        Ok(Self {
            config,
            current_view: View::Dashboard,
            state,
            should_quit: false,
            command_palette: CommandPaletteState::default(),
            show_help: false,
            last_tick: Instant::now(),
            event_handler,
            action_tx,
            action_rx,
        })
    }

    /// Run the main application loop
    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        info!("Starting main application loop");

        // Start the event handler
        self.event_handler.start();

        loop {
            // Draw the UI
            terminal.draw(|frame| self.draw(frame))?;

            // Handle actions
            while let Ok(action) = self.action_rx.try_recv() {
                self.handle_action(action).await?;
            }

            // Check for events with timeout
            if event::poll(Duration::from_millis(self.config.tick_rate))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key).await?;
                }
            }

            // Periodic updates
            if self.last_tick.elapsed() >= Duration::from_millis(self.config.tick_rate) {
                self.tick().await?;
                self.last_tick = Instant::now();
            }

            // Check if we should quit
            if self.should_quit {
                break;
            }
        }

        info!("Exiting main application loop");
        Ok(())
    }

    /// Draw the current view
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Draw the main view
        match self.current_view {
            View::Dashboard => ui::draw_dashboard(frame, area, &mut self.state),
            View::Simulations => ui::draw_simulations(frame, area, &mut self.state),
            View::SimulationDetail => ui::draw_simulation_detail(frame, area, &mut self.state),
            View::Agents => ui::draw_agents(frame, area, &mut self.state),
            View::AgentInspector => ui::draw_agent_inspector(frame, area, &mut self.state),
            View::Spaces => ui::draw_spaces(frame, area, &mut self.state),
            View::Logs => ui::draw_logs(frame, area, &mut self.state),
            View::Metrics => ui::draw_metrics(frame, area, &mut self.state),
            View::Settings => ui::draw_settings(frame, area, &mut self.state),
            View::Help => ui::draw_help(frame, area, &self.state),
        }

        // Draw command palette if active
        if self.command_palette.active {
            ui::draw_command_palette(frame, area, &mut self.command_palette);
        }

        // Draw help overlay if visible
        if self.show_help && !matches!(self.current_view, View::Help) {
            ui::draw_help_overlay(frame, area);
        }
    }

    /// Handle keyboard events
    async fn handle_key_event(&mut self, key: event::KeyEvent) -> Result<()> {
        // Command palette takes priority
        if self.command_palette.active {
            return self.handle_command_palette_key(key).await;
        }

        // Global shortcuts
        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) |
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            // Command palette
            (KeyModifiers::CONTROL, KeyCode::Char('p')) |
            (KeyModifiers::CONTROL | KeyModifiers::SHIFT, KeyCode::Char('P')) => {
                self.command_palette.active = true;
            }
            // Help
            (KeyModifiers::NONE, KeyCode::Char('?')) |
            (KeyModifiers::NONE, KeyCode::F(1)) => {
                self.show_help = !self.show_help;
            }
            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('1')) => self.current_view = View::Dashboard,
            (KeyModifiers::NONE, KeyCode::Char('2')) => self.current_view = View::Simulations,
            (KeyModifiers::NONE, KeyCode::Char('3')) => self.current_view = View::Agents,
            (KeyModifiers::NONE, KeyCode::Char('4')) => self.current_view = View::Spaces,
            (KeyModifiers::NONE, KeyCode::Char('5')) => self.current_view = View::Logs,
            (KeyModifiers::NONE, KeyCode::Char('6')) => self.current_view = View::Metrics,
            (KeyModifiers::NONE, KeyCode::Char('0')) => self.current_view = View::Settings,
            // Escape closes help or goes back
            (KeyModifiers::NONE, KeyCode::Esc) => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.go_back();
                }
            }
            // View-specific handling
            _ => {
                self.handle_view_key_event(key).await?;
            }
        }

        Ok(())
    }

    /// Handle view-specific key events
    async fn handle_view_key_event(&mut self, key: event::KeyEvent) -> Result<()> {
        match self.current_view {
            View::Dashboard => self.handle_dashboard_key(key).await,
            View::Simulations => self.handle_simulations_key(key).await,
            View::SimulationDetail => self.handle_simulation_detail_key(key).await,
            View::Agents => self.handle_agents_key(key).await,
            View::AgentInspector => self.handle_agent_inspector_key(key).await,
            View::Spaces => self.handle_spaces_key(key).await,
            View::Logs => self.handle_logs_key(key).await,
            View::Metrics => self.handle_metrics_key(key).await,
            View::Settings => self.handle_settings_key(key).await,
            View::Help => self.handle_help_key(key).await,
        }
    }

    /// Handle command palette key events
    async fn handle_command_palette_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.command_palette.active = false;
                self.command_palette.input.clear();
                self.command_palette.selected = 0;
            }
            KeyCode::Enter => {
                if let Some(cmd) = self.command_palette.get_selected_command() {
                    self.execute_command(cmd).await?;
                }
                self.command_palette.active = false;
                self.command_palette.input.clear();
            }
            KeyCode::Backspace => {
                self.command_palette.input.pop();
                self.command_palette.filter_commands();
            }
            KeyCode::Char(c) => {
                self.command_palette.input.push(c);
                self.command_palette.filter_commands();
            }
            KeyCode::Up => {
                self.command_palette.select_previous();
            }
            KeyCode::Down => {
                self.command_palette.select_next();
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute a command from the palette
    async fn execute_command(&mut self, command: Command) -> Result<()> {
        debug!("Executing command: {:?}", command);
        match command {
            Command::Quit => self.should_quit = true,
            Command::GoToDashboard => self.current_view = View::Dashboard,
            Command::GoToSimulations => self.current_view = View::Simulations,
            Command::GoToAgents => self.current_view = View::Agents,
            Command::GoToSpaces => self.current_view = View::Spaces,
            Command::GoToLogs => self.current_view = View::Logs,
            Command::GoToMetrics => self.current_view = View::Metrics,
            Command::GoToSettings => self.current_view = View::Settings,
            Command::GoToHelp => self.current_view = View::Help,
            Command::RefreshData => self.refresh_data().await?,
            Command::StartSimulation => self.start_selected_simulation().await?,
            Command::StopSimulation => self.stop_selected_simulation().await?,
            Command::PauseSimulation => self.pause_selected_simulation().await?,
            Command::NewSimulation => self.create_new_simulation().await?,
            Command::ToggleTheme => self.toggle_theme(),
            Command::ClearLogs => self.state.logs.clear(),
        }
        Ok(())
    }

    /// Handle an action from the event handler
    async fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Tick => self.tick().await?,
            Action::Quit => self.should_quit = true,
            Action::Refresh => self.refresh_data().await?,
            Action::Error(msg) => {
                error!("Error: {}", msg);
                self.state.notifications.push(Notification::error(msg));
            }
            Action::Notification(notif) => {
                self.state.notifications.push(notif);
            }
            Action::SimulationUpdated(sim) => {
                self.state.update_simulation(sim);
            }
            Action::MetricsUpdated(metrics) => {
                self.state.update_metrics(metrics);
            }
        }
        Ok(())
    }

    /// Periodic tick for updates
    async fn tick(&mut self) -> Result<()> {
        // Update metrics history
        self.state.metrics.tick();

        // Remove expired notifications
        self.state.notifications.retain(|n| !n.is_expired());

        // Fetch latest data if needed
        if self.state.should_refresh() {
            self.refresh_data().await?;
        }

        Ok(())
    }

    /// Refresh all data from the API
    async fn refresh_data(&mut self) -> Result<()> {
        debug!("Refreshing data from API");
        // Implementation would fetch from API
        self.state.mark_refreshed();
        Ok(())
    }

    /// Go back to previous view
    fn go_back(&mut self) {
        self.current_view = match self.current_view {
            View::SimulationDetail => View::Simulations,
            View::AgentInspector => View::Agents,
            _ => View::Dashboard,
        };
    }

    /// Toggle theme between dark and light
    fn toggle_theme(&mut self) {
        self.state.theme = match self.state.theme.as_str() {
            "dark" => "light".to_string(),
            "light" => "tokyo-night".to_string(),
            "tokyo-night" => "gruvbox".to_string(),
            "gruvbox" => "nord".to_string(),
            _ => "dark".to_string(),
        };
    }

    // Placeholder methods for simulation control
    async fn start_selected_simulation(&mut self) -> Result<()> {
        if let Some(sim) = self.state.selected_simulation() {
            info!("Starting simulation: {}", sim.id);
        }
        Ok(())
    }

    async fn stop_selected_simulation(&mut self) -> Result<()> {
        if let Some(sim) = self.state.selected_simulation() {
            info!("Stopping simulation: {}", sim.id);
        }
        Ok(())
    }

    async fn pause_selected_simulation(&mut self) -> Result<()> {
        if let Some(sim) = self.state.selected_simulation() {
            info!("Pausing simulation: {}", sim.id);
        }
        Ok(())
    }

    async fn create_new_simulation(&mut self) -> Result<()> {
        info!("Creating new simulation");
        Ok(())
    }

    // View-specific key handlers
    async fn handle_dashboard_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('r') => self.refresh_data().await?,
            KeyCode::Enter => {
                if self.state.simulations.selected().is_some() {
                    self.current_view = View::SimulationDetail;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.state.simulations.next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.simulations.previous(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_simulations_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if self.state.simulations.selected().is_some() {
                    self.current_view = View::SimulationDetail;
                }
            }
            KeyCode::Char('n') => self.create_new_simulation().await?,
            KeyCode::Char('s') => self.start_selected_simulation().await?,
            KeyCode::Char('x') => self.stop_selected_simulation().await?,
            KeyCode::Char('p') => self.pause_selected_simulation().await?,
            KeyCode::Char('d') => {
                // Delete selected simulation
            }
            KeyCode::Down | KeyCode::Char('j') => self.state.simulations.next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.simulations.previous(),
            KeyCode::Char('/') => {
                // Enter search mode
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_simulation_detail_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('s') => self.start_selected_simulation().await?,
            KeyCode::Char('x') => self.stop_selected_simulation().await?,
            KeyCode::Char('p') => self.pause_selected_simulation().await?,
            KeyCode::Tab => self.state.detail_tab.next(),
            KeyCode::BackTab => self.state.detail_tab.previous(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_agents_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if self.state.agents.selected().is_some() {
                    self.current_view = View::AgentInspector;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => self.state.agents.next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.agents.previous(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_agent_inspector_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => self.state.agent_tab.next(),
            KeyCode::BackTab => self.state.agent_tab.previous(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_spaces_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            // Arrow keys for panning
            KeyCode::Left | KeyCode::Char('h') => self.state.space_view.pan_left(),
            KeyCode::Right | KeyCode::Char('l') => self.state.space_view.pan_right(),
            KeyCode::Up | KeyCode::Char('k') => self.state.space_view.pan_up(),
            KeyCode::Down | KeyCode::Char('j') => self.state.space_view.pan_down(),
            // Zoom
            KeyCode::Char('+') | KeyCode::Char('=') => self.state.space_view.zoom_in(),
            KeyCode::Char('-') => self.state.space_view.zoom_out(),
            KeyCode::Char('0') => self.state.space_view.reset_zoom(),
            _ => {}
        }
        Ok(())
    }

    async fn handle_logs_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.state.log_scroll.scroll_down(),
            KeyCode::Up | KeyCode::Char('k') => self.state.log_scroll.scroll_up(),
            KeyCode::PageDown => self.state.log_scroll.page_down(),
            KeyCode::PageUp => self.state.log_scroll.page_up(),
            KeyCode::Home => self.state.log_scroll.scroll_to_top(),
            KeyCode::End => self.state.log_scroll.scroll_to_bottom(),
            KeyCode::Char('c') => self.state.logs.clear(),
            KeyCode::Char('/') => {
                // Enter search mode
            }
            KeyCode::Char('f') => {
                // Cycle through log level filters
                self.state.log_filter.cycle();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_metrics_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Tab => self.state.metrics_tab.next(),
            KeyCode::BackTab => self.state.metrics_tab.previous(),
            KeyCode::Char('r') => self.refresh_data().await?,
            _ => {}
        }
        Ok(())
    }

    async fn handle_settings_key(&mut self, key: event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => self.state.settings_list.next(),
            KeyCode::Up | KeyCode::Char('k') => self.state.settings_list.previous(),
            KeyCode::Enter => {
                // Edit selected setting
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help_key(&mut self, _key: event::KeyEvent) -> Result<()> {
        // Help view - any key returns to previous view
        Ok(())
    }
}
