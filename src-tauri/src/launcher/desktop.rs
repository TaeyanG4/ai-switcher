use std::process::Command;

use crate::adapters::LaunchSpec;
use crate::error::{AppError, Result};

pub struct DesktopAppLauncher;

impl DesktopAppLauncher {
    /// Builds the structured std::process::Command without shell string concatenation
    pub fn build_command(spec: &LaunchSpec) -> Command {
        let mut cmd = Command::new(&spec.executable);

        // Arguments are passed as individual structured elements
        for arg in &spec.arguments {
            cmd.arg(arg);
        }

        // Set working directory
        cmd.current_dir(&spec.working_directory);

        // Inject environment variables
        for (k, v) in &spec.environment {
            cmd.env(k, v);
        }

        cmd
    }

    /// Spawns the desktop application as a detached process
    pub fn spawn(spec: &LaunchSpec) -> Result<u32> {
        let mut cmd = Self::build_command(spec);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NEW_PROCESS_GROUP = 0x00000200, DETACHED_PROCESS = 0x00000008
            // For native GUI apps, spawn in its own process group
            cmd.creation_flags(0x00000200);
        }

        let child = cmd.spawn().map_err(|e| {
            AppError::LaunchFailed(format!(
                "Failed to spawn desktop application '{}': {}",
                spec.executable.display(),
                e
            ))
        })?;

        Ok(child.id())
    }
}
