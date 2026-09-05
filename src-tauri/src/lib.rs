pub mod adapters;
pub mod browser;
pub mod commands;
pub mod db;
pub mod error;
pub mod fs_safety;
pub mod launcher;
pub mod models;
pub mod tray;

use std::path::PathBuf;
use tauri::Manager;

use crate::browser::BrowserProfileManager;
use crate::commands::{
    check_account_health, check_account_status, check_db_integrity, check_launch_conflict,
    cleanup_draft_account, create_account, create_browser_profile, create_db_backup,
    create_workspace, create_workspace_directory, delete_account, delete_browser_profile,
    delete_workspace, detect_available_browsers, export_support_bundle, get_account,
    get_app_settings, get_browser_profile, get_diagnostics, get_recent_launches, get_system_health,
    get_workspace, launch_browser_profile, launch_profile, launch_workspace_preset, list_accounts,
    list_active_processes, list_browser_profiles, list_favorites, list_platform_capabilities,
    list_workspaces, logout_account, refresh_tray_menu, repair_account_profile, save_app_settings,
    start_login_flow, terminate_process, toggle_account_enabled, toggle_favorite_account,
    toggle_favorite_workspace, update_account, update_browser_profile, update_workspace, AppState,
};
use crate::db::Db;
use crate::launcher::{LauncherEngine, ProcessManager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_dir = dirs::data_dir()
        .map(|p| p.join("AI-Switcher"))
        .unwrap_or_else(|| PathBuf::from("./data"));

    let db_path = app_dir.join("switcher.db");
    let base_profiles_dir = app_dir.join("profiles");
    let browser_profiles_dir = app_dir.join("browser-profiles");

    let db = Db::init(&db_path).expect("Failed to initialize SQLite database");
    let process_manager = ProcessManager::new();
    let launcher_engine = LauncherEngine::new(process_manager);
    let browser_manager = BrowserProfileManager::new(browser_profiles_dir);

    let app_state = AppState {
        db,
        base_profiles_dir,
        launcher_engine,
        browser_manager,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(settings) = state.db.get_app_settings() {
                        if settings.close_to_tray {
                            api.prevent_close();
                            let _ = window.hide();
                        }
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_platform_capabilities,
            list_accounts,
            get_account,
            create_account,
            update_account,
            toggle_account_enabled,
            delete_account,
            cleanup_draft_account,
            start_login_flow,
            check_account_status,
            logout_account,
            check_launch_conflict,
            launch_profile,
            terminate_process,
            list_active_processes,
            get_diagnostics,
            list_browser_profiles,
            get_browser_profile,
            create_browser_profile,
            update_browser_profile,
            delete_browser_profile,
            launch_browser_profile,
            detect_available_browsers,
            list_workspaces,
            get_workspace,
            create_workspace,
            update_workspace,
            delete_workspace,
            launch_workspace_preset,
            create_workspace_directory,
            get_app_settings,
            save_app_settings,
            list_favorites,
            toggle_favorite_account,
            toggle_favorite_workspace,
            get_recent_launches,
            refresh_tray_menu,
            check_account_health,
            repair_account_profile,
            get_system_health,
            create_db_backup,
            check_db_integrity,
            export_support_bundle,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            let _ = crate::tray::create_tray_icon(app.handle(), &state)?;

            let args: Vec<String> = std::env::args().collect();
            let is_minimized_arg = args.iter().any(|a| a == "--minimized");
            if is_minimized_arg {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            } else if let Ok(settings) = state.db.get_app_settings() {
                if settings.start_minimized {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
