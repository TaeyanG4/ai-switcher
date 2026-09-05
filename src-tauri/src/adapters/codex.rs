use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::adapters::{LaunchSpec, PlatformAdapter};
use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, AccountStatus, ExecutionSurface, InstancePolicy, LaunchTarget, PlatformType,
};

pub struct CodexAdapter;

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Check if Codex Desktop MSIX application package is installed on Windows
    pub fn is_desktop_installed(&self) -> bool {
        if let Some(local_app_data) = dirs::data_local_dir() {
            let pkg_dir = local_app_data.join("Packages\\OpenAI.Codex_2p2nqsd0c76g0");
            if pkg_dir.exists() {
                return true;
            }
        }

        // Check Program Files WindowsApps
        if let Ok(entries) = std::fs::read_dir("C:\\Program Files\\WindowsApps") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with("OpenAI.Codex_") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Build a launch specification to initiate official interactive login in a terminal
    pub fn build_login_spec(&self, profile: &AccountProfile) -> Result<LaunchSpec> {
        let exec = if let Some(custom) = &profile.custom_executable_path {
            PathBuf::from(custom)
        } else {
            self.detect_executable()?
        };

        let workdir = PathBuf::from(&profile.profile_path);
        let mut env = vec![("CODEX_HOME".to_string(), profile.profile_path.clone())];
        for (k, v) in &profile.environment_variables {
            env.push((k.clone(), v.clone()));
        }

        Ok(LaunchSpec {
            executable: exec,
            arguments: vec!["login".to_string()],
            environment: env,
            working_directory: workdir,
            is_terminal: true,
        })
    }
}

#[async_trait]
impl PlatformAdapter for CodexAdapter {
    fn platform(&self) -> PlatformType {
        PlatformType::Codex
    }

    fn display_name(&self) -> &'static str {
        "OpenAI Codex"
    }

    fn instance_policy(&self) -> InstancePolicy {
        InstancePolicy::SingleInstance
    }

    fn supported_surfaces(&self) -> Vec<ExecutionSurface> {
        vec![ExecutionSurface::DesktopApp, ExecutionSurface::Cli]
    }

    fn default_surface(&self) -> ExecutionSurface {
        ExecutionSurface::Cli
    }

    fn detect_executable(&self) -> Result<PathBuf> {
        let mut candidates = Vec::new();

        if let Some(local) = dirs::data_local_dir() {
            // Standalone user directory
            candidates.push(local.join("Programs\\OpenAI\\Codex\\bin\\codex.exe"));

            // Versioned runtimes subdirectories
            let runtimes_dir = local.join("OpenAI\\Codex\\bin");
            if runtimes_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&runtimes_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path().join("codex.exe");
                        if path.is_file() {
                            candidates.push(path);
                        }
                    }
                }
            }
        }

        // Check if `codex.exe` is in PATH
        if let Ok(output) = Command::new("where.exe").arg("codex.exe").output() {
            if output.status.success() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    for line in s.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            candidates.push(PathBuf::from(trimmed));
                        }
                    }
                }
            }
        }

        candidates.push(PathBuf::from("codex.exe"));

        for cand in candidates {
            if cand.exists() {
                return Ok(cand);
            }
        }

        Ok(PathBuf::from("codex.exe"))
    }

    fn initialize_profile(&self, profile_path: &Path) -> Result<()> {
        std::fs::create_dir_all(profile_path)?;
        std::fs::create_dir_all(profile_path.join("log"))?;
        std::fs::create_dir_all(profile_path.join("tmp"))?;

        let config_file = profile_path.join("config.toml");
        if !config_file.exists() {
            let default_config = r#"# OpenAI Codex Profile Configuration
# Managed by AI Switcher

# Default model configuration
model = "o3"
"#;
            std::fs::write(&config_file, default_config)?;
        }

        Ok(())
    }

    async fn check_status(&self, profile: &AccountProfile) -> Result<AccountStatus> {
        let exec = if let Some(custom) = &profile.custom_executable_path {
            PathBuf::from(custom)
        } else {
            self.detect_executable()?
        };

        let profile_dir = PathBuf::from(&profile.profile_path);
        if !profile_dir.exists() {
            return Ok(AccountStatus::Error);
        }

        let mut cmd = Command::new(&exec);
        cmd.args(["login", "status"]);
        cmd.env("CODEX_HOME", &profile.profile_path);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

                if output.status.success() {
                    Ok(AccountStatus::Ready)
                } else if stdout.contains("Not logged in")
                    || stderr.contains("Not logged in")
                    || stderr.contains("no Codex credentials")
                {
                    Ok(AccountStatus::LoginRequired)
                } else {
                    Ok(AccountStatus::Error)
                }
            }
            Err(_) => Ok(AccountStatus::Error),
        }
    }

    fn build_launch_spec(
        &self,
        profile: &AccountProfile,
        target: LaunchTarget,
        workspace_path: Option<&Path>,
    ) -> Result<LaunchSpec> {
        let exec = if let Some(custom) = &profile.custom_executable_path {
            PathBuf::from(custom)
        } else {
            self.detect_executable()?
        };

        let workdir = workspace_path
            .map(|p| p.to_path_buf())
            .or_else(|| profile.default_workspace_path.as_ref().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(&profile.profile_path));

        let mut env = vec![("CODEX_HOME".to_string(), profile.profile_path.clone())];
        for (k, v) in &profile.environment_variables {
            env.push((k.clone(), v.clone()));
        }

        match target {
            LaunchTarget::Desktop => {
                let path_arg = workdir.to_string_lossy().to_string();
                Ok(LaunchSpec {
                    executable: exec,
                    arguments: vec!["app".to_string(), path_arg],
                    environment: env,
                    working_directory: workdir,
                    is_terminal: false,
                })
            }
            _ => Ok(LaunchSpec {
                executable: exec,
                arguments: vec![],
                environment: env,
                working_directory: workdir,
                is_terminal: true,
            }),
        }
    }

    async fn logout(&self, profile: &AccountProfile) -> Result<()> {
        let exec = if let Some(custom) = &profile.custom_executable_path {
            PathBuf::from(custom)
        } else {
            self.detect_executable()?
        };

        let mut cmd = Command::new(&exec);
        cmd.arg("logout");
        cmd.env("CODEX_HOME", &profile.profile_path);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let output = cmd.output().map_err(|e| {
            AppError::LaunchFailed(format!("Failed to execute codex logout: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::AuthError(format!(
                "Codex logout failed: {}",
                stderr
            )));
        }

        let auth_file = Path::new(&profile.profile_path).join("auth.json");
        if auth_file.exists() {
            let _ = std::fs::remove_file(&auth_file);
        }

        Ok(())
    }

    fn build_login_spec(&self, profile: &AccountProfile) -> Result<LaunchSpec> {
        self.build_login_spec(profile)
    }
}
