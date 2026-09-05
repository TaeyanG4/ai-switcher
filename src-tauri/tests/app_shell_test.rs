use ai_switcher_lib::commands::AppState;
use ai_switcher_lib::db::Db;
use ai_switcher_lib::models::{CreateAccountInput, LoginMethod, PlatformType, UpdateAccountInput};

#[test]
fn test_app_shell_end_to_end_flow() {
    let temp_root =
        std::env::temp_dir().join(format!("ai_switcher_shell_test_{}", uuid::Uuid::new_v4()));
    let db_path = temp_root.join("test.db");
    let profiles_dir = temp_root.join("profiles");

    let db = Db::init(&db_path).expect("Failed to initialize test DB");
    let pm = ai_switcher_lib::launcher::ProcessManager::new();
    let launcher_engine = ai_switcher_lib::launcher::LauncherEngine::new(pm);
    let browser_manager =
        ai_switcher_lib::browser::BrowserProfileManager::new(temp_root.join("browser-profiles"));
    let state = AppState {
        db: db.clone(),
        base_profiles_dir: profiles_dir.clone(),
        launcher_engine,
        browser_manager,
    };

    // 1. Initially empty
    let initial_accounts = state
        .db
        .list_accounts(true)
        .expect("Failed to list accounts");
    assert_eq!(initial_accounts.len(), 0);

    // 2. Create Codex Account Profile
    let codex_input = CreateAccountInput {
        platform: PlatformType::Codex,
        display_name: "Personal 1".to_string(),
        account_identifier: Some("user@openai.com".to_string()),
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: None,
    };

    let codex_id = uuid::Uuid::new_v4().to_string();
    let codex_profile_path = profiles_dir.join("codex").join(&codex_id);
    let adapter = ai_switcher_lib::adapters::get_adapter(PlatformType::Codex);
    adapter
        .initialize_profile(&codex_profile_path)
        .expect("Failed to init profile");
    assert!(
        codex_profile_path.join("config.toml").exists(),
        "config.toml should exist"
    );

    let codex_account = ai_switcher_lib::models::AccountProfile {
        id: codex_id.clone(),
        platform: PlatformType::Codex,
        display_name: codex_input.display_name,
        account_identifier: codex_input.account_identifier,
        login_method: LoginMethod::Google,
        status: ai_switcher_lib::models::AccountStatus::Ready,
        auth_status: ai_switcher_lib::models::AuthStatus::Authenticated,
        runtime_status: ai_switcher_lib::models::RuntimeStatus::Stopped,
        profile_path: codex_profile_path.to_string_lossy().to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: std::collections::HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    state
        .db
        .insert_account(&codex_account)
        .expect("Insert account failed");

    // 3. Create Claude Account Profile
    let claude_id = uuid::Uuid::new_v4().to_string();
    let claude_profile_path = profiles_dir.join("claude").join(&claude_id);
    let claude_adapter = ai_switcher_lib::adapters::get_adapter(PlatformType::Claude);
    claude_adapter
        .initialize_profile(&claude_profile_path)
        .expect("Failed to init claude");
    assert!(
        claude_profile_path.join("projects").exists(),
        "projects dir should exist"
    );

    let claude_account = ai_switcher_lib::models::AccountProfile {
        id: claude_id.clone(),
        platform: PlatformType::Claude,
        display_name: "Work Claude".to_string(),
        account_identifier: Some("work@anthropic.com".to_string()),
        login_method: LoginMethod::EmailOtp,
        status: ai_switcher_lib::models::AccountStatus::Ready,
        auth_status: ai_switcher_lib::models::AuthStatus::Authenticated,
        runtime_status: ai_switcher_lib::models::RuntimeStatus::Stopped,
        profile_path: claude_profile_path.to_string_lossy().to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: std::collections::HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    state
        .db
        .insert_account(&claude_account)
        .expect("Insert claude failed");

    // 4. Verify listing
    let list = state.db.list_accounts(false).expect("List failed");
    assert_eq!(list.len(), 2);

    // 5. Test Update
    let updated = state
        .db
        .update_account(&UpdateAccountInput {
            id: codex_id.clone(),
            display_name: Some("Personal 1 (Renamed)".to_string()),
            account_identifier: None,
            default_workspace_path: Some("C:\\Projects\\MyProject".to_string()),
            custom_executable_path: None,
            browser_profile_id: None,
            is_enabled: None,
        })
        .expect("Update failed");
    assert_eq!(updated.display_name, "Personal 1 (Renamed)");
    assert_eq!(
        updated.default_workspace_path,
        Some("C:\\Projects\\MyProject".to_string())
    );

    // 6. Test Soft-Disable
    state
        .db
        .toggle_account_enabled(&claude_id, false)
        .expect("Toggle failed");
    let active_only = state.db.list_accounts(false).expect("Active list failed");
    assert_eq!(active_only.len(), 1);
    let all_including_disabled = state.db.list_accounts(true).expect("All list failed");
    assert_eq!(all_including_disabled.len(), 2);

    // 7. Test Delete
    state.db.delete_account(&codex_id).expect("Delete failed");
    let after_delete = state
        .db
        .list_accounts(true)
        .expect("List after delete failed");
    assert_eq!(after_delete.len(), 1);
    assert_eq!(after_delete[0].id, claude_id);

    // Cleanup
    let _ = std::fs::remove_dir_all(temp_root);
}
