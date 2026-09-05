use std::path::PathBuf;

use ai_switcher_lib::adapters::get_adapter;
use ai_switcher_lib::browser::BrowserProfileManager;
use ai_switcher_lib::commands::{
    cleanup_draft_account_impl, create_account_impl, create_browser_profile_impl,
    delete_account_impl, list_platform_capabilities, AppState,
};
use ai_switcher_lib::db::Db;
use ai_switcher_lib::launcher::{LauncherEngine, ProcessManager};
use ai_switcher_lib::models::{
    AccountStatus, BrowserKind, CreateAccountInput, CreateBrowserProfileInput, ExecutionSurface,
    InstancePolicy, LoginMethod, PlatformType,
};

fn setup_test_env() -> (PathBuf, AppState) {
    let test_id = uuid::Uuid::new_v4().to_string();
    let temp_root = std::env::temp_dir().join(format!("ai_switcher_wizard_test_{}", test_id));
    let db_path = temp_root.join("test_wizard.db");
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
fn test_platform_capabilities_truthfulness() {
    let capabilities = list_platform_capabilities().expect("Failed to list platform capabilities");
    assert_eq!(capabilities.len(), 3);

    // 1. OpenAI Codex
    let codex_cap = capabilities
        .iter()
        .find(|c| c.platform == PlatformType::Codex)
        .expect("Codex capabilities missing");
    assert_eq!(codex_cap.display_name, "OpenAI Codex");
    assert_eq!(codex_cap.primary_surface, ExecutionSurface::DesktopApp);
    assert!(codex_cap
        .supported_surfaces
        .contains(&ExecutionSurface::DesktopApp));
    assert!(codex_cap
        .supported_surfaces
        .contains(&ExecutionSurface::Cli));
    // CRITICAL: Desktop isolation is NOT supported / verified for Codex
    assert!(!codex_cap.supports_desktop_isolation);
    assert_eq!(codex_cap.instance_policy, InstancePolicy::SingleInstance);
    assert!(codex_cap.isolation_summary.contains("CODEX_HOME"));
    assert!(codex_cap.isolation_summary.contains("single-instance"));
    assert!(codex_cap.supports_automated_status_probe);

    // 2. Anthropic Claude
    let claude_cap = capabilities
        .iter()
        .find(|c| c.platform == PlatformType::Claude)
        .expect("Claude capabilities missing");
    assert_eq!(claude_cap.display_name, "Anthropic Claude");
    assert_eq!(claude_cap.primary_surface, ExecutionSurface::DesktopApp);
    assert!(claude_cap
        .supported_surfaces
        .contains(&ExecutionSurface::DesktopApp));
    assert!(claude_cap
        .supported_surfaces
        .contains(&ExecutionSurface::Cli));
    assert!(claude_cap
        .supported_surfaces
        .contains(&ExecutionSurface::Web));
    assert!(claude_cap.supports_desktop_isolation);
    assert_eq!(claude_cap.instance_policy, InstancePolicy::MultiInstance);

    // 3. Google Antigravity
    let antigravity_cap = capabilities
        .iter()
        .find(|c| c.platform == PlatformType::Antigravity)
        .expect("Antigravity capabilities missing");
    assert_eq!(antigravity_cap.display_name, "Google Antigravity");
    assert_eq!(
        antigravity_cap.primary_surface,
        ExecutionSurface::DesktopApp
    );
    assert!(antigravity_cap
        .supported_surfaces
        .contains(&ExecutionSurface::DesktopApp));
    assert!(antigravity_cap.supports_desktop_isolation);
    assert_eq!(
        antigravity_cap.instance_policy,
        InstancePolicy::MultiInstance
    );
}

#[test]
fn test_create_account_initial_status_login_required() {
    let (_temp, state) = setup_test_env();

    // Default without initial_status should be LoginRequired
    let input_default = CreateAccountInput {
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Work".to_string(),
        account_identifier: Some("work@google.com".to_string()),
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: None,
    };

    let created = create_account_impl(input_default, &state).expect("Failed to create account");
    assert_eq!(created.status, AccountStatus::LoginRequired);
    assert!(PathBuf::from(&created.profile_path).exists());

    // Explicit initial_status should be honored
    let input_explicit = CreateAccountInput {
        platform: PlatformType::Claude,
        display_name: "Claude Ready".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: Some(AccountStatus::Ready),
    };

    let created_explicit =
        create_account_impl(input_explicit, &state).expect("Failed to create explicit account");
    assert_eq!(created_explicit.status, AccountStatus::Ready);
}

#[test]
fn test_cleanup_draft_account_rollback() {
    let (_temp, state) = setup_test_env();

    let input = CreateAccountInput {
        platform: PlatformType::Claude,
        display_name: "Draft Cancelled".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: None,
    };

    let created = create_account_impl(input, &state).expect("Failed to create draft account");
    let profile_dir = PathBuf::from(&created.profile_path);
    assert!(profile_dir.exists(), "Profile dir should be scaffolded");
    assert!(state.db.get_account(&created.id).is_ok());

    // Execute cleanup rollback
    cleanup_draft_account_impl(&created.id, &state).expect("Failed to cleanup draft");

    assert!(
        !profile_dir.exists(),
        "Profile dir should be cleaned up on cancellation"
    );
    assert!(
        state.db.get_account(&created.id).is_err(),
        "Account should be deleted from DB"
    );
}

#[test]
fn test_login_spec_construction() {
    let (_temp, state) = setup_test_env();

    // 1. Codex login spec
    let codex_input = CreateAccountInput {
        platform: PlatformType::Codex,
        display_name: "Codex Profile".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: None,
    };
    let codex_acc = create_account_impl(codex_input, &state).unwrap();
    let codex_adapter = get_adapter(PlatformType::Codex);
    let codex_spec = codex_adapter
        .build_login_spec(&codex_acc)
        .expect("Failed to build Codex login spec");
    assert!(codex_spec.is_terminal, "Codex login must run in a terminal");
    assert_eq!(codex_spec.arguments, vec!["login"]);
    assert!(
        codex_spec
            .environment
            .iter()
            .any(|(k, v)| k == "CODEX_HOME" && v == &codex_acc.profile_path),
        "CODEX_HOME must be set to the isolated profile path"
    );

    // 2. Antigravity login spec
    let antigravity_input = CreateAccountInput {
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Profile".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: None,
    };
    let ag_acc = create_account_impl(antigravity_input, &state).unwrap();
    let ag_adapter = get_adapter(PlatformType::Antigravity);
    let ag_spec = ag_adapter
        .build_login_spec(&ag_acc)
        .expect("Failed to build Antigravity login spec");
    assert!(
        !ag_spec.is_terminal,
        "Antigravity login should be native desktop application"
    );
    assert!(
        ag_spec
            .arguments
            .iter()
            .any(|a| a.starts_with("--user-data-dir=")),
        "Antigravity desktop must isolate user-data-dir"
    );
    assert!(
        ag_spec.environment.iter().any(|(k, _)| k == "USERPROFILE"),
        "Antigravity desktop must isolate USERPROFILE for .gemini"
    );
}

#[test]
fn test_shared_browser_profile_deletion_protection() {
    let (_temp, state) = setup_test_env();

    // 1. Create a Browser Profile
    let bp_input = CreateBrowserProfileInput {
        display_name: "Work Browser".to_string(),
        browser_kind: BrowserKind::Chrome,
        custom_executable_path: None,
    };
    let bp = create_browser_profile_impl(bp_input, &state).unwrap();

    // 2. Create Account 1 linked to BP
    let acc1_input = CreateAccountInput {
        platform: PlatformType::Claude,
        display_name: "Claude Work 1".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: Some(bp.id.clone()),
        initial_status: None,
    };
    let acc1 = create_account_impl(acc1_input, &state).unwrap();

    // 3. Create Account 2 linked to the same BP
    let acc2_input = CreateAccountInput {
        platform: PlatformType::Claude,
        display_name: "Claude Work 2".to_string(),
        account_identifier: None,
        login_method: Some(LoginMethod::Google),
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: Some(bp.id.clone()),
        initial_status: None,
    };
    let acc2 = create_account_impl(acc2_input, &state).unwrap();

    // 4. Attempt to delete Account 1 with delete_browser_profile: Some(true)
    let del_result = delete_account_impl(&acc1.id, true, Some(true), &state);
    assert!(
        del_result.is_err(),
        "Must error and refuse to delete browser profile that is shared with Account 2"
    );

    // Browser profile must still exist in DB and disk
    assert!(state.db.get_browser_profile(&bp.id).is_ok());

    // 5. Delete Account 1 without deleting browser profile
    delete_account_impl(&acc1.id, true, Some(false), &state)
        .expect("Should succeed deleting Account 1 without deleting browser profile");
    assert!(state.db.get_account(&acc1.id).is_err());
    assert!(state.db.get_browser_profile(&bp.id).is_ok());

    // 6. Now only Account 2 references the browser profile. Delete Account 2 with delete_browser_profile: Some(true)
    let del_acc2_result = delete_account_impl(&acc2.id, true, Some(true), &state);
    assert!(
        del_acc2_result.is_ok(),
        "Should succeed deleting browser profile when referenced by only 1 account"
    );
    assert!(state.db.get_account(&acc2.id).is_err());
    assert!(
        state.db.get_browser_profile(&bp.id).is_err(),
        "Browser profile should be deleted"
    );
}
