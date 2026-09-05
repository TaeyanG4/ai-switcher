pub mod antigravity;
pub mod claude;
pub mod codex;

pub use antigravity::AntigravityAdapter;
pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::{
    AccountProfile, AccountStatus, ExecutionSurface, InstancePolicy, LaunchTarget, PlatformType,
};

#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: Vec<(String, String)>,
    pub working_directory: PathBuf,
    pub is_terminal: bool,
}

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> PlatformType;
    fn display_name(&self) -> &'static str;
    fn instance_policy(&self) -> InstancePolicy;
    fn supported_surfaces(&self) -> Vec<ExecutionSurface>;
    fn default_surface(&self) -> ExecutionSurface;
    fn detect_executable(&self) -> Result<PathBuf>;
    fn initialize_profile(&self, profile_path: &Path) -> Result<()>;
    async fn check_status(&self, profile: &AccountProfile) -> Result<AccountStatus>;
    fn build_launch_spec(
        &self,
        profile: &AccountProfile,
        target: LaunchTarget,
        workspace_path: Option<&Path>,
    ) -> Result<LaunchSpec>;
    fn build_login_spec(&self, profile: &AccountProfile) -> Result<LaunchSpec> {
        let target = match self.default_surface() {
            ExecutionSurface::DesktopApp => LaunchTarget::Desktop,
            ExecutionSurface::Cli => LaunchTarget::Cli,
            ExecutionSurface::Web => LaunchTarget::Web,
        };
        self.build_launch_spec(profile, target, None)
    }
    async fn logout(&self, profile: &AccountProfile) -> Result<()>;
}

pub fn get_adapter(platform: PlatformType) -> Box<dyn PlatformAdapter> {
    match platform {
        PlatformType::Codex => Box::new(CodexAdapter::new()),
        PlatformType::Claude => Box::new(ClaudeAdapter),
        PlatformType::Antigravity => Box::new(AntigravityAdapter),
    }
}
