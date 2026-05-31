//! Main TUI Application
//! Handles the core application state, event loop, and navigation
//! Enhanced with real server integration and comprehensive error handling

use crate::error::{GaussOSError, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Tabs, Widget, Wrap,
    },
    DefaultTerminal, Frame,
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use chrono::Local;

use super::theme::Theme;

/// Server client for backend communication
pub struct ServerClient {
    base_url: String,
    client: reqwest::Client,
}

impl ServerClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        match self.client.get(format!("{}/health", self.base_url)).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn get_metrics(&self) -> Result<ServerMetrics> {
        let resp = self.client
            .get(format!("{}/metrics", self.base_url))
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;
        
        resp.json().await.map_err(|e| GaussOSError::SerializationError(e.to_string()))
    }

    pub async fn get_memories(&self, limit: u64) -> Result<Vec<MemoryItem>> {
        let resp = self.client
            .get(format!("{}/api/v1/memories?limit={}", self.base_url, limit))
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;
        
        let data: serde_json::Value = resp.json().await
            .map_err(|e| GaussOSError::SerializationError(e.to_string()))?;
        
        // Parse memories from response
        if let Some(memories) = data.get("memories").and_then(|m| m.as_array()) {
            Ok(memories.iter().filter_map(|m| {
                Some(MemoryItem {
                    id: m.get("id")?.as_str()?.to_string(),
                    name: m.get("name").and_then(|n| n.as_str()).unwrap_or("Unnamed").to_string(),
                    memory_type: m.get("type").and_then(|t| t.as_str()).unwrap_or("Unknown").to_string(),
                    namespace: m.get("namespace").and_then(|n| n.as_str()).unwrap_or("default").to_string(),
                    size_bytes: m.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0),
                    created_at: m.get("created_at").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                })
            }).collect())
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_agents(&self) -> Result<Vec<AgentItem>> {
        let resp = self.client
            .get(format!("{}/api/v1/agents", self.base_url))
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;

        resp.json().await.map_err(|e| GaussOSError::SerializationError(e.to_string()))
    }

    /// Run a full-text memory search (powers the Query REPL).
    pub async fn search(&self, text: &str) -> Result<Vec<QueryRow>> {
        let body = serde_json::json!({ "text": text, "limit": 50 });
        let resp = self.client
            .post(format!("{}/api/v1/memories/search", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;
        let data: serde_json::Value = resp.json().await
            .map_err(|e| GaussOSError::SerializationError(e.to_string()))?;
        let arr = data.get("memories").and_then(|m| m.as_array()).cloned().unwrap_or_default();
        Ok(arr.iter().map(QueryRow::from_value).collect())
    }

    /// Fetch the knowledge-graph summary: (node_count, edge_count, top entities by degree).
    pub async fn get_fact_graph(&self) -> Result<(usize, usize, Vec<(String, usize)>)> {
        let resp = self.client
            .get(format!("{}/api/v1/facts/graph", self.base_url))
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;
        let data: serde_json::Value = resp.json().await
            .map_err(|e| GaussOSError::SerializationError(e.to_string()))?;
        let nodes = data.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();
        let edges = data.get("edges").and_then(|e| e.as_array()).map(|a| a.len()).unwrap_or(0);
        let mut top: Vec<(String, usize)> = nodes.iter().filter_map(|n| {
            Some((
                n.get("id")?.as_str()?.to_string(),
                n.get("degree").and_then(|d| d.as_u64()).unwrap_or(0) as usize,
            ))
        }).collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        top.truncate(12);
        Ok((nodes.len(), edges, top))
    }

    /// Fetch the active LLM provider status: (provider, model, configured).
    pub async fn get_llm_status(&self) -> Result<(String, String, bool)> {
        let resp = self.client
            .get(format!("{}/api/v1/llm/status", self.base_url))
            .send()
            .await
            .map_err(|e| GaussOSError::NetworkError(e.to_string()))?;
        let d: serde_json::Value = resp.json().await
            .map_err(|e| GaussOSError::SerializationError(e.to_string()))?;
        Ok((
            d.get("provider").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            d.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            d.get("configured").and_then(|v| v.as_bool()).unwrap_or(false),
        ))
    }
}

/// A row of query results shown in the Query REPL.
#[derive(Debug, Clone, Default)]
pub struct QueryRow {
    pub content: String,
    pub mem_type: String,
    pub namespace: String,
    pub quality: f64,
}

impl QueryRow {
    fn from_value(m: &serde_json::Value) -> Self {
        // Extract a human-readable content preview + type from the MemCube payload.
        let (mem_type, content) = match m.get("payload") {
            Some(serde_json::Value::Object(map)) => {
                let key = map.keys().next().cloned().unwrap_or_default();
                let content = match map.get(&key) {
                    Some(serde_json::Value::String(s)) => s.clone(),
                    Some(serde_json::Value::Object(inner)) => inner
                        .get("content")
                        .or_else(|| inner.get("thread_title"))
                        .or_else(|| inner.get("prompt_name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    _ => String::new(),
                };
                (key.to_lowercase(), content)
            }
            _ => ("unknown".to_string(), String::new()),
        };
        Self {
            content,
            mem_type,
            namespace: m.get("namespace").and_then(|n| n.as_str()).unwrap_or("default").to_string(),
            quality: m
                .get("metadata")
                .and_then(|md| md.get("quality_score"))
                .and_then(|q| q.as_f64())
                .unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub cache_hit_rate: f64,
    pub operations_per_second: u64,
    pub active_queries: u64,
}

/// Application tabs/views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Dashboard,
    Memories,
    Agents,
    Graphs,
    Logs,
    Config,
    Query,
    Help,
}

impl AppTab {
    fn all() -> Vec<AppTab> {
        vec![
            AppTab::Dashboard,
            AppTab::Memories,
            AppTab::Agents,
            AppTab::Graphs,
            AppTab::Logs,
            AppTab::Config,
            AppTab::Query,
            AppTab::Help,
        ]
    }

    fn title(&self) -> &'static str {
        match self {
            AppTab::Dashboard => "󰕮 Dashboard",
            AppTab::Memories => "󰍉 Memories",
            AppTab::Agents => "󰚩 Agents",
            AppTab::Graphs => "󰈈 Graphs",
            AppTab::Logs => "󰷐 Logs",
            AppTab::Config => "󰒓 Config",
            AppTab::Query => "󰘳 Query",
            AppTab::Help => "󰋖 Help",
        }
    }

    fn index(&self) -> usize {
        match self {
            AppTab::Dashboard => 0,
            AppTab::Memories => 1,
            AppTab::Agents => 2,
            AppTab::Graphs => 3,
            AppTab::Logs => 4,
            AppTab::Config => 5,
            AppTab::Query => 6,
            AppTab::Help => 7,
        }
    }
}

/// System metrics for dashboard
#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub memory_total: u64,
    pub memory_used: u64,
    pub api_requests_per_sec: u64,
    pub active_connections: u64,
    pub total_memories: u64,
    pub active_agents: u64,
    pub cache_hit_rate: f64,
    pub uptime_seconds: u64,
}

/// Main application state
pub struct App {
    /// Current active tab
    pub current_tab: AppTab,
    
    /// Should the application quit
    pub should_quit: bool,
    
    /// Theme configuration
    pub theme: Theme,
    
    /// System metrics
    pub metrics: SystemMetrics,
    
    /// Last metrics update time
    pub last_metrics_update: Instant,
    
    /// Command palette open
    pub command_palette_open: bool,
    
    /// Command input buffer
    pub command_input: String,
    
    /// Status message
    pub status_message: Option<(String, Instant)>,
    
    /// Selected item in lists
    pub list_state: ListState,
    
    /// Memory list items
    pub memory_items: Vec<MemoryItem>,
    
    /// Agent list items
    pub agent_items: Vec<AgentItem>,
    
    /// Log entries
    pub log_entries: Vec<LogEntry>,
    
    /// Server connection status
    pub connected: bool,
    
    /// Server URL
    pub server_url: String,
    
    /// Frame counter for animations
    pub frame_count: u64,
    
    /// Server client for API calls
    server_client: ServerClient,
    
    /// Error message to display
    pub error_message: Option<(String, Instant)>,

    /// Data loading state
    pub loading: bool,

    /// Query REPL input buffer
    pub query_input: String,
    /// Query REPL results
    pub query_results: Vec<QueryRow>,
    /// A query is pending execution (processed by the async loop)
    pub query_pending: bool,
    /// Whether a query has been run this session
    pub query_ran: bool,

    /// Knowledge-graph summary (nodes, edges, top entities)
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub graph_top: Vec<(String, usize)>,

    /// Active LLM provider status (provider, model, configured)
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_configured: bool,
}

/// Memory item representation
#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub id: String,
    pub name: String,
    pub memory_type: String,
    pub namespace: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Agent item representation
#[derive(Debug, Clone, Deserialize)]
pub struct AgentItem {
    pub id: String,
    pub name: String,
    pub status: String,
    #[serde(default)]
    pub agent_type: String,
    #[serde(default)]
    pub executions: u64,
    #[serde(default)]
    pub last_activity: String,
}

/// Log entry representation
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub source: String,
}

impl App {
    /// Create a new application instance with server connection
    pub async fn new() -> Result<Self> {
        let server_url = std::env::var("GAUSSOS_SERVER_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        
        let server_client = ServerClient::new(&server_url);
        
        // Check server health
        let connected = server_client.health_check().await.unwrap_or(false);
        
        let mut app = Self {
            current_tab: AppTab::Dashboard,
            should_quit: false,
            theme: Theme::default(),
            metrics: SystemMetrics::default(),
            last_metrics_update: Instant::now(),
            command_palette_open: false,
            command_input: String::new(),
            status_message: if connected {
                Some(("Connected to server".to_string(), Instant::now()))
            } else {
                Some(("Running in offline mode".to_string(), Instant::now()))
            },
            list_state: ListState::default(),
            memory_items: Vec::new(),
            agent_items: Vec::new(),
            log_entries: vec![
                LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    level: "INFO".to_string(),
                    message: "TUI application started".to_string(),
                    source: "tui".to_string(),
                },
            ],
            connected,
            server_url,
            frame_count: 0,
            server_client,
            error_message: None,
            loading: false,
            query_input: String::new(),
            query_results: Vec::new(),
            query_pending: false,
            query_ran: false,
            graph_nodes: 0,
            graph_edges: 0,
            graph_top: Vec::new(),
            llm_provider: String::new(),
            llm_model: String::new(),
            llm_configured: false,
        };

        // Load initial data if connected
        if connected {
            app.refresh_data().await;
        } else {
            // Use demo data when offline
            app.load_demo_data();
        }
        
        Ok(app)
    }
    
    /// Load demo data for offline mode
    fn load_demo_data(&mut self) {
        self.memory_items = vec![
            MemoryItem {
                id: "mem-001".to_string(),
                name: "User Preferences".to_string(),
                memory_type: "Semantic".to_string(),
                namespace: "default".to_string(),
                size_bytes: 2048,
                created_at: "2026-01-17 10:30:00".to_string(),
            },
            MemoryItem {
                id: "mem-002".to_string(),
                name: "Chat History".to_string(),
                memory_type: "Episodic".to_string(),
                namespace: "conversations".to_string(),
                size_bytes: 15360,
                created_at: "2026-01-17 09:15:00".to_string(),
            },
        ];
        
        self.agent_items = vec![
            AgentItem {
                id: "agent-001".to_string(),
                name: "ConversationAgent".to_string(),
                status: "Active".to_string(),
                agent_type: "Conversational".to_string(),
                executions: 1542,
                last_activity: "2s ago".to_string(),
            },
            AgentItem {
                id: "agent-002".to_string(),
                name: "DataAnalyzer".to_string(),
                status: "Idle".to_string(),
                agent_type: "TaskExecutor".to_string(),
                executions: 89,
                last_activity: "5m ago".to_string(),
            },
        ];
    }
    
    /// Refresh data from server
    pub async fn refresh_data(&mut self) {
        self.loading = true;
        
        // Fetch memories
        match self.server_client.get_memories(100).await {
            Ok(memories) => self.memory_items = memories,
            Err(e) => self.add_log("ERROR", &format!("Failed to fetch memories: {}", e)),
        }
        
        // Fetch agents
        match self.server_client.get_agents().await {
            Ok(agents) => self.agent_items = agents,
            Err(e) => self.add_log("ERROR", &format!("Failed to fetch agents: {}", e)),
        }
        
        // Fetch metrics
        match self.server_client.get_metrics().await {
            Ok(metrics) => {
                self.metrics.cpu_usage = metrics.cpu_usage_percent;
                self.metrics.cache_hit_rate = metrics.cache_hit_rate;
                self.metrics.api_requests_per_sec = metrics.operations_per_second;
            }
            Err(_) => {} // Silently ignore metrics errors
        }

        // Fetch knowledge-graph summary (for the Graphs tab).
        if let Ok((nodes, edges, top)) = self.server_client.get_fact_graph().await {
            self.graph_nodes = nodes;
            self.graph_edges = edges;
            self.graph_top = top;
        }

        // Fetch active LLM provider (for the Config tab).
        if let Ok((provider, model, configured)) = self.server_client.get_llm_status().await {
            self.llm_provider = provider;
            self.llm_model = model;
            self.llm_configured = configured;
        }

        self.loading = false;
        self.add_log("INFO", "Data refreshed from server");
    }

    /// Execute the pending query against the server and store results.
    pub async fn run_query(&mut self) {
        let q = self.query_input.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.query_ran = true;
        match self.server_client.search(&q).await {
            Ok(rows) => {
                let n = rows.len();
                self.query_results = rows;
                self.status_message = Some((format!("{} result(s)", n), Instant::now()));
            }
            Err(e) => {
                self.query_results.clear();
                self.show_error(&format!("Query failed: {}", e));
            }
        }
    }
    
    /// Add a log entry
    fn add_log(&mut self, level: &str, message: &str) {
        self.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
            source: "tui".to_string(),
        });
        
        // Keep only last 500 entries
        if self.log_entries.len() > 500 {
            self.log_entries.remove(0);
        }
    }

    /// Update metrics (simulated for now)
    pub fn update_metrics(&mut self) {
        if self.last_metrics_update.elapsed() > Duration::from_secs(1) {
            // Simulate metric updates
            self.metrics.cpu_usage = 25.0 + (self.frame_count as f64 % 20.0);
            self.metrics.memory_usage = 45.0 + (self.frame_count as f64 % 10.0);
            self.metrics.memory_total = 16_000_000_000;
            self.metrics.memory_used = (self.metrics.memory_total as f64 * self.metrics.memory_usage / 100.0) as u64;
            self.metrics.api_requests_per_sec = 12000 + (self.frame_count % 2000);
            self.metrics.active_connections = 150 + (self.frame_count % 50);
            self.metrics.total_memories = 15234;
            self.metrics.active_agents = 3;
            self.metrics.cache_hit_rate = 94.0 + (self.frame_count as f64 % 5.0) / 10.0;
            self.metrics.uptime_seconds = self.frame_count / 10;
            self.last_metrics_update = Instant::now();
        }
    }

    /// Handle key events
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Handle command palette
        if self.command_palette_open {
            match key.code {
                KeyCode::Esc => {
                    self.command_palette_open = false;
                    self.command_input.clear();
                }
                KeyCode::Enter => {
                    self.execute_command();
                    self.command_palette_open = false;
                    self.command_input.clear();
                }
                KeyCode::Char(c) => {
                    self.command_input.push(c);
                }
                KeyCode::Backspace => {
                    self.command_input.pop();
                }
                _ => {}
            }
            return;
        }

        // On the Query tab, printable keys edit the query (REPL input mode).
        // Tab/BackTab/Esc and Ctrl-chords still fall through to global handling
        // so the user can navigate away or quit.
        if self.current_tab == AppTab::Query && !key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char(c) => {
                    self.query_input.push(c);
                    return;
                }
                KeyCode::Backspace => {
                    self.query_input.pop();
                    return;
                }
                KeyCode::Enter => {
                    if !self.query_input.trim().is_empty() {
                        self.query_pending = true;
                    }
                    return;
                }
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc => { /* fall through */ }
                _ => return,
            }
        }

        // Global shortcuts
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.command_palette_open = true;
            }
            KeyCode::Char('1') => self.current_tab = AppTab::Dashboard,
            KeyCode::Char('2') => self.current_tab = AppTab::Memories,
            KeyCode::Char('3') => self.current_tab = AppTab::Agents,
            KeyCode::Char('4') => self.current_tab = AppTab::Graphs,
            KeyCode::Char('5') => self.current_tab = AppTab::Logs,
            KeyCode::Char('6') => self.current_tab = AppTab::Config,
            KeyCode::Char('7') => self.current_tab = AppTab::Query,
            KeyCode::Char('?') => self.current_tab = AppTab::Help,
            KeyCode::Tab => {
                let tabs = AppTab::all();
                let current_idx = self.current_tab.index();
                let next_idx = (current_idx + 1) % tabs.len();
                self.current_tab = tabs[next_idx];
            }
            KeyCode::BackTab => {
                let tabs = AppTab::all();
                let current_idx = self.current_tab.index();
                let prev_idx = if current_idx == 0 { tabs.len() - 1 } else { current_idx - 1 };
                self.current_tab = tabs[prev_idx];
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
            }
            KeyCode::Esc => {
                self.status_message = None;
            }
            _ => {}
        }
    }

    fn select_next(&mut self) {
        let items_len = match self.current_tab {
            AppTab::Memories => self.memory_items.len(),
            AppTab::Agents => self.agent_items.len(),
            AppTab::Logs => self.log_entries.len(),
            _ => 0,
        };
        if items_len > 0 {
            let i = match self.list_state.selected() {
                Some(i) => (i + 1) % items_len,
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn select_previous(&mut self) {
        let items_len = match self.current_tab {
            AppTab::Memories => self.memory_items.len(),
            AppTab::Agents => self.agent_items.len(),
            AppTab::Logs => self.log_entries.len(),
            _ => 0,
        };
        if items_len > 0 {
            let i = match self.list_state.selected() {
                Some(i) => {
                    if i == 0 { items_len - 1 } else { i - 1 }
                }
                None => 0,
            };
            self.list_state.select(Some(i));
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_input.trim().to_lowercase();
        match cmd.as_str() {
            "quit" | "q" | "exit" => self.should_quit = true,
            "dashboard" | "d" => self.current_tab = AppTab::Dashboard,
            "memories" | "m" => self.current_tab = AppTab::Memories,
            "agents" | "a" => self.current_tab = AppTab::Agents,
            "logs" | "l" => self.current_tab = AppTab::Logs,
            "config" | "c" => self.current_tab = AppTab::Config,
            "help" | "h" | "?" => self.current_tab = AppTab::Help,
            "refresh" | "r" => {
                self.status_message = Some(("Refreshing data...".to_string(), Instant::now()));
                // Mark for refresh in next tick
                self.loading = true;
            }
            "connect" => {
                self.status_message = Some(("Reconnecting to server...".to_string(), Instant::now()));
            }
            _ => {
                self.status_message = Some((format!("Unknown command: {}", cmd), Instant::now()));
            }
        }
    }
    
    /// Show error message
    pub fn show_error(&mut self, message: &str) {
        self.error_message = Some((message.to_string(), Instant::now()));
        self.add_log("ERROR", message);
    }
}

/// Run the application main loop
pub async fn run_app(mut terminal: DefaultTerminal, mut app: App) -> Result<()> {
    let tick_rate = Duration::from_millis(100);
    let refresh_rate = Duration::from_secs(30); // Refresh data every 30 seconds
    let mut last_tick = Instant::now();
    let mut last_refresh = Instant::now();

    loop {
        // Draw the UI
        terminal.draw(|frame| ui(frame, &mut app))?;

        // Handle events with timeout
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }

        // Tick updates
        if last_tick.elapsed() >= tick_rate {
            app.frame_count += 1;
            app.update_metrics();
            
            // Clear old status messages
            if let Some((_, created)) = &app.status_message {
                if created.elapsed() > Duration::from_secs(3) {
                    app.status_message = None;
                }
            }
            
            // Clear old error messages
            if let Some((_, created)) = &app.error_message {
                if created.elapsed() > Duration::from_secs(5) {
                    app.error_message = None;
                }
            }
            
            last_tick = Instant::now();
        }
        
        // Execute a pending query from the REPL.
        if app.query_pending {
            app.query_pending = false;
            app.run_query().await;
        }

        // Handle async refresh if loading flag is set
        if app.loading && app.connected {
            app.refresh_data().await;
            app.status_message = Some(("Data refreshed".to_string(), Instant::now()));
        }
        
        // Periodic data refresh when connected
        if app.connected && last_refresh.elapsed() >= refresh_rate {
            app.refresh_data().await;
            last_refresh = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Render the UI
fn ui(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();

    // Main layout: header, tabs, content, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Tabs
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, chunks[0], app);

    // Render tabs
    render_tabs(frame, chunks[1], app);

    // Render content based on current tab
    match app.current_tab {
        AppTab::Dashboard => render_dashboard(frame, chunks[2], app),
        AppTab::Memories => render_memories(frame, chunks[2], app),
        AppTab::Agents => render_agents(frame, chunks[2], app),
        AppTab::Graphs => render_graphs(frame, chunks[2], app),
        AppTab::Logs => render_logs(frame, chunks[2], app),
        AppTab::Config => render_config(frame, chunks[2], app),
        AppTab::Query => render_query(frame, chunks[2], app),
        AppTab::Help => render_help(frame, chunks[2], app),
    }

    // Render footer
    render_footer(frame, chunks[3], app);

    // Render command palette if open
    if app.command_palette_open {
        render_command_palette(frame, app);
    }
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let connection_status = if app.connected { "● Connected" } else { "○ Disconnected" };
    let connection_color = if app.connected { Color::Green } else { Color::Red };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰓅 GaussOS", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" │ "),
        Span::styled("v3.0.0", Style::default().fg(Color::Yellow)),
        Span::raw(" │ "),
        Span::styled(connection_status, Style::default().fg(connection_color)),
        Span::raw(" │ "),
        Span::styled(&app.server_url, Style::default().fg(Color::Blue)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(Color::Rgb(20, 20, 40)))
    );

    frame.render_widget(header, area);
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let titles: Vec<Line<'_>> = AppTab::all()
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let style = if app.current_tab == *tab {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(format!(" {} {} ", i + 1, tab.title())).style(style)
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Navigation (1-7 or Tab) ")
                .title_style(Style::default().fg(Color::Yellow))
        )
        .select(app.current_tab.index())
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .divider(symbols::line::VERTICAL);

    frame.render_widget(tabs, area);
}

fn render_dashboard(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),   // Stats cards
            Constraint::Min(0),      // Main content
        ])
        .split(area);

    // Stats row
    let stats_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(chunks[0]);

    // CPU Usage
    render_stat_card(
        frame,
        stats_chunks[0],
        "󰍛 CPU",
        &format!("{:.1}%", app.metrics.cpu_usage),
        Color::Cyan,
        app.metrics.cpu_usage as u16,
    );

    // Memory Usage
    render_stat_card(
        frame,
        stats_chunks[1],
        "󰘚 Memory",
        &format!("{:.1}%", app.metrics.memory_usage),
        Color::Magenta,
        app.metrics.memory_usage as u16,
    );

    // API Throughput
    render_stat_card(
        frame,
        stats_chunks[2],
        "󰒍 API",
        &format!("{}/s", app.metrics.api_requests_per_sec),
        Color::Green,
        (app.metrics.api_requests_per_sec / 200) as u16,
    );

    // Cache Hit Rate
    render_stat_card(
        frame,
        stats_chunks[3],
        "󰆼 Cache",
        &format!("{:.1}%", app.metrics.cache_hit_rate),
        Color::Yellow,
        app.metrics.cache_hit_rate as u16,
    );

    // Main content area
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[1]);

    // Recent activity
    let activity_block = Block::default()
        .title(" 󰷐 Recent Activity ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::horizontal(1));

    let activity_items: Vec<ListItem<'_>> = app.log_entries.iter().take(10).map(|entry| {
        let level_color = match entry.level.as_str() {
            "ERROR" => Color::Red,
            "WARN" => Color::Yellow,
            "INFO" => Color::Green,
            "DEBUG" => Color::Blue,
            _ => Color::White,
        };
        ListItem::new(Line::from(vec![
            Span::styled(&entry.timestamp, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(&entry.level, Style::default().fg(level_color).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::raw(&entry.message),
        ]))
    }).collect();

    let activity_list = List::new(activity_items)
        .block(activity_block)
        .highlight_style(Style::default().bg(Color::Rgb(40, 40, 60)));

    frame.render_widget(activity_list, content_chunks[0]);

    // System info
    let info_block = Block::default()
        .title(" 󰋼 System Information ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::horizontal(1));

    let uptime_hours = app.metrics.uptime_seconds / 3600;
    let uptime_mins = (app.metrics.uptime_seconds % 3600) / 60;
    let uptime_secs = app.metrics.uptime_seconds % 60;

    let info_text = Text::from(vec![
        Line::from(vec![
            Span::styled("Uptime:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}h {}m {}s", uptime_hours, uptime_mins, uptime_secs),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Memories: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.metrics.total_memories),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Active Agents:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.metrics.active_agents),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Connections:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}", app.metrics.active_connections),
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Memory Used:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.2} GB / {:.2} GB", 
                    app.metrics.memory_used as f64 / 1_000_000_000.0,
                    app.metrics.memory_total as f64 / 1_000_000_000.0
                ),
                Style::default().fg(Color::Magenta),
            ),
        ]),
    ]);

    let info_para = Paragraph::new(info_text).block(info_block);
    frame.render_widget(info_para, content_chunks[1]);
}

