//! Command palette rendering

use crate::app::CommandPaletteState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Draw the command palette
pub fn draw_command_palette(frame: &mut Frame, area: Rect, state: &mut CommandPaletteState) {
    let popup_area = centered_rect(50, 50, area);

    // Clear the background
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Input
            Constraint::Min(5),     // Results
        ])
        .split(popup_area);

    // Input field
    let input = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().fg(colors::CYAN)),
        Span::raw(&state.input),
        Span::styled("█", Style::default().fg(colors::CYAN)), // Cursor
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors::CYAN))
            .title(" Command Palette ")
            .title_style(Style::default().fg(colors::CYAN).bold()),
    );
    frame.render_widget(input, chunks[0]);

    // Command list
    let items: Vec<ListItem> = state
        .filtered
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == state.selected;

            let shortcut = cmd.shortcut().unwrap_or("");
            let shortcut_span = if shortcut.is_empty() {
                Span::raw("")
            } else {
                Span::styled(format!(" [{}]", shortcut), Style::default().fg(colors::FG_DARK))
            };

            let content = Line::from(vec![
                Span::styled(
                    cmd.name(),
                    Style::default().fg(if is_selected { colors::CYAN } else { colors::FG }),
                ),
                shortcut_span,
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
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                .border_style(Style::default().fg(colors::CYAN)),
        );
    frame.render_widget(list, chunks[1]);
}
