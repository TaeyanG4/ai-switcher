use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use ai_switcher_lib::browser::BrowserProfileManager;
use ai_switcher_lib::commands::{create_account_impl, start_login_flow_impl, AppState};
use ai_switcher_lib::db::Db;
use ai_switcher_lib::launcher::process_manager::ProcessManager;
use ai_switcher_lib::launcher::LauncherEngine;
use ai_switcher_lib::models::{
    AccountStatus, AuthStatus, CreateAccountInput, ExecutionSurface, PlatformType, RuntimeStatus,
};
use ai_switcher_lib::protocol_broker::{
    cleanup_pending_auth, list_pending_auth_flows, redact_url, register_pending_auth,
};

struct TestFixture {
    temp_dir: PathBuf,
    state: AppState,
}

impl TestFixture {
    fn new() -> Self {
        let unique = Uuid::new_v4().to_string();
        let temp_dir = std::env::temp_dir().join(format!("ai_switcher_broker_test_{}", unique));
        fs::create_dir_all(&temp_dir).unwrap();

        let db_path = temp_dir.join("test.db");
        let db = Db::init(&db_path).unwrap();

        let base_profiles_dir = temp_dir.join("profiles");
        fs::create_dir_all(&base_profiles_dir).unwrap();

        let browser_base_dir = temp_dir.join("browsers");
        fs::create_dir_all(&browser_base_dir).unwrap();
        let browser_manager = BrowserProfileManager::new(browser_base_dir);

        let process_manager = ProcessManager::new();
        let launcher_engine = LauncherEngine::new(process_manager);

        let state = AppState {
            db,
            base_profiles_dir,
            launcher_engine,
            browser_manager,
        };

        Self { temp_dir, state }
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

#[test]
fn test_url_redaction_removes_sensitive_query_parameters() {
    let raw =
        "antigravity://auth/callback?code=super_secret_oauth_code_12345&state=random_state_xyz";
    let redacted = redact_url(raw);
    assert_eq!(redacted, "antigravity://auth/callback?[QUERY REDACTED]");
    assert!(!redacted.contains("super_secret_oauth_code_12345"));
    assert!(!redacted.contains("random_state_xyz"));

    let no_query = "claude://code/new";
    assert_eq!(redact_url(no_query), "claude://code/new");
}

#[test]
fn test_protocol_broker_pending_auth_lifecycle() {
    let account_id = format!("test-acc-{}", Uuid::new_v4());
    let profile_path = format!("C:\\profiles\\{}", account_id);

    // Register flow
    register_pending_auth(
        &account_id,
        PlatformType::Antigravity,
        ExecutionSurface::DesktopApp,
        &profile_path,
        None,
        Some(12345),
    );

    let flows = list_pending_auth_flows();
    let flow = flows.iter().find(|f| f.account_id == account_id);
    assert!(
        flow.is_some(),
        "Registered flow must be present in pending list"
    );
    let flow = flow.unwrap();
    assert_eq!(flow.platform, PlatformType::Antigravity);
    assert_eq!(flow.profile_path, profile_path);
    assert_eq!(flow.helper_pid, Some(12345));

    // Cleanup flow
    cleanup_pending_auth(Some(&account_id), None);
    let flows_after = list_pending_auth_flows();
    assert!(
        flows_after.iter().all(|f| f.account_id != account_id),
        "Cleaned up flow must not exist"
    );
}

#[test]
fn test_auth_status_and_runtime_status_decoupling() {
    let fixture = TestFixture::new();

    // 1. Create account: starts with LoginRequired / Authenticated if specified
    let input = CreateAccountInput {
        platform: PlatformType::Antigravity,
        display_name: "Work Antigravity".to_string(),
        account_identifier: Some("work@example.com".to_string()),
        login_method: None,
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: Some(AccountStatus::LoginRequired),
    };

    let acc = create_account_impl(input, &fixture.state).expect("Create account failed");
    assert_eq!(acc.status, AccountStatus::LoginRequired);
    assert_eq!(acc.auth_status, AuthStatus::LoginRequired);
    assert_eq!(acc.runtime_status, RuntimeStatus::Stopped);

    // 2. Querying list_accounts computes dynamic runtime_status
    let accounts = fixture.state.db.list_accounts(false).unwrap();
    let fetched = accounts.iter().find(|a| a.id == acc.id).unwrap();
    assert_eq!(fetched.auth_status, AuthStatus::LoginRequired);

    // Simulate process running in process_manager
    let current_exe = std::env::current_exe().unwrap();
    fixture
        .state
        .launcher_engine
        .process_manager
        .register_launch(
            &acc.id,
            PlatformType::Antigravity,
            ExecutionSurface::DesktopApp,
            Some(std::process::id()),
            &current_exe.to_string_lossy(),
        );

    // Verify dynamic runtime_status is Running
    let is_running = fixture
        .state
        .launcher_engine
        .process_manager
        .has_running_process(&acc.id);
    assert!(is_running);

    // 3. Updating auth status does not affect runtime status
    fixture
        .state
        .db
        .update_account_auth_status(&acc.id, AuthStatus::Authenticated)
        .unwrap();
    let updated = fixture.state.db.get_account(&acc.id).unwrap();
    assert_eq!(updated.auth_status, AuthStatus::Authenticated);
    assert_eq!(updated.status, AccountStatus::Ready);
}

#[test]
fn test_start_login_flow_returns_auth_flow_start_result() {
    let fixture = TestFixture::new();

    let input = CreateAccountInput {
        platform: PlatformType::Codex,
        display_name: "Codex Profile".to_string(),
        account_identifier: None,
        login_method: None,
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: Some(AccountStatus::LoginRequired),
    };

    let acc = create_account_impl(input, &fixture.state).expect("Create failed");
    let result = start_login_flow_impl(&acc.id, &fixture.state);

    assert!(
        result.is_ok(),
        "start_login_flow must succeed: {:?}",
        result.err()
    );
    let res = result.unwrap();
    assert_eq!(res.platform, PlatformType::Codex);
    assert_eq!(res.flow_type, "cli_oauth");
    assert!(res.process_started);
    assert!(res.message.contains("Official sign-in opened"));
}

#[test]
fn test_v4_to_v5_migration_resets_stale_running_accounts() {
    let unique = Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("ai_switcher_v4_test_{}", unique));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("switcher.db");

    // Create a mock v4 database with accounts having status 'running' and 'ready'
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
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
                updated_at TEXT NOT NULL,
                browser_profile_id TEXT
            );

