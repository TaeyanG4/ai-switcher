use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::adapters::get_adapter;
use crate::browser::BrowserProfileManager;
use crate::db::Db;
use crate::error::Result;
use crate::launcher::LauncherEngine;
use crate::models::{
    AccountHealthReport, AccountProfile, AccountStatus, AppSettings, AuthFlowStartResult,
    AuthStatus, BrowserProfile, CreateAccountInput, CreateBrowserProfileInput,
    CreateWorkspaceInput, DiagnosticsInfo, ExecutionSurface, FavoriteTarget, InstancePolicy,
    PlatformCapabilities, PlatformType, ProcessConflictInfo, ProcessRecord, ProfileHealth,
    RecentItem, RuntimeStatus, SupportBundle, SystemHealthReport, UpdateAccountInput,
    UpdateBrowserProfileInput, UpdateWorkspaceInput, WorkspacePreset,
};

pub struct AppState {
    pub db: Db,
    pub base_profiles_dir: PathBuf,
    pub launcher_engine: LauncherEngine,
    pub browser_manager: BrowserProfileManager,
}

pub fn list_platform_capabilities_impl() -> Vec<PlatformCapabilities> {
    vec![
        PlatformCapabilities {
            platform: PlatformType::Codex,
            display_name: "OpenAI Codex".to_string(),
            description: "OpenAI's desktop agent & CLI assistant. Isolated CLI via CODEX_HOME; desktop application runs in a shared Windows session.".to_string(),
            primary_surface: ExecutionSurface::Cli,
            supported_surfaces: vec![ExecutionSurface::Cli, ExecutionSurface::DesktopApp],
            supports_desktop_isolation: false,
            isolation_summary: "CLI profiles are isolated via CODEX_HOME. Desktop application is single-instance and shares global Windows session.".to_string(),
            instance_policy: InstancePolicy::SingleInstance,
            supports_automated_status_probe: true,
            supports_logout: true,
            requires_browser_profile: false,
            recommended_browser_profile: false,
        },
        PlatformCapabilities {
            platform: PlatformType::Claude,
            display_name: "Anthropic Claude".to_string(),
            description: "Anthropic's desktop assistant and Claude Code. Multi-instance Desktop profiles with isolated data directories.".to_string(),
            primary_surface: ExecutionSurface::DesktopApp,
            supported_surfaces: vec![ExecutionSurface::DesktopApp, ExecutionSurface::Cli, ExecutionSurface::Web],
            supports_desktop_isolation: true,
            isolation_summary: "Fully isolated Desktop profiles via dedicated user-data directories. Multi-instance concurrent execution supported.".to_string(),
            instance_policy: InstancePolicy::MultiInstance,
            supports_automated_status_probe: true,
            supports_logout: true,
            requires_browser_profile: false,
            recommended_browser_profile: true,
        },
        PlatformCapabilities {
            platform: PlatformType::Antigravity,
            display_name: "Google Antigravity".to_string(),
            description: "Google's next-generation desktop coding agent (Antigravity 2.0). Multi-instance Desktop profiles with isolated application and OAuth home directories.".to_string(),
            primary_surface: ExecutionSurface::DesktopApp,
            supported_surfaces: vec![ExecutionSurface::DesktopApp],
            supports_desktop_isolation: true,
            isolation_summary: "Fully isolated Desktop profiles via dedicated data and .gemini directories. Multi-instance concurrent execution supported.".to_string(),
            instance_policy: InstancePolicy::MultiInstance,
            supports_automated_status_probe: true,
            supports_logout: true,
            requires_browser_profile: false,
            recommended_browser_profile: false,
        },
    ]
}

#[tauri::command]
pub fn list_platform_capabilities() -> Result<Vec<PlatformCapabilities>> {
    Ok(list_platform_capabilities_impl())
}

#[tauri::command]
pub fn list_accounts(
    include_disabled: Option<bool>,
    state: State<AppState>,
) -> Result<Vec<AccountProfile>> {
    let mut accounts = state.db.list_accounts(include_disabled.unwrap_or(false))?;
    for account in &mut accounts {
        if state
            .launcher_engine
            .process_manager
            .has_running_process(&account.id)
        {
            account.runtime_status = RuntimeStatus::Running;
        } else {
            account.runtime_status = RuntimeStatus::Stopped;
        }
    }
    Ok(accounts)
}

#[tauri::command]
pub fn get_account(id: String, state: State<AppState>) -> Result<AccountProfile> {
    let mut account = state.db.get_account(&id)?;
    if state
        .launcher_engine
        .process_manager
        .has_running_process(&account.id)
    {
        account.runtime_status = RuntimeStatus::Running;
    } else {
        account.runtime_status = RuntimeStatus::Stopped;
    }
    Ok(account)
}

