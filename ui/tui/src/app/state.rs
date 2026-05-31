//! Application state management

use super::AppConfig;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Application state
pub struct AppState {
    /// Theme name
    pub theme: String,
    /// Simulations list
    pub simulations: StatefulList<Simulation>,
    /// Agents list
    pub agents: StatefulList<Agent>,
    /// Logs
    pub logs: VecDeque<LogEntry>,
    /// System metrics
    pub metrics: Metrics,
    /// Notifications
    pub notifications: Vec<Notification>,
    /// Currently selected simulation (for detail view)
    pub selected_simulation_id: Option<String>,
    /// Detail view tab
    pub detail_tab: TabState,
    /// Agent inspector tab
    pub agent_tab: TabState,
    /// Metrics view tab
    pub metrics_tab: TabState,
    /// Space view state
    pub space_view: SpaceViewState,
    /// Log scroll state
    pub log_scroll: ScrollState,
    /// Log level filter
    pub log_filter: LogFilter,
    /// Settings list
    pub settings_list: StatefulList<SettingItem>,
    /// Last refresh time
    last_refresh: Instant,
    /// Refresh interval
    refresh_interval: Duration,
}

impl AppState {
    /// Create new application state
    pub async fn new(config: &AppConfig) -> Result<Self> {
        Ok(Self {
            theme: config.theme.clone(),
            simulations: StatefulList::with_items(Self::load_sample_simulations()),
            agents: StatefulList::with_items(Self::load_sample_agents()),
            logs: VecDeque::with_capacity(1000),
            metrics: Metrics::default(),
            notifications: Vec::new(),
            selected_simulation_id: None,
            detail_tab: TabState::new(vec!["Overview", "Agents", "Events", "Config"]),
            agent_tab: TabState::new(vec!["State", "Memory", "Actions", "Messages"]),
            metrics_tab: TabState::new(vec!["System", "Simulation", "Agents", "Network"]),
            space_view: SpaceViewState::default(),
            log_scroll: ScrollState::default(),
            log_filter: LogFilter::default(),
            settings_list: StatefulList::with_items(Self::load_settings()),
            last_refresh: Instant::now(),
            refresh_interval: Duration::from_secs(5),
        })
    }

    /// Load sample simulations for demo
    fn load_sample_simulations() -> Vec<Simulation> {
        vec![
            Simulation {
                id: "sim-001".to_string(),
                name: "Traffic Flow Optimization".to_string(),
                description: "Urban traffic simulation with 10,000 vehicles".to_string(),
                status: SimulationStatus::Running,
                agent_count: 10000,
                current_step: 15234,
                total_steps: 100000,
                progress: 0.15,
                created_at: Utc::now() - chrono::Duration::hours(2),
                updated_at: Utc::now(),
            },
            Simulation {
                id: "sim-002".to_string(),
                name: "Factory Production Line".to_string(),
                description: "Manufacturing digital twin with 500 robots".to_string(),
                status: SimulationStatus::Paused,
                agent_count: 500,
                current_step: 45000,
                total_steps: 50000,
                progress: 0.90,
                created_at: Utc::now() - chrono::Duration::days(1),
                updated_at: Utc::now() - chrono::Duration::minutes(30),
            },
            Simulation {
                id: "sim-003".to_string(),
                name: "Supply Chain Network".to_string(),
                description: "Global supply chain with 2,500 nodes".to_string(),
                status: SimulationStatus::Stopped,
                agent_count: 2500,
                current_step: 0,
                total_steps: 200000,
                progress: 0.0,
                created_at: Utc::now() - chrono::Duration::days(7),
                updated_at: Utc::now() - chrono::Duration::days(2),
            },
            Simulation {
                id: "sim-004".to_string(),
                name: "Smart Grid Energy".to_string(),
                description: "Energy distribution simulation".to_string(),
                status: SimulationStatus::Running,
                agent_count: 1200,
                current_step: 78500,
                total_steps: 100000,
                progress: 0.785,
                created_at: Utc::now() - chrono::Duration::hours(6),
                updated_at: Utc::now(),
            },
        ]
    }

