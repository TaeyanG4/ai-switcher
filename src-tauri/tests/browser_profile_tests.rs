use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use ai_switcher_lib::browser::discovery::{
    detect_available_browsers, find_browser_executable, resolve_browser_executable,
};
use ai_switcher_lib::browser::launcher::{build_browser_args, redact_url_for_diagnostics};
use ai_switcher_lib::browser::safety::{
    safe_delete_browser_dir, validate_browser_profile_dir, validate_profile_id,
};
use ai_switcher_lib::browser::BrowserProfileManager;
use ai_switcher_lib::db::Db;
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, AuthStatus, BrowserKind, BrowserProfile, LoginMethod,
    PlatformType, RuntimeStatus,
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ai_switcher_browser_test_{}_{}",
            prefix,
            Uuid::new_v4()
        ));
        fs::create_dir_all(&path).expect("Failed to create test dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_browser_discovery_on_windows() {
    let browsers = detect_available_browsers();
    assert_eq!(browsers.len(), 3);

    // Verify kinds
    let kinds: Vec<BrowserKind> = browsers.iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BrowserKind::Chrome));
    assert!(kinds.contains(&BrowserKind::Edge));
    assert!(kinds.contains(&BrowserKind::Brave));

    // At least one browser should be installed and available
    let available: Vec<&ai_switcher_lib::models::DetectedBrowserInfo> =
        browsers.iter().filter(|b| b.is_available).collect();
    assert!(
        !available.is_empty(),
        "Expected at least one installed browser"
    );

    for b in &available {
        let p = Path::new(&b.executable_path);
        assert!(
            p.is_file(),
            "Executable should exist: {}",
            b.executable_path
        );
        assert!(
            b.version.is_some(),
            "Version should be detected for available browser: {}",
            b.name
        );
    }
}

#[test]
fn test_resolve_browser_executable() {
    // Chrome
    if let Some(chrome_path) = find_browser_executable(BrowserKind::Chrome) {
        let resolved = resolve_browser_executable(BrowserKind::Chrome, None);
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap(), chrome_path);
    }

    // Edge
    if let Some(edge_path) = find_browser_executable(BrowserKind::Edge) {
        let resolved = resolve_browser_executable(BrowserKind::Edge, None);
        assert!(resolved.is_ok());
        assert_eq!(resolved.unwrap(), edge_path);
    }

    // Custom path override
    let temp = TestDir::new("custom_browser");
    let dummy_exe = temp.path().join("my_browser.exe");
    fs::write(&dummy_exe, "dummy").unwrap();

    let resolved =
        resolve_browser_executable(BrowserKind::Custom, Some(dummy_exe.to_str().unwrap()));
    assert!(resolved.is_ok());
    assert_eq!(resolved.unwrap(), dummy_exe);

    // Non-existent custom path
    let bad_res =
        resolve_browser_executable(BrowserKind::Custom, Some("C:\\does_not_exist\\browser.exe"));
    assert!(bad_res.is_err());
}

#[test]
fn test_browser_profile_directory_isolation() {
    let temp = TestDir::new("profile_mgr");
    let manager = BrowserProfileManager::new(temp.path().to_path_buf());

    let profile_id_1 = "prof-chrome-account-1";
    let profile_id_2 = "prof-chrome-account-2";

    let dir_1 = manager.ensure_profile_dir(profile_id_1).unwrap();
    let dir_2 = manager.ensure_profile_dir(profile_id_2).unwrap();

    assert!(dir_1.exists());
    assert!(dir_2.exists());
    assert_ne!(dir_1, dir_2);
    assert_eq!(dir_1, temp.path().join(profile_id_1));
    assert_eq!(dir_2, temp.path().join(profile_id_2));

    // Delete one profile, ensure other is untouched
    manager.safe_delete_profile_dir(profile_id_1).unwrap();
    assert!(!dir_1.exists());
    assert!(dir_2.exists());
}

#[test]
fn test_browser_args_construction_with_spaces_and_korean() {
    let dir = Path::new("H:\\개발자 폴더\\AI Switcher Profiles\\계정 1");
    let url = "https://claude.ai/chat";

    let args = build_browser_args(dir, Some(url)).unwrap();
    assert_eq!(args.len(), 4);
    assert_eq!(
        args[0],
        "--user-data-dir=H:\\개발자 폴더\\AI Switcher Profiles\\계정 1"
    );
    assert_eq!(args[1], "--no-first-run");
    assert_eq!(args[2], "--no-default-browser-check");
    assert_eq!(args[3], "https://claude.ai/chat");

    // Empty URL
    let no_url_args = build_browser_args(dir, None).unwrap();
    assert_eq!(no_url_args.len(), 3);
}

