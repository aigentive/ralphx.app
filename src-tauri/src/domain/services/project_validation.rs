/// Shared path validation for project registration.
///
/// Used by both `register_project_external` (HTTP) and `create_cross_project_session` (Tauri IPC)
/// to enforce consistent safety rules on user-supplied filesystem paths.
use std::path::{Component, Path, PathBuf};

use crate::error::{AppError, AppResult};

/// System directories that are never valid project locations.
pub const BLOCKED_PREFIXES: &[&str] = &[
    "/etc", "/usr", "/var", "/tmp", "/private", "/System", "/Library", "/Volumes",
];

/// Validate a path for use as a project working directory.
///
/// Steps:
/// 1. Lexically normalize an absolute path and reject traversal components.
/// 2. Blocklist: reject paths under known system dirs.
/// 3. Allowlist: require path under the user's home directory.
///
/// # Errors
/// Returns `AppError::Validation` with a descriptive message on any failure.
pub fn validate_project_path(path: &str) -> AppResult<PathBuf> {
    let input_path = Path::new(path);

    // Step 1: Normalize without filesystem access so validation itself cannot
    // dereference a user-controlled path.
    let canonical = normalize_absolute_project_path(input_path, "project")?;

    let canonical_str = canonical.to_string_lossy();

    // Step 2: Blocklist — reject system dirs
    for blocked in BLOCKED_PREFIXES {
        if canonical_str.starts_with(blocked) {
            return Err(AppError::Validation(format!(
                "Path is in a restricted system directory: {blocked}"
            )));
        }
    }

    // Step 3: Allowlist — must be under home directory
    let home = std::env::var("HOME")
        .map_err(|_| AppError::Validation("Cannot determine home directory".to_string()))?;
    let home = normalize_absolute_project_path(Path::new(&home), "home directory")?;

    if !canonical.starts_with(&home) {
        return Err(AppError::Validation(
            "Path must be within the user's home directory".to_string(),
        ));
    }

    Ok(canonical)
}

fn normalize_absolute_project_path(input_path: &Path, context: &str) -> AppResult<PathBuf> {
    if !input_path.is_absolute() {
        return Err(AppError::Validation(format!(
            "{context} path must be absolute"
        )));
    }

    let mut normalized = PathBuf::new();
    let mut normal_components = 0usize;

    for component in input_path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => {
                normal_components += 1;
                normalized.push(part);
            }
            Component::CurDir | Component::ParentDir => {
                return Err(AppError::Validation(format!(
                    "{context} path contains unsafe traversal components"
                )));
            }
        }
    }

    if normal_components == 0 {
        return Err(AppError::Validation(format!(
            "{context} path must not be a filesystem root"
        )));
    }

    Ok(normalized)
}

#[cfg(test)]
#[path = "project_validation_tests.rs"]
mod tests;