    /// Load sample agents for demo
    fn load_sample_agents() -> Vec<Agent> {
        vec![
            Agent {
                id: "agent-001".to_string(),
                name: "Vehicle #1".to_string(),
                agent_type: "Vehicle".to_string(),
                status: AgentStatus::Active,
                position: (45.5, 23.1),
                memory_usage: 1024,
                messages_sent: 523,
                messages_received: 412,
            },
            Agent {
                id: "agent-002".to_string(),
                name: "Vehicle #2".to_string(),
                agent_type: "Vehicle".to_string(),
                status: AgentStatus::Active,
                position: (12.3, 67.8),
                memory_usage: 890,
                messages_sent: 234,
                messages_received: 198,
            },
            Agent {
                id: "agent-003".to_string(),
                name: "Traffic Light #1".to_string(),
                agent_type: "TrafficLight".to_string(),
                status: AgentStatus::Active,
                position: (50.0, 50.0),
                memory_usage: 256,
                messages_sent: 1205,
                messages_received: 0,
            },
            Agent {
                id: "agent-004".to_string(),
                name: "Sensor #1".to_string(),
                agent_type: "Sensor".to_string(),
                status: AgentStatus::Idle,
                position: (75.2, 30.5),
                memory_usage: 128,
                messages_sent: 8920,
                messages_received: 45,
            },
        ]
    }

    /// Load settings items
    fn load_settings() -> Vec<SettingItem> {
        vec![
            SettingItem::new("theme", "Theme", "tokyo-night", SettingType::Select(vec![
                "dark".to_string(), "light".to_string(), "tokyo-night".to_string(), 
                "gruvbox".to_string(), "nord".to_string()
            ])),
            SettingItem::new("api_url", "API URL", "http://localhost:8080", SettingType::Text),
            SettingItem::new("refresh_interval", "Refresh Interval (s)", "5", SettingType::Number),
            SettingItem::new("mouse_enabled", "Mouse Support", "true", SettingType::Toggle),
            SettingItem::new("log_level", "Log Level", "info", SettingType::Select(vec![
                "trace".to_string(), "debug".to_string(), "info".to_string(),
                "warn".to_string(), "error".to_string()
            ])),
            SettingItem::new("max_logs", "Max Log Entries", "1000", SettingType::Number),
            SettingItem::new("auto_scroll_logs", "Auto-scroll Logs", "true", SettingType::Toggle),
        ]
    }

    /// Check if data should be refreshed
    pub fn should_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    /// Mark data as refreshed
    pub fn mark_refreshed(&mut self) {
        self.last_refresh = Instant::now();
    }

    /// Get selected simulation
    pub fn selected_simulation(&self) -> Option<&Simulation> {
        self.simulations.selected()
    }

    /// Update a simulation in the list
    pub fn update_simulation(&mut self, updated: Simulation) {
        if let Some(sim) = self.simulations.items.iter_mut().find(|s| s.id == updated.id) {
            *sim = updated;
        }
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: MetricsSnapshot) {
        self.metrics.add_snapshot(metrics);
    }
}

/// Stateful list for UI components
pub struct StatefulList<T> {
    pub items: Vec<T>,
    pub state: usize,
}

impl<T> StatefulList<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            state: 0,
        }
    }

    pub fn with_items(items: Vec<T>) -> Self {
        Self { items, state: 0 }
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.state = (self.state + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            self.state = self.state.checked_sub(1).unwrap_or(self.items.len() - 1);
        }
    }

    pub fn selected(&self) -> Option<&T> {
        self.items.get(self.state)
    }

    pub fn selected_index(&self) -> usize {
        self.state
    }
}

impl<T> Default for StatefulList<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Simulation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: SimulationStatus,
    pub agent_count: u64,
    pub current_step: u64,
    pub total_steps: u64,
    pub progress: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Simulation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulationStatus {
    Running,
    Paused,
    Stopped,
    Completed,
    Error,
}

impl SimulationStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Stopped => "Stopped",
            Self::Completed => "Completed",
            Self::Error => "Error",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Running => Color::Green,
            Self::Paused => Color::Yellow,
            Self::Stopped => Color::Gray,
            Self::Completed => Color::Blue,
            Self::Error => Color::Red,
        }
    }
}

/// Agent data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub status: AgentStatus,
    pub position: (f64, f64),
    pub memory_usage: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Idle,
    Blocked,
    Error,
}

impl AgentStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Active => "Active",
            Self::Idle => "Idle",
            Self::Blocked => "Blocked",
            Self::Error => "Error",
        }
    }
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub target: String,
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Trace => Color::DarkGray,
            Self::Debug => Color::Cyan,
            Self::Info => Color::Green,
            Self::Warn => Color::Yellow,
            Self::Error => Color::Red,
        }
    }
}

/// Log filter
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub min_level: Option<LogLevel>,
    pub search: String,
}

impl LogFilter {
    pub fn cycle(&mut self) {
        self.min_level = match self.min_level {
            None => Some(LogLevel::Trace),
            Some(LogLevel::Trace) => Some(LogLevel::Debug),
            Some(LogLevel::Debug) => Some(LogLevel::Info),
            Some(LogLevel::Info) => Some(LogLevel::Warn),
            Some(LogLevel::Warn) => Some(LogLevel::Error),
            Some(LogLevel::Error) => None,
        };
    }
}

