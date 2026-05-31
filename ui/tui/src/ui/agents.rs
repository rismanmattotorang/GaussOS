//! Agents view rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};

/// Draw the agents list view
pub fn draw_agents(frame: &mut Frame, area: Rect, state: &mut AppState) {
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
        Span::styled("󰀄 Agents", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" │ "),
        Span::styled("/", Style::default().fg(colors::GREEN)),
        Span::raw(" Search "),
        Span::styled("f", Style::default().fg(colors::GREEN)),
        Span::raw(" Filter "),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_table(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let header_cells = ["Status", "Name", "Type", "Position", "Memory", "Messages"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(colors::CYAN).bold()));
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = state
        .agents
        .items
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let is_selected = i == state.agents.state;
            let status_color = match agent.status {
                crate::app::AgentStatus::Active => colors::GREEN,
                crate::app::AgentStatus::Idle => colors::YELLOW,
                crate::app::AgentStatus::Blocked => colors::ORANGE,
                crate::app::AgentStatus::Error => colors::RED,
            };

            let cells = vec![
                Cell::from(Span::styled(agent.status.as_str(), Style::default().fg(status_color))),
                Cell::from(&*agent.name),
                Cell::from(&*agent.agent_type),
                Cell::from(format!("({:.1}, {:.1})", agent.position.0, agent.position.1)),
                Cell::from(format_bytes(agent.memory_usage)),
                Cell::from(format!("↑{} ↓{}", format_number(agent.messages_sent), format_number(agent.messages_received))),
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
        Constraint::Length(8),
        Constraint::Min(15),
        Constraint::Length(12),
        Constraint::Length(15),
        Constraint::Length(10),
        Constraint::Length(18),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(titled_block(" Agent List "))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    frame.render_widget(table, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(colors::CYAN)),
        Span::raw(" Navigate "),
        Span::styled(" Enter", Style::default().fg(colors::CYAN)),
        Span::raw(" Inspect "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}

/// Draw the agent inspector view
pub fn draw_agent_inspector(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let Some(agent) = state.agents.selected() else {
        let msg = Paragraph::new("No agent selected")
            .alignment(Alignment::Center)
            .block(titled_block(" Agent Inspector "));
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

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰍉 ", Style::default().fg(colors::CYAN)),
        Span::styled(&agent.name, Style::default().fg(colors::FG).bold()),
        Span::raw(" │ "),
        Span::styled(&agent.agent_type, Style::default().fg(colors::PURPLE)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));
    frame.render_widget(header, chunks[0]);

    // Tabs
    let titles: Vec<Line> = state.agent_tab.titles.iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .select(state.agent_tab.selected)
        .style(Style::default().fg(colors::FG_DARK))
        .highlight_style(Style::default().fg(colors::CYAN).bold())
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Content based on selected tab
    match state.agent_tab.selected {
        0 => draw_agent_state(frame, chunks[2], agent),
        1 => draw_agent_memory(frame, chunks[2], agent),
        2 => draw_agent_actions(frame, chunks[2]),
        3 => draw_agent_messages(frame, chunks[2], agent),
        _ => {}
    }

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(colors::CYAN)),
        Span::raw(" Switch Tabs "),
        Span::styled(" Esc", Style::default().fg(colors::CYAN)),
        Span::raw(" Back "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));
    frame.render_widget(footer, chunks[3]);
}

fn draw_agent_state(frame: &mut Frame, area: Rect, agent: &crate::app::Agent) {
    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(&agent.id),
        ]),
        Line::from(vec![
            Span::styled("Type: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(&agent.agent_type, Style::default().fg(colors::PURPLE)),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(agent.status.as_str()),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Style::default().fg(colors::FG_DARK)),
            Span::raw(format!("({:.2}, {:.2})", agent.position.0, agent.position.1)),
        ]),
    ])
    .block(titled_block(" State "));
    frame.render_widget(content, area);
}

fn draw_agent_memory(frame: &mut Frame, area: Rect, agent: &crate::app::Agent) {
    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Usage: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_bytes(agent.memory_usage), Style::default().fg(colors::CYAN)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Memory contents...", Style::default().fg(colors::FG_DARK))),
    ])
    .block(titled_block(" Memory "));
    frame.render_widget(content, area);
}

fn draw_agent_actions(frame: &mut Frame, area: Rect) {
    let content = Paragraph::new("Action history...")
        .block(titled_block(" Actions "));
    frame.render_widget(content, area);
}

fn draw_agent_messages(frame: &mut Frame, area: Rect, agent: &crate::app::Agent) {
    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Sent: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_number(agent.messages_sent), Style::default().fg(colors::GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Received: ", Style::default().fg(colors::FG_DARK)),
            Span::styled(format_number(agent.messages_received), Style::default().fg(colors::BLUE)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Message log...", Style::default().fg(colors::FG_DARK))),
    ])
    .block(titled_block(" Messages "));
    frame.render_widget(content, area);
}
