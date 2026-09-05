pub mod discovery;
pub mod launcher;
pub mod safety;

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::{BrowserKind, BrowserProfile, DetectedBrowserInfo};

#[derive(Debug, Clone)]
pub struct BrowserProfileManager {
    pub base_dir: PathBuf,
}

impl BrowserProfileManager {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Resolve and ensure the profile directory exists safely
    pub fn ensure_profile_dir(&self, profile_id: &str) -> Result<PathBuf> {
        let dir = safety::validate_browser_profile_dir(&self.base_dir, profile_id)?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Safely delete the profile directory
    pub fn safe_delete_profile_dir(&self, profile_id: &str) -> Result<()> {
        safety::safe_delete_browser_dir(&self.base_dir, profile_id)
    }

    /// Discover available browsers on this system
    pub fn detect_available_browsers(&self) -> Vec<DetectedBrowserInfo> {
        discovery::detect_available_browsers()
    }

    /// Resolve browser executable
    pub fn resolve_executable(
        &self,
        kind: BrowserKind,
        custom_path: Option<&str>,
    ) -> Result<PathBuf> {
        discovery::resolve_browser_executable(kind, custom_path)
    }

    /// Launch browser profile with an optional target URL
    pub fn launch(&self, profile: &BrowserProfile, target_url: Option<&str>) -> Result<u32> {
        let exe = self.resolve_executable(
            profile.browser_kind,
            profile.custom_executable_path.as_deref(),
        )?;
        let user_data_dir = Path::new(&profile.user_data_directory);
        launcher::launch_browser(&exe, user_data_dir, target_url)
    }
}
