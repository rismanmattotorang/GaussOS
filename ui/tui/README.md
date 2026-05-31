# GaussTwin Terminal UI (TUI)

A high-performance terminal user interface for managing and monitoring GaussTwin digital twin simulations.

## Features

### Core Views
- **Dashboard**: Real-time overview with simulation status, CPU/memory sparklines, and agent counts
- **Simulations**: Table view with sorting, filtering, and quick actions (start/stop/pause)
- **Agents**: Browse and inspect individual agents with state, memory, and communication details
- **Spaces**: ASCII canvas visualization of simulation spaces with pan/zoom
- **Logs**: Real-time log streaming with level filtering
- **Metrics**: System and simulation metrics with charts and gauges
- **Settings**: Configure theme, API endpoint, and TUI preferences

### Interactive Features
- **Command Palette** (`Ctrl+P`): Fuzzy search for commands and navigation
- **Keyboard Navigation**: Full vim-style (`j/k`) and arrow key support
- **Help System** (`?`): Context-sensitive keyboard shortcuts

### Themes
- Tokyo Night (default)
- Dark
- Light
- Gruvbox
- Nord

## Installation

```bash
# Build from source
cd ui/tui
cargo build --release

# Install locally
cargo install --path .
```

## Usage

```bash
# Start with default settings
gausstwin-tui

# Connect to custom API endpoint
gausstwin-tui --api-url http://localhost:8080

# Use specific theme
gausstwin-tui --theme gruvbox

# Show help
gausstwin-tui --help
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `1-6` | Switch views (Dashboard, Simulations, Agents, Spaces, Logs, Metrics) |
| `0` | Settings |
| `?` / `F1` | Help |
| `Ctrl+P` | Command Palette |
| `Ctrl+Q` | Quit |
| `Esc` | Back / Close |
| `r` | Refresh |

### Lists & Tables
| Key | Action |
|-----|--------|
| `↑/k` | Move up |
| `↓/j` | Move down |
| `Enter` | Select / Open |
| `/` | Search |

### Simulations
| Key | Action |
|-----|--------|
| `n` | New simulation |
| `s` | Start |
| `p` | Pause |
| `x` | Stop |
| `d` | Delete |

### Space View
| Key | Action |
|-----|--------|
| `←↑↓→` | Pan |
| `+/-` | Zoom in/out |
| `0` | Reset zoom |

### Logs
| Key | Action |
|-----|--------|
| `f` | Cycle log level filter |
| `c` | Clear logs |
| `PgUp/PgDn` | Page scroll |
| `Home/End` | Jump to top/bottom |

## Configuration

Configuration file is stored at `~/.config/gausstwin/tui.toml`:

```toml
api_url = "http://localhost:8080"
theme = "tokyo-night"
mouse_enabled = true
tick_rate = 250
```

## Architecture

```
ui/tui/
├── src/
│   ├── main.rs          # Entry point and CLI
│   ├── app/             # Application state and logic
│   │   ├── mod.rs       # Main App struct and loop
│   │   ├── state.rs     # State management
│   │   ├── config.rs    # Configuration
│   │   └── actions.rs   # Commands and actions
│   ├── ui/              # UI rendering
│   │   ├── dashboard.rs # Dashboard view
│   │   ├── simulations.rs
│   │   ├── agents.rs
│   │   ├── spaces.rs
│   │   ├── logs.rs
│   │   ├── metrics.rs
│   │   ├── settings.rs
│   │   ├── help.rs
│   │   └── command_palette.rs
│   ├── views/           # View definitions
│   ├── handlers/        # Event handlers
│   ├── widgets/         # Custom widgets
│   └── utils/           # Utilities and API client
└── Cargo.toml
```

## Dependencies

- **ratatui**: Terminal UI framework
- **crossterm**: Cross-platform terminal manipulation
- **tokio**: Async runtime
- **tui-logger**: Log widget with tracing support
- **fuzzy-matcher**: Command palette fuzzy search
- **reqwest**: HTTP client for API communication

## License

MIT
