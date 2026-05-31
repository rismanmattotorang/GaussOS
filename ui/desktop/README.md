# GaussTwin Desktop

Native desktop application for the GaussTwin Digital Twin Framework, built with Tauri 2.0.

## 🚀 Features

- **Native Performance** - Rust backend with WebView frontend
- **Cross-Platform** - Windows, macOS, and Linux support
- **Offline Support** - Local SQLite database for offline simulation management
- **System Integration** - System tray, native menus, keyboard shortcuts
- **Secure Storage** - OS keychain integration for credentials
- **Auto Updates** - Built-in update mechanism
- **File Associations** - `.gausstwin` and `.gts` file support

## 🛠️ Tech Stack

### Frontend
- React 18 with TypeScript
- Vite 5 for fast builds
- Shared codebase with Web UI
- TailwindCSS + shadcn/ui

### Backend (Rust)
- Tauri 2.0
- SQLite (rusqlite)
- Keyring for secure credential storage
- Tokio async runtime

## 📦 Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- Platform-specific dependencies:

### macOS
```bash
xcode-select --install
```

### Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

### Windows
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) (usually pre-installed on Windows 10/11)

## 🚀 Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri:dev

# Build for production
npm run tauri:build
```

## 📁 Project Structure

```
ui/desktop/
├── src/                    # Frontend source (extends web UI)
│   ├── App.tsx            # Desktop-specific App component
│   ├── main.tsx           # Entry point
│   └── hooks/
│       └── use-tauri.ts   # Tauri integration hooks
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs        # Entry point
│   │   ├── lib.rs         # Main application setup
│   │   ├── commands/      # Tauri command handlers
│   │   ├── state/         # Application state
│   │   ├── db/            # SQLite database
│   │   └── utils/         # Utility functions
│   ├── Cargo.toml         # Rust dependencies
│   ├── tauri.conf.json    # Tauri configuration
│   └── capabilities/      # Permission capabilities
├── package.json
└── vite.config.ts
```

## 🔌 IPC Commands

The desktop app exposes these Tauri commands:

### Simulations
- `list_simulations` - List all local simulations
- `get_simulation` - Get simulation by ID
- `create_simulation` - Create new simulation
- `update_simulation` - Update simulation
- `delete_simulation` - Delete simulation
- `start_simulation` - Start simulation
- `pause_simulation` - Pause simulation
- `stop_simulation` - Stop simulation
- `export_simulation` - Export to file
- `import_simulation` - Import from file

### Files
- `open_file` - Open and read file
- `save_file` - Save file
- `get_recent_files` - Get recent files list
- `clear_recent_files` - Clear recent files
- `watch_directory` - Watch directory for changes
- `unwatch_directory` - Stop watching directory

### Settings
- `get_settings` - Get app settings
- `update_settings` - Update settings
- `reset_settings` - Reset to defaults

### Auth
- `get_stored_credentials` - Get credentials from keychain
- `store_credentials` - Store credentials in keychain
- `delete_credentials` - Delete stored credentials

### System
- `get_system_info` - Get system information
- `get_app_paths` - Get application paths
- `check_for_updates` - Check for app updates

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd/Ctrl + N` | New Simulation |
| `Cmd/Ctrl + O` | Open File |
| `Cmd/Ctrl + S` | Save |
| `Cmd/Ctrl + Shift + S` | Save As |
| `Cmd/Ctrl + E` | Export |
| `Cmd/Ctrl + I` | Import |
| `F5` | Start Simulation |
| `F6` | Pause Simulation |
| `F7` | Stop Simulation |
| `Shift + F5` | Restart Simulation |
| `Cmd/Ctrl + Shift + P` | Command Palette |

## 📦 Building for Distribution

```bash
# Build for current platform
npm run tauri:build

# Output will be in src-tauri/target/release/bundle/
```

### Build outputs by platform:
- **macOS**: `.app` bundle, `.dmg` installer
- **Windows**: `.exe`, `.msi` installer, NSIS installer
- **Linux**: `.deb`, `.AppImage`, `.rpm`

## 🔒 Security

- Credentials stored in OS keychain (Keychain on macOS, Windows Credential Manager, Secret Service on Linux)
- SQLite database encrypted at rest (optional)
- Content Security Policy configured
- Capability-based permissions system

## 📝 License

MIT License - see LICENSE file for details.
