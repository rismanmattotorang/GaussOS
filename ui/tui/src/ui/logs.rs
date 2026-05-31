//! Log viewer rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Draw the log viewer
pub fn draw_logs(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Log list
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0], state);
    draw_log_list(frame, chunks[1], state);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let filter_text = match &state.log_filter.min_level {
        Some(level) => format!("≥{}", level.as_str()),
        None => "All".to_string(),
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰷐 Logs", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" │ "),
        Span::styled("Filter: ", Style::default().fg(colors::FG_DARK)),
        Span::styled(filter_text, Style::default().fg(colors::YELLOW)),
        Span::raw(" │ "),
        Span::styled(format!("{} entries", state.logs.len()), Style::default().fg(colors::FG_DARK)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_log_list(frame: &mut Frame, area: Rect, state: &AppState) {
    // If no logs, show placeholder
    if state.logs.is_empty() {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("No log entries", Style::default().fg(colors::FG_DARK))),
            Line::from(""),
            Line::from(Span::styled("Logs will appear here when simulations are running", Style::default().fg(colors::FG_DARK))),
        ])
        .alignment(Alignment::Center)
        .block(titled_block(" Log Output "));
        frame.render_widget(placeholder, area);
        return;
    }

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .filter(|log| {
            if let Some(min_level) = &state.log_filter.min_level {
                log.level >= *min_level
            } else {
                true
            }
        })
        .map(|log| {
            let timestamp = log.timestamp.format("%H:%M:%S%.3f");
            let level_color = log.level.color();

            let content = Line::from(vec![
                Span::styled(format!("{} ", timestamp), Style::default().fg(colors::FG_DARK)),
                Span::styled(format!("{:5} ", log.level.as_str()), Style::default().fg(level_color)),
                Span::styled(format!("[{}] ", log.target), Style::default().fg(colors::PURPLE)),
                Span::raw(&log.message),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(titled_block(" Log Output "));

    frame.render_widget(list, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(colors::CYAN)),
        Span::raw(" Scroll "),
        Span::styled(" PgUp/PgDn", Style::default().fg(colors::CYAN)),
        Span::raw(" Page "),
        Span::styled(" Home/End", Style::default().fg(colors::CYAN)),
        Span::raw(" Top/Bottom "),
        Span::styled(" f", Style::default().fg(colors::CYAN)),
        Span::raw(" Filter "),
        Span::styled(" /", Style::default().fg(colors::CYAN)),
        Span::raw(" Search "),
        Span::styled(" c", Style::default().fg(colors::CYAN)),
        Span::raw(" Clear "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}