/// System metrics
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub cpu_history: VecDeque<f64>,
    pub memory_history: VecDeque<f64>,
    pub agent_count_history: VecDeque<u64>,
    pub event_rate_history: VecDeque<f64>,
    pub current: MetricsSnapshot,
}

impl Metrics {
    const MAX_HISTORY: usize = 60;

    pub fn add_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.push_bounded(&mut self.cpu_history.clone(), snapshot.cpu_usage);
        self.push_bounded(&mut self.memory_history.clone(), snapshot.memory_usage);
        self.push_bounded(&mut self.agent_count_history.clone(), snapshot.active_agents);
        self.push_bounded(&mut self.event_rate_history.clone(), snapshot.events_per_second);
        self.current = snapshot;
    }

    fn push_bounded<T>(&self, history: &mut VecDeque<T>, value: T) {
        if history.len() >= Self::MAX_HISTORY {
            history.pop_front();
        }
        history.push_back(value);
    }

    pub fn tick(&mut self) {
        // Simulate metrics updates for demo
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let cpu: f64 = (self.current.cpu_usage + rng.gen_range(-5.0..5.0)).clamp(0.0, 100.0);
        let mem: f64 = (self.current.memory_usage + rng.gen_range(-2.0..2.0)).clamp(0.0, 100.0);
        
        if self.cpu_history.len() >= Self::MAX_HISTORY {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(cpu);
        
        if self.memory_history.len() >= Self::MAX_HISTORY {
            self.memory_history.pop_front();
        }
        self.memory_history.push_back(mem);
        
        self.current.cpu_usage = cpu;
        self.current.memory_usage = mem;
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub active_agents: u64,
    pub events_per_second: f64,
    pub simulation_step: u64,
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub level: NotificationLevel,
    pub message: String,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl Notification {
    pub fn info(msg: impl Into<String>) -> Self {
        Self {
            level: NotificationLevel::Info,
            message: msg.into(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(5),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            level: NotificationLevel::Error,
            message: msg.into(),
            created_at: Instant::now(),
            ttl: Duration::from_secs(10),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

/// Tab state for tabbed views
#[derive(Debug, Clone)]
pub struct TabState {
    pub titles: Vec<&'static str>,
    pub selected: usize,
}

impl TabState {
    pub fn new(titles: Vec<&'static str>) -> Self {
        Self { titles, selected: 0 }
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.titles.len();
    }

    pub fn previous(&mut self) {
        self.selected = self.selected.checked_sub(1).unwrap_or(self.titles.len() - 1);
    }
}

/// Space visualization state
#[derive(Debug, Clone, Default)]
pub struct SpaceViewState {
    pub offset_x: i32,
    pub offset_y: i32,
    pub zoom: f32,
}

impl SpaceViewState {
    pub fn pan_left(&mut self) {
        self.offset_x -= 1;
    }

    pub fn pan_right(&mut self) {
        self.offset_x += 1;
    }

    pub fn pan_up(&mut self) {
        self.offset_y -= 1;
    }

    pub fn pan_down(&mut self) {
        self.offset_y += 1;
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.2).min(10.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.2).max(0.1);
    }

    pub fn reset_zoom(&mut self) {
        self.zoom = 1.0;
        self.offset_x = 0;
        self.offset_y = 0;
    }
}

/// Scroll state for scrollable views
#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub offset: usize,
    pub page_size: usize,
}

impl ScrollState {
    pub fn scroll_down(&mut self) {
        self.offset = self.offset.saturating_add(1);
    }

    pub fn scroll_up(&mut self) {
        self.offset = self.offset.saturating_sub(1);
    }

    pub fn page_down(&mut self) {
        self.offset = self.offset.saturating_add(self.page_size);
    }

    pub fn page_up(&mut self) {
        self.offset = self.offset.saturating_sub(self.page_size);
    }

    pub fn scroll_to_top(&mut self) {
        self.offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        // Would need total count to properly implement
    }
}

/// Setting item
#[derive(Debug, Clone)]
pub struct SettingItem {
    pub key: String,
    pub label: String,
    pub value: String,
    pub setting_type: SettingType,
}

impl SettingItem {
    pub fn new(key: &str, label: &str, value: &str, setting_type: SettingType) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            setting_type,
        }
    }
}

/// Setting type
#[derive(Debug, Clone)]
pub enum SettingType {
    Text,
    Number,
    Toggle,
    Select(Vec<String>),
}
