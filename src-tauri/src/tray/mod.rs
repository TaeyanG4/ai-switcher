use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::adapters::get_adapter;
use crate::commands::{launch_profile_impl, launch_workspace_preset_impl, AppState};
use crate::error::AppError;
use crate::models::{AccountProfile, FavoriteTarget, PlatformType, RecentItem, WorkspacePreset};

// =========================================================================
// Pure Tray Menu Model (Headless & Testable)
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMenuItemModel {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub is_separator: bool,
    pub children: Vec<TrayMenuItemModel>,
}

impl TrayMenuItemModel {
    pub fn separator() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            enabled: false,
            is_separator: true,
            children: Vec::new(),
        }
    }

    pub fn item(id: &str, label: &str, enabled: bool) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            enabled,
            is_separator: false,
            children: Vec::new(),
        }
    }

    pub fn submenu(label: &str, children: Vec<TrayMenuItemModel>) -> Self {
        Self {
            id: String::new(),
            label: label.to_string(),
            enabled: true,
            is_separator: false,
            children,
        }
    }
}

pub fn build_tray_menu_model(
    favorites: &[FavoriteTarget],
    recents: &[RecentItem],
    accounts: &[AccountProfile],
    workspaces: &[WorkspacePreset],
) -> Vec<TrayMenuItemModel> {
    let mut items = Vec::new();

    // 1. Header Item
    items.push(TrayMenuItemModel::item(
        "header_title",
        "AI Switcher",
        false,
    ));

    // Active accounts only
    let active_accounts: Vec<_> = accounts.iter().filter(|a| a.is_enabled).collect();

    // 2. Favorites Submenu (omit if empty)
    let mut fav_children = Vec::new();
    for fav in favorites {
        if fav.target_type == "account" {
            if let Some(ref acc_id) = fav.account_id {
                if let Some(acc) = active_accounts.iter().find(|a| &a.id == acc_id) {
                    let label = format!("{} ({})", acc.display_name, acc.platform.as_str());
                    fav_children.push(TrayMenuItemModel::item(
                        &format!("fav:acc:{}", acc.id),
                        &label,
                        true,
                    ));
                }
            }
        } else if fav.target_type == "workspace_platform" {
            if let Some(ref ws_id) = fav.workspace_id {
                if let Some(ws) = workspaces.iter().find(|w| &w.id == ws_id) {
                    let plat_label = fav.platform.map(|p| p.as_str()).unwrap_or("All");
                    let label = format!("{} → {}", ws.name, plat_label);
                    fav_children.push(TrayMenuItemModel::item(
                        &format!("fav:ws:{}:{}", ws.id, plat_label),
                        &label,
                        true,
                    ));
                }
            }
        }
    }

    let has_favorites = !fav_children.is_empty();
    if has_favorites {
        items.push(TrayMenuItemModel::separator());
        items.push(TrayMenuItemModel::submenu("Favorites", fav_children));
    }

    // 3. Recent Launches Submenu (omit if empty)
    let mut recent_children = Vec::new();
    for r in recents {
        if r.kind == "account" {
            if let Some(acc) = active_accounts.iter().find(|a| a.id == r.id) {
                let label = format!("{} ({})", acc.display_name, acc.platform.as_str());
                recent_children.push(TrayMenuItemModel::item(
                    &format!("recent:account:{}", acc.id),
                    &label,
                    true,
                ));
            }
        } else if r.kind == "workspace" {
            if let Some(ws) = workspaces.iter().find(|w| w.id == r.id) {
                let label = format!("{} [Workspace]", ws.name);
                recent_children.push(TrayMenuItemModel::item(
                    &format!("recent:workspace:{}", ws.id),
                    &label,
                    true,
                ));
            }
        }
    }

    if !recent_children.is_empty() {
        if !has_favorites {
            items.push(TrayMenuItemModel::separator());
        }
        items.push(TrayMenuItemModel::submenu("Recent", recent_children));
    }

    // 4. Platforms (Codex, Claude, Antigravity) - omit any empty platform
    let codex_accs: Vec<_> = active_accounts
        .iter()
        .filter(|a| a.platform == PlatformType::Codex)
        .collect();
    let claude_accs: Vec<_> = active_accounts
        .iter()
        .filter(|a| a.platform == PlatformType::Claude)
        .collect();
    let ag_accs: Vec<_> = active_accounts
        .iter()
        .filter(|a| a.platform == PlatformType::Antigravity)
        .collect();

    let has_platforms = !codex_accs.is_empty() || !claude_accs.is_empty() || !ag_accs.is_empty();
    if has_platforms {
        items.push(TrayMenuItemModel::separator());

        if !codex_accs.is_empty() {
            let children = codex_accs
                .iter()
                .map(|a| {
                    TrayMenuItemModel::item(&format!("launch:acc:{}", a.id), &a.display_name, true)
                })
                .collect();
            items.push(TrayMenuItemModel::submenu("Codex", children));
        }

        if !claude_accs.is_empty() {
            let children = claude_accs
                .iter()
                .map(|a| {
                    TrayMenuItemModel::item(&format!("launch:acc:{}", a.id), &a.display_name, true)
                })
                .collect();
            items.push(TrayMenuItemModel::submenu("Claude", children));
        }

        if !ag_accs.is_empty() {
            let children = ag_accs
                .iter()
                .map(|a| {
                    TrayMenuItemModel::item(&format!("launch:acc:{}", a.id), &a.display_name, true)
                })
                .collect();
            items.push(TrayMenuItemModel::submenu("Antigravity", children));
        }
    }

    // 5. Workspaces Submenu (omit if empty)
    if !workspaces.is_empty() {
        items.push(TrayMenuItemModel::separator());
        let mut ws_children = Vec::new();
        for ws in workspaces {
            let mut platform_items = Vec::new();
            if !claude_accs.is_empty() {
                platform_items.push(TrayMenuItemModel::item(
                    &format!("launch:ws:{}:claude", ws.id),
                    "Claude",
                    true,
                ));
            }
            if !codex_accs.is_empty() {
                platform_items.push(TrayMenuItemModel::item(
                    &format!("launch:ws:{}:codex", ws.id),
                    "Codex",
                    true,
                ));
            }
            if !ag_accs.is_empty() {
                platform_items.push(TrayMenuItemModel::item(
                    &format!("launch:ws:{}:antigravity", ws.id),
                    "Antigravity",
                    true,
                ));
            }
            if !platform_items.is_empty() {
                ws_children.push(TrayMenuItemModel::submenu(&ws.name, platform_items));
            }
        }
        if !ws_children.is_empty() {
            items.push(TrayMenuItemModel::submenu("Workspaces", ws_children));
        }
    }

    // 6. Navigation and Quit actions
    items.push(TrayMenuItemModel::separator());
    items.push(TrayMenuItemModel::item(
        "show_main_window",
        "Open AI Switcher",
        true,
    ));
    items.push(TrayMenuItemModel::item("open_settings", "Settings", true));
    items.push(TrayMenuItemModel::separator());
    items.push(TrayMenuItemModel::item("quit_app", "Quit", true));

    items
}