fn render_stat_card(frame: &mut Frame<'_>, area: Rect, title: &str, value: &str, color: Color, percentage: u16) {
    let block = Block::default()
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Value
    let value_para = Paragraph::new(value)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .centered();
    
    let value_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 2,
    };
    frame.render_widget(value_para, value_area);

    // Progress bar
    if inner.height > 3 {
        let bar_area = Rect {
            x: inner.x,
            y: inner.y + 3,
            width: inner.width,
            height: 1,
        };
        let filled = (bar_area.width as u16 * percentage.min(100)) / 100;
        let bar_text = format!(
            "{}{}",
            "█".repeat(filled as usize),
            "░".repeat((bar_area.width.saturating_sub(filled)) as usize)
        );
        let bar = Paragraph::new(bar_text).style(Style::default().fg(color));
        frame.render_widget(bar, bar_area);
    }
}

fn render_memories(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let items: Vec<ListItem<'_>> = app.memory_items.iter().map(|mem| {
        let type_color = match mem.memory_type.as_str() {
            "Semantic" => Color::Cyan,
            "Episodic" => Color::Magenta,
            "Parametric" => Color::Yellow,
            "Procedural" => Color::Green,
            _ => Color::White,
        };
        ListItem::new(Line::from(vec![
            Span::styled(&mem.id, Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled(&mem.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" │ "),
            Span::styled(&mem.memory_type, Style::default().fg(type_color)),
            Span::raw(" │ "),
            Span::styled(&mem.namespace, Style::default().fg(Color::Blue)),
            Span::raw(" │ "),
            Span::styled(format!("{} bytes", mem.size_bytes), Style::default().fg(Color::DarkGray)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" 󰍉 Memory Browser ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(1))
        )
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 80)).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_agents(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let items: Vec<ListItem<'_>> = app.agent_items.iter().map(|agent| {
        let status_color = match agent.status.as_str() {
            "Active" => Color::Green,
            "Idle" => Color::Yellow,
            "Processing" => Color::Cyan,
            "Error" => Color::Red,
            _ => Color::White,
        };
        ListItem::new(Line::from(vec![
            Span::styled(&agent.id, Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled(&agent.name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" │ "),
            Span::styled(&agent.status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
            Span::raw(" │ "),
            Span::styled(&agent.agent_type, Style::default().fg(Color::Blue)),
            Span::raw(" │ "),
            Span::styled(format!("{} executions", agent.executions), Style::default().fg(Color::DarkGray)),
            Span::raw(" │ "),
            Span::styled(&agent.last_activity, Style::default().fg(Color::Magenta)),
        ]))
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" 󰚩 Agent Manager ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(1))
        )
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 80)).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_graphs(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Knowledge graph  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} entities", app.graph_nodes), Style::default().fg(Color::Green)),
            Span::styled("  ·  ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{} relations", app.graph_edges), Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
    ];
    if app.graph_top.is_empty() {
        lines.push(Line::from(Span::styled(
            "No facts yet. Ingest facts via POST /api/v1/facts or the Web UI to populate the graph.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        lines.push(Line::from(Span::styled("Most-connected entities:", Style::default().fg(Color::DarkGray))));
        lines.push(Line::from(""));
        let max = app.graph_top.iter().map(|(_, d)| *d).max().unwrap_or(1).max(1);
        for (name, deg) in &app.graph_top {
            let bar_len = (*deg * 24 / max).max(1);
            let bar: String = "█".repeat(bar_len);
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<22} ", truncate(name, 22)), Style::default().fg(Color::White)),
                Span::styled(bar, Style::default().fg(Color::Cyan)),
                Span::styled(format!(" {}", deg), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tip: the Web UI Knowledge Graph page draws this with a bi-temporal 'as-of' slider and PPR tracing.",
        Style::default().fg(Color::DarkGray),
    )));

    let text = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(" 󰈈 Knowledge Graph ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::horizontal(1)),
    );
    frame.render_widget(text, area);
}

/// Truncate a string to `n` chars with an ellipsis.
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    }
}

fn render_logs(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let items: Vec<ListItem<'_>> = app.log_entries.iter().map(|entry| {
        let level_color = match entry.level.as_str() {
            "ERROR" => Color::Red,
            "WARN" => Color::Yellow,
            "INFO" => Color::Green,
            "DEBUG" => Color::Blue,
            "TRACE" => Color::DarkGray,
            _ => Color::White,
        };
        ListItem::new(Line::from(vec![
            Span::styled(&entry.timestamp, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(
                format!("{:5}", entry.level),
                Style::default().fg(level_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(&entry.source, Style::default().fg(Color::Blue)),
            Span::raw(": "),
            Span::raw(&entry.message),
        ]))
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" 󰷐 Log Viewer ")
                .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .padding(Padding::horizontal(1))
        )
        .highlight_style(Style::default().bg(Color::Rgb(50, 50, 80)))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn render_config(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let kv = |k: &str, v: String, c: Color| {
        Line::from(vec![
            Span::styled(format!("  {:<22}", k), Style::default().fg(Color::DarkGray)),
            Span::styled(v, Style::default().fg(c)),
        ])
    };
    let conn = if app.connected { ("connected", Color::Green) } else { ("offline", Color::Red) };
    let provider = if app.llm_provider.is_empty() { "—".to_string() } else { app.llm_provider.clone() };
    let model = if app.llm_model.is_empty() { "—".to_string() } else { app.llm_model.clone() };
    let llm_state = if app.llm_configured { ("configured", Color::Green) } else { ("no API key", Color::Yellow) };
    // Compile-time feature flags (honest — reflects how this binary was built).
    let simd = if cfg!(feature = "simd") { "enabled" } else { "disabled (build with --features simd)" };

    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled("Live server configuration", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        kv("server.url", app.server_url.clone(), Color::White),
        kv("server.status", conn.0.to_string(), conn.1),
        Line::from(""),
        Line::from(Span::styled("LLM provider (live)", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        kv("llm.provider", provider, Color::Cyan),
        kv("llm.model", model, Color::White),
        kv("llm.status", llm_state.0.to_string(), llm_state.1),
        Line::from(""),
        Line::from(Span::styled("Build features", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        kv("performance.simd", simd.to_string(), if cfg!(feature = "simd") { Color::Green } else { Color::DarkGray }),
        kv("backend", "embedded SurrealDB (default)".to_string(), Color::Green),
        Line::from(""),
        Line::from(Span::styled("Values are read live from the server; edit via env vars / .env (see .env.example).", Style::default().fg(Color::DarkGray))),
    ]))
    .block(
        Block::default()
            .title(" 󰒓 Configuration ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(text, area);
}

fn render_query(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = vec![
        Line::from(Span::styled(
            "Search your memories. Type a query and press Enter.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        // Live input line with a block cursor.
        Line::from(vec![
            Span::styled("gaussos> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(app.query_input.clone(), Style::default().fg(Color::White)),
            Span::styled("▏", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
    ];

    if !app.query_ran {
        lines.push(Line::from(Span::styled(
            "Examples:  rust memory   ·   surrealdb   ·   forgetting curve",
            Style::default().fg(Color::DarkGray),
        )));
    } else if app.query_results.is_empty() {
        lines.push(Line::from(Span::styled("No matching memories.", Style::default().fg(Color::Yellow))));
    } else {
        lines.push(Line::from(vec![
            Span::styled(format!("{} result(s)", app.query_results.len()), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(""));
        // Column header.
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<10} {:<8} ", "TYPE", "QUALITY"), Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            Span::styled("CONTENT", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        ]));
        for r in &app.query_results {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<10} ", truncate(&r.mem_type, 10)), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:<8.2} ", r.quality), Style::default().fg(Color::Green)),
                Span::styled(truncate(&r.content, 60), Style::default().fg(Color::White)),
            ]));
        }
    }

    let text = Paragraph::new(Text::from(lines)).block(
        Block::default()
            .title(" 󰘳 Query REPL ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .padding(Padding::horizontal(1)),
    );

    frame.render_widget(text, area);
}

fn render_help(frame: &mut Frame<'_>, area: Rect, _app: &App) {
    let text = Paragraph::new(Text::from(vec![
        Line::from(""),
        Line::from(Span::styled("󰋖 GaussOS TUI Help", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Navigation:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  1-7     ", Style::default().fg(Color::Green)),
            Span::styled("Switch to tab by number", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Tab     ", Style::default().fg(Color::Green)),
            Span::styled("Next tab", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab ", Style::default().fg(Color::Green)),
            Span::styled("Previous tab", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  j/k     ", Style::default().fg(Color::Green)),
            Span::styled("Navigate up/down in lists", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Commands:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  Ctrl+K  ", Style::default().fg(Color::Green)),
            Span::styled("Open command palette", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+Q  ", Style::default().fg(Color::Green)),
            Span::styled("Quit application", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  ?       ", Style::default().fg(Color::Green)),
            Span::styled("Show this help", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Esc     ", Style::default().fg(Color::Green)),
            Span::styled("Close dialogs / clear selection", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Command Palette Commands:", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  quit, q     ", Style::default().fg(Color::Magenta)),
            Span::styled("Exit application", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  dashboard   ", Style::default().fg(Color::Magenta)),
            Span::styled("Go to dashboard", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  memories    ", Style::default().fg(Color::Magenta)),
            Span::styled("Go to memory browser", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  refresh     ", Style::default().fg(Color::Magenta)),
            Span::styled("Refresh current view", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Query REPL (tab 7):", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::styled("  type…       ", Style::default().fg(Color::Green)),
            Span::styled("Edit the query (keys go to the input on this tab)", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Enter       ", Style::default().fg(Color::Green)),
            Span::styled("Run the search against the live server", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Tab         ", Style::default().fg(Color::Green)),
            Span::styled("Leave the Query tab", Style::default().fg(Color::White)),
        ]),
    ]))
    .block(
        Block::default()
            .title(" 󰋖 Help ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
    );

    frame.render_widget(text, area);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let status = if let Some((msg, _)) = &app.status_message {
        Span::styled(msg, Style::default().fg(Color::Yellow))
    } else {
        Span::styled("Ready", Style::default().fg(Color::Green))
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Ctrl+K: Command Palette ", Style::default().fg(Color::DarkGray)),
        Span::raw("│"),
        Span::styled(" ?: Help ", Style::default().fg(Color::DarkGray)),
        Span::raw("│"),
        Span::styled(" Ctrl+Q: Quit ", Style::default().fg(Color::DarkGray)),
        Span::raw("│ "),
        status,
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
    );

    frame.render_widget(footer, area);
}

fn render_command_palette(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 5;
    
    let popup_area = Rect {
        x: (area.width - popup_width) / 2,
        y: (area.height - popup_height) / 3,
        width: popup_width,
        height: popup_height,
    };

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(&app.command_input),
        Span::styled("█", Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .title(" 󰘳 Command Palette ")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(30, 30, 50)))
            .padding(Padding::horizontal(1))
    );

    frame.render_widget(input, popup_area);
}
