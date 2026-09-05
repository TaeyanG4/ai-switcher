use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, Result};

pub struct WebLauncher;

impl WebLauncher {
    /// Detects a supported Chromium browser or returns None for system default
    pub fn detect_browser() -> Option<PathBuf> {
        let candidates = [
            "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
            "C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
        ];

        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    /// Pure builder for opening a Web URL in a browser
    pub fn build_command(url: &str, browser_executable: Option<&Path>) -> Command {
        if let Some(browser) = browser_executable {
            let mut cmd = Command::new(browser);
            cmd.arg(url);
            cmd
        } else if let Some(browser) = Self::detect_browser() {
            let mut cmd = Command::new(browser);
            cmd.arg(url);
            cmd
        } else {
            // Windows fallback using explorer.exe to launch default web browser
            let mut cmd = Command::new("explorer.exe");
            cmd.arg(url);
            cmd
        }
    }

    /// Launches the browser opening the target URL
    pub fn open(url: &str, browser_executable: Option<&Path>) -> Result<u32> {
        let mut cmd = Self::build_command(url, browser_executable);

        let child = cmd.spawn().map_err(|e| {
            AppError::LaunchFailed(format!("Failed to open web URL '{}': {}", url, e))
        })?;

        Ok(child.id())
    }
}