// =========================================================================
// Native Tauri Tray Menu Builder
// =========================================================================

pub fn build_tray_menu(app: &AppHandle, state: &AppState) -> Result<Menu<Wry>, tauri::Error> {
    let favorites = state.db.list_favorites().unwrap_or_default();
    let recents = state.db.get_recent_launches(5).unwrap_or_default();
    let accounts = state.db.list_accounts(false).unwrap_or_default();
    let workspaces = state.db.list_workspaces().unwrap_or_default();

    let model_items = build_tray_menu_model(&favorites, &recents, &accounts, &workspaces);

    let menu = Menu::new(app)?;
    populate_menu(app, &menu, &model_items)?;
    Ok(menu)
}

fn populate_menu(
    app: &AppHandle,
    menu: &Menu<Wry>,
    items: &[TrayMenuItemModel],
) -> Result<(), tauri::Error> {
    for item in items {
        if item.is_separator {
            menu.append(&PredefinedMenuItem::separator(app)?)?;
        } else if !item.children.is_empty() {
            let submenu = Submenu::new(app, &item.label, item.enabled)?;
            populate_submenu(app, &submenu, &item.children)?;
            menu.append(&submenu)?;
        } else {
            let mi = MenuItem::with_id(app, &item.id, &item.label, item.enabled, None::<&str>)?;
            menu.append(&mi)?;
        }
    }
    Ok(())
}

fn populate_submenu(
    app: &AppHandle,
    submenu: &Submenu<Wry>,
    items: &[TrayMenuItemModel],
) -> Result<(), tauri::Error> {
    for item in items {
        if item.is_separator {
            submenu.append(&PredefinedMenuItem::separator(app)?)?;
        } else if !item.children.is_empty() {
            let child_submenu = Submenu::new(app, &item.label, item.enabled)?;
            populate_submenu(app, &child_submenu, &item.children)?;
            submenu.append(&child_submenu)?;
        } else {
            let mi = MenuItem::with_id(app, &item.id, &item.label, item.enabled, None::<&str>)?;
            submenu.append(&mi)?;
        }
    }
    Ok(())
}

// =========================================================================
// Tray Event Handling & Error Routing
// =========================================================================

