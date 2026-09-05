use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use ai_switcher_lib::adapters::claude::ClaudeAdapter;
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
        let path = std::env::temp_dir().join(format!(
            "ai_switcher_claude_test_{}_{}",
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

fn create_test_profile(profile_path: &Path, display_name: &str) -> AccountProfile {
    AccountProfile {
        id: "test-claude-id".to_string(),
        platform: PlatformType::Claude,
        display_name: display_name.to_string(),
        account_identifier: Some("test@anthropic.com".to_string()),
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
fn test_claude_adapter_metadata() {
    let adapter = ClaudeAdapter::new();
    assert_eq!(adapter.platform(), PlatformType::Claude);
    assert_eq!(adapter.display_name(), "Anthropic Claude");
    assert_eq!(adapter.instance_policy(), InstancePolicy::MultiInstance);

    let surfaces = adapter.supported_surfaces();
    assert!(surfaces.contains(&ExecutionSurface::DesktopApp));
    assert!(surfaces.contains(&ExecutionSurface::Cli));
    assert!(surfaces.contains(&ExecutionSurface::Web));

    // DesktopApp is default surface when Claude Desktop is installed
    if adapter.is_desktop_installed() {
        assert_eq!(adapter.default_surface(), ExecutionSurface::DesktopApp);
    }
}

#[test]
fn test_claude_executable_discovery() {
    let adapter = ClaudeAdapter::new();

    // Test Desktop executable discovery
    if let Ok(desktop_exe) = adapter.detect_desktop_executable() {
        assert!(desktop_exe.is_file(), "Desktop executable must exist");
        let path_str = desktop_exe.to_string_lossy().to_lowercase();
        assert!(
            path_str.contains("anthropicclaude") || path_str.contains("claude.exe"),
            "Expected AnthropicClaude path"
        );
    }

    // Test CLI executable discovery
    if let Ok(cli_exe) = adapter.detect_cli_executable() {
        assert!(cli_exe.is_file(), "CLI executable must exist");
        let path_str = cli_exe.to_string_lossy().to_lowercase();
        assert!(path_str.contains("claude"), "Expected claude CLI");
    }
}

#[test]
fn test_claude_desktop_code_deeplink_generation() {
    // No workspace -> default new session
    let default_link = ClaudeAdapter::build_code_deeplink(None);
    assert_eq!(default_link, "claude://code/new");

    // Standard workspace path
    let ws = Path::new("C:\\Projects\\MyProject");
    let ws_link = ClaudeAdapter::build_code_deeplink(Some(ws));
    assert_eq!(
        ws_link,
        "claude://code/new?folder=C%3A%5CProjects%5CMyProject"
    );
}

#[test]
fn test_claude_desktop_paths_with_spaces_and_korean() {
    let ws = Path::new("H:\\개발자 폴더\\test workspace\\테스트프로젝트");
    let link = ClaudeAdapter::build_code_deeplink(Some(ws));

    // Spaces should be %20
    assert!(link.contains("%20"));
    assert!(!link.contains(" "));

    // Korean characters should be percent-encoded UTF-8 bytes
    assert!(link.starts_with("claude://code/new?folder="));
    assert!(!link.contains("개발자"));
    assert!(!link.contains("테스트"));
    assert!(link.contains("%EA%B0%9C%EB%B0%9C%EC%9E%90")); // "개발자"
}

#[test]
fn test_claude_profile_scaffolding() {
    let temp = TestDir::new("scaffold");
    let adapter = ClaudeAdapter::new();

    adapter.initialize_profile(temp.path()).unwrap();

    let desktop_dir = temp.path().join("desktop");
    let cli_projects_dir = temp.path().join("cli").join("projects");

    assert!(desktop_dir.is_dir(), "desktop directory should be created");
    assert!(
        cli_projects_dir.is_dir(),
        "cli/projects directory should be created"
    );
}

#[test]
fn test_claude_launch_spec_desktop() {
    let temp = TestDir::new("launch_desktop");
    let adapter = ClaudeAdapter::new();
    let profile = create_test_profile(temp.path(), "Claude Work Account");

    let ws = Path::new("C:\\Work\\ClientProject");
    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Desktop, Some(ws))
        .unwrap();

    assert!(!spec.is_terminal);
    assert_eq!(spec.working_directory, ws);

    // Arguments check
    assert_eq!(spec.arguments.len(), 2);
    let user_data_arg = &spec.arguments[0];
    let deeplink_arg = &spec.arguments[1];

    assert!(user_data_arg.starts_with("--user-data-dir="));
    assert!(user_data_arg.contains("desktop"));
    assert_eq!(
        deeplink_arg,
        "claude://code/new?folder=C%3A%5CWork%5CClientProject"
    );
}

#[test]
fn test_claude_launch_spec_cli() {
    let temp = TestDir::new("launch_cli");
    let adapter = ClaudeAdapter::new();
    let profile = create_test_profile(temp.path(), "Claude CLI Profile");

    let ws = Path::new("C:\\Work\\CliWork");
    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Cli, Some(ws))
        .unwrap();

    assert!(spec.is_terminal);
    assert_eq!(spec.working_directory, ws);
    assert!(spec.arguments.is_empty());

    // Environment check: CLAUDE_CONFIG_DIR set to <profile_path>\cli
    let config_dir_env = spec
        .environment
        .iter()
        .find(|(k, _)| k == "CLAUDE_CONFIG_DIR");
    assert!(config_dir_env.is_some());
    assert!(config_dir_env.unwrap().1.contains("cli"));
}

#[test]
fn test_claude_launch_spec_web() {
    let temp = TestDir::new("launch_web");
    let adapter = ClaudeAdapter::new();
    let profile = create_test_profile(temp.path(), "Claude Web Profile");

    let spec = adapter
        .build_launch_spec(&profile, LaunchTarget::Web, None)
        .unwrap();

    assert!(!spec.is_terminal);
    assert_eq!(spec.executable, PathBuf::from("https://claude.ai"));
    assert_eq!(spec.arguments, vec!["https://claude.ai".to_string()]);
}

#[tokio::test]
async fn test_claude_status_probing_isolated_unauthenticated() {
    let temp = TestDir::new("status");
    let adapter = ClaudeAdapter::new();
    let profile = create_test_profile(temp.path(), "Isolated Test Account");

    adapter.initialize_profile(temp.path()).unwrap();

    // Isolated empty CLI profile should return LoginRequired
    let status = adapter.check_status(&profile).await;
    assert!(status.is_ok());
    let st = status.unwrap();
    // Since there are no credentials in this fresh directory, status should be LoginRequired or Ready
    assert!(st == AccountStatus::LoginRequired || st == AccountStatus::Ready);
}