            PRAGMA user_version = 4;
            ",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO accounts (id, platform, display_name, login_method, status, profile_path, launch_arguments, environment_variables, is_enabled, created_at, updated_at)
             VALUES ('stale-1', 'codex', 'Stale Running', 'google', 'running', 'C:/profiles/stale1', '[]', '{}', 1, '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO accounts (id, platform, display_name, login_method, status, profile_path, launch_arguments, environment_variables, is_enabled, created_at, updated_at)
             VALUES ('ready-1', 'claude', 'Ready Claude', 'google', 'ready', 'C:/profiles/ready1', '[]', '{}', 1, '2026-09-01T00:00:00Z', '2026-09-01T00:00:00Z')",
            [],
        )
        .unwrap();
    }

    // Run Db::init which executes Migration 5
    let db = Db::init(&db_path).expect("Failed to run Db::init");
    assert_eq!(db.get_schema_version().unwrap(), 5);

    let stale_acc = db.get_account("stale-1").unwrap();
    assert_eq!(stale_acc.status, AccountStatus::LoginRequired);
    assert_eq!(stale_acc.auth_status, AuthStatus::LoginRequired);

    let ready_acc = db.get_account("ready-1").unwrap();
    assert_eq!(ready_acc.status, AccountStatus::Ready);
    assert_eq!(ready_acc.auth_status, AuthStatus::Authenticated);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_host_accounts_auth_probing_real_environment() {
    let host_db_path = dirs::data_dir().map(|p| p.join("AI-Switcher").join("switcher.db"));

    if let Some(db_path) = host_db_path {
        if db_path.exists() {
            let db = Db::init(&db_path).expect("Host DB must open");
            let accounts = db.list_accounts(false).expect("List host accounts");

            for acc in accounts {
                let adapter = ai_switcher_lib::adapters::get_adapter(acc.platform);
                let status = adapter
                    .check_status(&acc)
                    .await
                    .expect("check_status must succeed");

                match acc.display_name.as_str() {
                    "user1" | "user2" | "user3" | "user4" => {
                        // User1-4 have real ChatGPT login credentials verified on this host
                        assert_eq!(
                            status,
                            AccountStatus::Ready,
                            "Codex profile {} should be Ready",
                            acc.display_name
                        );
                    }
                    "user 5" => {
                        // Claude profile does not have sessionKey cookie yet
                        assert_eq!(
                            status,
                            AccountStatus::LoginRequired,
                            "Claude profile user 5 should be LoginRequired"
                        );
                    }
                    "user6" => {
                        // Antigravity profile does not have oauth_creds.json yet
                        assert_eq!(
                            status,
                            AccountStatus::LoginRequired,
                            "Antigravity profile user6 should be LoginRequired"
                        );
                    }
                    _ => {}
                }
            }
        }
    }
}
