use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ai_switcher_lib::adapters::{get_adapter, LaunchSpec};
use ai_switcher_lib::launcher::{
    CliLauncher, DesktopAppLauncher, LauncherEngine, ProcessManager, WebLauncher,
};
use ai_switcher_lib::models::{
    AccountProfile, AccountStatus, AuthStatus, ExecutionSurface, InstancePolicy, LoginMethod,
    PlatformType, RuntimeStatus,
};

#[test]
fn test_desktop_app_command_construction() {
    let spec = LaunchSpec {
        executable: PathBuf::from("C:\\Program Files\\AI App\\app.exe"),
        arguments: vec!["--profile".to_string(), "user-1".to_string()],
        environment: vec![
            ("AI_HOME".to_string(), "C:\\profiles\\ai".to_string()),
            ("MODE".to_string(), "isolated".to_string()),
        ],
        working_directory: PathBuf::from("C:\\Projects\\My App"),
        is_terminal: false,
    };

    let cmd = DesktopAppLauncher::build_command(&spec);
    let program = cmd.get_program().to_string_lossy().to_string();
    assert_eq!(program, "C:\\Program Files\\AI App\\app.exe");

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert_eq!(args, vec!["--profile", "user-1"]);

    let current_dir = cmd
        .get_current_dir()
        .map(|p| p.to_string_lossy().to_string());
    assert_eq!(current_dir, Some("C:\\Projects\\My App".to_string()));

    let envs: HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.map(|s| s.to_string_lossy().to_string()),
            )
        })
        .collect();

    assert_eq!(
        envs.get("AI_HOME"),
        Some(&Some("C:\\profiles\\ai".to_string()))
    );
    assert_eq!(envs.get("MODE"), Some(&Some("isolated".to_string())));
}

#[test]
fn test_cli_command_construction_windows_terminal() {
    let workdir = Path::new("C:\\Projects\\Dev Project");
    let exec = Path::new("C:\\Users\\User\\.local\\bin\\claude.exe");
    let args = vec!["--profile".to_string(), "work".to_string()];
    let env = vec![(
        "CLAUDE_CONFIG_DIR".to_string(),
        "C:\\Profiles\\Claude".to_string(),
    )];

    let cmd =
        CliLauncher::build_wt_command("Work Claude (Anthropic Claude)", workdir, exec, &args, &env);

    assert_eq!(cmd.get_program().to_string_lossy(), "wt.exe");
    let cmd_args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    assert_eq!(cmd_args[0], "--title");
    assert_eq!(cmd_args[1], "Work Claude (Anthropic Claude)");
    assert_eq!(cmd_args[2], "-d");
    assert_eq!(cmd_args[3], "C:\\Projects\\Dev Project");
    assert_eq!(cmd_args[4], "C:\\Users\\User\\.local\\bin\\claude.exe");
    assert_eq!(cmd_args[5], "--profile");
    assert_eq!(cmd_args[6], "work");

    let envs: HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.map(|s| s.to_string_lossy().to_string()),
            )
        })
        .collect();

    assert_eq!(
        envs.get("CLAUDE_CONFIG_DIR"),
        Some(&Some("C:\\Profiles\\Claude".to_string()))
    );
}

#[test]
fn test_cli_command_construction_powershell_fallback() {
    let workdir = Path::new("C:\\Projects\\Dev Project");
    let exec = Path::new("C:\\Users\\User\\.local\\bin\\claude.exe");
    let args = vec!["--arg1".to_string(), "value with spaces".to_string()];
    let env = vec![(
        "CLAUDE_CONFIG_DIR".to_string(),
        "C:\\Profiles\\Claude".to_string(),
    )];

    let cmd = CliLauncher::build_powershell_command("Claude Session", workdir, exec, &args, &env);

    assert_eq!(cmd.get_program().to_string_lossy(), "powershell.exe");
    let cmd_args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    assert_eq!(cmd_args[0], "-NoExit");
    assert_eq!(cmd_args[1], "-Command");
    assert!(cmd_args[2].contains("Set-Location -LiteralPath 'C:\\Projects\\Dev Project'"));
    assert!(cmd_args[2].contains("& 'C:\\Users\\User\\.local\\bin\\claude.exe'"));
    assert!(cmd_args[2].contains("'--arg1' 'value with spaces'"));
}