#[test]
fn test_browser_args_flag_injection_prevention() {
    let dir = Path::new("C:\\profiles\\test");

    assert!(build_browser_args(dir, Some("--remote-debugging-port=9222")).is_err());
    assert!(build_browser_args(dir, Some("-incognito")).is_err());
    assert!(build_browser_args(dir, Some("/unsafe-flag")).is_err());
}

#[test]
fn test_url_redaction_security() {
    let raw = "https://accounts.google.com/o/oauth2/auth?response_type=code&client_id=myclient.apps.googleusercontent.com&redirect_uri=http://localhost:8080/callback&scope=openid&state=state_secret_123&code=4/0AbCdEf123456789";
    let redacted = redact_url_for_diagnostics(raw);

    assert!(!redacted.contains("state_secret_123"));
    assert!(!redacted.contains("4/0AbCdEf123456789"));
    assert!(redacted.contains("code=[REDACTED]"));
    assert!(redacted.contains("state=[REDACTED]"));
    assert!(redacted.contains("client_id=myclient.apps.googleusercontent.com"));
    assert!(redacted.contains("redirect_uri=http://localhost:8080/callback"));
}

#[test]
fn test_path_traversal_protection() {
    let base = PathBuf::from("C:\\AI-Switcher\\browser-profiles");

    // ID validation
    assert!(validate_profile_id("").is_err());
    assert!(validate_profile_id("..").is_err());
    assert!(validate_profile_id("../evil").is_err());
    assert!(validate_profile_id("..\\evil").is_err());
    assert!(validate_profile_id("valid-123_abc").is_ok());

    // Directory resolution validation
    assert!(validate_browser_profile_dir(&base, "../system32").is_err());
    assert!(validate_browser_profile_dir(&base, "safe-uuid-1").is_ok());

    // Safe delete prevents deleting outside base
    let res = safe_delete_browser_dir(&base, "../outside");
    assert!(res.is_err());
}

#[test]
fn test_reference_safe_deletion_in_db() {
    let temp = TestDir::new("ref_safety");
    let db_path = temp.path().join("test.db");
    let db = Db::init(&db_path).unwrap();

    let browser_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Create browser profile in DB
    let browser_prof = BrowserProfile {
        id: browser_id.clone(),
        display_name: "Claude Work Browser".to_string(),
        browser_kind: BrowserKind::Chrome,
        custom_executable_path: None,
        user_data_directory: temp.path().join(&browser_id).to_string_lossy().to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        last_used_at: None,
    };
    db.insert_browser_profile(&browser_prof).unwrap();

    // 2. Link account to this browser profile
    let account_id = Uuid::new_v4().to_string();
    let account = AccountProfile {
        id: account_id.clone(),
        platform: PlatformType::Claude,
        display_name: "Work Claude Account".to_string(),
        account_identifier: Some("work@example.com".to_string()),
        login_method: LoginMethod::EmailOtp,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: temp
            .path()
            .join("claude")
            .join(&account_id)
            .to_string_lossy()
            .to_string(),
        browser_profile_path: None,
        browser_profile_id: Some(browser_id.clone()),
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: std::collections::HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    db.insert_account(&account).unwrap();

    // 3. Verify reference count is 1
    let ref_count = db
        .count_accounts_referencing_browser_profile(&browser_id)
        .unwrap();
    assert_eq!(ref_count, 1);

    let linked = db
        .list_accounts_referencing_browser_profile(&browser_id)
        .unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].id, account_id);

    // 4. Delete the account
    db.delete_account(&account_id).unwrap();

    // 5. Verify reference count is now 0
    let ref_count_after = db
        .count_accounts_referencing_browser_profile(&browser_id)
        .unwrap();
    assert_eq!(ref_count_after, 0);

    // 6. Deleting browser profile from DB now succeeds
    db.delete_browser_profile(&browser_id).unwrap();
    let remaining = db.list_browser_profiles().unwrap();
    assert_eq!(remaining.len(), 0);
}
