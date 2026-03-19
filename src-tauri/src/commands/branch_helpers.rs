use std::path::Path;

use crate::application::git_service::GitService;

/// Ensures a base branch exists in the given git repository.
///
/// Checks if `base_branch` exists locally. If it does not exist, creates it
/// from `project_default` (falling back to `"main"` if `None`).
///
/// # Returns
/// - `Ok(true)` — branch was created (did not exist before)
/// - `Ok(false)` — branch already existed (no-op)
/// - `Err(String)` — branch existence check or creation failed
///
/// # Errors
/// Returns a descriptive `String` error if `GitService::branch_exists` or
/// `GitService::create_branch` fails.
pub async fn ensure_base_branch_exists(
    repo_path: &Path,
    base_branch: &str,
    project_default: Option<&str>,
) -> Result<bool, String> {
    let exists = GitService::branch_exists(repo_path, base_branch)
        .await
        .map_err(|e| format!("Failed to check branch existence: {}", e))?;
    if !exists {
        let source = project_default.unwrap_or("main");
        GitService::create_branch(repo_path, base_branch, source)
            .await
            .map_err(|e| {
                format!(
                    "Failed to create branch '{}' from '{}': {}",
                    base_branch, source, e
                )
            })?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
#[path = "branch_helpers_tests.rs"]
mod tests;
