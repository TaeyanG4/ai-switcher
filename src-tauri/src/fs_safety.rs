use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};

/// Checks if a file or directory is a symlink, directory junction, or reparse point.
#[cfg(windows)]
pub fn is_reparse_or_symlink(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_type().is_symlink() || (meta.file_attributes() & 0x00000400 != 0)
}

#[cfg(not(windows))]
pub fn is_reparse_or_symlink(meta: &std::fs::Metadata) -> bool {
    meta.file_type().is_symlink()
}

/// Validate that `target_dir` is safely contained within `base_dir` and does not
/// target any protected system or user root directories.
pub fn validate_safe_containment(base_dir: &Path, target_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    if !base_dir.exists() {
        return Err(AppError::ProfileDirectoryError(format!(
            "Base directory '{}' does not exist",
            base_dir.display()
        )));
    }

    let base_canonical = base_dir.canonicalize().map_err(|e| {
        AppError::ProfileDirectoryError(format!(
            "Failed to canonicalize base directory '{}': {}",
            base_dir.display(),
            e
        ))
    })?;

    // If target exists, canonicalize it; otherwise normalize and check parent containment
    let target_canonical = if target_dir.exists() {
        target_dir.canonicalize().map_err(|e| {
            AppError::ProfileDirectoryError(format!(
                "Failed to canonicalize target directory '{}': {}",
                target_dir.display(),
                e
            ))
        })?
    } else {
        let mut normalized = if target_dir.is_relative() {
            base_canonical.clone()
        } else {
            PathBuf::new()
        };

        for component in target_dir.components() {
            match component {
                std::path::Component::Prefix(p) => normalized.push(p.as_os_str()),
                std::path::Component::RootDir => {
                    normalized.push(std::path::MAIN_SEPARATOR.to_string())
                }
                std::path::Component::Normal(c) => normalized.push(c),
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                _ => {}
            }
        }
        normalized
    };

    // Strict prefix check: target must be inside base_dir and not equal to base_dir
    if !target_canonical.starts_with(&base_canonical) || target_canonical == base_canonical {
        return Err(AppError::SecurityViolation(format!(
            "Security violation: path '{}' escapes or equals base directory '{}'",
            target_dir.display(),
            base_dir.display()
        )));
    }

    // Check against forbidden critical paths
    if let Some(user_home) = dirs::home_dir() {
        if let Ok(home_canonical) = user_home.canonicalize() {
            if target_canonical == home_canonical {
                return Err(AppError::SecurityViolation(
                    "Security violation: target path is user home directory".to_string(),
                ));
            }
        }
    }

    if let Some(local_app_data) = dirs::data_local_dir() {
        let chrome_data = local_app_data.join("Google\\Chrome\\User Data");
        let edge_data = local_app_data.join("Microsoft\\Edge\\User Data");
        let brave_data = local_app_data.join("BraveSoftware\\Brave-Browser\\User Data");

        if target_canonical == chrome_data
            || target_canonical == edge_data
            || target_canonical == brave_data
        {
            return Err(AppError::SecurityViolation(
                "Security violation: target path is a default browser user data directory"
                    .to_string(),
            ));
        }
    }

    Ok((base_canonical, target_canonical))
}

/// Recursively delete directory contents while strictly protecting against
/// following directory junctions or symlinks into external directories.
fn safe_remove_dir_contents_internal(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let entry_path = entry.path();
        let meta = entry_path.symlink_metadata().map_err(|e| {
            AppError::PartialDeleteError(format!(
                "Failed to read metadata for '{}': {}",
                entry_path.display(),
                e
            ))
        })?;

        if is_reparse_or_symlink(&meta) {
            // It is a junction, symlink, or reparse point!
            // CRITICAL: Remove the link entry itself without recursing into target!
            let is_dir_reparse = {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    meta.is_dir() || (meta.file_attributes() & 0x00000010 != 0)
                }
                #[cfg(not(windows))]
                {
                    meta.is_dir()
                }
            };

            if is_dir_reparse {
                std::fs::remove_dir(&entry_path)
                    .or_else(|_| std::fs::remove_file(&entry_path))
                    .map_err(|e| {
                        AppError::PartialDeleteError(format!(
                            "Failed to remove reparse point directory link '{}': {}",
                            entry_path.display(),
                            e
                        ))
                    })?;
            } else {
                std::fs::remove_file(&entry_path)
                    .or_else(|_| std::fs::remove_dir(&entry_path))
                    .map_err(|e| {
                        AppError::PartialDeleteError(format!(
                            "Failed to remove symlink file '{}': {}",
                            entry_path.display(),
                            e
                        ))
                    })?;
            }
        } else if meta.is_dir() {
            // Genuine non-reparse directory: recurse, then remove directory
            safe_remove_dir_contents_internal(&entry_path)?;
            std::fs::remove_dir(&entry_path).map_err(|e| {
                AppError::PartialDeleteError(format!(
                    "Failed to remove subfolder '{}': {}",
                    entry_path.display(),
                    e
                ))
            })?;
        } else {
            // Regular file
            std::fs::remove_file(&entry_path).map_err(|e| {
                AppError::PartialDeleteError(format!(
                    "Failed to remove file '{}': {}",
                    entry_path.display(),
                    e
                ))
            })?;
        }
    }

    Ok(())
}

/// Safely delete a directory and all its contents:
/// 1. Verifies canonical containment within `base_dir`.
/// 2. Ensures no system/home root deletion.
/// 3. Traverses contents without following Windows junctions / symlinks outside the root.
/// 4. Removes the root target directory.
pub fn safe_remove_dir_all(base_dir: &Path, target_dir: &Path) -> Result<()> {
    if !target_dir.exists() {
        return Ok(());
    }

    let (_, canonical_target) = validate_safe_containment(base_dir, target_dir)?;

    safe_remove_dir_contents_internal(&canonical_target)?;

    std::fs::remove_dir(&canonical_target).map_err(|e| {
        AppError::PartialDeleteError(format!(
            "Failed to remove root target directory '{}': {}",
            canonical_target.display(),
            e
        ))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validate_safe_containment_valid() {
        let temp_base =
            std::env::temp_dir().join(format!("ai_switcher_fs_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_base).unwrap();
        let target = temp_base.join("profile_1");
        fs::create_dir_all(&target).unwrap();

        let res = validate_safe_containment(&temp_base, &target);
        assert!(res.is_ok());

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_validate_safe_containment_escaped() {
        let temp_base =
            std::env::temp_dir().join(format!("ai_switcher_fs_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_base).unwrap();
        let escaped = temp_base.join("..").join("escaped");

        let res = validate_safe_containment(&temp_base, &escaped);
        assert!(res.is_err());

        let _ = fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_safe_remove_dir_all_basic() {
        let temp_base =
            std::env::temp_dir().join(format!("ai_switcher_fs_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_base).unwrap();
        let target = temp_base.join("profile_target");
        fs::create_dir_all(target.join("subdir")).unwrap();
        fs::write(target.join("file.txt"), "hello").unwrap();
        fs::write(target.join("subdir").join("nested.txt"), "world").unwrap();

        assert!(target.exists());
        let res = safe_remove_dir_all(&temp_base, &target);
        assert!(res.is_ok());
        assert!(!target.exists());

        let _ = fs::remove_dir_all(&temp_base);
    }
}
