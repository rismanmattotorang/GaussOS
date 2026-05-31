//! Settings view rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Draw the settings view
pub fn draw_settings(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Settings list
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0]);
    draw_settings_list(frame, chunks[1], state);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰒓 Settings", Style::default().fg(colors::CYAN).bold()),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_settings_list(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let items: Vec<ListItem> = state
        .settings_list
        .items
        .iter()
        .enumerate()
        .map(|(i, setting)| {
            let is_selected = i == state.settings_list.state;

            let value_style = match &setting.setting_type {
                crate::app::SettingType::Toggle => {
                    if setting.value == "true" {
                        Style::default().fg(colors::GREEN)
                    } else {
                        Style::default().fg(colors::RED)
                    }
                }
                _ => Style::default().fg(colors::CYAN),
            };

            let content = Line::from(vec![
                Span::styled(&setting.label, Style::default().fg(if is_selected { colors::CYAN } else { colors::FG })),
                Span::raw(": "),
                Span::styled(&setting.value, value_style),
            ]);

            let style = if is_selected {
                Style::default().bg(colors::BG_HIGHLIGHT)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(titled_block(" Application Settings "));

    frame.render_widget(list, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(colors::CYAN)),
        Span::raw(" Navigate "),
        Span::styled(" Enter", Style::default().fg(colors::CYAN)),
        Span::raw(" Edit "),
        Span::styled(" Esc", Style::default().fg(colors::CYAN)),
        Span::raw(" Back "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}