pub fn handle_tray_menu_id(app: &AppHandle, item_id: &str) {
    match item_id {
        "show_main_window" => {
            restore_main_window(app);
        }
        "open_settings" => {
            restore_main_window(app);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("open-settings", ());
            }
        }
        "quit_app" => {
            app.exit(0);
        }
        id if id.starts_with("launch:acc:") || id.starts_with("fav:acc:") => {
            let acc_id = id
                .strip_prefix("launch:acc:")
                .or_else(|| id.strip_prefix("fav:acc:"))
                .unwrap_or_default();
            let acc_id_owned = acc_id.to_string();
            let app_handle = app.clone();

            tokio::spawn(async move {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    // Check conflict first for SingleInstance platforms
                    if let Ok(account) = state.db.get_account(&acc_id_owned) {
                        let adapter = get_adapter(account.platform);
                        let all_accounts = state.db.list_accounts(true).unwrap_or_default();
                        if let Some(conflict) = state.launcher_engine.check_conflict(
                            &account,
                            adapter.as_ref(),
                            &all_accounts,
                        ) {
                            // Restore window and show ConflictDialog!
                            restore_main_window(&app_handle);
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit(
                                    "tray-launch-conflict",
                                    serde_json::json!({
                                        "conflict": conflict,
                                        "targetAccount": account,
                                    }),
                                );
                            }
                            return;
                        }
                    }

                    match launch_profile_impl(&acc_id_owned, None, None, &state) {
                        Ok(_) => {
                            let _ = refresh_tray_menu(&app_handle, &state);
                        }
                        Err(err) => {
                            restore_main_window(&app_handle);
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit(
                                    "tray-launch-error",
                                    format!("Failed to launch profile: {}", err),
                                );
                            }
                        }
                    }
                }
            });
        }
        id if id.starts_with("launch:ws:") || id.starts_with("fav:ws:") => {
            let rest = id
                .strip_prefix("launch:ws:")
                .or_else(|| id.strip_prefix("fav:ws:"))
                .unwrap_or_default();
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() >= 2 {
                let ws_id = parts[0].to_string();
                let plat_str = parts[1].to_string();
                let app_handle = app.clone();

                tokio::spawn(async move {
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        if let Some(platform) = PlatformType::from_str(&plat_str) {
                            match launch_workspace_preset_impl(&ws_id, platform, None, &state) {
                                Ok(_) => {
                                    let _ = refresh_tray_menu(&app_handle, &state);
                                }
                                Err(AppError::WorkspaceDirectoryNotFound(path)) => {
                                    // Section 19: Restore main window and display existing missing folder dialog
                                    restore_main_window(&app_handle);
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        if let Ok(ws) = state.db.get_workspace(&ws_id) {
                                            let _ = window.emit(
                                                "tray-workspace-missing",
                                                serde_json::json!({
                                                    "workspace": ws,
                                                    "platform": platform,
                                                    "path": path,
                                                }),
                                            );
                                        }
                                    }
                                }
                                Err(err) => {
                                    restore_main_window(&app_handle);
                                    if let Some(window) = app_handle.get_webview_window("main") {
                                        let _ = window.emit(
                                            "tray-launch-error",
                                            format!("Failed to launch workspace: {}", err),
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
        id if id.starts_with("recent:account:") => {
            let acc_id = id["recent:account:".len()..].to_string();
            let app_handle = app.clone();
            tokio::spawn(async move {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    let _ = launch_profile_impl(&acc_id, None, None, &state);
                    let _ = refresh_tray_menu(&app_handle, &state);
                }
            });
        }
        id if id.starts_with("recent:workspace:") => {
            let ws_id = id["recent:workspace:".len()..].to_string();
            let app_handle = app.clone();
            tokio::spawn(async move {
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Ok(ws) = state.db.get_workspace(&ws_id) {
                        let platform = if ws.preferred_codex_account_id.is_some() {
                            PlatformType::Codex
                        } else if ws.preferred_claude_account_id.is_some() {
                            PlatformType::Claude
                        } else if ws.preferred_antigravity_account_id.is_some() {
                            PlatformType::Antigravity
                        } else {
                            PlatformType::Claude
                        };
                        let _ = launch_workspace_preset_impl(&ws_id, platform, None, &state);
                        let _ = refresh_tray_menu(&app_handle, &state);
                    }
                }
            });
        }
        _ => {}
    }
}

pub fn restore_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn create_tray_icon(app: &AppHandle, state: &AppState) -> Result<TrayIcon, tauri::Error> {
    let menu = build_tray_menu(app, state)?;

    let mut builder = TrayIconBuilder::with_id("main_tray")
        .tooltip("AI Switcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_tray_menu_id(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                restore_main_window(app);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)
}

pub fn refresh_tray_menu(app: &AppHandle, state: &AppState) -> Result<(), tauri::Error> {
    if let Some(tray) = app.tray_by_id("main_tray") {
        let new_menu = build_tray_menu(app, state)?;
        tray.set_menu(Some(new_menu))?;
    }
    Ok(())
}

// =========================================================================
// Headless Pure Model Unit Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AccountStatus, LoginMethod};
    use std::collections::HashMap;

    fn make_test_account(
        id: &str,
        platform: PlatformType,
        name: &str,
        enabled: bool,
    ) -> AccountProfile {
        AccountProfile {
            id: id.to_string(),
            platform,
            display_name: name.to_string(),
            account_identifier: None,
            login_method: LoginMethod::Google,
            status: AccountStatus::Ready,
            profile_path: format!("C:\\profiles\\{}", id),
            browser_profile_path: None,
            browser_profile_id: None,
            custom_executable_path: None,
            launch_arguments: vec![],
            environment_variables: HashMap::new(),
            default_workspace_path: None,
            is_enabled: enabled,
            last_launched_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_empty_sections_are_omitted() {
        // When there are no favorites, recents, accounts, or workspaces
        let model = build_tray_menu_model(&[], &[], &[], &[]);

        let labels: Vec<&str> = model.iter().map(|m| m.label.as_str()).collect();
        // Favorites, Recent, Codex, Claude, Antigravity, Workspaces must NOT appear!
        assert!(!labels.contains(&"Favorites"));
        assert!(!labels.contains(&"Recent"));
        assert!(!labels.contains(&"Codex"));
        assert!(!labels.contains(&"Claude"));
        assert!(!labels.contains(&"Antigravity"));
        assert!(!labels.contains(&"Workspaces"));

        // Only Header, Open, Settings, Quit appear
        assert!(labels.contains(&"AI Switcher"));
        assert!(labels.contains(&"Open AI Switcher"));
        assert!(labels.contains(&"Settings"));
        assert!(labels.contains(&"Quit"));
    }

    #[test]
    fn test_disabled_accounts_are_excluded() {
        let acc1 = make_test_account("acc-1", PlatformType::Claude, "Claude Active", true);
        let acc2 = make_test_account("acc-2", PlatformType::Codex, "Codex Disabled", false);

        let model = build_tray_menu_model(&[], &[], &[acc1, acc2], &[]);
        let labels: Vec<&str> = model.iter().map(|m| m.label.as_str()).collect();

        // Claude is active -> Claude submenu exists
        assert!(labels.contains(&"Claude"));
        let claude_sub = model.iter().find(|m| m.label == "Claude").unwrap();
        assert_eq!(claude_sub.children.len(), 1);
        assert_eq!(claude_sub.children[0].label, "Claude Active");

        // Codex is disabled -> Codex submenu does NOT exist!
        assert!(!labels.contains(&"Codex"));
    }

    #[test]
    fn test_favorites_and_recents_rendering() {
        let acc = make_test_account("acc-claude", PlatformType::Claude, "Personal", true);
        let ws = WorkspacePreset {
            id: "ws-1".to_string(),
            name: "OWOGG".to_string(),
            directory_path: "H:\\owogg".to_string(),
            preferred_codex_account_id: None,
            preferred_claude_account_id: Some("acc-claude".to_string()),
            preferred_antigravity_account_id: None,
            last_opened_at: Some("2026-09-05T10:00:00Z".to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        let fav1 = FavoriteTarget {
            id: "fav-1".to_string(),
            target_type: "account".to_string(),
            account_id: Some("acc-claude".to_string()),
            workspace_id: None,
            platform: None,
            sort_order: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let fav2 = FavoriteTarget {
            id: "fav-2".to_string(),
            target_type: "workspace_platform".to_string(),
            account_id: None,
            workspace_id: Some("ws-1".to_string()),
            platform: Some(PlatformType::Claude),
            sort_order: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let recent = RecentItem {
            id: "acc-claude".to_string(),
            kind: "account".to_string(),
            title: "Personal".to_string(),
            subtitle: "claude".to_string(),
            platform: Some(PlatformType::Claude),
            last_used_at: "2026-09-05T10:00:00Z".to_string(),
        };

        let model = build_tray_menu_model(&[fav1, fav2], &[recent], &[acc], &[ws]);

        let labels: Vec<&str> = model.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Favorites"));
        assert!(labels.contains(&"Recent"));

        let fav_sub = model.iter().find(|m| m.label == "Favorites").unwrap();
        assert_eq!(fav_sub.children.len(), 2);
        assert_eq!(fav_sub.children[0].label, "Personal (claude)");
        assert_eq!(fav_sub.children[1].label, "OWOGG → claude");

        let rec_sub = model.iter().find(|m| m.label == "Recent").unwrap();
        assert_eq!(rec_sub.children.len(), 1);
        assert_eq!(rec_sub.children[0].label, "Personal (claude)");
    }
}
