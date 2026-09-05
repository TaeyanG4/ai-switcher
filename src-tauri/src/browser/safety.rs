use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

/// Validate that a profile ID is safe and contains only allowed characters.
/// Prevents directory traversal attacks (e.g. `..`, `/`, `\`).
pub fn validate_profile_id(id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(AppError::ValidationError(
            "Browser profile ID cannot be empty".to_string(),
        ));
    }

    if id.len() > 128 {
        return Err(AppError::ValidationError(
            "Browser profile ID is too long".to_string(),
        ));
    }

    // Must be alphanumeric, hyphen, or underscore (standard UUID or safe slug)
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(AppError::ValidationError(format!(
            "Invalid characters in browser profile ID: '{}'",
            id
        )));
    }

    if id == "." || id == ".." {
        return Err(AppError::ValidationError(
            "Browser profile ID cannot be relative path navigation".to_string(),
        ));
    }

    Ok(())
}

/// Resolve and validate the target directory for a browser profile.
/// Ensures the path is strictly inside the base directory and does not escape.
pub fn validate_browser_profile_dir(base_dir: &Path, profile_id: &str) -> Result<PathBuf> {
    validate_profile_id(profile_id)?;

    // Ensure base directory path is canonical or normalized
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

    // Never allow targeting user's personal default browser data directory
    if let Some(local_app_data) = dirs::data_local_dir() {
        let chrome_data = local_app_data.join("Google\\Chrome\\User Data");
        let edge_data = local_app_data.join("Microsoft\\Edge\\User Data");
        let brave_data = local_app_data.join("BraveSoftware\\Brave-Browser\\User Data");

        if target == chrome_data || target == edge_data || target == brave_data {
            return Err(AppError::ProfileDirectoryError(
                "Targeting default user browser data directories is forbidden".to_string(),
            ));
        }
    }

    Ok(target)
}

/// Safely delete a browser profile directory.
/// Ensures strict path containment and validates that the directory belongs to AI Switcher.
pub fn safe_delete_browser_dir(base_dir: &Path, profile_id: &str) -> Result<()> {
    let target = validate_browser_profile_dir(base_dir, profile_id)?;

    if !target.exists() {
        return Ok(());
    }

    // Safety check: ensure target is indeed inside base_dir
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

    crate::fs_safety::safe_remove_dir_all(base_dir, &target)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validate_profile_id() {
        assert!(validate_profile_id("valid-id-123_abc").is_ok());
        assert!(validate_profile_id("b18b2bed-6d9e-41ea-a43a-c0889fb594a4").is_ok());

        assert!(validate_profile_id("").is_err());
        assert!(validate_profile_id("..").is_err());
        assert!(validate_profile_id("../escaped").is_err());
        assert!(validate_profile_id("foo/bar").is_err());
        assert!(validate_profile_id("foo\\bar").is_err());
        assert!(validate_profile_id("foo:bar").is_err());
        assert!(validate_profile_id("foo\0bar").is_err());
    }

    #[test]
    fn test_validate_browser_profile_dir() {
        let base = PathBuf::from("C:\\AI-Switcher\\browser-profiles");
        let valid = validate_browser_profile_dir(&base, "profile-1");
        assert!(valid.is_ok());
        assert_eq!(
            valid.unwrap(),
            PathBuf::from("C:\\AI-Switcher\\browser-profiles\\profile-1")
        );

        let traversal = validate_browser_profile_dir(&base, "../system32");
        assert!(traversal.is_err());
    }

    #[test]
    fn test_safe_delete_browser_dir() {
        let temp_dir = std::env::temp_dir().join("ai_switcher_test_browser_safe_delete");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let profile_id = "test-prof-123";
        let prof_dir = temp_dir.join(profile_id);
        fs::create_dir_all(&prof_dir).unwrap();
        fs::write(prof_dir.join("test.txt"), "hello").unwrap();

        assert!(prof_dir.exists());

        // Safe delete
        let del_res = safe_delete_browser_dir(&temp_dir, profile_id);
        assert!(del_res.is_ok());
        assert!(!prof_dir.exists());

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
