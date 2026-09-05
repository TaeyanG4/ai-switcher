use rusqlite::Connection;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use ai_switcher_lib::adapters::LaunchSpec;
use ai_switcher_lib::browser::launcher::redact_url_for_diagnostics;
use ai_switcher_lib::browser::BrowserProfileManager;
use ai_switcher_lib::commands::{
    check_account_health_impl, export_support_bundle_impl, repair_account_profile_impl, AppState,
};
use ai_switcher_lib::db::Db;
use ai_switcher_lib::fs_safety::safe_remove_dir_all;
use ai_switcher_lib::launcher::desktop::DesktopAppLauncher;
use ai_switcher_lib::launcher::process_manager::ProcessManager;
use ai_switcher_lib::launcher::LauncherEngine;
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, ExecutionSurface, LoginMethod, PlatformType, ProfileHealth,
    WorkspacePreset,
};

struct TestEnv {
    base_dir: PathBuf,
    db: Db,
    db_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let unique = Uuid::new_v4().to_string();
        let base_dir = std::env::temp_dir().join(format!("ai_switcher_rel_test_{}", unique));
        fs::create_dir_all(&base_dir).unwrap();

        let db_path = base_dir.join("test.db");
        let db = Db::init(&db_path).unwrap();

        Self {
            base_dir,
            db,
            db_path,
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base_dir);
    }
}

