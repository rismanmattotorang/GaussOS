//! Application actions and commands

use super::state::{MetricsSnapshot, Notification, Simulation};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Actions that can be triggered in the application
#[derive(Debug, Clone)]
pub enum Action {
    /// Periodic tick
    Tick,
    /// Quit the application
    Quit,
    /// Refresh data from API
    Refresh,
    /// Error occurred
    Error(String),
    /// Show notification
    Notification(Notification),
    /// Simulation updated
    SimulationUpdated(Simulation),
    /// Metrics updated
    MetricsUpdated(MetricsSnapshot),
}

/// Commands available in the command palette
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Quit,
    GoToDashboard,
    GoToSimulations,
    GoToAgents,
    GoToSpaces,
    GoToLogs,
    GoToMetrics,
    GoToSettings,
    GoToHelp,
    RefreshData,
    StartSimulation,
    StopSimulation,
    PauseSimulation,
    NewSimulation,
    ToggleTheme,
    ClearLogs,
}

impl Command {
    /// Get all available commands
    pub fn all() -> Vec<Self> {
        vec![
            Self::Quit,
            Self::GoToDashboard,
            Self::GoToSimulations,
            Self::GoToAgents,
            Self::GoToSpaces,
            Self::GoToLogs,
            Self::GoToMetrics,
            Self::GoToSettings,
            Self::GoToHelp,
            Self::RefreshData,
            Self::StartSimulation,
            Self::StopSimulation,
            Self::PauseSimulation,
            Self::NewSimulation,
            Self::ToggleTheme,
            Self::ClearLogs,
        ]
    }

    /// Get command name
    pub fn name(&self) -> &str {
        match self {
            Self::Quit => "Quit",
            Self::GoToDashboard => "Go to Dashboard",
            Self::GoToSimulations => "Go to Simulations",
            Self::GoToAgents => "Go to Agents",
            Self::GoToSpaces => "Go to Spaces",
            Self::GoToLogs => "Go to Logs",
            Self::GoToMetrics => "Go to Metrics",
            Self::GoToSettings => "Go to Settings",
            Self::GoToHelp => "Go to Help",
            Self::RefreshData => "Refresh Data",
            Self::StartSimulation => "Start Simulation",
            Self::StopSimulation => "Stop Simulation",
            Self::PauseSimulation => "Pause Simulation",
            Self::NewSimulation => "New Simulation",
            Self::ToggleTheme => "Toggle Theme",
            Self::ClearLogs => "Clear Logs",
        }
    }

    /// Get command description
    pub fn description(&self) -> &str {
        match self {
            Self::Quit => "Exit the application",
            Self::GoToDashboard => "Navigate to the dashboard view",
            Self::GoToSimulations => "Navigate to the simulations list",
            Self::GoToAgents => "Navigate to the agents list",
            Self::GoToSpaces => "Navigate to the space visualization",
            Self::GoToLogs => "Navigate to the log viewer",
            Self::GoToMetrics => "Navigate to the metrics dashboard",
            Self::GoToSettings => "Navigate to application settings",
            Self::GoToHelp => "Show help and keyboard shortcuts",
            Self::RefreshData => "Refresh all data from the API",
            Self::StartSimulation => "Start the selected simulation",
            Self::StopSimulation => "Stop the selected simulation",
            Self::PauseSimulation => "Pause the selected simulation",
            Self::NewSimulation => "Create a new simulation",
            Self::ToggleTheme => "Switch between light and dark themes",
            Self::ClearLogs => "Clear all log entries",
        }
    }

    /// Get keyboard shortcut (if any)
    pub fn shortcut(&self) -> Option<&str> {
        match self {
            Self::Quit => Some("Ctrl+Q"),
            Self::GoToDashboard => Some("1"),
            Self::GoToSimulations => Some("2"),
            Self::GoToAgents => Some("3"),
            Self::GoToSpaces => Some("4"),
            Self::GoToLogs => Some("5"),
            Self::GoToMetrics => Some("6"),
            Self::GoToSettings => Some("0"),
            Self::GoToHelp => Some("?"),
            Self::RefreshData => Some("r"),
            Self::StartSimulation => Some("s"),
            Self::StopSimulation => Some("x"),
            Self::PauseSimulation => Some("p"),
            Self::NewSimulation => Some("n"),
            _ => None,
        }
    }
}

/// Command palette state
pub struct CommandPaletteState {
    /// Whether the palette is active
    pub active: bool,
    /// Current input
    pub input: String,
    /// Filtered commands
    pub filtered: Vec<Command>,
    /// Selected index
    pub selected: usize,
    /// Fuzzy matcher
    matcher: SkimMatcherV2,
}

impl Default for CommandPaletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CommandPaletteState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandPaletteState")
            .field("active", &self.active)
            .field("input", &self.input)
            .field("filtered", &self.filtered)
            .field("selected", &self.selected)
            .finish()
    }
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self {
            active: false,
            input: String::new(),
            filtered: Command::all(),
            selected: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Filter commands based on input
    pub fn filter_commands(&mut self) {
        if self.input.is_empty() {
            self.filtered = Command::all();
        } else {
            let mut scored: Vec<(Command, i64)> = Command::all()
                .into_iter()
                .filter_map(|cmd| {
                    self.matcher
                        .fuzzy_match(cmd.name(), &self.input)
                        .map(|score| (cmd, score))
                })
                .collect();

            scored.sort_by(|a, b| b.1.cmp(&a.1));
            self.filtered = scored.into_iter().map(|(cmd, _)| cmd).collect();
        }

        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = 0;
        }
    }

    /// Select next command
    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    /// Select previous command
    pub fn select_previous(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.filtered.len() - 1);
        }
    }

    /// Get the currently selected command
    pub fn get_selected_command(&self) -> Option<Command> {
        self.filtered.get(self.selected).copied()
    }
}
