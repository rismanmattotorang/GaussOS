//! Simulations view rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Gauge, Paragraph, Row, Table, Tabs},
};

/// Draw the simulations list view
pub fn draw_simulations(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Table
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0]);
    draw_table(frame, chunks[1], state);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰐊 Simulations", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" │ "),
        Span::styled("n", Style::default().fg(colors::GREEN)),
        Span::raw(" New "),
        Span::styled("s", Style::default().fg(colors::GREEN)),
        Span::raw(" Start "),
        Span::styled("p", Style::default().fg(colors::YELLOW)),
        Span::raw(" Pause "),
        Span::styled("x", Style::default().fg(colors::RED)),
        Span::raw(" Stop "),
        Span::styled("d", Style::default().fg(colors::RED)),
        Span::raw(" Delete "),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_table(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let header_cells = ["Status", "Name", "Agents", "Progress", "Steps", "Updated"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(colors::CYAN).bold()));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = state
        .simulations
        .items
        .iter()
        .enumerate()
        .map(|(i, sim)| {
            let is_selected = i == state.simulations.state;
            let status_color = sim.status.color();

            let cells = vec![
                Cell::from(Span::styled(sim.status.as_str(), Style::default().fg(status_color))),
                Cell::from(Span::styled(&sim.name, Style::default().fg(colors::FG))),
                Cell::from(format_number(sim.agent_count)),
                Cell::from(format!("{:.1}%", sim.progress * 100.0)),
                Cell::from(format!("{}/{}", format_number(sim.current_step), format_number(sim.total_steps))),
                Cell::from(sim.updated_at.format("%H:%M:%S").to_string()),
            ];

            let style = if is_selected {
                Style::default().bg(colors::BG_HIGHLIGHT)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(20),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(titled_block(" Simulation List "))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(colors::CYAN)),
        Span::raw(" Navigate "),
        Span::styled(" Enter", Style::default().fg(colors::CYAN)),
        Span::raw(" View Details "),
        Span::styled(" /", Style::default().fg(colors::CYAN)),
        Span::raw(" Search "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}

/// Draw the simulation detail view
pub fn draw_simulation_detail(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let Some(sim) = state.simulations.selected() else {
        let msg = Paragraph::new("No simulation selected")
            .alignment(Alignment::Center)
            .block(titled_block(" Simulation Detail "));
        frame.render_widget(msg, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Tabs
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    // Header with simulation name
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰈙 ", Style::default().fg(colors::CYAN)),
        Span::styled(&sim.name, Style::default().fg(colors::FG).bold()),
        Span::raw(" │ "),
        Span::styled(sim.status.as_str(), Style::default().fg(sim.status.color())),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));
    frame.render_widget(header, chunks[0]);

    // Tabs
    let titles: Vec<Line> = state.detail_tab.titles.iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .select(state.detail_tab.selected)
        .style(Style::default().fg(colors::FG_DARK))
        .highlight_style(Style::default().fg(colors::CYAN).bold())
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Content based on selected tab
    match state.detail_tab.selected {
        0 => draw_detail_overview(frame, chunks[2], sim),
        1 => draw_detail_agents(frame, chunks[2], state),
        2 => draw_detail_events(frame, chunks[2], state),
        3 => draw_detail_config(frame, chunks[2], sim),
        _ => {}
    }

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(colors::CYAN)),
        Span::raw(" Switch Tabs "),
        Span::styled(" Esc", Style::default().fg(colors::CYAN)),
        Span::raw(" Back "),
        Span::styled(" s", Style::default().fg(colors::GREEN)),
        Span::raw(" Start "),
        Span::styled(" p", Style::default().fg(colors::YELLOW)),
        Span::raw(" Pause "),
        Span::styled(" x", Style::default().fg(colors::RED)),
        Span::raw(" Stop "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));
    frame.render_widget(footer, chunks[3]);
}

fn draw_detail_overview(frame: &mut Frame, area: Rect, sim: &crate::app::Simulation) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Info
            Constraint::Length(3),  // Progress
            Constraint::Min(5),     // Description
        ])
        .split(area);

    // Info
    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(&sim.id),
        ]),
        Line::from(vec![
            Span::styled("Agents: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_number(sim.agent_count), Style::default().fg(colors::CYAN)),
        ]),
        Line::from(vec![
            Span::styled("Steps: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(format!("{} / {}", format_number(sim.current_step), format_number(sim.total_steps))),
        ]),
        Line::from(vec![
            Span::styled("Created: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(sim.created_at.format("%Y-%m-%d %H:%M:%S").to_string()),
        ]),
    ])
    .block(titled_block(" Information "));
    frame.render_widget(info, chunks[0]);

    // Progress bar
    let progress = Gauge::default()
        .block(titled_block(" Progress "))
        .gauge_style(Style::default().fg(colors::GREEN).bg(colors::BG_DARK))
        .percent((sim.progress * 100.0) as u16)
        .label(format!("{:.1}%", sim.progress * 100.0));
    frame.render_widget(progress, chunks[1]);

    // Description
    let desc = Paragraph::new(&*sim.description)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .block(titled_block(" Description "));
    frame.render_widget(desc, chunks[2]);
}

fn draw_detail_agents(frame: &mut Frame, area: Rect, _state: &AppState) {
    let content = Paragraph::new("Agent list for this simulation...")
        .block(titled_block(" Agents "));
    frame.render_widget(content, area);
}

fn draw_detail_events(frame: &mut Frame, area: Rect, _state: &AppState) {
    let content = Paragraph::new("Event log for this simulation...")
        .block(titled_block(" Events "));
    frame.render_widget(content, area);
}

fn draw_detail_config(frame: &mut Frame, area: Rect, _sim: &crate::app::Simulation) {
    let content = Paragraph::new("Configuration (JSON)...")
        .block(titled_block(" Configuration "));
    frame.render_widget(content, area);
}
