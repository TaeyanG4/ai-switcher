use ai_switcher_lib::db::Db;
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, AppSettings, AuthStatus, LoginMethod, PlatformType,
    RuntimeStatus, WorkspacePreset,
};
use std::collections::HashMap;

#[test]
fn test_app_settings_persistence_and_defaults() {
    let temp_dir = std::env::temp_dir().join(format!(
        "ai_switcher_tray_test_settings_{}",
        uuid::Uuid::new_v4()
    ));
    let db_path = temp_dir.join("test.db");
    let db = Db::init(&db_path).expect("Failed to init db");

    let defaults = db.get_app_settings().expect("Failed to get defaults");
    assert_eq!(defaults.theme, "system");
    assert!(defaults.close_to_tray);
    assert!(!defaults.start_minimized);
    assert!(!defaults.first_close_shown);
    assert_eq!(defaults.global_shortcut, "CommandOrControl+Alt+S");
    assert_eq!(defaults.language, "en");

    let updated = AppSettings {
        theme: "light".to_string(),
        close_to_tray: false,
        start_minimized: true,
        first_close_shown: true,
        global_shortcut: "Ctrl+Shift+K".to_string(),
        language: "ko".to_string(),
    };
    db.save_app_settings(&updated)
        .expect("Failed to save settings");

    let reloaded = db.get_app_settings().expect("Failed to reload settings");
    assert_eq!(reloaded.theme, "light");
    assert!(!reloaded.close_to_tray);
    assert!(reloaded.start_minimized);
    assert!(reloaded.first_close_shown);
    assert_eq!(reloaded.global_shortcut, "Ctrl+Shift+K");
    assert_eq!(reloaded.language, "ko");

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_favorites_management_and_cascades() {
    let temp_dir = std::env::temp_dir().join(format!(
        "ai_switcher_tray_test_fav_{}",
        uuid::Uuid::new_v4()
    ));
    let db_path = temp_dir.join("test.db");
    let db = Db::init(&db_path).expect("Failed to init db");

    // Create 2 accounts
    let acc1 = AccountProfile {
        id: "acc-tray-1".to_string(),
        platform: PlatformType::Codex,
        display_name: "Codex Fast".to_string(),
        account_identifier: None,
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: "C:\\profiles\\tray1".to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    let acc2 = AccountProfile {
        id: "acc-tray-2".to_string(),
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Work".to_string(),
        account_identifier: None,
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: "C:\\profiles\\tray2".to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.insert_account(&acc1).unwrap();
    db.insert_account(&acc2).unwrap();

    // Create 1 workspace
    let ws = WorkspacePreset {
        id: "ws-tray-1".to_string(),
        name: "Main Repo".to_string(),
        directory_path: "H:\\repo".to_string(),
        preferred_codex_account_id: Some(acc1.id.clone()),
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
        last_opened_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.insert_workspace(&ws).unwrap();

    // Add favorites
    assert!(db.toggle_favorite_account("acc-tray-1").unwrap());
    assert!(db.toggle_favorite_account("acc-tray-2").unwrap());
    assert!(db
        .toggle_favorite_workspace("ws-tray-1", Some("codex"))
        .unwrap());

    let list = db.list_favorites().unwrap();
    assert_eq!(list.len(), 3);

    // Delete acc1 -> cascade removes acc-tray-1 favorite
    db.delete_account("acc-tray-1").unwrap();
    let list2 = db.list_favorites().unwrap();
    assert_eq!(list2.len(), 2);
    assert!(!db.is_favorite_account("acc-tray-1").unwrap());
    assert!(db.is_favorite_account("acc-tray-2").unwrap());

    // Delete workspace -> cascade removes workspace favorite
    db.delete_workspace("ws-tray-1").unwrap();
    let list3 = db.list_favorites().unwrap();
    assert_eq!(list3.len(), 1);
    assert_eq!(list3[0].account_id.as_deref(), Some("acc-tray-2"));

    // Toggle remaining favorite off
    assert!(!db.toggle_favorite_account("acc-tray-2").unwrap());
    assert_eq!(db.list_favorites().unwrap().len(), 0);

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_recents_aggregation_and_ordering() {
    let temp_dir = std::env::temp_dir().join(format!(
        "ai_switcher_tray_test_recents_{}",
        uuid::Uuid::new_v4()
    ));
    let db_path = temp_dir.join("test.db");
    let db = Db::init(&db_path).expect("Failed to init db");

    let acc = AccountProfile {
        id: "acc-rec-1".to_string(),
        platform: PlatformType::Claude,
        display_name: "Claude Dev".to_string(),
        account_identifier: Some("claude@dev.com".to_string()),
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: "C:\\profiles\\claude-rec".to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: Some("2026-09-05T11:30:00Z".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.insert_account(&acc).unwrap();

    let ws = WorkspacePreset {
        id: "ws-rec-1".to_string(),
        name: "Client Project".to_string(),
        directory_path: "H:\\client".to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
        last_opened_at: Some("2026-09-05T12:15:00Z".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.insert_workspace(&ws).unwrap();

    let recents = db.get_recent_launches(5).unwrap();
    assert_eq!(recents.len(), 2);
    // Most recent is ws (12:15 > 11:30)
    assert_eq!(recents[0].id, "ws-rec-1");
    assert_eq!(recents[0].kind, "workspace");
    assert_eq!(recents[0].title, "Client Project");

    assert_eq!(recents[1].id, "acc-rec-1");
    assert_eq!(recents[1].kind, "account");
    assert_eq!(recents[1].title, "Claude Dev");
    assert_eq!(recents[1].platform, Some(PlatformType::Claude));

    let _ = std::fs::remove_dir_all(temp_dir);
}