pub fn create_account_impl(input: CreateAccountInput, state: &AppState) -> Result<AccountProfile> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Directory structure: %APPDATA%\AI-Switcher\profiles\<platform>\<id>
    let platform_str = input.platform.as_str();
    let profile_path = state.base_profiles_dir.join(platform_str).join(&id);

    let browser_profile_path = state.base_profiles_dir.join("browser").join(&id);

    // Initialize profile via platform adapter
    let adapter = get_adapter(input.platform);
    adapter.initialize_profile(&profile_path)?;
    std::fs::create_dir_all(&browser_profile_path)?;

    let initial_status = input.initial_status.unwrap_or(AccountStatus::LoginRequired);
    let initial_auth = match initial_status {
        AccountStatus::Ready => AuthStatus::Authenticated,
        AccountStatus::LoginRequired => AuthStatus::LoginRequired,
        AccountStatus::Error => AuthStatus::Error,
        _ => AuthStatus::Unknown,
    };

    let account = AccountProfile {
        id: id.clone(),
        platform: input.platform,
        display_name: input.display_name,
        account_identifier: input.account_identifier,
        login_method: input
            .login_method
            .unwrap_or(crate::models::LoginMethod::Unknown),
        status: initial_status,
        auth_status: initial_auth,
        runtime_status: RuntimeStatus::Stopped,
        profile_path: profile_path.to_string_lossy().to_string(),
        browser_profile_path: Some(browser_profile_path.to_string_lossy().to_string()),
        browser_profile_id: input.browser_profile_id,
        custom_executable_path: input.custom_executable_path,
        launch_arguments: vec![],
        environment_variables: std::collections::HashMap::new(),
        default_workspace_path: input.default_workspace_path,
        is_enabled: true,
        last_launched_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.insert_account(&account)?;
    Ok(account)
}

#[tauri::command]
pub fn create_account(input: CreateAccountInput, state: State<AppState>) -> Result<AccountProfile> {
    create_account_impl(input, &state)
}

#[tauri::command]
pub fn update_account(input: UpdateAccountInput, state: State<AppState>) -> Result<AccountProfile> {
    state.db.update_account(&input)
}

#[tauri::command]
pub fn toggle_account_enabled(id: String, is_enabled: bool, state: State<AppState>) -> Result<()> {
    state.db.toggle_account_enabled(&id, is_enabled)
}

pub fn delete_account_impl(
    id: &str,
    delete_local_data: bool,
    delete_browser_profile: Option<bool>,
    state: &AppState,
) -> Result<()> {
    let account = state.db.get_account(id)?;

    if delete_browser_profile == Some(true) {
        if let Some(bp_id) = &account.browser_profile_id {
            let ref_count = state.db.count_accounts_referencing_browser_profile(bp_id)?;
            if ref_count > 1 {
                return Err(crate::error::AppError::ValidationError(format!(
                    "Cannot delete browser profile because it is shared with {} other account(s)",
                    ref_count - 1
                )));
            } else if ref_count == 1 {
                if delete_local_data {
                    let _ = state.browser_manager.safe_delete_profile_dir(bp_id);
                }
                let _ = state.db.delete_browser_profile(bp_id);
            }
        }
    }

    if delete_local_data {
        let path = PathBuf::from(&account.profile_path);
        if path.exists() {
            let _ = crate::fs_safety::safe_remove_dir_all(&state.base_profiles_dir, &path);
        }
        if let Some(browser_path) = &account.browser_profile_path {
            let b_path = PathBuf::from(browser_path);
            if b_path.exists() {
                let _ =
                    crate::fs_safety::safe_remove_dir_all(&state.browser_manager.base_dir, &b_path);
            }
        }
    }

    state.db.delete_account(id)
}

#[tauri::command]
pub fn delete_account(
    id: String,
    delete_local_data: bool,
    delete_browser_profile: Option<bool>,
    state: State<AppState>,
) -> Result<()> {
    delete_account_impl(&id, delete_local_data, delete_browser_profile, &state)
}

pub fn cleanup_draft_account_impl(id: &str, state: &AppState) -> Result<()> {
    let account = state.db.get_account(id)?;
    let path = PathBuf::from(&account.profile_path);
    if path.exists() {
        let _ = crate::fs_safety::safe_remove_dir_all(&state.base_profiles_dir, &path);
    }
    if let Some(browser_path) = &account.browser_profile_path {
        let b_path = PathBuf::from(browser_path);
        if b_path.exists() {
            let _ = crate::fs_safety::safe_remove_dir_all(&state.browser_manager.base_dir, &b_path);
        }
    }
    state.db.delete_account(id)
}

