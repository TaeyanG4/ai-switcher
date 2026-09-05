use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::adapters::{LaunchSpec, PlatformAdapter};
use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, AccountStatus, ExecutionSurface, InstancePolicy, LaunchTarget, PlatformType,
};

pub struct AntigravityAdapter;

impl AntigravityAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Detect the primary Antigravity desktop executable on Windows.
    pub fn detect_desktop_executable(&self) -> Result<PathBuf> {
        let mut candidates = Vec::new();

        // 1. Per-user standard installation
        if let Some(local_app_data) = dirs::data_local_dir() {
            candidates.push(
                local_app_data
                    .join("Programs")
                    .join("antigravity")
                    .join("Antigravity.exe"),
            );
        }

        // 2. Program Files 64-bit / 32-bit fallbacks
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            candidates.push(
                PathBuf::from(&program_files)
                    .join("Google")
                    .join("Antigravity")
                    .join("Antigravity.exe"),
            );
            candidates.push(
                PathBuf::from(&program_files)
                    .join("antigravity")
                    .join("Antigravity.exe"),
            );
        }

        // 3. UserProfile fallback
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join("AppData")
                    .join("Local")
                    .join("Programs")
                    .join("antigravity")
                    .join("Antigravity.exe"),
            );
        }

        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // 4. PATH lookup via `where.exe`
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let mut cmd = std::process::Command::new("where.exe");
            cmd.arg("Antigravity.exe");
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    if let Ok(text) = String::from_utf8(output.stdout) {
                        if let Some(first_line) = text.lines().next() {
                            let trimmed = first_line.trim();
                            if !trimmed.is_empty() {
                                let path = PathBuf::from(trimmed);
                                if path.exists() {
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(AppError::ExecutableNotFound(
            "Antigravity Desktop executable was not found on this system. Please verify that Google Antigravity is installed in AppData\\Local\\Programs\\antigravity.".to_string(),
        ))
    }

    /// Extract Antigravity product version from executable info or app-update.yml
    pub fn detect_version(&self, exe_path: &Path) -> Option<String> {
        let resources_dir = exe_path.parent()?.join("resources");
        let app_update_yml = resources_dir.join("app-update.yml");
        if app_update_yml.exists() {
            if let Ok(content) = std::fs::read_to_string(&app_update_yml) {
                for line in content.lines() {
                    if line.starts_with("version:") {
                        let v = line
                            .trim_start_matches("version:")
                            .trim()
                            .trim_matches('\'')
                            .trim_matches('"');
                        if !v.is_empty() {
                            return Some(v.to_string());
                        }
                    }
                }
            }
        }

        // Windows VersionInfo probe
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let ps_script = format!(
                "(Get-Item '{}').VersionInfo.ProductVersion",
                exe_path.display()
            );
            let mut cmd = std::process::Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    if let Ok(out) = String::from_utf8(output.stdout) {
                        let trimmed = out.trim();
                        if !trimmed.is_empty() {
                            return Some(trimmed.to_string());
                        }
                    }
                }
            }
        }

        None
    }
}

impl Default for AntigravityAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate that a profile ID is safe and contains only allowed characters.
/// Prevents path traversal attacks (e.g. `..`, `/`, `\`).
pub fn validate_profile_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(AppError::ValidationError(
            "Antigravity profile ID cannot be empty".to_string(),
        ));
    }

    if id.len() > 128 {
        return Err(AppError::ValidationError(
            "Antigravity profile ID is too long".to_string(),
        ));
    }

    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::ValidationError(format!(
            "Invalid characters in Antigravity profile ID: '{}'",
            id
        )));
    }

    if id == "." || id == ".." {
        return Err(AppError::ValidationError(
            "Antigravity profile ID cannot be relative path navigation".to_string(),
        ));
    }

    Ok(())
}