#[test]
fn test_cli_command_powershell_login_auto_closes() {
    let workdir = Path::new("C:\\Profiles\\Codex");
    let exec = Path::new("C:\\Users\\User\\.local\\bin\\codex.exe");
    let args = vec!["login".to_string()];
    let env = vec![("CODEX_HOME".to_string(), "C:\\Profiles\\Codex".to_string())];

    let cmd =
        CliLauncher::build_powershell_command("user2 Official Login", workdir, exec, &args, &env);

    assert_eq!(cmd.get_program().to_string_lossy(), "powershell.exe");
    let cmd_args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();

    // Must NOT contain -NoExit
    assert_ne!(cmd_args[0], "-NoExit");
    assert_eq!(cmd_args[0], "-Command");
    assert!(cmd_args[1].contains("& 'C:\\Users\\User\\.local\\bin\\codex.exe' 'login'"));
    assert!(cmd_args[1].contains("if ($LASTEXITCODE -eq 0) { exit 0 }"));
}

#[test]
fn test_unicode_and_korean_paths() {
    let spec = LaunchSpec {
        executable: PathBuf::from("H:\\개발\\AI스위처\\app.exe"),
        arguments: vec!["프로젝트명".to_string(), "공백 포함 인자".to_string()],
        environment: vec![
            ("USER_LANG".to_string(), "한국어".to_string()),
            ("PROJ_PATH".to_string(), "H:\\개발\\프로젝트".to_string()),
        ],
        working_directory: PathBuf::from("H:\\개발\\작업공간"),
        is_terminal: false,
    };

    let cmd = DesktopAppLauncher::build_command(&spec);

    assert_eq!(
        cmd.get_program().to_string_lossy(),
        "H:\\개발\\AI스위처\\app.exe"
    );
    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert_eq!(args, vec!["프로젝트명", "공백 포함 인자"]);

    assert_eq!(
        cmd.get_current_dir()
            .map(|p| p.to_string_lossy().to_string()),
        Some("H:\\개발\\작업공간".to_string())
    );

    let envs: HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().to_string(),
                v.map(|s| s.to_string_lossy().to_string()),
            )
        })
        .collect();

    assert_eq!(envs.get("USER_LANG"), Some(&Some("한국어".to_string())));
    assert_eq!(
        envs.get("PROJ_PATH"),
        Some(&Some("H:\\개발\\프로젝트".to_string()))
    );
}

#[test]
fn test_web_command_construction() {
    let url = "https://claude.ai";
    let browser = Path::new("C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe");

    let cmd = WebLauncher::build_command(url, Some(browser));
    assert_eq!(
        cmd.get_program().to_string_lossy(),
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
    );

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert_eq!(args, vec!["https://claude.ai"]);
}

#[test]
fn test_cross_platform_non_conflict() {
    let pm = ProcessManager::new();
    let engine = LauncherEngine::new(pm.clone());

    // Use current process PID so OS process check confirms it is alive
    let current_pid = std::process::id();

    // Register a running Codex process for Account 1
    pm.register_launch(
        "codex-account-1",
        PlatformType::Codex,
        ExecutionSurface::DesktopApp,
        Some(current_pid),
        "codex.exe",
    );

    let all_accounts = vec![
        make_account("codex-account-1", PlatformType::Codex, "Codex Personal 1"),
        make_account("claude-account-1", PlatformType::Claude, "Claude Work"),
        make_account(
            "antigravity-account-1",
            PlatformType::Antigravity,
            "Antigravity Main",
        ),
    ];

    // Case 1: Codex running + Claude launch -> NO CONFLICT
    let claude_adapter = get_adapter(PlatformType::Claude);
    let conflict_claude = engine.check_conflict(
        &all_accounts[1],
        claude_adapter.as_ref(),
        &all_accounts,
        None,
    );
    assert!(
        conflict_claude.is_none(),
        "Launching Claude must NEVER conflict with running Codex!"
    );

    // Case 2: Codex running + Antigravity launch -> NO CONFLICT
    let antigravity_adapter = get_adapter(PlatformType::Antigravity);
    let conflict_ag = engine.check_conflict(
        &all_accounts[2],
        antigravity_adapter.as_ref(),
        &all_accounts,
        None,
    );
    assert!(
        conflict_ag.is_none(),
        "Launching Antigravity must NEVER conflict with running Codex!"
    );
}

