use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
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
    cleanup_pending_auth, clear_test_base_dir, clear_test_registry_backend, is_auth_callback_url,
    list_pending_auth_flows, redact_url, register_pending_auth, restore_protocol,
    set_test_base_dir, set_test_registry_backend, setup_protocol_broker, MockRegistryBackend,
};

static TEST_REGISTRY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    let _guard = TEST_REGISTRY_LOCK.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("ai_switcher_lifecycle_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    set_test_base_dir(temp_dir.clone());
    cleanup_pending_auth(None, None);

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
    )
    .expect("Register pending auth should succeed");

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

    let _ = fs::remove_dir_all(&temp_dir);
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
    let _guard = TEST_REGISTRY_LOCK.lock().unwrap();
    let fixture = TestFixture::new();
    let mock = Arc::new(MockRegistryBackend::new());
    set_test_registry_backend(mock);
    set_test_base_dir(fixture.temp_dir.clone());
    cleanup_pending_auth(None, None);

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

    cleanup_pending_auth(None, None);
    clear_test_registry_backend();
    clear_test_base_dir();
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

    // Run Db::init which executes Migration 5 and Migration 6
    let db = Db::init(&db_path).expect("Failed to run Db::init");
    assert_eq!(db.get_schema_version().unwrap(), 6);

    let stale_acc = db.get_account("stale-1").unwrap();
    assert_eq!(stale_acc.status, AccountStatus::LoginRequired);
    assert_eq!(stale_acc.auth_status, AuthStatus::LoginRequired);

    let ready_acc = db.get_account("ready-1").unwrap();
    // In migration 6, Claude's false-positive ready status is reset to Unknown
    assert_eq!(ready_acc.status, AccountStatus::Unknown);
    assert_eq!(ready_acc.auth_status, AuthStatus::Unknown);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_protocol_broker_mock_registry_isolation() {
    let _guard = TEST_REGISTRY_LOCK.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("ai_switcher_mock_reg_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let mock = Arc::new(MockRegistryBackend::new());
    mock.set("antigravity", "\"C:\\Original\\antigravity.exe\" \"%1\"");
    set_test_registry_backend(mock.clone());
    set_test_base_dir(temp_dir.clone());
    cleanup_pending_auth(None, None);

    // Setup protocol broker
    let reg = setup_protocol_broker("antigravity").expect("Setup broker must succeed");
    assert_eq!(
        reg.original_command.as_deref(),
        Some("\"C:\\Original\\antigravity.exe\" \"%1\"")
    );
    assert!(reg.broker_command.contains("--broker-protocol antigravity"));

    // Verify mock registry received the broker command without OS side effects
    let current_cmd = mock.get("antigravity").unwrap();
    assert!(current_cmd.contains("--broker-protocol antigravity"));

    // Restore protocol
    restore_protocol("antigravity");
    assert_eq!(
        mock.get("antigravity").as_deref(),
        Some("\"C:\\Original\\antigravity.exe\" \"%1\"")
    );

    cleanup_pending_auth(None, None);
    clear_test_registry_backend();
    clear_test_base_dir();
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_protocol_broker_ownership_safety() {
    let _guard = TEST_REGISTRY_LOCK.lock().unwrap();
    let temp_dir = std::env::temp_dir().join(format!("ai_switcher_ownership_{}", Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let mock = Arc::new(MockRegistryBackend::new());
    mock.set("antigravity", "\"C:\\Original\\antigravity.exe\" \"%1\"");
    set_test_registry_backend(mock.clone());
    set_test_base_dir(temp_dir.clone());
    cleanup_pending_auth(None, None);

    let _reg = setup_protocol_broker("antigravity").expect("Setup broker must succeed");

    // Simulate an external app/update modifying the registry command while broker was registered
    mock.set(
        "antigravity",
        "\"C:\\Program Files\\ExternalTool.exe\" \"%1\"",
    );

    // Attempt to restore
    restore_protocol("antigravity");

    // Ownership check must abort restoration and preserve the external tool!
    assert_eq!(
        mock.get("antigravity").as_deref(),
        Some("\"C:\\Program Files\\ExternalTool.exe\" \"%1\"")
    );

    cleanup_pending_auth(None, None);
    clear_test_registry_backend();
    clear_test_base_dir();
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_is_auth_callback_url_filtering() {
    // Auth callback URLs
    assert!(is_auth_callback_url(
        "antigravity://auth/callback?code=abc123xyz"
    ));
    assert!(is_auth_callback_url("antigravity://auth/return"));
    assert!(is_auth_callback_url("antigravity://oauth?code=token123"));

    // Non-auth callback deep links (must NOT be intercepted by auth broker)
    assert!(!is_auth_callback_url(
        "antigravity://workspace/open?folder=H:/dev"
    ));
    assert!(!is_auth_callback_url("antigravity://command/open"));
    assert!(!is_auth_callback_url("claude://code/new"));
    assert!(!is_auth_callback_url("vscode://file/path/to/file"));
}

#[test]
fn test_single_pending_auth_conflict_rejection() {
    let _guard = TEST_REGISTRY_LOCK.lock().unwrap();
    let fixture = TestFixture::new();
    let mock = Arc::new(MockRegistryBackend::new());
    set_test_registry_backend(mock);
    set_test_base_dir(fixture.temp_dir.clone());
    cleanup_pending_auth(None, None);

    let input1 = CreateAccountInput {
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Profile 1".to_string(),
        account_identifier: None,
        login_method: None,
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: Some(AccountStatus::LoginRequired),
    };
    let acc1 = create_account_impl(input1, &fixture.state).expect("Create acc1 failed");

    let input2 = CreateAccountInput {
        platform: PlatformType::Antigravity,
        display_name: "Antigravity Profile 2".to_string(),
        account_identifier: None,
        login_method: None,
        default_workspace_path: None,
        custom_executable_path: None,
        browser_profile_id: None,
        initial_status: Some(AccountStatus::LoginRequired),
    };
    let acc2 = create_account_impl(input2, &fixture.state).expect("Create acc2 failed");

    // Start first login flow
    let res1 = start_login_flow_impl(&acc1.id, &fixture.state);
    assert!(res1.is_ok(), "First flow must succeed: {:?}", res1.err());

    // Start second login flow for same platform concurrently -> MUST fail with conflict
    let res2 = start_login_flow_impl(&acc2.id, &fixture.state);
    assert!(res2.is_err(), "Concurrent flow must be rejected");
    let err_msg = res2.err().unwrap().to_string();
    assert!(
        err_msg
            .to_lowercase()
            .contains("another antigravity sign-in is already in progress"),
        "Unexpected error: {}",
        err_msg
    );

    // Clean up
    cleanup_pending_auth(None, None);
    clear_test_registry_backend();
    clear_test_base_dir();
}

#[tokio::test]
#[ignore = "requires real Windows host profiles and credentials"]
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