/// Resolve and validate the target directory for an Antigravity profile.
/// Ensures the path is strictly inside the base directory and does not escape.
pub fn validate_antigravity_profile_dir(base_dir: &Path, profile_id: &str) -> Result<PathBuf> {
    validate_profile_id(profile_id)?;

    let target = base_dir.join(profile_id);

    // Verify parent is exactly base_dir
    if let Some(parent) = target.parent() {
        if parent != base_dir {
            return Err(AppError::ProfileDirectoryError(format!(
                "Path traversal detected: target '{}' escapes base directory '{}'",
                target.display(),
                base_dir.display()
            )));
        }
    } else {
        return Err(AppError::ProfileDirectoryError(
            "Invalid target path structure".to_string(),
        ));
    }

    // Never allow targeting user's home or system default Antigravity / .gemini directories
    if let Some(home) = dirs::home_dir() {
        let gemini_dir = home.join(".gemini");
        if target == home || target == gemini_dir {
            return Err(AppError::ProfileDirectoryError(
                "Targeting user home or default .gemini directory is strictly forbidden"
                    .to_string(),
            ));
        }
    }

    if let Some(app_data) = dirs::data_dir() {
        let default_antigravity = app_data.join("Antigravity");
        if target == app_data || target == default_antigravity {
            return Err(AppError::ProfileDirectoryError(
                "Targeting AppData\\Roaming\\Antigravity is strictly forbidden".to_string(),
            ));
        }
    }

    Ok(target)
}

/// Safely delete an Antigravity profile directory.
/// Ensures strict path containment and validates that the directory belongs to AI Switcher.
pub fn safe_delete_antigravity_profile_dir(base_dir: &Path, profile_id: &str) -> Result<()> {
    let target = validate_antigravity_profile_dir(base_dir, profile_id)?;

    if !target.exists() {
        return Ok(());
    }

    let base_canonical = match std::fs::canonicalize(base_dir) {
        Ok(c) => c,
        Err(_) => base_dir.to_path_buf(),
    };

    let target_canonical = match std::fs::canonicalize(&target) {
        Ok(c) => c,
        Err(_) => target.clone(),
    };

    if !target_canonical.starts_with(&base_canonical) {
        return Err(AppError::ProfileDirectoryError(format!(
            "Refusing to delete path '{}': does not reside within base directory '{}'",
            target.display(),
            base_dir.display()
        )));
    }

    std::fs::remove_dir_all(&target).map_err(|e| {
        AppError::ProfileDirectoryError(format!(
            "Failed to delete Antigravity profile directory '{}': {}",
            target.display(),
            e
        ))
    })?;

    Ok(())
}

#[async_trait]
impl PlatformAdapter for AntigravityAdapter {
    fn platform(&self) -> PlatformType {
        PlatformType::Antigravity
    }

