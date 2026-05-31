//! View definitions

/// Available views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Main dashboard with overview
    Dashboard,
    /// List of all simulations
    Simulations,
    /// Detail view for a specific simulation
    SimulationDetail,
    /// List of all agents
    Agents,
    /// Agent inspector view
    AgentInspector,
    /// Space visualization
    Spaces,
    /// Log viewer
    Logs,
    /// Metrics dashboard
    Metrics,
    /// Application settings
    Settings,
    /// Help and keyboard shortcuts
    Help,
}

impl View {
    /// Get the view title
    pub fn title(&self) -> &str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Simulations => "Simulations",
            Self::SimulationDetail => "Simulation Detail",
            Self::Agents => "Agents",
            Self::AgentInspector => "Agent Inspector",
            Self::Spaces => "Space Visualization",
            Self::Logs => "Logs",
            Self::Metrics => "Metrics",
            Self::Settings => "Settings",
            Self::Help => "Help",
        }
    }

    /// Get the view icon (for tab display)
    pub fn icon(&self) -> &str {
        match self {
            Self::Dashboard => "󰕮",
            Self::Simulations => "󰐊",
            Self::SimulationDetail => "󰈙",
            Self::Agents => "󰀄",
            Self::AgentInspector => "󰍉",
            Self::Spaces => "󰕰",
            Self::Logs => "󰷐",
            Self::Metrics => "󰄪",
            Self::Settings => "󰒓",
            Self::Help => "󰋖",
        }
    }

    /// Get all navigable views (for tab bar)
    pub fn all_navigable() -> Vec<Self> {
        vec![
            Self::Dashboard,
            Self::Simulations,
            Self::Agents,
            Self::Spaces,
            Self::Logs,
            Self::Metrics,
        ]
    }
}
