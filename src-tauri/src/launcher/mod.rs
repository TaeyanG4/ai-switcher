pub mod cli;
pub mod desktop;
pub mod process_manager;
pub mod web;

use std::path::Path;

pub use cli::CliLauncher;
pub use desktop::DesktopAppLauncher;
pub use process_manager::ProcessManager;
pub use web::WebLauncher;

use crate::adapters::PlatformAdapter;
use crate::error::{AppError, Result};
use crate::models::{
    AccountProfile, ExecutionSurface, LaunchTarget, ProcessConflictInfo, ProcessRecord,
};

#[derive(Clone)]
pub struct LauncherEngine {
    pub process_manager: ProcessManager,
}

impl LauncherEngine {
    pub fn new(process_manager: ProcessManager) -> Self {
        Self { process_manager }
    }

    /// Checks if launching `target_account` will conflict with an already-running same-platform profile
    pub fn check_conflict(
        &self,
        target_account: &AccountProfile,
        adapter: &dyn PlatformAdapter,
        all_accounts: &[AccountProfile],
        surface: Option<ExecutionSurface>,
    ) -> Option<ProcessConflictInfo> {
        let effective_surface = surface.unwrap_or_else(|| adapter.default_surface());
        let policy = adapter.instance_policy_for(effective_surface);
        self.process_manager
            .check_platform_conflict(target_account, policy, all_accounts)
    }

    /// Launches the given account profile on the requested or default execution surface
    pub fn launch(
        &self,
        account: &AccountProfile,
        adapter: &dyn PlatformAdapter,
        surface: Option<ExecutionSurface>,
        workspace_path: Option<&Path>,
    ) -> Result<ProcessRecord> {
        // Invariant: Disabled accounts must not launch
        if !account.is_enabled {
            return Err(AppError::AccountDisabled(account.display_name.clone()));
        }

        // Determine target execution surface
        let target_surface = surface.unwrap_or_else(|| adapter.default_surface());

        // Validate surface support
        let supported = adapter.supported_surfaces();
        if !supported.contains(&target_surface) {
            return Err(AppError::UnsupportedExecutionSurface {
                platform: account.platform.as_str().to_string(),
                surface: target_surface.as_str().to_string(),
            });
        }

        // Map ExecutionSurface to LaunchTarget
        let launch_target = match target_surface {
            ExecutionSurface::DesktopApp => LaunchTarget::Desktop,
            ExecutionSurface::Cli => LaunchTarget::Cli,
            ExecutionSurface::Web => LaunchTarget::Web,
        };

        // Build LaunchSpec via adapter
        let spec = adapter.build_launch_spec(account, launch_target, workspace_path)?;

        // Execute according to the chosen surface
        let pid = match target_surface {
            ExecutionSurface::DesktopApp => DesktopAppLauncher::spawn(&spec)?,
            ExecutionSurface::Cli => {
                let title = format!("{} ({})", account.display_name, adapter.display_name());
                CliLauncher::spawn(&title, &spec)?
            }
            ExecutionSurface::Web => {
                let url = if !spec.arguments.is_empty() {
                    &spec.arguments[0]
                } else {
                    "https://claude.ai"
                };
                WebLauncher::open(url, None)?
            }
        };

        // Register with ProcessManager
        let record = self.process_manager.register_launch(
            &account.id,
            account.platform,
            target_surface,
            Some(pid),
            &spec.executable.to_string_lossy(),
        );

        Ok(record)
    }
}
