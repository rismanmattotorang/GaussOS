//! Space visualization rendering

use crate::app::AppState;
use crate::ui::common::*;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, canvas::{Canvas, Points}},
};

/// Draw the space visualization view
pub fn draw_spaces(frame: &mut Frame, area: Rect, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Canvas
            Constraint::Length(3),  // Footer
        ])
        .split(area);

    draw_header(frame, chunks[0], state);
    draw_canvas(frame, chunks[1], state);
    draw_footer(frame, chunks[2]);
}

fn draw_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled("󰕰 Space Visualization", Style::default().fg(colors::CYAN).bold()),
        Span::raw(" │ "),
        Span::styled(format!("Zoom: {:.1}x", state.space_view.zoom), Style::default().fg(colors::FG_DARK)),
        Span::raw(" │ "),
        Span::styled(format!("Offset: ({}, {})", state.space_view.offset_x, state.space_view.offset_y), Style::default().fg(colors::FG_DARK)),
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(header, area);
}

fn draw_canvas(frame: &mut Frame, area: Rect, state: &AppState) {
    // Get agent positions
    let points: Vec<(f64, f64)> = state.agents.items.iter()
        .map(|a| a.position)
        .collect();

    let canvas = Canvas::default()
        .block(titled_block(" Grid Space "))
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            // Draw grid lines
            for i in 0..=10 {
                let y = i as f64 * 10.0;
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: 0.0,
                    y1: y,
                    x2: 100.0,
                    y2: y,
                    color: colors::BG_HIGHLIGHT,
                });
            }
            for i in 0..=10 {
                let x = i as f64 * 10.0;
                ctx.draw(&ratatui::widgets::canvas::Line {
                    x1: x,
                    y1: 0.0,
                    x2: x,
                    y2: 100.0,
                    color: colors::BG_HIGHLIGHT,
                });
            }

            // Draw agents as points
            ctx.draw(&Points {
                coords: &points,
                color: colors::CYAN,
            });

            // Draw agent labels
            for (i, agent) in state.agents.items.iter().enumerate() {
                ctx.print(
                    agent.position.0,
                    agent.position.1 + 2.0,
                    Span::styled(
                        format!("A{}", i + 1),
                        Style::default().fg(colors::FG_DARK),
                    ),
                );
            }
        });

    frame.render_widget(canvas, area);
}

fn draw_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" ←↑↓→", Style::default().fg(colors::CYAN)),
        Span::raw(" Pan "),
        Span::styled(" +/-", Style::default().fg(colors::CYAN)),
        Span::raw(" Zoom "),
        Span::styled(" 0", Style::default().fg(colors::CYAN)),
        Span::raw(" Reset "),
    ]))
    .style(Style::default().fg(colors::FG_DARK))
    .block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors::BG_HIGHLIGHT)));

    frame.render_widget(footer, area);
}