#[test]
fn test_same_platform_conflict_single_instance() {
    let pm = ProcessManager::new();
    let engine = LauncherEngine::new(pm.clone());

    // Use current process PID so OS process check confirms it is alive
    let current_pid = std::process::id();

    // Register Codex Account 1 as running
    pm.register_launch(
        "codex-account-1",
        PlatformType::Codex,
        ExecutionSurface::Cli,
        Some(current_pid),
        "codex.exe",
    );

    let all_accounts = vec![
        make_account("codex-account-1", PlatformType::Codex, "Codex Personal 1"),
        make_account("codex-account-2", PlatformType::Codex, "Codex Work 2"),
    ];

    let codex_adapter = get_adapter(PlatformType::Codex);
    assert_eq!(
        codex_adapter.instance_policy(),
        InstancePolicy::SingleInstance
    );

    // Attempt to launch Codex Account 2 -> Conflict detected!
    let conflict = engine.check_conflict(
        &all_accounts[1],
        codex_adapter.as_ref(),
        &all_accounts,
        None,
    );
    assert!(
        conflict.is_some(),
        "Same-platform SingleInstance must detect conflict!"
    );

    let c = conflict.unwrap();
    assert_eq!(c.running_account_id, "codex-account-1");
    assert_eq!(c.running_display_name, "Codex Personal 1");
    assert_eq!(c.running_pid, Some(current_pid));
    assert_eq!(c.platform, PlatformType::Codex);
    assert_eq!(c.policy, InstancePolicy::SingleInstance);
}

#[test]
fn test_same_platform_multi_instance_no_conflict() {
    let pm = ProcessManager::new();
    let engine = LauncherEngine::new(pm.clone());

    // Use current process PID so OS process check confirms it is alive
    let current_pid = std::process::id();

    // Register Claude Account 1 as running
    pm.register_launch(
        "claude-account-1",
        PlatformType::Claude,
        ExecutionSurface::Cli,
        Some(current_pid),
        "claude.exe",
    );

    let all_accounts = vec![
        make_account("claude-account-1", PlatformType::Claude, "Claude Account 1"),
        make_account("claude-account-2", PlatformType::Claude, "Claude Account 2"),
    ];

    let claude_adapter = get_adapter(PlatformType::Claude);
    assert_eq!(
        claude_adapter.instance_policy(),
        InstancePolicy::MultiInstance
    );

    // Attempt to launch Claude Account 2 -> MultiInstance permits concurrent execution!
    let conflict = engine.check_conflict(
        &all_accounts[1],
        claude_adapter.as_ref(),
        &all_accounts,
        None,
    );
    assert!(
        conflict.is_none(),
        "MultiInstance platform must allow concurrent execution without conflict!"
    );
}

#[test]
fn test_disabled_account_cannot_launch() {
    let pm = ProcessManager::new();
    let engine = LauncherEngine::new(pm);

    let mut account = make_account("disabled-1", PlatformType::Codex, "Disabled Account");
    account.is_enabled = false;

    let adapter = get_adapter(PlatformType::Codex);
    let result = engine.launch(&account, adapter.as_ref(), None, None);

    assert!(result.is_err(), "Disabled account must not be launched");
    match result.err().unwrap() {
        ai_switcher_lib::error::AppError::AccountDisabled(name) => {
            assert_eq!(name, "Disabled Account");
        }
        other => panic!("Expected AccountDisabled error, got {:?}", other),
    }
}

fn make_account(id: &str, platform: PlatformType, name: &str) -> AccountProfile {
    AccountProfile {
        id: id.to_string(),
        platform,
        display_name: name.to_string(),
        account_identifier: None,
        login_method: LoginMethod::Google,
        status: AccountStatus::Ready,
        auth_status: AuthStatus::Authenticated,
        runtime_status: RuntimeStatus::Stopped,
        auth_states: vec![],
        profile_path: format!("C:\\profiles\\{}", id),
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
    }
}
