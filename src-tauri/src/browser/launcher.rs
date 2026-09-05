use std::path::Path;
use std::process::Command;

use crate::error::{AppError, Result};

/// Redact sensitive query parameters in URLs before logging or diagnostics display.
/// Prevents OAuth authorization codes, session tokens, and secrets from appearing in logs.
pub fn redact_url_for_diagnostics(url: &str) -> String {
    let sensitive_keys = [
        "code",
        "token",
        "access_token",
        "refresh_token",
        "state",
        "id_token",
        "auth_code",
        "session_token",
        "secret",
        "password",
        "key",
    ];

    if let Some(query_start) = url.find('?') {
        let (base, query_str) = url.split_at(query_start);
        let query = &query_str[1..]; // skip '?'

        let parts: Vec<String> = query
            .split('&')
            .map(|param| {
                if let Some(eq_idx) = param.find('=') {
                    let key = &param[..eq_idx];
                    let key_lower = key.to_lowercase();
                    if sensitive_keys.contains(&key_lower.as_str()) {
                        format!("{}=[REDACTED]", key)
                    } else {
                        param.to_string()
                    }
                } else {
                    param.to_string()
                }
            })
            .collect();

        format!("{}?{}", base, parts.join("&"))
    } else {
        url.to_string()
    }
}

/// Build structured command line arguments for launching Chromium browsers
pub fn build_browser_args(user_data_dir: &Path, target_url: Option<&str>) -> Result<Vec<String>> {
    let mut args = Vec::new();

    // Use pure structured flag: --user-data-dir=<path>
    args.push(format!(
        "--user-data-dir={}",
        user_data_dir.to_string_lossy()
    ));
    args.push("--no-first-run".to_string());
    args.push("--no-default-browser-check".to_string());

    if let Some(raw_url) = target_url {
        let trimmed = raw_url.trim();
        if !trimmed.is_empty() {
            // Validate to prevent CLI argument injection
            if trimmed.starts_with('-') || trimmed.starts_with('/') {
                return Err(AppError::ValidationError(format!(
                    "Invalid URL '{}': cannot begin with '-' or '/'",
                    trimmed
                )));
            }
            args.push(trimmed.to_string());
        }
    }

    Ok(args)
}

/// Launch a Chromium browser with the specified user data directory and optional target URL
pub fn launch_browser(
    executable: &Path,
    user_data_dir: &Path,
    target_url: Option<&str>,
) -> Result<u32> {
    if !executable.is_file() {
        return Err(AppError::ExecutableNotFound(format!(
            "Browser executable '{}' does not exist",
            executable.display()
        )));
    }

    // Ensure the profile directory exists
    std::fs::create_dir_all(user_data_dir)?;

    let args = build_browser_args(user_data_dir, target_url)?;

    let display_url = target_url.map(redact_url_for_diagnostics);
    println!(
        "[BROWSER LAUNCH] Executable: '{}', UserDataDir: '{}', TargetURL: {:?}",
        executable.display(),
        user_data_dir.display(),
        display_url
    );

    let child = Command::new(executable).args(&args).spawn().map_err(|e| {
        AppError::LaunchFailed(format!(
            "Failed to launch browser '{}': {}",
            executable.display(),
            e
        ))
    })?;

    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url_for_diagnostics() {
        let plain_url = "https://claude.ai/login";
        assert_eq!(redact_url_for_diagnostics(plain_url), plain_url);

        let oauth_url = "https://auth0.openai.com/authorize?client_id=123&code=super_secret_code_abc&state=csrf_state_xyz&response_type=code";
        let redacted = redact_url_for_diagnostics(oauth_url);
        assert!(!redacted.contains("super_secret_code_abc"));
        assert!(!redacted.contains("csrf_state_xyz"));
        assert!(redacted.contains("client_id=123"));
        assert!(redacted.contains("code=[REDACTED]"));
        assert!(redacted.contains("state=[REDACTED]"));
    }

    #[test]
    fn test_build_browser_args() {
        let dir = Path::new("C:\\Data\\Profile 1 with spaces");
        let args = build_browser_args(dir, Some("https://claude.ai")).unwrap();

        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "--user-data-dir=C:\\Data\\Profile 1 with spaces");
        assert_eq!(args[1], "--no-first-run");
        assert_eq!(args[2], "--no-default-browser-check");
        assert_eq!(args[3], "https://claude.ai");

        // Test rejection of argument injection
        let malicious = build_browser_args(dir, Some("--disable-web-security"));
        assert!(malicious.is_err());
    }
}
