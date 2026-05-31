//! TUI Theme System
//! Customizable color schemes for the GaussOS TUI

use ratatui::style::Color;

/// Theme configuration for the TUI
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

/// Color palette for the theme
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Primary colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    
    // Background colors
    pub bg_primary: Color,
    pub bg_secondary: Color,
    pub bg_tertiary: Color,
    pub bg_highlight: Color,
    
    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    
    // Status colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    
    // Border colors
    pub border: Color,
    pub border_focus: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::cyberpunk()
    }
}

impl Theme {
    /// Default GaussOS cyberpunk theme
    pub fn cyberpunk() -> Self {
        Self {
            name: "Cyberpunk".to_string(),
            colors: ThemeColors {
                primary: Color::Cyan,
                secondary: Color::Magenta,
                accent: Color::Yellow,
                
                bg_primary: Color::Rgb(15, 23, 42),
                bg_secondary: Color::Rgb(30, 41, 59),
                bg_tertiary: Color::Rgb(51, 65, 85),
                bg_highlight: Color::Rgb(50, 50, 80),
                
                text_primary: Color::Rgb(248, 250, 252),
                text_secondary: Color::Rgb(203, 213, 225),
                text_muted: Color::Rgb(100, 116, 139),
                
                success: Color::Rgb(16, 185, 129),
                warning: Color::Rgb(245, 158, 11),
                error: Color::Rgb(239, 68, 68),
                info: Color::Rgb(59, 130, 246),
                
                border: Color::Rgb(55, 65, 81),
                border_focus: Color::Cyan,
            },
        }
    }

    /// Nord-inspired theme
    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            colors: ThemeColors {
                primary: Color::Rgb(136, 192, 208),
                secondary: Color::Rgb(180, 142, 173),
                accent: Color::Rgb(235, 203, 139),
                
                bg_primary: Color::Rgb(46, 52, 64),
                bg_secondary: Color::Rgb(59, 66, 82),
                bg_tertiary: Color::Rgb(67, 76, 94),
                bg_highlight: Color::Rgb(76, 86, 106),
                
                text_primary: Color::Rgb(236, 239, 244),
                text_secondary: Color::Rgb(229, 233, 240),
                text_muted: Color::Rgb(143, 188, 187),
                
                success: Color::Rgb(163, 190, 140),
                warning: Color::Rgb(235, 203, 139),
                error: Color::Rgb(191, 97, 106),
                info: Color::Rgb(129, 161, 193),
                
                border: Color::Rgb(76, 86, 106),
                border_focus: Color::Rgb(136, 192, 208),
            },
        }
    }

    /// Dracula theme
    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            colors: ThemeColors {
                primary: Color::Rgb(189, 147, 249),
                secondary: Color::Rgb(255, 121, 198),
                accent: Color::Rgb(241, 250, 140),
                
                bg_primary: Color::Rgb(40, 42, 54),
                bg_secondary: Color::Rgb(68, 71, 90),
                bg_tertiary: Color::Rgb(98, 114, 164),
                bg_highlight: Color::Rgb(68, 71, 90),
                
                text_primary: Color::Rgb(248, 248, 242),
                text_secondary: Color::Rgb(248, 248, 242),
                text_muted: Color::Rgb(98, 114, 164),
                
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(255, 184, 108),
                error: Color::Rgb(255, 85, 85),
                info: Color::Rgb(139, 233, 253),
                
                border: Color::Rgb(98, 114, 164),
                border_focus: Color::Rgb(189, 147, 249),
            },
        }
    }

    /// Gruvbox dark theme
    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox".to_string(),
            colors: ThemeColors {
                primary: Color::Rgb(215, 153, 33),
                secondary: Color::Rgb(211, 134, 155),
                accent: Color::Rgb(250, 189, 47),
                
                bg_primary: Color::Rgb(40, 40, 40),
                bg_secondary: Color::Rgb(60, 56, 54),
                bg_tertiary: Color::Rgb(80, 73, 69),
                bg_highlight: Color::Rgb(102, 92, 84),
                
                text_primary: Color::Rgb(235, 219, 178),
                text_secondary: Color::Rgb(213, 196, 161),
                text_muted: Color::Rgb(168, 153, 132),
                
                success: Color::Rgb(152, 151, 26),
                warning: Color::Rgb(215, 153, 33),
                error: Color::Rgb(204, 36, 29),
                info: Color::Rgb(69, 133, 136),
                
                border: Color::Rgb(102, 92, 84),
                border_focus: Color::Rgb(215, 153, 33),
            },
        }
    }

    /// Get all available themes
    pub fn all_themes() -> Vec<Theme> {
        vec![
            Theme::cyberpunk(),
            Theme::nord(),
            Theme::dracula(),
            Theme::gruvbox(),
        ]
    }
}
