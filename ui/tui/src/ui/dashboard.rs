// ! Enhanced dashboard view rendering with improved visualizations

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, Gauge, List, ListItem, Paragraph, Sparkline,
    },
};

/// Draw the main dashboard view
pub fn draw_dashboard(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0], state);
    draw_main_content(frame, chunks[1], state);
    draw_footer(frame, chunks[2], state);
}

fn draw_header(frame: &mut Frame, area: Rect, _state: &AppState) {
    let current_time = chrono::Local::now().format("%H:%M:%S").to_string();
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("█▓▒░ ", Style::default().fg(colors::CYAN).bold()),
            Span::styled("GaussTwin ", Style::default().fg(colors::CYAN).bold()),
            Span::styled("Digital Twin Framework", Style::default().fg(colors::FG)),
            Span::raw(" │ "),
            Span::styled("Dashboard", Style::default().fg(colors::PURPLE).bold()),
            Span::raw(" │ "),
            Span::styled(&current_time, Style::default().fg(colors::FG_DARK)),
            Span::raw(" ░▒▓█"),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_main_content(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    draw_left_panel(frame, chunks[0], state);
    draw_right_panel(frame, chunks[1], state);
}

fn draw_left_panel(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9),  // Stats cards
            Constraint::Min(10),    // Simulations list
            Constraint::Length(6),  // Resource gauges
        ])
        .split(area);

    draw_stats_cards(frame, chunks[0], state);
    draw_simulations_list(frame, chunks[1], state);
    draw_resource_gauges(frame, chunks[2], state);
}