    fn display_name(&self) -> &'static str {
        "Google Antigravity"
    }

    /// Antigravity supports MultiInstance across distinct isolated `--user-data-dir` profiles
    /// (verified experimentally on Windows host).
    fn instance_policy(&self) -> InstancePolicy {
        InstancePolicy::MultiInstance
    }

    fn supported_surfaces(&self) -> Vec<ExecutionSurface> {
        vec![ExecutionSurface::DesktopApp]
    }

    fn default_surface(&self) -> ExecutionSurface {
        ExecutionSurface::DesktopApp
    }

    fn detect_executable(&self) -> Result<PathBuf> {
        self.detect_desktop_executable()
    }

    fn initialize_profile(&self, profile_path: &Path) -> Result<()> {
        // Isolated Home & Gemini directory
        std::fs::create_dir_all(profile_path.join("home").join(".gemini"))?;
        // Isolated Electron/Chromium user-data directory
        std::fs::create_dir_all(profile_path.join("data"))?;
        // Isolated AppData
        std::fs::create_dir_all(profile_path.join("appdata"))?;
        Ok(())
    }

    /// Probe Antigravity authentication status non-intrusively.
    /// In accordance with the security policy, we NEVER read or parse raw tokens.
    /// We check if `oauth_creds.json` exists and is non-empty within the profile's isolated .gemini folder.
    async fn check_status(&self, profile: &AccountProfile) -> Result<AccountStatus> {
        let creds_file = PathBuf::from(&profile.profile_path)
            .join("home")
            .join(".gemini")
            .join("oauth_creds.json");

        if creds_file.exists() {
            if let Ok(metadata) = std::fs::metadata(&creds_file) {
                if metadata.len() > 0 {
                    return Ok(AccountStatus::Ready);
                }
            }
        }

        Ok(AccountStatus::LoginRequired)
    }

    fn build_launch_spec(
        &self,
        profile: &AccountProfile,
        target: LaunchTarget,
        workspace_path: Option<&Path>,
    ) -> Result<LaunchSpec> {
        match target {
            LaunchTarget::Desktop | LaunchTarget::Default => {
                let exec = if let Some(custom) = &profile.custom_executable_path {
                    let custom_buf = PathBuf::from(custom);
                    if !custom_buf.exists() {
                        return Err(AppError::ExecutableNotFound(format!(
                            "Custom Antigravity executable not found at: {}",
                            custom
                        )));
                    }
                    custom_buf
                } else {
                    self.detect_executable()?
                };

                let profile_buf = PathBuf::from(&profile.profile_path);
                let data_dir = profile_buf.join("data");
                let home_dir = profile_buf.join("home");
                let appdata_dir = profile_buf.join("appdata");

                let workdir = workspace_path
                    .map(|p| p.to_path_buf())
                    .or_else(|| profile.default_workspace_path.as_ref().map(PathBuf::from))
                    .unwrap_or_else(|| profile_buf.clone());

                // Construct arguments:
                // `--user-data-dir="<profile>\data"` ensures full Chromium/Electron storage isolation
                // and enables distinct single-instance mutex locks per profile.
                let mut arguments = vec![format!("--user-data-dir={}", data_dir.to_string_lossy())];
                for arg in &profile.launch_arguments {
                    arguments.push(arg.clone());
                }

                // Construct environment:
                // Isolating USERPROFILE and HOME directs Electron Node.js runtime and Go language server
                // to write .gemini state exclusively inside the profile directory without touching the user's real home.
                let mut environment = vec![
                    (
                        "USERPROFILE".to_string(),
                        home_dir.to_string_lossy().to_string(),
                    ),
                    ("HOME".to_string(), home_dir.to_string_lossy().to_string()),
                    (
                        "APPDATA".to_string(),
                        appdata_dir.to_string_lossy().to_string(),
                    ),
                ];
                for (k, v) in &profile.environment_variables {
                    environment.push((k.clone(), v.clone()));
                }

                Ok(LaunchSpec {
                    executable: exec,
                    arguments,
                    environment,
                    working_directory: workdir,
                    is_terminal: false,
                })
            }
            LaunchTarget::Cli => Err(AppError::UnsupportedExecutionSurface {
                platform: "Antigravity".to_string(),
                surface: "Cli".to_string(),
            }),
            LaunchTarget::Web => Err(AppError::UnsupportedExecutionSurface {
                platform: "Antigravity".to_string(),
                surface: "Web".to_string(),
            }),
        }
    }

    /// Logout removes the local OAuth credential cache file from the profile's .gemini directory
    /// and clears the isolated Chromium Network/Cookies file.
    async fn logout(&self, profile: &AccountProfile) -> Result<()> {
        let creds_file = PathBuf::from(&profile.profile_path)
            .join("home")
            .join(".gemini")
            .join("oauth_creds.json");

        if creds_file.exists() {
            let _ = std::fs::remove_file(&creds_file);
        }

        let cookies_file = PathBuf::from(&profile.profile_path)
            .join("data")
            .join("Network")
            .join("Cookies");

        if cookies_file.exists() {
            let _ = std::fs::remove_file(&cookies_file);
        }

        Ok(())
    }
}
