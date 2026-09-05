use std::path::{Path, PathBuf};

use ai_switcher_lib::adapters::codex::CodexAdapter;
use ai_switcher_lib::adapters::PlatformAdapter;
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, AuthStatus, ExecutionSurface, InstancePolicy, LaunchTarget,
    LoginMethod, PlatformType, RuntimeStatus,
};

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("ai_switcher_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn create_test_profile(profile_path: &Path, display_name: &str) -> AccountProfile {
    AccountProfile {
        id: "test-codex-id".to_string(),
        platform: PlatformType::Codex,
        display_name: display_name.to_string(),
        account_identifier: Some("test@example.com".to_string()),
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
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
fn test_codex_adapter_metadata() {
    let adapter = CodexAdapter::new();
    assert_eq!(adapter.platform(), PlatformType::Codex);
    assert_eq!(adapter.display_name(), "OpenAI Codex");
    assert_eq!(adapter.instance_policy(), InstancePolicy::SingleInstance);
    assert_eq!(
        adapter.supported_surfaces(),
        vec![ExecutionSurface::DesktopApp, ExecutionSurface::Cli]
    );
    assert_eq!(adapter.default_surface(), ExecutionSurface::Cli);
}

#[test]
fn test_codex_executable_discovery() {
    let adapter = CodexAdapter::new();
    let exec = adapter
        .detect_executable()
        .expect("detect_executable failed");
    assert!(
        exec.exists() || exec == *"codex.exe",
        "Detected executable must exist or default to codex.exe: {:?}",
        exec
    );

    // On this live Windows system, Codex Desktop is installed as an MSIX package
    assert!(
        adapter.is_desktop_installed(),
        "Codex Desktop MSIX package should be detected on this system"
    );
}

#[test]
fn test_codex_profile_initialization() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    let profile_path = dir.path().join("codex-profile-1");

    adapter
        .initialize_profile(&profile_path)
        .expect("Failed to initialize profile");

    assert!(profile_path.exists());
    assert!(profile_path.join("log").is_dir());
    assert!(profile_path.join("tmp").is_dir());

    let config_file = profile_path.join("config.toml");
    assert!(config_file.is_file());
    let content = std::fs::read_to_string(&config_file).expect("Failed to read config.toml");
    assert!(content.contains("OpenAI Codex Profile Configuration"));
    assert!(content.contains("model = \"o3\""));
}

#[test]
fn test_codex_launch_spec_desktop_with_workspace() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    let profile = create_test_profile(dir.path(), "Work Profile");

    let workspace = Path::new("H:\\dev\\ai-switcher");
    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Desktop, Some(workspace))
        .expect("Failed to build desktop launch spec");

    assert_eq!(spec.arguments.len(), 2);
    assert_eq!(spec.arguments[0], "app");
    assert_eq!(spec.arguments[1], "H:\\dev\\ai-switcher");
    assert!(!spec.is_terminal);
    assert_eq!(spec.working_directory, workspace);

    // CODEX_HOME must point to profile directory
    let codex_home = spec
        .environment
        .iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| v.as_str());
    assert_eq!(codex_home, Some(dir.path().to_str().unwrap()));
}

#[test]
fn test_codex_launch_spec_cli() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    let profile = create_test_profile(dir.path(), "CLI Profile");

    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Cli, None)
        .expect("Failed to build CLI launch spec");

    assert!(spec.arguments.is_empty());
    assert!(spec.is_terminal);

    let codex_home = spec
        .environment
        .iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| v.as_str());
    assert_eq!(codex_home, Some(dir.path().to_str().unwrap()));
}

#[test]
fn test_codex_paths_with_spaces_and_korean() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    let unicode_profile_dir = dir.path().join("코덱스 프로필 - 개발팀 A");
    std::fs::create_dir_all(&unicode_profile_dir).unwrap();

    let profile = create_test_profile(&unicode_profile_dir, "한글 프로필");

    let unicode_workspace = Path::new("H:\\내 문서\\인공지능 프로젝트\\switcher");
    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Desktop, Some(unicode_workspace))
        .expect("Failed to build spec with Korean paths");

    assert_eq!(spec.arguments[0], "app");
    assert_eq!(
        spec.arguments[1],
        "H:\\내 문서\\인공지능 프로젝트\\switcher"
    );
    assert_eq!(spec.working_directory, unicode_workspace);

    let codex_home = spec
        .environment
        .iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| v.as_str());
    assert_eq!(codex_home, Some(unicode_profile_dir.to_str().unwrap()));
}

#[tokio::test]
async fn test_codex_status_probing_isolated_unauthenticated() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    adapter
        .initialize_profile(dir.path())
        .expect("initialize failed");

    let profile = create_test_profile(dir.path(), "Fresh Profile");
    let status = adapter
        .check_status(&profile)
        .await
        .expect("check_status failed");

    // Fresh profile has no auth.json, so codex login status must report LoginRequired
    assert_eq!(
        status,
        AccountStatus::LoginRequired,
        "Isolated profile without credentials must report LoginRequired"
    );
}

#[test]
fn test_codex_login_spec_terminal_command() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    let profile = create_test_profile(dir.path(), "Login Target");

    let spec = adapter
        .build_login_spec(&profile)
        .expect("Failed to build login spec");

    assert_eq!(spec.arguments, vec!["login".to_string()]);
    assert!(spec.is_terminal);

    let codex_home = spec
        .environment
        .iter()
        .find(|(k, _)| k == "CODEX_HOME")
        .map(|(_, v)| v.as_str());
    assert_eq!(codex_home, Some(dir.path().to_str().unwrap()));
}

#[tokio::test]
async fn test_codex_logout_on_isolated_profile() {
    let adapter = CodexAdapter::new();
    let dir = TestDir::new();
    adapter
        .initialize_profile(dir.path())
        .expect("initialize failed");

    let profile = create_test_profile(dir.path(), "Logout Target");
    let result = adapter.logout(&profile).await;
    assert!(result.is_ok(), "Logout on isolated profile should succeed");

    // auth.json should not exist
    assert!(!dir.path().join("auth.json").exists());
}
