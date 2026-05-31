//! GaussTwin Desktop Application
//!
//! High-performance desktop client for the GaussTwin Digital Twin Framework.

mod commands;
mod db;
mod state;
mod utils;

use state::AppState;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, RunEvent, WindowEvent,
};
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize the application
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gausstwin_desktop=debug,tauri=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting GaussTwin Desktop Application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Initialize application state
            let app_state = Arc::new(RwLock::new(AppState::new(app.handle().clone())?));
            app.manage(app_state.clone());

            // Setup system tray
            setup_tray(app)?;

            // Setup menus
            setup_menus(app)?;

            // Setup global shortcuts
            setup_shortcuts(app)?;

            // Initialize database
            let state = app_state.clone();
            tauri::async_runtime::spawn(async move {
                let mut state = state.write().await;
                if let Err(e) = state.init_database().await {
                    tracing::error!("Failed to initialize database: {}", e);
                }
            });

            info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Simulation commands
            commands::simulation::list_simulations,
            commands::simulation::get_simulation,
            commands::simulation::create_simulation,
            commands::simulation::update_simulation,
            commands::simulation::delete_simulation,
            commands::simulation::start_simulation,
            commands::simulation::pause_simulation,
            commands::simulation::stop_simulation,
            commands::simulation::export_simulation,
            commands::simulation::import_simulation,
            // File commands
            commands::file::open_file,
            commands::file::save_file,
            commands::file::get_recent_files,
            commands::file::clear_recent_files,
            commands::file::watch_directory,
            commands::file::unwatch_directory,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::reset_settings,
            // Auth commands
            commands::auth::get_stored_credentials,
            commands::auth::store_credentials,
            commands::auth::delete_credentials,
            // System commands
            commands::system::get_system_info,
            commands::system::get_app_paths,
            commands::system::check_for_updates,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Hide to tray instead of closing on macOS
                #[cfg(target_os = "macos")]
                {
                    window.hide().unwrap();
                    api.prevent_close();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("Failed to build Tauri application")
        .run(|app_handle, event| {
            if let RunEvent::ExitRequested { code, .. } = event {
                if code.is_none() {
                    // Cleanup before exit
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) = handle.try_state::<Arc<RwLock<AppState>>>() {
                            let state = state.read().await;
                            if let Err(e) = state.cleanup().await {
                                tracing::error!("Cleanup error: {}", e);
                            }
                        }
                    });
                }
            }
        });
}

