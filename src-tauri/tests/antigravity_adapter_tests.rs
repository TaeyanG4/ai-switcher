use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use ai_switcher_lib::adapters::antigravity::{
    safe_delete_antigravity_profile_dir, validate_antigravity_profile_dir, validate_profile_id,
    AntigravityAdapter,
};
use ai_switcher_lib::adapters::PlatformAdapter;
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, ExecutionSurface, InstancePolicy, LaunchTarget, LoginMethod,
    PlatformType,
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(prefix: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("ai_switcher_ag_test_{}_{}", prefix, Uuid::new_v4()));
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

fn create_test_profile(profile_path: &Path, display_name: &str) -> AccountProfile {
    AccountProfile {
        id: "test-ag-id".to_string(),
        platform: PlatformType::Antigravity,
        display_name: display_name.to_string(),
        account_identifier: Some("developer@gmail.com".to_string()),
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        profile_path: profile_path.to_string_lossy().to_string(),
        browser_profile_path: None,
        browser_profile_id: None,
        custom_executable_path: None,
        launch_arguments: vec![],
        environment_variables: std::collections::HashMap::new(),
        default_workspace_path: None,
        is_enabled: true,
        last_launched_at: None,
        created_at: "2026-09-05T00:00:00Z".to_string(),
        updated_at: "2026-09-05T00:00:00Z".to_string(),
    }
}

#[test]
fn test_antigravity_adapter_metadata() {
    let adapter = AntigravityAdapter::new();
    assert_eq!(adapter.platform(), PlatformType::Antigravity);
    assert_eq!(adapter.display_name(), "Google Antigravity");
    assert_eq!(adapter.instance_policy(), InstancePolicy::MultiInstance);

    let surfaces = adapter.supported_surfaces();
    assert_eq!(surfaces.len(), 1);
    assert!(surfaces.contains(&ExecutionSurface::DesktopApp));
    assert_eq!(adapter.default_surface(), ExecutionSurface::DesktopApp);
}

#[test]
fn test_antigravity_executable_discovery() {
    let adapter = AntigravityAdapter::new();
    let exe = adapter.detect_executable();
    assert!(
        exe.is_ok(),
        "Should discover Antigravity executable on host: {:?}",
        exe
    );
    let path = exe.unwrap();
    assert!(
        path.exists(),
        "Discovered executable must exist: {:?}",
        path
    );

    let version = adapter.detect_version(&path);
    if let Some(v) = version {
        assert!(v.contains("2.") || !v.is_empty(), "Version string: {}", v);
    }
}

#[test]
fn test_antigravity_profile_scaffolding() {
    let temp = TestDir::new("scaffold");
    let adapter = AntigravityAdapter::new();

    adapter.initialize_profile(temp.path()).unwrap();

    assert!(temp.path().join("home").exists());
    assert!(temp.path().join("home").join(".gemini").exists());
    assert!(temp.path().join("data").exists());
    assert!(temp.path().join("appdata").exists());
}

#[test]
fn test_antigravity_launch_spec_desktop() {
    let temp = TestDir::new("launch_desktop");
    let adapter = AntigravityAdapter::new();
    adapter.initialize_profile(temp.path()).unwrap();

    let profile = create_test_profile(temp.path(), "Google Work");
    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Default, None)
        .unwrap();

    assert!(!spec.is_terminal);
    assert!(spec
        .executable
        .to_string_lossy()
        .contains("Antigravity.exe"));

    // Verify --user-data-dir
    let expected_data = temp.path().join("data");
    let has_user_data = spec.arguments.iter().any(|arg| {
        arg.starts_with("--user-data-dir=")
            && arg.contains(&expected_data.to_string_lossy().to_string())
    });
    assert!(
        has_user_data,
        "Must have --user-data-dir matching profile data directory"
    );

    // Verify isolated USERPROFILE, HOME, and APPDATA
    let env_map: std::collections::HashMap<_, _> = spec.environment.into_iter().collect();
    let expected_home = temp.path().join("home").to_string_lossy().to_string();
    let expected_appdata = temp.path().join("appdata").to_string_lossy().to_string();

    assert_eq!(env_map.get("USERPROFILE"), Some(&expected_home));
    assert_eq!(env_map.get("HOME"), Some(&expected_home));
    assert_eq!(env_map.get("APPDATA"), Some(&expected_appdata));
}

#[test]
fn test_antigravity_paths_with_spaces_and_korean() {
    let temp = TestDir::new("공백과 한글 경로");
    let profile_dir = temp.path().join("구글 안티그래비티 프로필 A");
    let adapter = AntigravityAdapter::new();
    adapter.initialize_profile(&profile_dir).unwrap();

    let mut profile = create_test_profile(&profile_dir, "한국어 계정");
    let workspace = temp.path().join("작업공간 폴더");
    fs::create_dir_all(&workspace).unwrap();
    profile.default_workspace_path = Some(workspace.to_string_lossy().to_string());

    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Default, Some(&workspace))
        .unwrap();

    assert_eq!(spec.working_directory, workspace);

    let expected_data = profile_dir.join("data").to_string_lossy().to_string();
    assert!(
        spec.arguments
            .iter()
            .any(|arg| arg.contains(&expected_data)),
        "Data dir with Korean and spaces must be properly preserved"
    );

    let env_map: std::collections::HashMap<_, _> = spec.environment.into_iter().collect();
    let expected_home = profile_dir.join("home").to_string_lossy().to_string();
    assert_eq!(env_map.get("USERPROFILE"), Some(&expected_home));
}