#[tauri::command]
pub fn cleanup_draft_account(id: String, state: State<AppState>) -> Result<()> {
    cleanup_draft_account_impl(&id, &state)
}

pub fn start_login_flow_impl(id: &str, state: &AppState) -> Result<AuthFlowStartResult> {
    let account = state.db.get_account(id)?;
    let adapter = get_adapter(account.platform);
    let spec = adapter.build_login_spec(&account)?;

    let target_surface = if spec.is_terminal {
        ExecutionSurface::Cli
    } else {
        adapter.default_surface()
    };

    let pid = if spec.is_terminal {
        let title = format!("{} Official Login", account.display_name);
        crate::launcher::CliLauncher::spawn(&title, &spec)?
    } else {
        match target_surface {
            ExecutionSurface::DesktopApp => crate::launcher::DesktopAppLauncher::spawn(&spec)?,
            ExecutionSurface::Cli => {
                let title = format!("{} Official Login", account.display_name);
                crate::launcher::CliLauncher::spawn(&title, &spec)?
            }
            ExecutionSurface::Web => {
                let url = if !spec.arguments.is_empty() {
                    &spec.arguments[0]
                } else {
                    "https://claude.ai"
                };
                crate::launcher::WebLauncher::open(url, None)?
            }
        }
    };

    crate::protocol_broker::register_pending_auth(
        &account.id,
        account.platform,
        target_surface,
        &account.profile_path,
        account.custom_executable_path.as_deref(),
        Some(pid),
    );

    let (flow_type, verification_mode, message) = match account.platform {
        PlatformType::Codex => (
            "cli_oauth".to_string(),
            "cli_status".to_string(),
            "Official sign-in opened in terminal. Complete sign-in, then click Verify.".to_string(),
        ),
        PlatformType::Claude => (
            "electron_oauth".to_string(),
            "cookies_sqlite".to_string(),
            "Official sign-in opened. Complete sign-in in the Claude app, then click Verify.".to_string(),
        ),
        PlatformType::Antigravity => (
            "electron_oauth".to_string(),
            "oauth_creds_json".to_string(),
            "Official sign-in opened. Complete browser authorization, return here and click Verify.".to_string(),
        ),
    };

    Ok(AuthFlowStartResult {
        platform: account.platform,
        flow_type,
        process_started: true,
        helper_pid: Some(pid),
        verification_mode,
        message,
    })
}

#[tauri::command]
pub fn start_login_flow(id: String, state: State<AppState>) -> Result<AuthFlowStartResult> {
    start_login_flow_impl(&id, &state)
}

#[tauri::command]
pub async fn check_account_status(id: String, state: State<'_, AppState>) -> Result<AccountStatus> {
    let account = state.db.get_account(&id)?;
    let adapter = get_adapter(account.platform);
    let status = adapter.check_status(&account).await?;
    let auth_status = match status {
        AccountStatus::Ready => AuthStatus::Authenticated,
        AccountStatus::LoginRequired => AuthStatus::LoginRequired,
        AccountStatus::Error => AuthStatus::Error,
        _ => AuthStatus::Unknown,
    };
    state.db.update_account_auth_status(&id, auth_status)?;
    if status == AccountStatus::Ready {
        crate::protocol_broker::cleanup_pending_auth(Some(&id), None);
    }
    Ok(status)
}

#[tauri::command]
pub async fn logout_account(id: String, state: State<'_, AppState>) -> Result<AccountStatus> {
    let account = state.db.get_account(&id)?;
    let adapter = get_adapter(account.platform);
    adapter.logout(&account).await?;
    let status = adapter.check_status(&account).await?;
    state.db.update_account_status(&id, status)?;
    Ok(status)
}

#[tauri::command]
pub fn check_launch_conflict(
    account_id: String,
    state: State<AppState>,
) -> Result<Option<ProcessConflictInfo>> {
    let account = state.db.get_account(&account_id)?;
    let adapter = get_adapter(account.platform);
    let all_accounts = state.db.list_accounts(true)?;

    Ok(state
        .launcher_engine
        .check_conflict(&account, adapter.as_ref(), &all_accounts))
}

