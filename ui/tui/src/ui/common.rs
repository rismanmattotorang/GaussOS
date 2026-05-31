//! Common UI components and utilities

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Create a styled block with title
pub fn titled_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title)
        .title_style(Style::default().fg(Color::White).bold())
}

/// Create a focused block with highlighted border
pub fn focused_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold())
}

/// Create a status bar paragraph
pub fn status_bar(content: &str) -> Paragraph<'_> {
    Paragraph::new(content)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
}

/// Format bytes to human readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration to human readable string
pub fn format_duration(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Format number with thousands separator
pub fn format_number(n: u64) -> String {
    use num_format::{Locale, ToFormattedString};
    n.to_formatted_string(&Locale::en)
}

/// Create a centered popup area
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Tokyo Night theme colors
pub mod colors {
    use ratatui::style::Color;

    pub const BG: Color = Color::Rgb(26, 27, 38);
    pub const BG_DARK: Color = Color::Rgb(22, 23, 34);
    pub const BG_HIGHLIGHT: Color = Color::Rgb(41, 46, 66);
    pub const FG: Color = Color::Rgb(169, 177, 214);
    pub const FG_DARK: Color = Color::Rgb(86, 95, 137);
    pub const COMMENT: Color = Color::Rgb(86, 95, 137);
    pub const CYAN: Color = Color::Rgb(125, 207, 255);
    pub const BLUE: Color = Color::Rgb(122, 162, 247);
    pub const PURPLE: Color = Color::Rgb(187, 154, 247);
    pub const GREEN: Color = Color::Rgb(158, 206, 106);
    pub const ORANGE: Color = Color::Rgb(255, 158, 100);
    pub const RED: Color = Color::Rgb(247, 118, 142);
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);
    pub const MAGENTA: Color = Color::Rgb(255, 0, 127);
}

/// Create a sparkline from data
pub fn create_sparkline_data(data: &[f64], max_points: usize) -> Vec<u64> {
    let data: Vec<f64> = if data.len() > max_points {
        data.iter().rev().take(max_points).copied().collect::<Vec<_>>().into_iter().rev().collect()
    } else {
        data.to_vec()
    };

    let max = data.iter().cloned().fold(f64::MIN, f64::max);
    let min = data.iter().cloned().fold(f64::MAX, f64::min);
    let range = max - min;

    if range == 0.0 {
        return vec![50; data.len()];
    }

    data.iter()
        .map(|&v| ((v - min) / range * 100.0) as u64)
        .collect()
}