fn draw_stats_cards(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Running simulations
    let running = state.simulations.items.iter()
        .filter(|s| matches!(s.status, crate::app::SimulationStatus::Running))
        .count();

    let running_card = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!("  {}  ", running), Style::default().fg(colors::GREEN).bold().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Running", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(titled_block(" 󰐊 Simulations ")
        .border_style(Style::default().fg(colors::GREEN)));
    frame.render_widget(running_card, chunks[0]);

    // Total agents
    let total_agents: u64 = state.simulations.items.iter().map(|s| s.agent_count).sum();
    let agents_card = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!(" {} ", format_number(total_agents)), Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(Span::styled("Agents", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(titled_block(" 󰀄 Total Agents ")
        .border_style(Style::default().fg(colors::CYAN)));
    frame.render_widget(agents_card, chunks[1]);

    // Events/sec
    let events_per_sec = state.metrics.current.events_per_second;
    let events_card = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!(" {} ", format_number(events_per_sec as u64)), Style::default().fg(colors::PURPLE).bold())),
        Line::from(""),
        Line::from(Span::styled("Events/s", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(titled_block(" ⚡ Throughput ")
        .border_style(Style::default().fg(colors::PURPLE)));
    frame.render_widget(events_card, chunks[2]);

    // Uptime - simulated for now
    let uptime_secs = 7200u64; // 2 hours
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let uptime_card = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(format!(" {}h {}m ", hours, minutes), Style::default().fg(colors::BLUE).bold())),
        Line::from(""),
        Line::from(Span::styled("Uptime", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(titled_block(" 󰥔 Uptime ")
        .border_style(Style::default().fg(colors::BLUE)));
    frame.render_widget(uptime_card, chunks[3]);
}

fn draw_simulations_list(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let items: Vec<ListItem> = state
        .simulations
        .items
        .iter()
        .enumerate()
        .map(|(i, sim)| {
            let is_selected = i == state.simulations.state;
            let status_color = sim.status.color();
            let status_icon = match sim.status {
                crate::app::SimulationStatus::Running => "▶ ",
                crate::app::SimulationStatus::Paused => "⏸ ",
                crate::app::SimulationStatus::Stopped => "⏹ ",
                crate::app::SimulationStatus::Completed => "✓ ",
                crate::app::SimulationStatus::Error => "✗ ",
            };

            let progress = sim.current_step as f64 / sim.total_steps.max(1) as f64 * 100.0;
            let progress_str = create_mini_progress_bar(progress as u8);

            let content = Line::from(vec![
                Span::styled(status_icon, Style::default().fg(status_color).bold()),
                Span::styled(&sim.name, Style::default().fg(if is_selected { colors::CYAN } else { colors::FG }).bold()),
                Span::raw("  "),
                Span::styled(format_number(sim.agent_count), Style::default().fg(colors::FG_DARK)),
                Span::styled(" agents", Style::default().fg(colors::FG_DARK)),
                Span::raw("  "),
                Span::styled(progress_str, Style::default().fg(colors::GREEN)),
                Span::raw(" "),
                Span::styled(format!("{}%", progress as u8), Style::default().fg(colors::FG_DARK)),
            ]);

            let style = if is_selected {
                Style::default().bg(colors::BG_HIGHLIGHT).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(titled_block(" 󰐊 Active Simulations ")
            .border_style(Style::default().fg(if state.simulations.items.is_empty() { colors::FG_DARK } else { colors::CYAN })))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(list, area);
}

fn draw_resource_gauges(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    // CPU gauge
    let cpu = state.metrics.current.cpu_usage;
    let cpu_gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" 󰻠 CPU Usage ")
            .border_style(Style::default().fg(if cpu > 80.0 { colors::RED } else { colors::BLUE })))
        .gauge_style(Style::default().fg(if cpu > 80.0 { colors::RED } else { colors::BLUE }).bg(colors::BG_HIGHLIGHT))
        .percent(cpu as u16)
        .label(format!("{:.1}%", cpu));

    frame.render_widget(cpu_gauge, chunks[0]);

    // Memory gauge
    let mem = state.metrics.current.memory_usage;
    let mem_gauge = Gauge::default()
        .block(Block::default()
            .borders(Borders::ALL)
            .title(" 󰍛 Memory Usage ")
            .border_style(Style::default().fg(if mem > 80.0 { colors::RED } else { colors::PURPLE })))
        .gauge_style(Style::default().fg(if mem > 80.0 { colors::RED } else { colors::PURPLE }).bg(colors::BG_HIGHLIGHT))
        .percent(mem as u16)
        .label(format!("{:.1}%", mem));

    frame.render_widget(mem_gauge, chunks[1]);
}

fn draw_right_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(area);

    draw_cpu_chart(frame, chunks[0], state);
    draw_memory_chart(frame, chunks[1], state);
    draw_activity_log(frame, chunks[2], state);
}

fn draw_cpu_chart(frame: &mut Frame, area: Rect, state: &AppState) {
    let data: Vec<u64> = state.metrics.cpu_history.iter().map(|&v| v as u64).collect();

    let max_val = data.iter().max().copied().unwrap_or(100);
    let sparkline = Sparkline::default()
        .block(titled_block(" 󰻠 CPU Usage History ")
            .border_style(Style::default().fg(colors::BLUE)))
        .data(&data)
        .max(max_val)
        .style(Style::default().fg(colors::BLUE));

    frame.render_widget(sparkline, area);
}

fn draw_memory_chart(frame: &mut Frame, area: Rect, state: &AppState) {
    let data: Vec<u64> = state.metrics.memory_history.iter().map(|&v| v as u64).collect();

    let max_val = data.iter().max().copied().unwrap_or(100);
    let sparkline = Sparkline::default()
        .block(titled_block(" 󰍛 Memory Usage History ")
            .border_style(Style::default().fg(colors::PURPLE)))
        .data(&data)
        .max(max_val)
        .style(Style::default().fg(colors::PURPLE));

    frame.render_widget(sparkline, area);
}

fn draw_activity_log(frame: &mut Frame, area: Rect, state: &AppState) {
    let log_lines: Vec<Line> = state
        .logs
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|log| {
            let level_color = match log.level.as_str() {
                "ERROR" => colors::RED,
                "WARN" => colors::YELLOW,
                "INFO" => colors::CYAN,
                _ => colors::FG_DARK,
            };
            Line::from(vec![
                Span::styled(format!("[{}] ", log.level.as_str()), Style::default().fg(level_color).bold()),
                Span::styled(&log.message, Style::default().fg(colors::FG)),
            ])
        })
        .collect();

    let logs = Paragraph::new(log_lines)
        .block(titled_block(" 󰌱 Recent Activity ")
            .border_style(Style::default().fg(colors::GREEN)))
        .wrap(ratatui::widgets::Wrap { trim: true });

    frame.render_widget(logs, area);
}

fn draw_footer(frame: &mut Frame, area: Rect, _state: &AppState) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" 1", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Dashboard │ "),
        Span::styled("2", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Simulations │ "),
        Span::styled("3", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Agents │ "),
        Span::styled("4", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Spaces │ "),
        Span::styled("5", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Logs │ "),
        Span::styled("6", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" Metrics │ "),
        Span::styled("?", Style::default().fg(colors::YELLOW).bold()),
        Span::raw(" Help │ "),
        Span::styled("Ctrl+P", Style::default().fg(colors::PURPLE).bold()),
        Span::raw(" Commands │ "),
        Span::styled("Q", Style::default().fg(colors::RED).bold()),
        Span::raw(" Quit"),
    ]))
    .alignment(Alignment::Center)
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}

/// Create a mini ASCII progress bar
fn create_mini_progress_bar(percent: u8) -> String {
    let width = 10;
    let filled = (percent as usize * width) / 100;
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