pub fn launch_profile_impl(
    account_id: &str,
    surface: Option<ExecutionSurface>,
    workspace_path: Option<String>,
    state: &AppState,
) -> Result<ProcessRecord> {
    let account = state.db.get_account(account_id)?;
    let adapter = get_adapter(account.platform);

    let workspace_ref = workspace_path
        .as_ref()
        .map(|s| Path::new(s.as_str()))
        .or_else(|| {
            account
                .default_workspace_path
                .as_ref()
                .map(|s| Path::new(s.as_str()))
        });

    if surface == Some(ExecutionSurface::Web) {
        if let Some(ref bp_id) = account.browser_profile_id {
            if let Ok(bp) = state.db.get_browser_profile(bp_id) {
                let target_url = match account.platform {
                    PlatformType::Claude => "https://claude.ai",
                    PlatformType::Codex => "https://chatgpt.com",
                    _ => "https://claude.ai",
                };

                let pid = state.browser_manager.launch(&bp, Some(target_url))?;
                let _ = state.db.update_last_launched(&account.id);
                let _ = state.db.update_browser_last_used(bp_id);

                let record = state.launcher_engine.process_manager.register_launch(
                    &account.id,
                    account.platform,
                    ExecutionSurface::Web,
                    Some(pid),
                    &bp.user_data_directory,
                );

                return Ok(record);
            }
        }
    }

    let record =
        state
            .launcher_engine
            .launch(&account, adapter.as_ref(), surface, workspace_ref)?;

    let _ = state.db.update_last_launched(&account.id);

    Ok(record)
}

#[tauri::command]
pub fn launch_profile(
    account_id: String,
    surface: Option<ExecutionSurface>,
    workspace_path: Option<String>,
    state: State<AppState>,
) -> Result<ProcessRecord> {
    launch_profile_impl(&account_id, surface, workspace_path, &state)
}

#[tauri::command]
pub fn terminate_process(
    launch_id: String,
    force: Option<bool>,
    state: State<AppState>,
) -> Result<()> {
    state
        .launcher_engine
        .process_manager
        .terminate_process(&launch_id, force.unwrap_or(false))
}

#[tauri::command]
pub fn list_active_processes(state: State<AppState>) -> Result<Vec<ProcessRecord>> {
    Ok(state
        .launcher_engine
        .process_manager
        .list_active_processes())
}

#[tauri::command]
pub fn list_browser_profiles(state: State<AppState>) -> Result<Vec<BrowserProfile>> {
    state.db.list_browser_profiles()
}

#[tauri::command]
pub fn get_browser_profile(id: String, state: State<AppState>) -> Result<BrowserProfile> {
    state.db.get_browser_profile(&id)
}

pub fn create_browser_profile_impl(
    input: CreateBrowserProfileInput,
    state: &AppState,
) -> Result<BrowserProfile> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Create persistent directory: %APPDATA%\AI-Switcher\browser-profiles\<id>
    let user_data_dir = state.browser_manager.ensure_profile_dir(&id)?;

    let profile = BrowserProfile {
        id: id.clone(),
        display_name: input.display_name,
        browser_kind: input.browser_kind,
        custom_executable_path: input.custom_executable_path,
        user_data_directory: user_data_dir.to_string_lossy().to_string(),
        created_at: now.clone(),
        updated_at: now,
        last_used_at: None,
    };

    state.db.insert_browser_profile(&profile)?;
    Ok(profile)
}

#[tauri::command]
pub fn create_browser_profile(
    input: CreateBrowserProfileInput,
    state: State<AppState>,
) -> Result<BrowserProfile> {
    create_browser_profile_impl(input, &state)
}

#[tauri::command]
pub fn update_browser_profile(
    input: UpdateBrowserProfileInput,
    state: State<AppState>,
) -> Result<BrowserProfile> {
    state.db.update_browser_profile(&input)
}

#[tauri::command]
pub fn delete_browser_profile(
    id: String,
    delete_local_data: bool,
    state: State<AppState>,
) -> Result<()> {
    // Reference safety check: ensure no accounts are linked
    let ref_count = state.db.count_accounts_referencing_browser_profile(&id)?;
    if ref_count > 0 {
        let linked_accounts = state.db.list_accounts_referencing_browser_profile(&id)?;
        let names: Vec<String> = linked_accounts
            .iter()
            .map(|a| a.display_name.clone())
            .collect();
        return Err(crate::error::AppError::ValidationError(format!(
            "Cannot delete browser profile because it is referenced by {} account(s): {}",
            ref_count,
            names.join(", ")
        )));
    }

    if delete_local_data {
        state.browser_manager.safe_delete_profile_dir(&id)?;
    }

    state.db.delete_browser_profile(&id)
}

#[tauri::command]
pub fn launch_browser_profile(
    id: String,
    url: Option<String>,
    state: State<AppState>,
) -> Result<u32> {
    let profile = state.db.get_browser_profile(&id)?;
    let pid = state.browser_manager.launch(&profile, url.as_deref())?;
    let _ = state.db.update_browser_last_used(&id);
    Ok(pid)
}

