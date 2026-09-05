use std::path::{Path, PathBuf};
use std::process::Command;

use crate::adapters::LaunchSpec;
use crate::error::{AppError, Result};

pub struct CliLauncher;

impl CliLauncher {
    /// Detects if Windows Terminal (wt.exe) is available on the system
    pub fn is_wt_available() -> bool {
        let wt_candidates = [
            dirs::data_local_dir().map(|p| p.join("Microsoft\\WindowsApps\\wt.exe")),
            Some(PathBuf::from("wt.exe")),
        ];

        for cand in wt_candidates.into_iter().flatten() {
            if cand.exists() {
                return true;
            }
        }

        // Test with `where.exe wt`
        Command::new("where.exe")
            .arg("wt.exe")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    /// Pure builder for Windows Terminal (wt.exe) command
    pub fn build_wt_command(
        title: &str,
        workdir: &Path,
        executable: &Path,
        arguments: &[String],
        environment: &[(String, String)],
    ) -> Command {
        let mut cmd = Command::new("wt.exe");

        // Set window / tab title
        cmd.arg("--title").arg(title);

        // Set starting working directory
        cmd.arg("-d").arg(workdir);

        // Target executable
        cmd.arg(executable);

        // Arguments for the target executable
        for arg in arguments {
            cmd.arg(arg);
        }

        // Working directory for wt process itself
        cmd.current_dir(workdir);

        // Inject environment variables
        for (k, v) in environment {
            cmd.env(k, v);
        }

        cmd
    }

    /// Pure builder for PowerShell fallback command
    pub fn build_powershell_command(
        title: &str,
        workdir: &Path,
        executable: &Path,
        arguments: &[String],
        environment: &[(String, String)],
    ) -> Command {
        let is_login = arguments.iter().any(|a| a == "login");
        let mut cmd = Command::new("powershell.exe");
        if !is_login {
            cmd.arg("-NoExit");
        }

        // Construct PowerShell invocation while preserving arguments safely
        // Sets window title, sets working directory, and invokes executable
        let title_escaped = title.replace('\'', "''");
        let workdir_str = workdir.to_string_lossy().replace('\'', "''");
        let exec_str = executable.to_string_lossy().replace('\'', "''");

        let args_str = arguments
            .iter()
            .map(|a| format!("'{}'", a.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(" ");

        let script = if is_login {
            if args_str.is_empty() {
                format!(
                    "$host.UI.RawUI.WindowTitle = '{}'; Set-Location -LiteralPath '{}'; & '{}'; if ($LASTEXITCODE -eq 0) {{ exit 0 }} else {{ Write-Host 'Login failed. Press Enter to close...'; Read-Host; exit $LASTEXITCODE }}",
                    title_escaped, workdir_str, exec_str
                )
            } else {
                format!(
                    "$host.UI.RawUI.WindowTitle = '{}'; Set-Location -LiteralPath '{}'; & '{}' {}; if ($LASTEXITCODE -eq 0) {{ exit 0 }} else {{ Write-Host 'Login failed. Press Enter to close...'; Read-Host; exit $LASTEXITCODE }}",
                    title_escaped, workdir_str, exec_str, args_str
                )
            }
        } else if args_str.is_empty() {
            format!(
                "$host.UI.RawUI.WindowTitle = '{}'; Set-Location -LiteralPath '{}'; & '{}'",
                title_escaped, workdir_str, exec_str
            )
        } else {
            format!(
                "$host.UI.RawUI.WindowTitle = '{}'; Set-Location -LiteralPath '{}'; & '{}' {}",
                title_escaped, workdir_str, exec_str, args_str
            )
        };

        cmd.arg("-Command").arg(script);
        cmd.current_dir(workdir);

        for (k, v) in environment {
            cmd.env(k, v);
        }

        cmd
    }

    /// Builds the appropriate terminal command based on availability and preference
    pub fn build_cli_command(title: &str, spec: &LaunchSpec, prefer_wt: bool) -> Command {
        if prefer_wt && Self::is_wt_available() {
            Self::build_wt_command(
                title,
                &spec.working_directory,
                &spec.executable,
                &spec.arguments,
                &spec.environment,
            )
        } else {
            Self::build_powershell_command(
                title,
                &spec.working_directory,
                &spec.executable,
                &spec.arguments,
                &spec.environment,
            )
        }
    }

    /// Spawns the CLI application in an external terminal window
    pub fn spawn(title: &str, spec: &LaunchSpec) -> Result<u32> {
        let mut cmd = Self::build_cli_command(title, spec, true);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NEW_CONSOLE = 0x00000010
            cmd.creation_flags(0x00000010);
        }

        let child = cmd.spawn().map_err(|e| {
            AppError::LaunchFailed(format!(
                "Failed to spawn external terminal for CLI '{}': {}",
                spec.executable.display(),
                e
            ))
        })?;

        Ok(child.id())
    }
}
