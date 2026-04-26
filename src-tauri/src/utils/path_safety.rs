use std::path::{Component, Path};

use crate::error::{AppError, AppResult};

/// Minimal lexical guard for filesystem sinks that receive paths already scoped by
/// higher-level project/worktree logic.
pub fn validate_absolute_non_root_path(path: &Path, context: &str) -> AppResult<()> {
    if !path.is_absolute() {
        return Err(AppError::Validation(format!(
            "{context} path must be absolute: {}",
            path.display()
        )));
    }

    if path.parent().is_none() {
        return Err(AppError::Validation(format!(
            "{context} path must not be a filesystem root: {}",
            path.display()
        )));
    }

    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::Prefix(_) | Component::CurDir
        )
    }) {
        return Err(AppError::Validation(format!(
            "{context} path contains unsafe components: {}",
            path.display()
        )));
    }

    Ok(())
}

pub fn checked_exists(path: &Path, context: &str) -> AppResult<bool> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    Ok(path.exists())
}

pub fn checked_is_file(path: &Path, context: &str) -> AppResult<bool> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    Ok(path.is_file())
}

pub fn checked_is_symlink(path: &Path, context: &str) -> AppResult<bool> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    Ok(path.is_symlink())
}

pub fn checked_read_to_string(path: &Path, context: &str) -> AppResult<String> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    std::fs::read_to_string(path).map_err(|e| {
        AppError::Infrastructure(format!(
            "Failed to read {context} file {}: {e}",
            path.display()
        ))
    })
}

pub fn checked_remove_file(path: &Path, context: &str) -> AppResult<()> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    std::fs::remove_file(path).map_err(|e| {
        AppError::Infrastructure(format!(
            "Failed to remove {context} file {}: {e}",
            path.display()
        ))
    })
}

pub async fn checked_remove_dir_all(path: &Path, context: &str) -> AppResult<()> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    tokio::fs::remove_dir_all(path).await.map_err(|e| {
        AppError::Infrastructure(format!(
            "Failed to remove {context} directory {}: {e}",
            path.display()
        ))
    })
}

pub async fn checked_read_dir(path: &Path, context: &str) -> AppResult<tokio::fs::ReadDir> {
    validate_absolute_non_root_path(path, context)?;

    // codeql[rust/path-injection]
    tokio::fs::read_dir(path).await.map_err(|e| {
        AppError::Infrastructure(format!(
            "Failed to read {context} directory {}: {e}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_path() {
        let err = validate_absolute_non_root_path(Path::new("relative/path"), "test")
            .expect_err("relative path should be rejected");
        assert!(err.to_string().contains("absolute"));
    }

    #[test]
    fn rejects_parent_components() {
        let err = validate_absolute_non_root_path(Path::new("/tmp/../etc"), "test")
            .expect_err("parent path should be rejected");
        assert!(err.to_string().contains("unsafe components"));
    }

    #[test]
    fn accepts_absolute_child_path() {
        validate_absolute_non_root_path(Path::new("/tmp/ralphx-child"), "test")
            .expect("normal absolute child path should be accepted");
    }
}
