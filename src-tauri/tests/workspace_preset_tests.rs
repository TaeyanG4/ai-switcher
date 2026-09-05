use std::path::PathBuf;

use ai_switcher_lib::browser::BrowserProfileManager;
use ai_switcher_lib::commands::{
    create_account_impl, create_workspace_directory_impl, create_workspace_impl,
    delete_account_impl, delete_workspace_impl, get_workspace_impl, launch_workspace_preset_impl,
    list_workspaces_impl, update_workspace_impl, AppState,
};
use ai_switcher_lib::db::Db;
use ai_switcher_lib::error::AppError;
use ai_switcher_lib::launcher::{LauncherEngine, ProcessManager};
use ai_switcher_lib::models::{
    CreateAccountInput, CreateWorkspaceInput, ExecutionSurface, LoginMethod, PlatformType,
    UpdateWorkspaceInput,
};

fn setup_test_env() -> (PathBuf, AppState) {
    let test_id = uuid::Uuid::new_v4().to_string();
    let temp_root = std::env::temp_dir().join(format!("ai_switcher_ws_int_test_{}", test_id));
    let db_path = temp_root.join("test_ws.db");
    let profiles_dir = temp_root.join("profiles");
    let browser_profiles_dir = temp_root.join("browser-profiles");

    std::fs::create_dir_all(&profiles_dir).unwrap();
    std::fs::create_dir_all(&browser_profiles_dir).unwrap();

    let db = Db::init(&db_path).expect("Failed to initialize DB");
    let process_manager = ProcessManager::new();
    let launcher_engine = LauncherEngine::new(process_manager);
    let browser_manager = BrowserProfileManager::new(browser_profiles_dir);

    let state = AppState {
        db,
        base_profiles_dir: profiles_dir,
        launcher_engine,
        browser_manager,
    };

    (temp_root, state)
}