#[test]
fn test_reparse_point_and_junction_safe_deletion() {
    let env = TestEnv::new();
    let managed_profiles_root = env.base_dir.join("profiles");
    fs::create_dir_all(&managed_profiles_root).unwrap();

    let external_user_folder = env.base_dir.join("ExternalUserDocuments");
    fs::create_dir_all(external_user_folder.join("nested_folder")).unwrap();
    let secret_file = external_user_folder.join("precious_project_code.rs");
    fs::write(
        &secret_file,
        "fn important() { println!(\"Do not delete me!\"); }",
    )
    .unwrap();
    let nested_file = external_user_folder.join("nested_folder").join("file.txt");
    fs::write(&nested_file, "Nested content").unwrap();

    let account_profile_dir = managed_profiles_root.join("claude_account_1");
    fs::create_dir_all(account_profile_dir.join("desktop")).unwrap();
    fs::write(account_profile_dir.join("desktop").join("state.json"), "{}").unwrap();

    // Create a directory junction or symlink pointing from inside the profile to the external folder
    let link_path = account_profile_dir.join("linked_external_docs");
    #[cfg(windows)]
    {
        // Try creating a junction via cmd mklink /J
        let status = std::process::Command::new("cmd")
            .args([
                "/c",
                "mklink",
                "/J",
                link_path.to_str().unwrap(),
                external_user_folder.to_str().unwrap(),
            ])
            .output();

        if let Ok(output) = status {
            if !output.status.success() {
                // Fallback to directory symlink if junction creation requires privileges
                let _ = std::os::windows::fs::symlink_dir(&external_user_folder, &link_path);
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = std::os::unix::fs::symlink(&external_user_folder, &link_path);
    }

    // Now call safe_remove_dir_all on the account profile directory
    let res = safe_remove_dir_all(&managed_profiles_root, &account_profile_dir);
    assert!(res.is_ok(), "Safe deletion failed: {:?}", res.err());

    // Invariant: The account profile directory MUST be gone
    assert!(!account_profile_dir.exists(), "Profile dir was not deleted");

    // CRITICAL INVARIANT: The external user folder and its files MUST remain completely intact!
    assert!(
        external_user_folder.exists(),
        "External user folder was incorrectly deleted!"
    );
    assert!(
        secret_file.exists(),
        "External file was destroyed by junction traversal!"
    );
    let content = fs::read_to_string(&secret_file).unwrap();
    assert_eq!(
        content,
        "fn important() { println!(\"Do not delete me!\"); }"
    );
    assert!(nested_file.exists(), "External nested file was destroyed!");
}

#[test]
fn test_workspace_preset_deletion_preserves_user_files() {
    let env = TestEnv::new();

    // Create real user workspace folder with project files
    let user_project_dir = env.base_dir.join("my_rust_project");
    fs::create_dir_all(user_project_dir.join("src")).unwrap();
    let main_rs = user_project_dir.join("src").join("main.rs");
    fs::write(&main_rs, "fn main() { println!(\"Preserve me!\"); }").unwrap();

    let preset = WorkspacePreset {
        id: "ws-preset-1".to_string(),
        name: "My Project".to_string(),
        directory_path: user_project_dir.to_str().unwrap().to_string(),
        preferred_codex_account_id: None,
        preferred_claude_account_id: None,
        preferred_antigravity_account_id: None,
        last_opened_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    env.db.insert_workspace(&preset).unwrap();
    assert!(env.db.get_workspace("ws-preset-1").is_ok());

    // Delete workspace preset metadata
    env.db.delete_workspace("ws-preset-1").unwrap();
    assert!(env.db.get_workspace("ws-preset-1").is_err());

    // CRITICAL INVARIANT: The actual project directory and all files MUST remain 100% intact
    assert!(
        user_project_dir.exists(),
        "User project directory was deleted!"
    );
    assert!(main_rs.exists(), "User project files were deleted!");
    let content = fs::read_to_string(&main_rs).unwrap();
    assert_eq!(content, "fn main() { println!(\"Preserve me!\"); }");
}

#[test]
fn test_database_backup_and_retention() {
    let env = TestEnv::new();

    // Create 7 backups
    for _ in 0..7 {
        let _ = env.db.backup_database().unwrap();
        // small sleep to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    let (count, latest) = env.db.get_backup_info().unwrap();
    // Retention policy keeps at most 5 backups
    assert_eq!(count, 5, "Retention did not cap backups at 5");
    assert!(latest.is_some(), "Latest backup filename should exist");

    // Verify backup file can be opened as a valid SQLite DB
    let backup_dir = env.db_path.parent().unwrap().join("backups");
    let backup_file = backup_dir.join(latest.unwrap());
    assert!(backup_file.exists());

    let backup_conn = Connection::open(&backup_file).unwrap();
    let integrity: String = backup_conn
        .query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
}

#[test]
fn test_database_integrity_and_schema_version() {
    let env = TestEnv::new();
    assert!(env.db.check_integrity().unwrap());
    assert_eq!(env.db.get_schema_version().unwrap(), 4);
}

#[test]
fn test_database_migration_v1_to_v4() {
    let unique = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("ai_switcher_mig_test_{}", unique));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("v1_test.db");

    // Manually create a v1 database schema fixture
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE accounts (
                id TEXT PRIMARY KEY NOT NULL,
                platform TEXT NOT NULL,
                display_name TEXT NOT NULL,
                account_identifier TEXT,
                login_method TEXT NOT NULL,
                status TEXT NOT NULL,
                profile_path TEXT NOT NULL,
                browser_profile_path TEXT,
                custom_executable_path TEXT,
                launch_arguments TEXT NOT NULL,
                environment_variables TEXT NOT NULL,
                default_workspace_path TEXT,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                last_launched_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE workspaces (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                directory_path TEXT NOT NULL UNIQUE,
                preferred_codex_account_id TEXT,
                preferred_claude_account_id TEXT,
                preferred_antigravity_account_id TEXT,
                last_opened_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE app_settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            PRAGMA user_version = 1;
            ",
        )
        .unwrap();

        // Insert a fixture account in v1
        conn.execute(
            "INSERT INTO accounts (id, platform, display_name, login_method, status, profile_path, launch_arguments, environment_variables, is_enabled, created_at, updated_at)
             VALUES ('acc-1', 'claude', 'Claude V1', 'oauth', 'ready', 'C:/profiles/1', '[]', '{}', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            [],
        )
        .unwrap();
    }

    // Now open via Db::init, which must run migrations up to v4
    let upgraded_db = Db::init(&db_path).unwrap();
    assert_eq!(upgraded_db.get_schema_version().unwrap(), 4);

    // Verify existing v1 data survived intact
    let acc = upgraded_db.get_account("acc-1").unwrap();
    assert_eq!(acc.display_name, "Claude V1");
    assert_eq!(acc.platform, PlatformType::Claude);
    assert_eq!(acc.browser_profile_id, None);

    // Verify v2/v3/v4 tables exist and work
    assert!(upgraded_db.list_favorites().is_ok());
    assert!(upgraded_db.list_browser_profiles().is_ok());

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_profile_health_and_repair_structure() {
    let env = TestEnv::new();
    let profiles_dir = env.base_dir.join("profiles");
    let browser_dir = env.base_dir.join("browser-profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    fs::create_dir_all(&browser_dir).unwrap();

    let account_id = "test-antigravity-repair";
    let profile_path = profiles_dir.join(account_id);

    let account = AccountProfile {
        id: account_id.to_string(),
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Dev".to_string(),
        account_identifier: Some("dev@google.com".to_string()),
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        profile_path: profile_path.to_str().unwrap().to_string(),
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

    env.db.insert_account(&account).unwrap();

    // 1. Initially profile directory does NOT exist
    let report_missing = check_account_health_impl(&account, &profiles_dir, &browser_dir, &env.db);
    assert_eq!(
        report_missing.health,
        ProfileHealth::ProfileDirectoryMissing
    );
    assert!(report_missing.can_repair);

    // 2. Setup AppState and perform repair
    let proc_mgr = ProcessManager::new();
    let launcher = LauncherEngine::new(proc_mgr);
    let browser_mgr = BrowserProfileManager::new(browser_dir.clone());

    let app_state = AppState {
        db: env.db.clone(),
        base_profiles_dir: profiles_dir.clone(),
        launcher_engine: launcher,
        browser_manager: browser_mgr,
    };

    let repaired_report = repair_account_profile_impl(account_id, &app_state).unwrap();

    // 3. Verify scaffolding was created
    assert!(profile_path.exists());
    assert!(profile_path.join("home").exists());
    assert!(profile_path.join("data").exists());
    assert!(profile_path.join("appdata").exists());

    // 4. Invariant: Status transitioned to LoginRequired, never manufactured credentials
    let updated_account = env.db.get_account(account_id).unwrap();
    assert_eq!(updated_account.status, AccountStatus::LoginRequired);
    assert_eq!(
        repaired_report.health,
        ProfileHealth::AuthenticationRequired
    );
}

#[test]
fn test_stale_pid_reuse_protection() {
    let proc_mgr = ProcessManager::new();

    // Register a fake process with an impossible or mismatched PID
    let record = proc_mgr.register_launch(
        "acc-test",
        PlatformType::Codex,
        ExecutionSurface::Cli,
        Some(999999), // non-existent PID
        "C:\\NonExistent\\codex.exe",
    );

    assert!(record.is_running);

    // Calling list_active_processes updates running state by querying OS table
    let active = proc_mgr.list_active_processes();
    assert_eq!(
        active.len(),
        0,
        "Non-existent PID was falsely reported as running"
    );
    assert!(!proc_mgr.is_account_running("acc-test"));
}

#[test]
fn test_adversarial_command_argument_injection() {
    let spec = LaunchSpec {
        executable: PathBuf::from("C:\\Program Files\\App\\app.exe"),
        arguments: vec![
            "normal_arg".to_string(),
            "arg with spaces".to_string(),
            "\"quoted_arg\"".to_string(),
            "--evil-flag; rm -rf /".to_string(),
            "한글_경로_테스트".to_string(),
            "line1\nline2".to_string(),
        ],
        environment: vec![("SAFE_VAR".to_string(), "safe_val".to_string())],
        working_directory: PathBuf::from("C:\\Projects"),
        is_terminal: false,
    };

    let cmd = DesktopAppLauncher::build_command(&spec);
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    assert_eq!(args.len(), 6);
    assert_eq!(args[0], "normal_arg");
    assert_eq!(args[1], "arg with spaces");
    assert_eq!(args[2], "\"quoted_arg\"");
    assert_eq!(args[3], "--evil-flag; rm -rf /");
    assert_eq!(args[4], "한글_경로_테스트");
    assert_eq!(args[5], "line1\nline2");
}

#[test]
fn test_comprehensive_url_redaction() {
    let raw_url = "https://auth.openai.com/oauth?client_id=12345&code=secret_auth_code_999&state=random_state_xyz&redirect_uri=http://localhost:8080/callback";
    let redacted = redact_url_for_diagnostics(raw_url);

    assert!(!redacted.contains("secret_auth_code_999"));
    assert!(!redacted.contains("random_state_xyz"));
    assert!(redacted.contains("code=[REDACTED]"));
    assert!(redacted.contains("state=[REDACTED]"));
    assert!(redacted.contains("client_id=12345"));
}

#[test]
fn test_corrupt_settings_fallback() {
    let env = TestEnv::new();

    // Insert completely invalid and malformed setting value in database
    env.db
        .set_setting("app_settings_json", "{ this is invalid json !!! }")
        .unwrap();

    // get_app_settings must fall back to safe defaults without crashing
    let settings = env.db.get_app_settings().unwrap_or_default();
    assert_eq!(settings.theme, "system");
    assert!(settings.close_to_tray);
    assert_eq!(settings.global_shortcut, "CommandOrControl+Alt+S");
}

#[test]
fn test_support_bundle_is_sanitized() {
    let env = TestEnv::new();
    let profiles_dir = env.base_dir.join("profiles");
    let browser_dir = env.base_dir.join("browser-profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    fs::create_dir_all(&browser_dir).unwrap();

    let proc_mgr = ProcessManager::new();
    let launcher = LauncherEngine::new(proc_mgr);
    let browser_mgr = BrowserProfileManager::new(browser_dir);

    let app_state = AppState {
        db: env.db.clone(),
        base_profiles_dir: profiles_dir,
        launcher_engine: launcher,
        browser_manager: browser_mgr,
    };

    let bundle = export_support_bundle_impl(&app_state).unwrap();
    let serialized = serde_json::to_string(&bundle).unwrap();

    // Verify critical redaction: zero tokens, zero cookies, zero secrets
    assert!(!serialized.contains("access_token"));
    assert!(!serialized.contains("refresh_token"));
    assert!(!serialized.contains("cookie"));
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("client_secret"));
    assert!(bundle.db_integrity_ok);
    assert_eq!(bundle.schema_version, 4);
}
