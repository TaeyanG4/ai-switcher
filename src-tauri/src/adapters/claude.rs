use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::adapters::{LaunchSpec, PlatformAdapter};
use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, AccountStatus, AuthStatus, ExecutionSurface, InstancePolicy, LaunchTarget,
    PlatformType,
};

pub struct ClaudeAdapter;

impl ClaudeAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Check if Claude Desktop application is installed on Windows
    pub fn is_desktop_installed(&self) -> bool {
        self.detect_desktop_executable().is_ok()
    }

    /// Locate Claude Desktop executable on Windows.
    /// Checks Squirrel AppData installation directories:
    /// - %LOCALAPPDATA%\AnthropicClaude\claude.exe
    /// - %LOCALAPPDATA%\AnthropicClaude\app-*\claude.exe
    pub fn detect_desktop_executable(&self) -> Result<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(local) = dirs::data_local_dir() {
            let anthropic_dir = local.join("AnthropicClaude");

            // Look for latest versioned app directory: app-1.46388.2/claude.exe
            if anthropic_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&anthropic_dir) {
                    let mut versioned = Vec::new();
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with("app-") {
                                    let exe = path.join("claude.exe");
                                    if exe.is_file() {
                                        versioned.push((name.to_string(), exe));
                                    }
                                }
                            }
                        }
                    }
                    // Sort to pick latest app-X.Y.Z
                    versioned.sort_by(|a, b| b.0.cmp(&a.0));
                    if let Some((_, exe)) = versioned.into_iter().next() {
                        candidates.push(exe);
                    }
                }

                // Fallback to top-level launcher stub
                let stub = anthropic_dir.join("claude.exe");
                if stub.is_file() {
                    candidates.push(stub);
                }
            }
        }

        // Program Files check
        if let Ok(pf) = std::env::var("ProgramFiles") {
            let p = PathBuf::from(pf).join("AnthropicClaude\\claude.exe");
            if p.is_file() {
                candidates.push(p);
            }
        }

        for c in candidates {
            if c.is_file() {
                return Ok(c);
            }
        }

        Err(AppError::ExecutableNotFound(
            "Claude Desktop executable not found in %LOCALAPPDATA%\\AnthropicClaude".to_string(),
        ))
    }

    /// Locate Claude Code CLI executable on Windows.
    /// Checks:
    /// - %USERPROFILE%\.local\bin\claude.exe
    /// - %APPDATA%\npm\claude.cmd
    /// - PATH via `where.exe`
    pub fn detect_cli_executable(&self) -> Result<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local\\bin\\claude.exe"));
        }

        if let Some(appdata) = dirs::data_dir() {
            candidates.push(appdata.join("npm\\claude.cmd"));
            candidates.push(appdata.join("npm\\claude.exe"));
        }

        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("where.exe").arg("claude.exe").output() {
                if output.status.success() {
                    if let Ok(s) = String::from_utf8(output.stdout) {
                        for line in s.lines() {
                            let trimmed = line.trim();
                            if !trimmed.is_empty() {
                                let p = PathBuf::from(trimmed);
                                // Skip Claude Desktop binary if in PATH
                                if !trimmed.contains("AnthropicClaude") && p.is_file() {
                                    candidates.push(p);
                                }
                            }
                        }
                    }
                }
            }
        }

        for c in candidates {
            if c.is_file() {
                return Ok(c);
            }
        }

        Err(AppError::ExecutableNotFound(
            "Claude Code CLI executable not found".to_string(),
        ))
    }

    /// Build the official claude:// deep link URL for launching into Claude Code.
    /// Format: claude://code/new?folder=<percent_encoded_folder_path>
    pub fn build_code_deeplink(workspace: Option<&Path>) -> String {
        match workspace {
            Some(ws) => {
                let path_str = ws.to_string_lossy();
                let encoded = Self::url_encode_path(&path_str);
                format!("claude://code/new?folder={}", encoded)
            }
            None => "claude://code/new".to_string(),
        }
    }

    /// Percent-encode file path for safe URL inclusion (RFC 3986 / encodeURIComponent style)
    pub fn url_encode_path(path: &str) -> String {
        let mut encoded = String::new();
        for b in path.as_bytes() {
            match *b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(*b as char);
                }
                _ => {
                    encoded.push_str(&format!("%{:02X}", b));
                }
            }
        }
        encoded
    }
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformAdapter for ClaudeAdapter {
    fn platform(&self) -> PlatformType {
        PlatformType::Claude
    }

    fn display_name(&self) -> &'static str {
        "Anthropic Claude"
    }

    fn instance_policy(&self) -> InstancePolicy {
        InstancePolicy::MultiInstance
    }

    fn supported_surfaces(&self) -> Vec<ExecutionSurface> {
        vec![
            ExecutionSurface::DesktopApp,
            ExecutionSurface::Cli,
            ExecutionSurface::Web,
        ]
    }

    fn default_surface(&self) -> ExecutionSurface {
        if self.is_desktop_installed() {
            ExecutionSurface::DesktopApp
        } else {
            ExecutionSurface::Cli
        }
    }

    fn detect_executable(&self) -> Result<PathBuf> {
        if let Ok(desktop_exe) = self.detect_desktop_executable() {
            Ok(desktop_exe)
        } else {
            self.detect_cli_executable()
        }
    }

    fn initialize_profile(&self, profile_path: &Path) -> Result<()> {
        // Desktop isolated user-data-dir
        std::fs::create_dir_all(profile_path.join("desktop"))?;
        // CLI isolated config dir and projects
        std::fs::create_dir_all(profile_path.join("cli").join("projects"))?;
        std::fs::create_dir_all(profile_path.join("projects"))?;
        Ok(())
    }

    async fn check_status(&self, profile: &AccountProfile) -> Result<AccountStatus> {
        let auth = self
            .check_surface_auth_status(profile, ExecutionSurface::DesktopApp)
            .await?;
        Ok(auth.to_account_status())
    }

    async fn check_surface_auth_status(
        &self,
        profile: &AccountProfile,
        surface: ExecutionSurface,
    ) -> Result<AuthStatus> {
        let profile_dir = PathBuf::from(&profile.profile_path);
        if !profile_dir.exists() {
            return Ok(AuthStatus::LoginRequired);
        }

        match surface {
            ExecutionSurface::DesktopApp => {
                let candidate_cookie_paths = [
                    profile_dir.join("desktop").join("Network").join("Cookies"),
                    profile_dir.join("desktop").join("Cookies"),
                ];

                let mut any_cookie_file_found = false;
                let mut db_error = false;

                for cookie_path in &candidate_cookie_paths {
                    if cookie_path.exists() {
                        any_cookie_file_found = true;
                        match rusqlite::Connection::open_with_flags(
                            cookie_path,
                            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
                        ) {
                            Ok(conn) => {
                                let _ = conn.execute("PRAGMA query_only = ON;", []);
                                let count_res: rusqlite::Result<i64> = conn.query_row(
                                    "SELECT COUNT(*) FROM cookies WHERE host_key LIKE '%claude.ai%' AND name = 'sessionKey'",
                                    [],
                                    |r| r.get(0),
                                );
                                match count_res {
                                    Ok(count) => {
                                        if count > 0 {
                                            return Ok(AuthStatus::Authenticated);
                                        }
                                    }
                                    Err(_) => {
                                        db_error = true;
                                    }
                                }
                            }
                            Err(_) => {
                                db_error = true;
                            }
                        }
                    }
                }

                if db_error {
                    // DB locked by running Claude instance, permissions, or schema change
                    return Ok(AuthStatus::Unknown);
                }

                if !any_cookie_file_found {
                    return Ok(AuthStatus::LoginRequired);
                }

                Ok(AuthStatus::LoginRequired)
            }
            ExecutionSurface::Cli => {
                if let Ok(cli_exe) = self.detect_cli_executable() {
                    let cli_dir = profile_dir.join("cli");
                    let mut cmd = Command::new(cli_exe);
                    cmd.args(["auth", "status", "--json"]);
                    cmd.env("CLAUDE_CONFIG_DIR", &cli_dir);

                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                    }

                    if let Ok(output) = cmd.output() {
                        if let Ok(stdout) = String::from_utf8(output.stdout) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                                if let Some(logged_in) =
                                    json.get("loggedIn").and_then(|v| v.as_bool())
                                {
                                    if logged_in {
                                        return Ok(AuthStatus::Authenticated);
                                    } else {
                                        return Ok(AuthStatus::LoginRequired);
                                    }
                                }
                            }
                        }
                        if output.status.success() {
                            return Ok(AuthStatus::Authenticated);
                        }
                        return Ok(AuthStatus::LoginRequired);
                    }
                    Ok(AuthStatus::Unknown)
                } else {
                    Ok(AuthStatus::Unknown)
                }
            }
            ExecutionSurface::Web => Ok(AuthStatus::Unknown),
        }
    }

    fn build_launch_spec(
        &self,
        profile: &AccountProfile,
        target: LaunchTarget,
        workspace_path: Option<&Path>,
    ) -> Result<LaunchSpec> {
        let workspace_ref = workspace_path
            .map(|p| p.to_path_buf())
            .or_else(|| profile.default_workspace_path.as_ref().map(PathBuf::from));

        let workdir = workspace_ref
            .clone()
            .unwrap_or_else(|| PathBuf::from(&profile.profile_path));

        match target {
            LaunchTarget::Desktop => {
                let exec = if let Some(custom) = &profile.custom_executable_path {
                    PathBuf::from(custom)
                } else {
                    self.detect_desktop_executable()?
                };

                let desktop_data_dir = PathBuf::from(&profile.profile_path).join("desktop");
                std::fs::create_dir_all(&desktop_data_dir)?;

                let deeplink = Self::build_code_deeplink(workspace_ref.as_deref());

                let arguments = vec![
                    format!("--user-data-dir={}", desktop_data_dir.to_string_lossy()),
                    deeplink,
                ];

                let mut env = Vec::new();
                for (k, v) in &profile.environment_variables {
                    env.push((k.clone(), v.clone()));
                }

                Ok(LaunchSpec {
                    executable: exec,
                    arguments,
                    environment: env,
                    working_directory: workdir,
                    is_terminal: false,
                })
            }
            LaunchTarget::Cli => {
                let exec = if let Some(custom) = &profile.custom_executable_path {
                    let p = PathBuf::from(custom);
                    if p.is_file() {
                        p
                    } else {
                        self.detect_cli_executable()?
                    }
                } else {
                    self.detect_cli_executable()?
                };

                let cli_config_dir = PathBuf::from(&profile.profile_path).join("cli");
                std::fs::create_dir_all(&cli_config_dir)?;

                let mut env = vec![(
                    "CLAUDE_CONFIG_DIR".to_string(),
                    cli_config_dir.to_string_lossy().to_string(),
                )];
                for (k, v) in &profile.environment_variables {
                    env.push((k.clone(), v.clone()));
                }

                Ok(LaunchSpec {
                    executable: exec,
                    arguments: vec![],
                    environment: env,
                    working_directory: workdir,
                    is_terminal: true,
                })
            }
            LaunchTarget::Web => {
                let mut env = Vec::new();
                for (k, v) in &profile.environment_variables {
                    env.push((k.clone(), v.clone()));
                }

                Ok(LaunchSpec {
                    executable: PathBuf::from("https://claude.ai"),
                    arguments: vec!["https://claude.ai".to_string()],
                    environment: env,
                    working_directory: workdir,
                    is_terminal: false,
                })
            }
            LaunchTarget::Default => {
                // If default target requested, redirect to default surface
                if self.is_desktop_installed() {
                    self.build_launch_spec(profile, LaunchTarget::Desktop, workspace_path)
                } else {
                    self.build_launch_spec(profile, LaunchTarget::Cli, workspace_path)
                }
            }
        }
    }

    async fn logout(&self, profile: &AccountProfile) -> Result<()> {
        let cli_dir = PathBuf::from(&profile.profile_path).join("cli");

        // Logout CLI session if executable is present
        if let Ok(cli_exe) = self.detect_cli_executable() {
            let mut cmd = Command::new(cli_exe);
            cmd.args(["auth", "logout"]);
            cmd.env("CLAUDE_CONFIG_DIR", &cli_dir);

            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x08000000);
            }

            let _ = cmd.output();
        }

        // Clean up desktop session cache files and cookies in isolated directory
        let desktop_dir = PathBuf::from(&profile.profile_path).join("desktop");
        if desktop_dir.exists() {
            let _ = std::fs::remove_dir_all(desktop_dir.join("Session Storage"));
            let _ = std::fs::remove_dir_all(desktop_dir.join("Local Storage"));
            let _ = std::fs::remove_file(desktop_dir.join("Network").join("Cookies"));
            let _ = std::fs::remove_file(desktop_dir.join("Cookies"));
        }

        Ok(())
    }
}