#[test]
fn test_workspace_preset_crud_impl() {
    let (temp_root, state) = setup_test_env();

    // 1. Validation: empty name
    let empty_name_input = CreateWorkspaceInput {
        name: "   ".to_string(),
        directory_path: "C:\\Projects\\App".to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    assert!(create_workspace_impl(empty_name_input, &state).is_err());

    // 2. Validation: empty directory path
    let empty_dir_input = CreateWorkspaceInput {
        name: "Project Zero".to_string(),
        directory_path: "   ".to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    assert!(create_workspace_impl(empty_dir_input, &state).is_err());

    // 3. Create valid preset
    let create_input = CreateWorkspaceInput {
        name: "AI Switcher Project".to_string(),
        directory_path: "C:\\Projects\\AI-Switcher".to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    let created = create_workspace_impl(create_input, &state).expect("Create preset failed");
    assert_eq!(created.name, "AI Switcher Project");
    assert_eq!(created.directory_path, "C:\\Projects\\AI-Switcher");
    assert!(created.last_opened_at.is_none());

    // 4. List workspaces
    let list = list_workspaces_impl(&state).expect("List failed");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, created.id);

    // 5. Get workspace
    let fetched = get_workspace_impl(&created.id, &state).expect("Get failed");
    assert_eq!(fetched.name, "AI Switcher Project");

    // 6. Update workspace
    let update_input = UpdateWorkspaceInput {
        id: created.id.clone(),
        name: Some("AI Switcher Core".to_string()),
        directory_path: Some("C:\\Projects\\AI-Switcher-Core".to_string()),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    let updated = update_workspace_impl(update_input, &state).expect("Update failed");
    assert_eq!(updated.name, "AI Switcher Core");
    assert_eq!(updated.directory_path, "C:\\Projects\\AI-Switcher-Core");

    // 7. Delete workspace
    delete_workspace_impl(&created.id, &state).expect("Delete failed");
    assert!(get_workspace_impl(&created.id, &state).is_err());
    let list_after = list_workspaces_impl(&state).expect("List after failed");
    assert_eq!(list_after.len(), 0);

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn test_workspace_preset_foreign_key_cascade() {
    let (temp_root, state) = setup_test_env();

    // Create 3 accounts
    let codex_acc = create_account_impl(
        CreateAccountInput {
            platform: PlatformType::Codex,
            display_name: "Codex Work".to_string(),
            account_identifier: Some("work@openai.com".to_string()),
            login_method: Some(LoginMethod::Google),
            initial_status: Some(ai_switcher_lib::models::AccountStatus::Ready),
            default_workspace_path: None,
            custom_executable_path: None,
            browser_profile_id: None,
        },
        &state,
    )
    .expect("Create Codex account failed");

    let claude_acc = create_account_impl(
        CreateAccountInput {
            platform: PlatformType::Claude,
            display_name: "Claude Team".to_string(),
            account_identifier: Some("team@anthropic.com".to_string()),
            login_method: Some(LoginMethod::Google),
            initial_status: Some(ai_switcher_lib::models::AccountStatus::Ready),
            default_workspace_path: None,
            custom_executable_path: None,
            browser_profile_id: None,
        },
        &state,
    )
    .expect("Create Claude account failed");

    let antigravity_acc = create_account_impl(
        CreateAccountInput {
            platform: PlatformType::Antigravity,
            display_name: "Antigravity Research".to_string(),
            account_identifier: Some("research@google.com".to_string()),
            login_method: Some(LoginMethod::Google),
            initial_status: Some(ai_switcher_lib::models::AccountStatus::Ready),
            default_workspace_path: None,
            custom_executable_path: None,
            browser_profile_id: None,
        },
        &state,
    )
    .expect("Create Antigravity account failed");

    // Create preset referencing all 3
    let create_input = CreateWorkspaceInput {
        name: "Omni Repo".to_string(),
        directory_path: "C:\\Projects\\Omni".to_string(),
        preferred_codex_account_id: Some(codex_acc.id.clone()),
        preferred_claude_account_id: Some(claude_acc.id.clone()),
        preferred_antigravity_account_id: Some(antigravity_acc.id.clone()),
    };
    let preset = create_workspace_impl(create_input, &state).expect("Create preset failed");
    assert_eq!(
        preset.preferred_codex_account_id.as_deref(),
        Some(codex_acc.id.as_str())
    );
    assert_eq!(
        preset.preferred_claude_account_id.as_deref(),
        Some(claude_acc.id.as_str())
    );
    assert_eq!(
        preset.preferred_antigravity_account_id.as_deref(),
        Some(antigravity_acc.id.as_str())
    );

    // Delete the Claude account
    delete_account_impl(&claude_acc.id, true, None, &state).expect("Delete Claude account failed");

    // Preset should still exist, with preferred_claude_account_id set to None via ON DELETE SET NULL
    let reloaded = get_workspace_impl(&preset.id, &state).expect("Get preset failed");
    assert_eq!(reloaded.preferred_claude_account_id, None);
    // Other accounts remain intact
    assert_eq!(
        reloaded.preferred_codex_account_id.as_deref(),
        Some(codex_acc.id.as_str())
    );
    assert_eq!(
        reloaded.preferred_antigravity_account_id.as_deref(),
        Some(antigravity_acc.id.as_str())
    );

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn test_launch_workspace_preset_account_resolution_and_dir_creation() {
    let (temp_root, state) = setup_test_env();

    // Create a target workspace directory path inside temp_root
    let ws_path = temp_root.join("test_project_dir");
    assert!(!ws_path.exists());

    // 1. Launch when no accounts exist -> returns NotFound
    let preset_input = CreateWorkspaceInput {
        name: "Test Project".to_string(),
        directory_path: ws_path.to_string_lossy().to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    let preset = create_workspace_impl(preset_input, &state).expect("Create preset failed");

    let err_no_acc = launch_workspace_preset_impl(
        &preset.id,
        PlatformType::Claude,
        Some(ExecutionSurface::DesktopApp),
        &state,
    );
    assert!(err_no_acc.is_err());

    // 2. Create Claude account
    let claude_acc = create_account_impl(
        CreateAccountInput {
            platform: PlatformType::Claude,
            display_name: "Claude Dev".to_string(),
            account_identifier: None,
            login_method: Some(LoginMethod::Google),
            initial_status: Some(ai_switcher_lib::models::AccountStatus::Ready),
            default_workspace_path: None,
            custom_executable_path: None,
            browser_profile_id: None,
        },
        &state,
    )
    .expect("Create Claude account failed");

    // Link as preferred account
    let update_input = UpdateWorkspaceInput {
        id: preset.id.clone(),
        name: None,
        directory_path: None,
        preferred_codex_account_id: None,
        preferred_claude_account_id: Some(claude_acc.id.clone()),
        preferred_antigravity_account_id: None,
    };
    update_workspace_impl(update_input, &state).expect("Update preset failed");

    // Launch should fail with WorkspaceDirectoryNotFound and NOT silently auto-create directory
    let launch_missing = launch_workspace_preset_impl(
        &preset.id,
        PlatformType::Claude,
        Some(ExecutionSurface::DesktopApp),
        &state,
    );
    match launch_missing {
        Err(AppError::WorkspaceDirectoryNotFound(p)) => {
            assert_eq!(p, ws_path.to_string_lossy().to_string());
        }
        other => panic!("Expected WorkspaceDirectoryNotFound, got {:?}", other),
    }
    assert!(
        !ws_path.exists(),
        "Workspace directory must NOT be silently auto-created"
    );

    // Explicit user action: create_workspace_directory
    create_workspace_directory_impl(&preset.id, &state).expect("Explicit create directory failed");
    assert!(
        ws_path.exists(),
        "Workspace directory should exist after explicit creation"
    );

    // Now launch succeeds
    let launch_res = launch_workspace_preset_impl(
        &preset.id,
        PlatformType::Claude,
        Some(ExecutionSurface::DesktopApp),
        &state,
    );
    assert!(
        launch_res.is_ok(),
        "Launch preset should succeed after directory exists: {:?}",
        launch_res.err()
    );

    // Check that last_opened_at was updated
    let reloaded = get_workspace_impl(&preset.id, &state).expect("Get preset failed");
    assert!(reloaded.last_opened_at.is_some());

    // Locate Folder update flow: point preset to a relocated folder
    let relocated_dir = temp_root.join("relocated_project");
    std::fs::create_dir_all(&relocated_dir).unwrap();
    let relocate_input = UpdateWorkspaceInput {
        id: preset.id.clone(),
        name: None,
        directory_path: Some(relocated_dir.to_string_lossy().to_string()),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
    };
    update_workspace_impl(relocate_input, &state).expect("Relocate update failed");

    let launch_relocated = launch_workspace_preset_impl(
        &preset.id,
        PlatformType::Claude,
        Some(ExecutionSurface::DesktopApp),
        &state,
    );
    assert!(
        launch_relocated.is_ok(),
        "Launch should succeed after locating new folder: {:?}",
        launch_relocated.err()
    );

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn test_antigravity_workspace_launch_cwd_only() {
    let (temp_root, state) = setup_test_env();

    // Create target directory
    let ws_path = temp_root.join("antigravity_workspace");
    std::fs::create_dir_all(&ws_path).unwrap();

    let antigravity_acc = create_account_impl(
        CreateAccountInput {
            platform: PlatformType::Antigravity,
            display_name: "Antigravity Dev".to_string(),
            account_identifier: None,
            login_method: Some(LoginMethod::Google),
            initial_status: Some(ai_switcher_lib::models::AccountStatus::Ready),
            default_workspace_path: None,
            custom_executable_path: None,
            browser_profile_id: None,
        },
        &state,
    )
    .expect("Create Antigravity account failed");

    let preset = create_workspace_impl(
        CreateWorkspaceInput {
            name: "AG Project".to_string(),
            directory_path: ws_path.to_string_lossy().to_string(),
            preferred_codex_account_id: None,
            preferred_claude_account_id: None,
            preferred_antigravity_account_id: Some(antigravity_acc.id.clone()),
        },
        &state,
    )
    .expect("Create preset failed");

    // Launch Antigravity workspace
    let launch_res = launch_workspace_preset_impl(
        &preset.id,
        PlatformType::Antigravity,
        Some(ExecutionSurface::DesktopApp),
        &state,
    );
    assert!(
        launch_res.is_ok(),
        "Antigravity launch failed: {:?}",
        launch_res.err()
    );

    // Verify adapter build_launch_spec directly:
    // working_directory MUST be ws_path, and arguments must NOT contain ws_path as a positional argument!
    let adapter = ai_switcher_lib::adapters::get_adapter(PlatformType::Antigravity);
    let spec = adapter
        .build_launch_spec(
            &antigravity_acc,
            ai_switcher_lib::models::LaunchTarget::Desktop,
            Some(&ws_path),
        )
        .expect("build_launch_spec failed");

    assert_eq!(
        spec.working_directory, ws_path,
        "Working directory must match workspace path"
    );
    for arg in &spec.arguments {
        assert!(
            !arg.contains("antigravity_workspace"),
            "Positional workspace path argument must NOT be passed to Antigravity CLI args: arg='{}'",
            arg
        );
    }

    let _ = std::fs::remove_dir_all(temp_root);
}