#[tauri::command]
pub fn detect_available_browsers(
    state: State<AppState>,
) -> Result<Vec<crate::models::DetectedBrowserInfo>> {
    Ok(state.browser_manager.detect_available_browsers())
}

#[tauri::command]
pub fn get_diagnostics(state: State<AppState>) -> Result<DiagnosticsInfo> {
    let codex_exec = get_adapter(PlatformType::Codex)
        .detect_executable()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let claude_exec = get_adapter(PlatformType::Claude)
        .detect_executable()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let antigravity_exec = get_adapter(PlatformType::Antigravity)
        .detect_executable()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let detected_browsers = state.browser_manager.detect_available_browsers();
    let browser_exec = detected_browsers
        .iter()
        .find(|b| b.is_available)
        .map(|b| b.executable_path.clone());

    let browser_profiles = state.db.list_browser_profiles().unwrap_or_default();
    let active_count = state
        .launcher_engine
        .process_manager
        .list_active_processes()
        .len();

    Ok(DiagnosticsInfo {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_version: "Windows 10 / 11 (64-bit)".to_string(),
        data_dir: state.base_profiles_dir.to_string_lossy().to_string(),
        codex_executable: codex_exec,
        claude_executable: claude_exec,
        antigravity_executable: antigravity_exec,
        browser_executable: browser_exec,
        active_process_count: active_count,
        detected_browsers,
        browser_profile_count: browser_profiles.len(),
    })
}

pub fn list_workspaces_impl(state: &AppState) -> Result<Vec<WorkspacePreset>> {
    state.db.list_workspaces()
}

#[tauri::command]
pub fn list_workspaces(state: State<AppState>) -> Result<Vec<WorkspacePreset>> {
    list_workspaces_impl(&state)
}

pub fn get_workspace_impl(id: &str, state: &AppState) -> Result<WorkspacePreset> {
    state.db.get_workspace(id)
}

#[tauri::command]
pub fn get_workspace(id: String, state: State<AppState>) -> Result<WorkspacePreset> {
    get_workspace_impl(&id, &state)
}

