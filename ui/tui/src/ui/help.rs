//! Help view rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Draw the help view
pub fn draw_help(frame: &mut Frame, area: Rect, _state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Content
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0]);
    draw_content(frame, chunks[1]);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰋖 Help & Keyboard Shortcuts", Style::default().fg(colors::CYAN).bold()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_content(frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Navigation shortcuts
    let nav_help = Paragraph::new(vec![
        Line::from(Span::styled("Navigation", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("1      ", Style::default().fg(colors::GREEN)), Span::raw("Dashboard")]),
        Line::from(vec![Span::styled("2      ", Style::default().fg(colors::GREEN)), Span::raw("Simulations")]),
        Line::from(vec![Span::styled("3      ", Style::default().fg(colors::GREEN)), Span::raw("Agents")]),
        Line::from(vec![Span::styled("4      ", Style::default().fg(colors::GREEN)), Span::raw("Spaces")]),
        Line::from(vec![Span::styled("5      ", Style::default().fg(colors::GREEN)), Span::raw("Logs")]),
        Line::from(vec![Span::styled("6      ", Style::default().fg(colors::GREEN)), Span::raw("Metrics")]),
        Line::from(vec![Span::styled("0      ", Style::default().fg(colors::GREEN)), Span::raw("Settings")]),
        Line::from(vec![Span::styled("?/F1   ", Style::default().fg(colors::GREEN)), Span::raw("Help")]),
        Line::from(""),
        Line::from(Span::styled("General", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("Ctrl+P ", Style::default().fg(colors::GREEN)), Span::raw("Command Palette")]),
        Line::from(vec![Span::styled("Ctrl+Q ", Style::default().fg(colors::GREEN)), Span::raw("Quit")]),
        Line::from(vec![Span::styled("Esc    ", Style::default().fg(colors::GREEN)), Span::raw("Back / Close")]),
        Line::from(vec![Span::styled("r      ", Style::default().fg(colors::GREEN)), Span::raw("Refresh")]),
    ])
    .block(titled_block(" Shortcuts "));
    frame.render_widget(nav_help, chunks[0]);

    // List/Table shortcuts
    let list_help = Paragraph::new(vec![
        Line::from(Span::styled("Lists & Tables", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("↑/k    ", Style::default().fg(colors::GREEN)), Span::raw("Move up")]),
        Line::from(vec![Span::styled("↓/j    ", Style::default().fg(colors::GREEN)), Span::raw("Move down")]),
        Line::from(vec![Span::styled("Enter  ", Style::default().fg(colors::GREEN)), Span::raw("Select / Open")]),
        Line::from(vec![Span::styled("/      ", Style::default().fg(colors::GREEN)), Span::raw("Search")]),
        Line::from(""),
        Line::from(Span::styled("Simulations", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("n      ", Style::default().fg(colors::GREEN)), Span::raw("New Simulation")]),
        Line::from(vec![Span::styled("s      ", Style::default().fg(colors::GREEN)), Span::raw("Start")]),
        Line::from(vec![Span::styled("p      ", Style::default().fg(colors::GREEN)), Span::raw("Pause")]),
        Line::from(vec![Span::styled("x      ", Style::default().fg(colors::GREEN)), Span::raw("Stop")]),
        Line::from(vec![Span::styled("d      ", Style::default().fg(colors::GREEN)), Span::raw("Delete")]),
        Line::from(""),
        Line::from(Span::styled("Space View", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("←↑↓→   ", Style::default().fg(colors::GREEN)), Span::raw("Pan")]),
        Line::from(vec![Span::styled("+/-    ", Style::default().fg(colors::GREEN)), Span::raw("Zoom in/out")]),
    ])
    .block(titled_block(" Actions "));
    frame.render_widget(list_help, chunks[1]);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Press any key to return", Style::default().fg(colors::FG_DARK)),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}

/// Draw a help overlay on top of any view
pub fn draw_help_overlay(frame: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 60, area);

    frame.render_widget(Clear, popup_area);

    let help_text = Paragraph::new(vec![
        Line::from(Span::styled("Quick Help", Style::default().fg(colors::CYAN).bold())),
        Line::from(""),
        Line::from(vec![Span::styled("1-6    ", Style::default().fg(colors::GREEN)), Span::raw("Switch views")]),
        Line::from(vec![Span::styled("Ctrl+P ", Style::default().fg(colors::GREEN)), Span::raw("Command palette")]),
        Line::from(vec![Span::styled("?      ", Style::default().fg(colors::GREEN)), Span::raw("Full help")]),
        Line::from(vec![Span::styled("Esc    ", Style::default().fg(colors::GREEN)), Span::raw("Close this help")]),
        Line::from(""),
        Line::from(Span::styled("Press ? for full keyboard shortcuts", Style::default().fg(colors::FG_DARK))),
    ])
    .alignment(Alignment::Center)
    .block(focused_block(" Help (Esc to close) "));

    frame.render_widget(help_text, popup_area);
}
