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
/// 1. Canonicalize via `std::fs::canonicalize` (resolves symlinks, `..`, etc).
///    Falls back to component-filtering for non-existent paths.
/// 2. Blocklist: reject paths under known system dirs.
/// 3. Allowlist: require path under the user's home directory.
///
/// # Errors
/// Returns `AppError::Validation` with a descriptive message on any failure.
pub fn validate_project_path(path: &str) -> AppResult<PathBuf> {
    let input_path = Path::new(path);

    // Step 1: Canonicalize
    let canonical = if {
        // This is the validation boundary for a user-provided project path.
        // codeql[rust/path-injection]
        input_path.exists()
    } {
        // This canonicalization is the containment validation step before the
        // path is accepted by project registration.
        // codeql[rust/path-injection]
        std::fs::canonicalize(input_path)
            .map_err(|e| AppError::Validation(format!("Failed to canonicalize path: {e}")))?
    } else {
        // Path doesn't exist: canonicalize parent + append basename (for auto-create support)
        canonicalize_nonexistent(input_path)?
    };

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
        .map(PathBuf::from)
        .map_err(|_| AppError::Validation("Cannot determine home directory".to_string()))?;

    if !canonical.starts_with(&home) {
        return Err(AppError::Validation(
            "Path must be within the user's home directory".to_string(),
        ));
    }

    Ok(canonical)
}

/// Canonicalize a path that doesn't exist yet by filtering `..` / `.` components
/// from the lexical path without hitting the filesystem.
///
/// This is a best-effort canonicalization for paths that will be created later.
/// Requires the parent directory to exist.
fn canonicalize_nonexistent(input_path: &Path) -> AppResult<PathBuf> {
    let parent = input_path
        .parent()
        .ok_or_else(|| AppError::Validation("Invalid path: no parent directory".to_string()))?;

    // Try to canonicalize parent if it exists
    let canonical_parent = if {
        // This is the validation boundary for a user-provided project path.
        // codeql[rust/path-injection]
        parent.exists()
    } {
        // This canonicalization validates the trusted existing parent before
        // appending the requested basename for auto-create support.
        // codeql[rust/path-injection]
        std::fs::canonicalize(parent).map_err(|e| {
            AppError::Validation(format!("Failed to canonicalize parent directory: {e}"))
        })?
    } else {
        // Walk components manually — filters `.` and normalises `..`
        let mut components = Vec::new();
        for component in parent.components() {
            match component {
                Component::ParentDir => {
                    components.pop();
                }
                Component::CurDir => {}
                Component::RootDir => {
                    components.clear();
                    components.push(component);
                }
                other => {
                    components.push(other);
                }
            }
        }
        components.iter().fold(PathBuf::new(), |mut acc, c| {
            acc.push(c);
            acc
        })
    };

    let basename = input_path
        .file_name()
        .ok_or_else(|| AppError::Validation("Invalid path: no basename component".to_string()))?;

    Ok(canonical_parent.join(basename))
}

#[cfg(test)]
#[path = "project_validation_tests.rs"]
mod tests;
