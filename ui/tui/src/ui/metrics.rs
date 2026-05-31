//! Metrics dashboard rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{BarChart, Block, Borders, Gauge, Paragraph, Sparkline, Tabs},
};

/// Draw the metrics dashboard
pub fn draw_metrics(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Tabs
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0]);
    
    // Tabs
    let titles: Vec<Line> = state.metrics_tab.titles.iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .select(state.metrics_tab.selected)
        .style(Style::default().fg(colors::FG_DARK))
        .highlight_style(Style::default().fg(colors::CYAN).bold())
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Content based on selected tab
    match state.metrics_tab.selected {
        0 => draw_system_metrics(frame, chunks[2], state),
        1 => draw_simulation_metrics(frame, chunks[2], state),
        2 => draw_agent_metrics(frame, chunks[2], state),
        3 => draw_network_metrics(frame, chunks[2], state),
        _ => {}
    }

    draw_footer(frame, chunks[3]);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰄪 Metrics Dashboard", Style::default().fg(colors::CYAN).bold()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_system_metrics(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // CPU gauge
            Constraint::Length(3),  // Memory gauge
            Constraint::Min(5),     // CPU history
            Constraint::Min(5),     // Memory history
        ])
        .split(area);

    // CPU gauge
    let cpu = state.metrics.current.cpu_usage;
    let cpu_gauge = Gauge::default()
        .block(titled_block(" CPU Usage "))
        .gauge_style(Style::default().fg(if cpu > 80.0 { colors::RED } else { colors::BLUE }))
        .percent(cpu as u16)
        .label(format!("{:.1}%", cpu));
    frame.render_widget(cpu_gauge, chunks[0]);

    // Memory gauge
    let mem = state.metrics.current.memory_usage;
    let mem_gauge = Gauge::default()
        .block(titled_block(" Memory Usage "))
        .gauge_style(Style::default().fg(if mem > 80.0 { colors::RED } else { colors::PURPLE }))
        .percent(mem as u16)
        .label(format!("{:.1}%", mem));
    frame.render_widget(mem_gauge, chunks[1]);

    // CPU history sparkline
    let cpu_data: Vec<u64> = state.metrics.cpu_history.iter().map(|&v| v as u64).collect();
    let cpu_sparkline = Sparkline::default()
        .block(titled_block(" CPU History (60s) "))
        .data(&cpu_data)
        .style(Style::default().fg(colors::BLUE));
    frame.render_widget(cpu_sparkline, chunks[2]);

    // Memory history sparkline
    let mem_data: Vec<u64> = state.metrics.memory_history.iter().map(|&v| v as u64).collect();
    let mem_sparkline = Sparkline::default()
        .block(titled_block(" Memory History (60s) "))
        .data(&mem_data)
        .style(Style::default().fg(colors::PURPLE));
    frame.render_widget(mem_sparkline, chunks[3]);
}

fn draw_simulation_metrics(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Running simulations by status
    let running = state.simulations.items.iter()
        .filter(|s| matches!(s.status, crate::app::SimulationStatus::Running)).count();
    let paused = state.simulations.items.iter()
        .filter(|s| matches!(s.status, crate::app::SimulationStatus::Paused)).count();
    let stopped = state.simulations.items.iter()
        .filter(|s| matches!(s.status, crate::app::SimulationStatus::Stopped)).count();

    let data: Vec<(&str, u64)> = vec![
        ("Run", running as u64),
        ("Pause", paused as u64),
        ("Stop", stopped as u64),
    ];

    let barchart = BarChart::default()
        .block(titled_block(" Simulation Status "))
        .data(&data)
        .bar_width(8)
        .bar_gap(2)
        .bar_style(Style::default().fg(colors::CYAN))
        .value_style(Style::default().fg(colors::FG).bold());
    frame.render_widget(barchart, chunks[0]);

    // Simulation progress
    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Total Simulations: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format!("{}", state.simulations.items.len()), Style::default().fg(colors::CYAN)),
        ]),
        Line::from(vec![
            Span::styled("Total Agents: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(
                format_number(state.simulations.items.iter().map(|s| s.agent_count).sum()),
                Style::default().fg(colors::GREEN),
            ),
        ]),
        Line::from(vec![
            Span::styled("Total Steps: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(
                format_number(state.simulations.items.iter().map(|s| s.current_step).sum()),
                Style::default().fg(colors::PURPLE),
            ),
        ]),
    ])
    .block(titled_block(" Summary "));
    frame.render_widget(content, chunks[1]);
}

fn draw_agent_metrics(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Agent status distribution
    let active = state.agents.items.iter()
        .filter(|a| matches!(a.status, crate::app::AgentStatus::Active)).count();
    let idle = state.agents.items.iter()
        .filter(|a| matches!(a.status, crate::app::AgentStatus::Idle)).count();

    let data: Vec<(&str, u64)> = vec![
        ("Active", active as u64),
        ("Idle", idle as u64),
    ];

    let barchart = BarChart::default()
        .block(titled_block(" Agent Status "))
        .data(&data)
        .bar_width(10)
        .bar_gap(2)
        .bar_style(Style::default().fg(colors::GREEN))
        .value_style(Style::default().fg(colors::FG).bold());
    frame.render_widget(barchart, chunks[0]);

    // Agent communication
    let total_sent: u64 = state.agents.items.iter().map(|a| a.messages_sent).sum();
    let total_recv: u64 = state.agents.items.iter().map(|a| a.messages_received).sum();

    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Total Agents: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format!("{}", state.agents.items.len()), Style::default().fg(colors::CYAN)),
        ]),
        Line::from(vec![
            Span::styled("Messages Sent: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_number(total_sent), Style::default().fg(colors::GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Messages Received: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_number(total_recv), Style::default().fg(colors::BLUE)),
        ]),
    ])
    .block(titled_block(" Communication "));
    frame.render_widget(content, chunks[1]);
}

fn draw_network_metrics(frame: &mut Frame, area: Rect, _state: &AppState) {
    let content = Paragraph::new(vec![
        Line::from(Span::styled("Network metrics coming soon...", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(titled_block(" Network "));
    frame.render_widget(content, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(colors::CYAN)),
        Span::raw(" Switch Tabs "),
        Span::styled(" r", Style::default().fg(colors::CYAN)),
        Span::raw(" Refresh "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}