/// Setup system tray icon and menu
fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show GaussTwin", true, None::<&str>)?;
    let new_sim = MenuItem::with_id(app, "new_simulation", "New Simulation", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &new_sim, &separator, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("GaussTwin - Digital Twin Framework")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "new_simulation" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    let _ = window.emit("navigate", "/simulations/new");
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// Setup application menus
fn setup_menus(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // File menu
    let new_sim = MenuItem::with_id(app, "new", "New Simulation", true, Some("CmdOrCtrl+N"))?;
    let open = MenuItem::with_id(app, "open", "Open...", true, Some("CmdOrCtrl+O"))?;
    let save = MenuItem::with_id(app, "save", "Save", true, Some("CmdOrCtrl+S"))?;
    let save_as = MenuItem::with_id(app, "save_as", "Save As...", true, Some("CmdOrCtrl+Shift+S"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    let export = MenuItem::with_id(app, "export", "Export...", true, Some("CmdOrCtrl+E"))?;
    let import = MenuItem::with_id(app, "import", "Import...", true, Some("CmdOrCtrl+I"))?;

    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new_sim, &open, &separator, &save, &save_as, &separator, &export, &import,
        ],
    )?;

    // Edit menu
    let undo = PredefinedMenuItem::undo(app, None)?;
    let redo = PredefinedMenuItem::redo(app, None)?;
    let cut = PredefinedMenuItem::cut(app, None)?;
    let copy = PredefinedMenuItem::copy(app, None)?;
    let paste = PredefinedMenuItem::paste(app, None)?;
    let select_all = PredefinedMenuItem::select_all(app, None)?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &undo, &redo, &separator, &cut, &copy, &paste, &separator, &select_all,
        ],
    )?;

    // View menu
    let reload = MenuItem::with_id(app, "reload", "Reload", true, Some("CmdOrCtrl+R"))?;
    let dev_tools = MenuItem::with_id(
        app,
        "dev_tools",
        "Toggle Developer Tools",
        true,
        Some("CmdOrCtrl+Shift+I"),
    )?;
    let fullscreen = PredefinedMenuItem::fullscreen(app, None)?;
    let zoom_in = MenuItem::with_id(app, "zoom_in", "Zoom In", true, Some("CmdOrCtrl+Plus"))?;
    let zoom_out = MenuItem::with_id(app, "zoom_out", "Zoom Out", true, Some("CmdOrCtrl+Minus"))?;
    let zoom_reset = MenuItem::with_id(app, "zoom_reset", "Actual Size", true, Some("CmdOrCtrl+0"))?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &reload,
            &dev_tools,
            &separator,
            &fullscreen,
            &separator,
            &zoom_in,
            &zoom_out,
            &zoom_reset,
        ],
    )?;

    // Simulation menu
    let start = MenuItem::with_id(app, "sim_start", "Start Simulation", true, Some("F5"))?;
    let pause = MenuItem::with_id(app, "sim_pause", "Pause", true, Some("F6"))?;
    let stop = MenuItem::with_id(app, "sim_stop", "Stop", true, Some("F7"))?;
    let restart = MenuItem::with_id(app, "sim_restart", "Restart", true, Some("Shift+F5"))?;

    let simulation_menu = Submenu::with_items(
        app,
        "Simulation",
        true,
        &[&start, &pause, &stop, &separator, &restart],
    )?;

    // Window menu
    let minimize = PredefinedMenuItem::minimize(app, None)?;
    let maximize = MenuItem::with_id(app, "maximize", "Maximize", true, None::<&str>)?;
    let close = PredefinedMenuItem::close_window(app, None)?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[&minimize, &maximize, &separator, &close],
    )?;

    // Help menu
    let docs = MenuItem::with_id(app, "docs", "Documentation", true, Some("F1"))?;
    let website = MenuItem::with_id(app, "website", "Visit Website", true, None::<&str>)?;
    let check_updates = MenuItem::with_id(app, "check_updates", "Check for Updates...", true, None::<&str>)?;
    let about = PredefinedMenuItem::about(
        app,
        Some("About GaussTwin"),
        None,
    )?;

    let help_menu = Submenu::with_items(
        app,
        "Help",
        true,
        &[&docs, &website, &separator, &check_updates, &separator, &about],
    )?;

    // Build menu bar
    let menu = Menu::with_items(
        app,
        &[
            &file_menu,
            &edit_menu,
            &view_menu,
            &simulation_menu,
            &window_menu,
            &help_menu,
        ],
    )?;

    app.set_menu(menu)?;

    // Handle menu events
    app.on_menu_event(|app, event| {
        let window = app.get_webview_window("main");

        match event.id.as_ref() {
            "new" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "new-simulation");
                }
            }
            "open" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "open-file");
                }
            }
            "save" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "save-file");
                }
            }
            "save_as" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "save-file-as");
                }
            }
            "export" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "export");
                }
            }
            "import" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "import");
                }
            }
            "reload" => {
                if let Some(w) = window {
                    let _ = w.eval("window.location.reload()");
                }
            }
            "dev_tools" => {
                #[cfg(debug_assertions)]
                if let Some(w) = window {
                    // Devtools are only available in debug builds
                    let _ = w.eval("if (window.__TAURI_INTERNALS__) { console.log('DevTools toggled'); }");
                }
                #[cfg(not(debug_assertions))]
                {
                    // In release mode, show a notification that devtools are disabled
                    tracing::info!("Developer tools are disabled in release builds");
                }
            }
            "sim_start" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "start-simulation");
                }
            }
            "sim_pause" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "pause-simulation");
                }
            }
            "sim_stop" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "stop-simulation");
                }
            }
            "sim_restart" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "restart-simulation");
                }
            }
            "maximize" => {
                if let Some(w) = window {
                    if w.is_maximized().unwrap_or(false) {
                        let _ = w.unmaximize();
                    } else {
                        let _ = w.maximize();
                    }
                }
            }
            "docs" => {
                let _ = tauri::async_runtime::spawn(async {
                    let _ = open::that("https://docs.gausstwin.io");
                });
            }
            "website" => {
                let _ = tauri::async_runtime::spawn(async {
                    let _ = open::that("https://gausstwin.io");
                });
            }
            "check_updates" => {
                if let Some(w) = window {
                    let _ = w.emit("menu-action", "check-updates");
                }
            }
            _ => {}
        }
    });

    Ok(())
}

/// Setup global keyboard shortcuts
fn setup_shortcuts(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

    // Command palette shortcut
    let command_palette = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyP);
    app.global_shortcut().on_shortcut(command_palette, |app, _shortcut, _event| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("shortcut", "command-palette");
        }
    })?;

    Ok(())
}
