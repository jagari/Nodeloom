//! Nodeloom desktop app. It runs the master and/or the agent in-process
//! (`services`), lives in the tray, and shows one of two pages:
//!
//! - this app's own page (setup, agent status, settings), which may call
//!   the commands below;
//! - on a master machine, the dashboard the master serves, loaded from
//!   loopback like in a browser. It is a remote page to Tauri and gets no
//!   access to these commands. Cluster data only ever comes through the
//!   master's HTTP API (D4); the commands here manage the app itself (D16).

mod services;
mod settings;

use std::path::PathBuf;

use nodeloom_agent::AgentStatus;
use serde::Serialize;
use services::{Options, Running};
use settings::{Role, Settings};
use tauri::{
    AppHandle, Manager, State, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::ManagerExt;

/// Passed when the OS starts the app at login: start in the tray.
const HIDDEN_FLAG: &str = "--hidden";

struct AppState {
    settings_path: PathBuf,
    role: Option<Role>,
    running: Option<Running>,
    /// Why the services for `role` did not start.
    error: Option<String>,
}

#[derive(Serialize)]
struct MasterView {
    lan_url: String,
}

/// Everything the app's own page shows. The admin token is left out on
/// purpose: only the dashboard window needs it, and Rust hands it over.
#[derive(Serialize)]
struct AppView {
    version: &'static str,
    role: Option<Role>,
    /// This machine's confirmation code.
    code: Option<String>,
    agent: Option<AgentStatus>,
    master: Option<MasterView>,
    autostart: bool,
    error: Option<String>,
}

#[tauri::command]
fn app_state(app: AppHandle, state: State<'_, AppState>) -> AppView {
    let running = state.running.as_ref();
    AppView {
        version: env!("CARGO_PKG_VERSION"),
        role: state.role,
        code: running.map(|r| r.code.clone()),
        agent: running.map(|r| r.agent.borrow().clone()),
        master: running.and_then(|r| r.master.as_ref()).map(|m| MasterView {
            lan_url: m.lan_url.clone(),
        }),
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        error: state.error.clone(),
    }
}

/// Saves the role (or clears it to ask again) and restarts into it.
/// Restarting is simpler and safer than stopping a master in place.
#[tauri::command]
fn choose_role(
    app: AppHandle,
    state: State<'_, AppState>,
    role: Option<Role>,
    autostart: Option<bool>,
) -> Result<(), String> {
    settings::save(&state.settings_path, &Settings { role }).map_err(|e| e.to_string())?;
    if let Some(enabled) = autostart {
        set_autostart(app.clone(), enabled)?;
    }
    app.restart()
}

#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| e.to_string())
}

/// Gives this machine a new identity, for one the dashboard removed: the
/// old key stays refused, the new one shows up for approval with a new code.
#[tauri::command]
fn reset_identity(app: AppHandle) -> Result<(), String> {
    let dirs = services::Dirs::standard().map_err(|e| e.to_string())?;
    nodeloom_agent::identity::reset(&dirs.agent_state).map_err(|e| e.to_string())?;
    app.restart()
}

/// Opens the dashboard window (master machines only).
#[tauri::command]
fn open_dashboard(app: AppHandle) {
    show_main(&app);
}

fn page_for(state: &AppState) -> WebviewUrl {
    match state.running.as_ref().and_then(|r| r.master.as_ref()) {
        // The token rides in the URL fragment, which never leaves the
        // window; the dashboard stores it and removes it from the address.
        Some(master) => match Url::parse(&format!("{}#token={}", master.local_url, master.token)) {
            Ok(url) => WebviewUrl::External(url),
            Err(_) => WebviewUrl::default(),
        },
        None => WebviewUrl::default(),
    }
}

fn build_main(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    WebviewWindowBuilder::new(app, "main", page_for(&state))
        .title("Nodeloom")
        .inner_size(1100.0, 800.0)
        .min_inner_size(420.0, 480.0)
        .visible(visible)
        .build()?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    match app.get_webview_window("main") {
        Some(window) => {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
        None => {
            if let Err(e) = build_main(app, true) {
                tracing::error!("cannot open the window: {e}");
            }
        }
    }
}

/// The app's own page. On a master machine the main window holds the
/// dashboard, so settings get a small window of their own.
fn show_settings(app: &AppHandle) {
    let has_dashboard = app
        .state::<AppState>()
        .running
        .as_ref()
        .is_some_and(|r| r.master.is_some());
    if !has_dashboard {
        return show_main(app);
    }
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, "settings", WebviewUrl::default())
        .title("Nodeloom settings")
        .inner_size(520.0, 640.0)
        .min_inner_size(420.0, 480.0)
        .build();
    if let Err(e) = built {
        tracing::error!("cannot open settings: {e}");
    }
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Nodeloom", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Nodeloom", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Nodeloom")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = tauri::Builder::default()
        // First, so a second launch only brings this one forward instead
        // of starting a second master on the same port.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app)
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                // A LaunchAgent on macOS, the default.
                .args([HIDDEN_FLAG])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            app_state,
            choose_role,
            set_autostart,
            reset_identity,
            open_dashboard
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let role = settings::load(&settings_path).role;
            let (running, error) = match role {
                None => (None, None),
                Some(role) => {
                    let started = tauri::async_runtime::block_on(async {
                        services::start(role, Options::standard()?).await
                    });
                    match started {
                        Ok(running) => (Some(running), None),
                        Err(e) => {
                            tracing::error!("cannot start as {role:?}: {e:#}");
                            (None, Some(format!("{e:#}")))
                        }
                    }
                }
            };
            app.manage(AppState {
                settings_path,
                role,
                running,
                error,
            });

            build_tray(app.handle())?;
            // Started at login: stay in the tray. A first run always shows
            // the setup page.
            let hidden = role.is_some() && std::env::args().any(|a| a == HIDDEN_FLAG);
            build_main(app.handle(), !hidden)?;
            Ok(())
        })
        // Closing a window hides it; the services keep running until
        // "Quit" in the tray.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start the Nodeloom desktop app");

    app.run(|app, event| {
        // Clicking the Dock icon brings the hidden window back.
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            show_main(app);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
