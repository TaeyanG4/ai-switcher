use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, Result};
use crate::models::{BrowserKind, DetectedBrowserInfo};

/// Detect all supported browsers on the current system
pub fn detect_available_browsers() -> Vec<DetectedBrowserInfo> {
    let kinds = [BrowserKind::Chrome, BrowserKind::Edge, BrowserKind::Brave];

    kinds
        .into_iter()
        .map(|kind| {
            if let Some(path) = find_browser_executable(kind) {
                let version = extract_chromium_version(&path);
                DetectedBrowserInfo {
                    kind,
                    name: kind.display_name().to_string(),
                    executable_path: path.to_string_lossy().to_string(),
                    version,
                    is_available: true,
                }
            } else {
                DetectedBrowserInfo {
                    kind,
                    name: kind.display_name().to_string(),
                    executable_path: String::new(),
                    version: None,
                    is_available: false,
                }
            }
        })
        .collect()
}

/// Locate the executable for a given browser kind
pub fn find_browser_executable(kind: BrowserKind) -> Option<PathBuf> {
    let candidate_paths = get_candidate_paths(kind);

    for path in candidate_paths {
        if path.is_file() {
            return Some(path);
        }
    }

    // Fallback: check PATH using `where.exe` on Windows
    let exe_name = match kind {
        BrowserKind::Chrome => "chrome.exe",
        BrowserKind::Edge => "msedge.exe",
        BrowserKind::Brave => "brave.exe",
        BrowserKind::Custom => return None,
    };

    if let Ok(output) = Command::new("where.exe").arg(exe_name).output() {
        if output.status.success() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                for line in s.lines() {
                    let p = PathBuf::from(line.trim());
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
    }

    None
}

/// Resolve executable for a profile, respecting custom path override if specified
pub fn resolve_browser_executable(kind: BrowserKind, custom_path: Option<&str>) -> Result<PathBuf> {
    if let Some(custom) = custom_path {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let path = PathBuf::from(trimmed);
            if path.is_file() {
                return Ok(path);
            } else {
                return Err(AppError::ExecutableNotFound(format!(
                    "Custom browser executable not found at '{}'",
                    trimmed
                )));
            }
        }
    }

    find_browser_executable(kind).ok_or_else(|| {
        AppError::ExecutableNotFound(format!(
            "No installed browser found for '{}'",
            kind.display_name()
        ))
    })
}

/// Get standard candidate installation paths for a browser kind on Windows
fn get_candidate_paths(kind: BrowserKind) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let local_app_data = dirs::data_local_dir();
    let program_files = std::env::var("ProgramFiles").map(PathBuf::from).ok();
    let program_files_x86 = std::env::var("ProgramFiles(x86)").map(PathBuf::from).ok();

    match kind {
        BrowserKind::Chrome => {
            if let Some(pf) = &program_files {
                candidates.push(pf.join("Google\\Chrome\\Application\\chrome.exe"));
            }
            if let Some(pfx) = &program_files_x86 {
                candidates.push(pfx.join("Google\\Chrome\\Application\\chrome.exe"));
            }
            if let Some(ref local) = local_app_data {
                candidates.push(local.join("Google\\Chrome\\Application\\chrome.exe"));
            }
            // Hardcoded common fallbacks
            candidates.push(PathBuf::from(
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
            ));
            candidates.push(PathBuf::from(
                "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
            ));
        }
        BrowserKind::Edge => {
            if let Some(pfx) = &program_files_x86 {
                candidates.push(pfx.join("Microsoft\\Edge\\Application\\msedge.exe"));
            }
            if let Some(pf) = &program_files {
                candidates.push(pf.join("Microsoft\\Edge\\Application\\msedge.exe"));
            }
            if let Some(ref local) = local_app_data {
                candidates.push(local.join("Microsoft\\Edge\\Application\\msedge.exe"));
            }
            // Hardcoded common fallbacks
            candidates.push(PathBuf::from(
                "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
            ));
            candidates.push(PathBuf::from(
                "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
            ));
        }
        BrowserKind::Brave => {
            if let Some(pf) = &program_files {
                candidates.push(pf.join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"));
            }
            if let Some(pfx) = &program_files_x86 {
                candidates.push(pfx.join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"));
            }
            if let Some(ref local) = local_app_data {
                candidates.push(local.join("BraveSoftware\\Brave-Browser\\Application\\brave.exe"));
            }
            // Hardcoded common fallbacks
            candidates.push(PathBuf::from(
                "C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
            ));
            candidates.push(PathBuf::from(
                "C:\\Program Files (x86)\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
            ));
        }
        BrowserKind::Custom => {}
    }

    candidates
}

/// Extract version from Chromium application folder structure.
/// Chromium browsers place a versioned directory (e.g. `152.0.7977.76`) adjacent to `<browser>.exe`.
pub fn extract_chromium_version(exe_path: &Path) -> Option<String> {
    let app_dir = exe_path.parent()?;

    if let Ok(entries) = std::fs::read_dir(app_dir) {
        let mut versions = Vec::new();
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    // Version folders look like "133.0.6943.142" or "152.0.7977.76"
                    if name.contains('.') && name.chars().all(|c| c.is_ascii_digit() || c == '.') {
                        versions.push(name);
                    }
                }
            }
        }
        // If multiple, sort to take the latest
        versions.sort();
        return versions.pop();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_available_browsers() {
        let detected = detect_available_browsers();
        assert_eq!(detected.len(), 3);
        // At least one browser should be available on standard dev Windows
        let available_count = detected.iter().filter(|b| b.is_available).count();
        assert!(
            available_count > 0,
            "Expected at least one browser to be detected"
        );
    }

    #[test]
    fn test_resolve_browser_executable_fallback() {
        // Unknown custom path should return error
        let res =
            resolve_browser_executable(BrowserKind::Chrome, Some("C:\\nonexistent\\browser.exe"));
        assert!(res.is_err());
    }
}