#[test]
fn test_antigravity_profile_isolation_separation() {
    let base = TestDir::new("profile_sep");
    let profile_a_dir = base.path().join("profile_a");
    let profile_b_dir = base.path().join("profile_b");

    let adapter = AntigravityAdapter::new();
    adapter.initialize_profile(&profile_a_dir).unwrap();
    adapter.initialize_profile(&profile_b_dir).unwrap();

    let profile_a = create_test_profile(&profile_a_dir, "Google Account A");
    let profile_b = create_test_profile(&profile_b_dir, "Google Account B");

    let spec_a = adapter
        .build_launch_spec(&profile_a, LaunchTarget::Default, None)
        .unwrap();
    let spec_b = adapter
        .build_launch_spec(&profile_b, LaunchTarget::Default, None)
        .unwrap();

    let env_a: std::collections::HashMap<_, _> = spec_a.environment.into_iter().collect();
    let env_b: std::collections::HashMap<_, _> = spec_b.environment.into_iter().collect();

    // Data directories must differ
    assert_ne!(spec_a.arguments[0], spec_b.arguments[0]);
    // USERPROFILE and HOME must differ
    assert_ne!(env_a.get("USERPROFILE"), env_b.get("USERPROFILE"));
    assert_ne!(env_a.get("HOME"), env_b.get("HOME"));
    assert_ne!(env_a.get("APPDATA"), env_b.get("APPDATA"));
}

#[tokio::test]
async fn test_antigravity_status_probing() {
    let temp = TestDir::new("status_probe");
    let adapter = AntigravityAdapter::new();
    adapter.initialize_profile(temp.path()).unwrap();

    let profile = create_test_profile(temp.path(), "Google Test");

    // 1. Fresh profile without credentials -> LoginRequired
    let status = adapter.check_status(&profile).await.unwrap();
    assert_eq!(status, AccountStatus::LoginRequired);

    // 2. Profile with empty credentials file (0 bytes) -> LoginRequired
    let creds_file = temp
        .path()
        .join("home")
        .join(".gemini")
        .join("oauth_creds.json");
    fs::write(&creds_file, b"").unwrap();
    let status = adapter.check_status(&profile).await.unwrap();
    assert_eq!(status, AccountStatus::LoginRequired);

    // 3. Profile with non-empty credentials file -> Ready
    fs::write(&creds_file, b"{\"mock\": true}").unwrap();
    let status = adapter.check_status(&profile).await.unwrap();
    assert_eq!(status, AccountStatus::Ready);
}

#[tokio::test]
async fn test_antigravity_logout() {
    let temp = TestDir::new("logout");
    let adapter = AntigravityAdapter::new();
    adapter.initialize_profile(temp.path()).unwrap();

    let creds_file = temp
        .path()
        .join("home")
        .join(".gemini")
        .join("oauth_creds.json");
    fs::write(&creds_file, b"{\"mock\": true}").unwrap();

    let cookies_dir = temp.path().join("data").join("Network");
    fs::create_dir_all(&cookies_dir).unwrap();
    let cookies_file = cookies_dir.join("Cookies");
    fs::write(&cookies_file, b"mock-cookie-data").unwrap();

    let profile = create_test_profile(temp.path(), "Google Logout Test");

    assert!(creds_file.exists());
    assert!(cookies_file.exists());

    adapter.logout(&profile).await.unwrap();

    assert!(
        !creds_file.exists(),
        "Credentials file must be deleted on logout"
    );
    assert!(
        !cookies_file.exists(),
        "Cookies file must be deleted on logout"
    );
}

#[test]
fn test_antigravity_directory_safety_and_traversal() {
    // 1. validate_profile_id
    assert!(validate_profile_id("valid-id_123").is_ok());
    assert!(validate_profile_id("550e8400-e29b-41d4-a716-446655440000").is_ok());
    assert!(validate_profile_id("").is_err());
    assert!(validate_profile_id("..").is_err());
    assert!(validate_profile_id(".").is_err());
    assert!(validate_profile_id("foo/bar").is_err());
    assert!(validate_profile_id("foo\\\\bar").is_err());
    assert!(validate_profile_id("id with spaces").is_err());

    // 2. validate_antigravity_profile_dir
    let base = TestDir::new("safety_base");
    let valid = validate_antigravity_profile_dir(base.path(), "profile-1");
    assert!(valid.is_ok());
    assert_eq!(valid.unwrap(), base.path().join("profile-1"));

    // Traversal rejection
    assert!(validate_antigravity_profile_dir(base.path(), "../other").is_err());

    // 3. safe_delete_antigravity_profile_dir
    let profile_id = "safe-del-profile";
    let profile_dir = base.path().join(profile_id);
    fs::create_dir_all(&profile_dir).unwrap();
    fs::write(profile_dir.join("test.txt"), b"data").unwrap();

    assert!(profile_dir.exists());
    safe_delete_antigravity_profile_dir(base.path(), profile_id).unwrap();
    assert!(!profile_dir.exists());
    assert!(base.path().exists(), "Base directory must remain intact");
}