pub fn create_workspace_impl(
    input: CreateWorkspaceInput,
    state: &AppState,
) -> Result<WorkspacePreset> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(crate::error::AppError::ValidationError(
            "Workspace preset name cannot be empty".to_string(),
        ));
    }
    let dir = input.directory_path.trim();
    if dir.is_empty() {
        return Err(crate::error::AppError::ValidationError(
            "Workspace directory path cannot be empty".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let preset = WorkspacePreset {
        id: id.clone(),
        name: name.to_string(),
        directory_path: dir.to_string(),
        preferred_codex_account_id: input.preferred_codex_account_id,
        preferred_claude_account_id: input.preferred_claude_account_id,
        preferred_antigravity_account_id: input.preferred_antigravity_account_id,
        last_opened_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    state.db.insert_workspace(&preset)?;
    state.db.get_workspace(&id)
}

#[tauri::command]
pub fn create_workspace(
    input: CreateWorkspaceInput,
    state: State<AppState>,
) -> Result<WorkspacePreset> {
    create_workspace_impl(input, &state)
}

pub fn update_workspace_impl(
    input: UpdateWorkspaceInput,
    state: &AppState,
) -> Result<WorkspacePreset> {
    if let Some(ref name) = input.name {
        if name.trim().is_empty() {
            return Err(crate::error::AppError::ValidationError(
                "Workspace preset name cannot be empty".to_string(),
            ));
        }
    }
    if let Some(ref dir) = input.directory_path {
        if dir.trim().is_empty() {
            return Err(crate::error::AppError::ValidationError(
                "Workspace directory path cannot be empty".to_string(),
            ));
        }
    }

    state.db.update_workspace(&input)
}

#[tauri::command]
pub fn update_workspace(
    input: UpdateWorkspaceInput,
    state: State<AppState>,
) -> Result<WorkspacePreset> {
    update_workspace_impl(input, &state)
}

pub fn delete_workspace_impl(id: &str, state: &AppState) -> Result<()> {
    state.db.delete_workspace(id)
}

#[tauri::command]
pub fn delete_workspace(id: String, state: State<AppState>) -> Result<()> {
    delete_workspace_impl(&id, &state)
}

pub fn launch_workspace_preset_impl(
    workspace_id: &str,
    platform: PlatformType,
    surface: Option<ExecutionSurface>,
    state: &AppState,
) -> Result<ProcessRecord> {
    let ws = state.db.get_workspace(workspace_id)?;

    let preferred_id = match platform {
        PlatformType::Codex => ws.preferred_codex_account_id.as_deref(),
        PlatformType::Claude => ws.preferred_claude_account_id.as_deref(),
        PlatformType::Antigravity => ws.preferred_antigravity_account_id.as_deref(),
    };

    let target_account = if let Some(pref_id) = preferred_id {
        match state.db.get_account(pref_id) {
            Ok(acc) => {
                if !acc.is_enabled {
                    return Err(crate::error::AppError::AccountDisabled(acc.display_name));
                }
                acc
            }
            Err(_) => {
                let accounts = state.db.list_accounts(false)?;
                accounts
                    .into_iter()
                    .find(|a| a.platform == platform && a.is_enabled)
                    .ok_or_else(|| {
                        crate::error::AppError::NotFound(format!(
                            "No enabled account found for platform {}",
                            platform.as_str()
                        ))
                    })?
            }
        }
    } else {
        let accounts = state.db.list_accounts(false)?;
        accounts
            .into_iter()
            .find(|a| a.platform == platform && a.is_enabled)
            .ok_or_else(|| {
                crate::error::AppError::NotFound(format!(
                    "No enabled account configured for platform {}",
                    platform.as_str()
                ))
            })?
    };

    let ws_dir = Path::new(&ws.directory_path);
    if !ws_dir.exists() {
        return Err(crate::error::AppError::WorkspaceDirectoryNotFound(
            ws.directory_path.clone(),
        ));
    }

    let record = launch_profile_impl(
        &target_account.id,
        surface,
        Some(ws.directory_path.clone()),
        state,
    )?;
    let _ = state.db.update_workspace_last_opened(&ws.id);
    Ok(record)
}

#[tauri::command]
pub fn launch_workspace_preset(
    workspace_id: String,
    platform: PlatformType,
    surface: Option<ExecutionSurface>,
    state: State<AppState>,
) -> Result<ProcessRecord> {
    launch_workspace_preset_impl(&workspace_id, platform, surface, &state)
}

pub fn create_workspace_directory_impl(workspace_id: &str, state: &AppState) -> Result<()> {
    let ws = state.db.get_workspace(workspace_id)?;
    let ws_dir = Path::new(&ws.directory_path);
    std::fs::create_dir_all(ws_dir).map_err(|e| {
        crate::error::AppError::IoError(format!(
            "Failed to create workspace directory '{}': {}",
            ws.directory_path, e
        ))
    })?;
    Ok(())
}

#[tauri::command]
pub fn create_workspace_directory(workspace_id: String, state: State<AppState>) -> Result<()> {
    create_workspace_directory_impl(&workspace_id, &state)
}

#[tauri::command]
pub fn get_app_settings(state: State<AppState>) -> Result<AppSettings> {
    state.db.get_app_settings()
}

#[tauri::command]
pub fn save_app_settings(settings: AppSettings, state: State<AppState>) -> Result<()> {
    state.db.save_app_settings(&settings)
}

#[tauri::command]
pub fn list_favorites(state: State<AppState>) -> Result<Vec<FavoriteTarget>> {
    state.db.list_favorites()
}

#[tauri::command]
pub fn toggle_favorite_account(account_id: String, state: State<AppState>) -> Result<bool> {
    state.db.toggle_favorite_account(&account_id)
}

#[tauri::command]
pub fn toggle_favorite_workspace(
    workspace_id: String,
    platform: Option<String>,
    state: State<AppState>,
) -> Result<bool> {
    state
        .db
        .toggle_favorite_workspace(&workspace_id, platform.as_deref())
}

#[tauri::command]
pub fn get_recent_launches(
    limit: Option<usize>,
    state: State<AppState>,
) -> Result<Vec<RecentItem>> {
    state.db.get_recent_launches(limit.unwrap_or(5))
}

#[tauri::command]
pub fn refresh_tray_menu(app_handle: AppHandle, state: State<AppState>) -> Result<()> {
    let _ = crate::tray::refresh_tray_menu(&app_handle, &state);
    Ok(())
}

// ==========================================
// Phase 10: Health, Recovery & Reliability
// ==========================================

pub fn check_account_health_impl(
    account: &AccountProfile,
    _base_profiles_dir: &Path,
    _browser_profiles_dir: &Path,
    db: &Db,
) -> AccountHealthReport {
    let profile_dir = PathBuf::from(&account.profile_path);

    // 1. Check directory existence
    if !profile_dir.exists() {
        return AccountHealthReport {
            account_id: account.id.clone(),
            platform: account.platform,
            display_name: account.display_name.clone(),
            status: account.status,
            health: ProfileHealth::ProfileDirectoryMissing,
            details: format!(
                "Profile directory '{}' does not exist on disk.",
                profile_dir.display()
            ),
            can_repair: true,
        };
    }

    // 2. Check scaffolding
    match account.platform {
        PlatformType::Codex => {
            if !profile_dir.join("config.toml").exists() {
                return AccountHealthReport {
                    account_id: account.id.clone(),
                    platform: account.platform,
                    display_name: account.display_name.clone(),
                    status: account.status,
                    health: ProfileHealth::ProfileDirectoryMissing,
                    details: "Missing configuration file (config.toml).".to_string(),
                    can_repair: true,
                };
            }
        }
        PlatformType::Claude => {
            if !profile_dir.join("desktop").exists() {
                return AccountHealthReport {
                    account_id: account.id.clone(),
                    platform: account.platform,
                    display_name: account.display_name.clone(),
                    status: account.status,
                    health: ProfileHealth::ProfileDirectoryMissing,
                    details: "Missing desktop data directory scaffolding.".to_string(),
                    can_repair: true,
                };
            }
        }
        PlatformType::Antigravity => {
            if !profile_dir.join("home").exists() || !profile_dir.join("data").exists() {
                return AccountHealthReport {
                    account_id: account.id.clone(),
                    platform: account.platform,
                    display_name: account.display_name.clone(),
                    status: account.status,
                    health: ProfileHealth::ProfileDirectoryMissing,
                    details: "Missing isolated home/data directory scaffolding.".to_string(),
                    can_repair: true,
                };
            }
        }
    }

    // 3. Check executable existence
    if let Some(custom_path) = &account.custom_executable_path {
        if !Path::new(custom_path).exists() {
            return AccountHealthReport {
                account_id: account.id.clone(),
                platform: account.platform,
                display_name: account.display_name.clone(),
                status: account.status,
                health: ProfileHealth::ExecutableMissing,
                details: format!(
                    "Configured custom executable was not found: '{}'",
                    custom_path
                ),
                can_repair: false,
            };
        }
    } else {
        let adapter = get_adapter(account.platform);
        if adapter.detect_executable().is_err() {
            return AccountHealthReport {
                account_id: account.id.clone(),
                platform: account.platform,
                display_name: account.display_name.clone(),
                status: account.status,
                health: ProfileHealth::ExecutableMissing,
                details: format!(
                    "{} executable was not found in PATH or standard install paths.",
                    adapter.display_name()
                ),
                can_repair: false,
            };
        }
    }

    // 4. Check browser profile if linked
    if let Some(bp_id) = &account.browser_profile_id {
        match db.get_browser_profile(bp_id) {
            Ok(bp) => {
                let bp_dir = PathBuf::from(&bp.user_data_directory);
                if !bp_dir.exists() {
                    return AccountHealthReport {
                        account_id: account.id.clone(),
                        platform: account.platform,
                        display_name: account.display_name.clone(),
                        status: account.status,
                        health: ProfileHealth::BrowserProfileUnavailable,
                        details: format!(
                            "Linked browser profile directory '{}' is missing.",
                            bp_dir.display()
                        ),
                        can_repair: false,
                    };
                }
                // Check browser lock
                let lock_file = bp_dir.join("SingletonLock");
                if lock_file.exists() {
                    return AccountHealthReport {
                        account_id: account.id.clone(),
                        platform: account.platform,
                        display_name: account.display_name.clone(),
                        status: account.status,
                        health: ProfileHealth::Locked,
                        details:
                            "Browser profile is currently in use by an active browser instance."
                                .to_string(),
                        can_repair: false,
                    };
                }
            }
            Err(_) => {
                return AccountHealthReport {
                    account_id: account.id.clone(),
                    platform: account.platform,
                    display_name: account.display_name.clone(),
                    status: account.status,
                    health: ProfileHealth::BrowserProfileUnavailable,
                    details: "Linked browser profile record was deleted or not found.".to_string(),
                    can_repair: false,
                };
            }
        }
    }

    // 5. Check account authentication status
    match account.status {
        AccountStatus::LoginRequired => AccountHealthReport {
            account_id: account.id.clone(),
            platform: account.platform,
            display_name: account.display_name.clone(),
            status: account.status,
            health: ProfileHealth::AuthenticationRequired,
            details: "Session expired or authentication required.".to_string(),
            can_repair: false,
        },
        AccountStatus::Error => AccountHealthReport {
            account_id: account.id.clone(),
            platform: account.platform,
            display_name: account.display_name.clone(),
            status: account.status,
            health: ProfileHealth::Error,
            details: "Account is in an error state.".to_string(),
            can_repair: false,
        },
        _ => AccountHealthReport {
            account_id: account.id.clone(),
            platform: account.platform,
            display_name: account.display_name.clone(),
            status: account.status,
            health: ProfileHealth::Healthy,
            details: "Profile directory, configuration, and executable are intact.".to_string(),
            can_repair: false,
        },
    }
}

pub fn repair_account_profile_impl(id: &str, state: &AppState) -> Result<AccountHealthReport> {
    let mut account = state.db.get_account(id)?;
    let profile_dir = PathBuf::from(&account.profile_path);

    // Recreate base profile dir
    std::fs::create_dir_all(&profile_dir)?;

    // Recreate platform-specific scaffolding
    match account.platform {
        PlatformType::Codex => {
            let config_path = profile_dir.join("config.toml");
            if !config_path.exists() {
                let default_cfg =
                    "# OpenAI Codex Isolated Configuration\n# Generated by AI Switcher\n";
                std::fs::write(&config_path, default_cfg)?;
            }
        }
        PlatformType::Claude => {
            std::fs::create_dir_all(profile_dir.join("desktop"))?;
        }
        PlatformType::Antigravity => {
            std::fs::create_dir_all(profile_dir.join("home"))?;
            std::fs::create_dir_all(profile_dir.join("data"))?;
            std::fs::create_dir_all(profile_dir.join("appdata"))?;
        }
    }

    // If the directory had to be recreated from scratch, session is lost -> set to LoginRequired.
    // NEVER manufacture credentials!
    if account.status == AccountStatus::Ready {
        account.status = AccountStatus::LoginRequired;
        state
            .db
            .update_account_status(&account.id, AccountStatus::LoginRequired)?;
    }

    Ok(check_account_health_impl(
        &account,
        &state.base_profiles_dir,
        &state.browser_manager.base_dir,
        &state.db,
    ))
}

pub fn get_system_health_impl(state: &AppState) -> Result<SystemHealthReport> {
    let db_integrity = state.db.check_integrity().unwrap_or(false);
    let schema_ver = state.db.get_schema_version().unwrap_or(0);
    let (backup_count, last_backup_at) = state.db.get_backup_info().unwrap_or((0, None));
    let accounts = state.db.list_accounts(true)?;

    let mut account_reports = Vec::new();
    for acc in &accounts {
        account_reports.push(check_account_health_impl(
            acc,
            &state.base_profiles_dir,
            &state.browser_manager.base_dir,
            &state.db,
        ));
    }

    let settings = state.db.get_app_settings().unwrap_or_default();

    Ok(SystemHealthReport {
        db_healthy: true,
        db_integrity_ok: db_integrity,
        schema_version: schema_ver,
        db_path: state.db.get_path().to_string_lossy().to_string(),
        backup_count,
        last_backup_at,
        managed_root_exists: state.base_profiles_dir.exists(),
        managed_root_path: state.base_profiles_dir.to_string_lossy().to_string(),
        accounts: account_reports,
        tray_status: "active".to_string(),
        shortcut_status: if settings.global_shortcut.is_empty() {
            "disabled".to_string()
        } else {
            "registered".to_string()
        },
        autostart_status: "active".to_string(),
    })
}

pub fn export_support_bundle_impl(state: &AppState) -> Result<SupportBundle> {
    let health = get_system_health_impl(state)?;
    let browsers = state.browser_manager.detect_available_browsers();
    let capabilities = list_platform_capabilities_impl();
    let settings = state.db.get_app_settings().unwrap_or_default();

    Ok(SupportBundle {
        generated_at: chrono::Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        os_version: "Windows (x86_64)".to_string(),
        schema_version: health.schema_version,
        db_integrity_ok: health.db_integrity_ok,
        platform_capabilities: capabilities,
        accounts: health.accounts,
        backup_count: health.backup_count,
        detected_browsers: browsers,
        settings_summary: settings,
    })
}

#[tauri::command]
pub fn check_account_health(id: String, state: State<AppState>) -> Result<AccountHealthReport> {
    let account = state.db.get_account(&id)?;
    Ok(check_account_health_impl(
        &account,
        &state.base_profiles_dir,
        &state.browser_manager.base_dir,
        &state.db,
    ))
}

#[tauri::command]
pub fn repair_account_profile(id: String, state: State<AppState>) -> Result<AccountHealthReport> {
    repair_account_profile_impl(&id, &state)
}

#[tauri::command]
pub fn get_system_health(state: State<AppState>) -> Result<SystemHealthReport> {
    get_system_health_impl(&state)
}

#[tauri::command]
pub fn create_db_backup(state: State<AppState>) -> Result<String> {
    let backup_path = state.db.backup_database()?;
    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn check_db_integrity(state: State<AppState>) -> Result<bool> {
    state.db.check_integrity()
}

#[tauri::command]
pub fn export_support_bundle(state: State<AppState>) -> Result<SupportBundle> {
    export_support_bundle_impl(&state)
}
