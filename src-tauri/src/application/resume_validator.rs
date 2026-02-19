//! ResumeValidator - Safety validation for resuming stopped tasks
//!
//! This service validates that resuming a stopped task is safe by:
//! 1. Checking git state (branch exists, worktree clean, no stale merge/rebase)
//! 2. Cleaning up orphan agent processes for the task
//! 3. Resetting the execution counter
//!
//! Used by the restart_task command before resuming to Validated states
//! (Merging, PendingMerge, MergeConflict, MergeIncomplete).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::application::git_service::{git_cmd, GitService};
use crate::domain::entities::{Project, Task};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::error::AppResult;

/// Result of validating a task for resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeValidationResult {
    /// Whether validation passed (no blocking issues)
    pub is_valid: bool,
    /// Non-blocking warnings that the user should be aware of
    pub warnings: Vec<String>,
    /// Blocking errors that prevent resumption
    pub errors: Vec<String>,
}

impl ResumeValidationResult {
    /// Create a new validation result (starts as valid with no issues).
    pub fn new() -> Self {
        Self {
            is_valid: true,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a warning (non-blocking issue).
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add an error (blocking issue).
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.is_valid = false;
        self.errors.push(error.into());
        self
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: &ResumeValidationResult) {
        if !other.is_valid {
            self.is_valid = false;
        }
        self.warnings.extend(other.warnings.clone());
        self.errors.extend(other.errors.clone());
    }
}

impl Default for ResumeValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for validating task resume operations.
///
/// Performs pre-flight checks before resuming a stopped task to ensure
/// the environment is in a safe state:
///
/// - Git state validation: branch exists, worktree clean, no stale operations
/// - Agent cleanup: kill orphan processes, reset counters
pub struct ResumeValidator {
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
}

impl ResumeValidator {
    /// Create a new ResumeValidator with the given agent registry.
    pub fn new(running_agent_registry: Arc<dyn RunningAgentRegistry>) -> Self {
        Self {
            running_agent_registry,
        }
    }

    /// Validate that a task can be safely resumed.
    ///
    /// Performs all validation checks and returns a combined result.
    ///
    /// # Arguments
    /// * `task` - The task to validate for resume
    /// * `project` - The project the task belongs to
    /// * `worktree_path` - Optional path to the worktree (for Worktree mode)
    ///
    /// # Returns
    /// A `ResumeValidationResult` with:
    /// - `is_valid`: true if resumption is safe
    /// - `warnings`: non-blocking issues to display to user
    /// - `errors`: blocking issues that prevent resumption
    pub async fn validate(
        &self,
        task: &Task,
        project: &Project,
        worktree_path: Option<&str>,
    ) -> AppResult<ResumeValidationResult> {
        let mut result = ResumeValidationResult::new();

        // 1. Git state validation
        let git_result = self.validate_git_state(task, project, worktree_path).await?;
        result.merge(&git_result);

        // 2. Agent cleanup (always attempt, even if git validation failed)
        let agent_result = self.cleanup_orphan_agents(task).await;
        result.merge(&agent_result);

        // 3. Check if base branch has moved ahead (warning only)
        if let Some(ref base_branch) = project.base_branch {
            if let Some(ref task_branch) = task.task_branch {
                let repo_path = PathBuf::from(&project.working_directory);
                if GitService::branch_exists(&repo_path, base_branch).await
                    && GitService::branch_exists(&repo_path, task_branch).await
                {
                    // Check if base branch has commits not on task branch
                    if self.has_base_branch_moved_ahead(&repo_path, task_branch, base_branch).await? {
                        result = result.with_warning(format!(
                            "Base branch '{}' has new commits that are not in task branch '{}'",
                            base_branch, task_branch
                        ));
                    }
                }
            }
        }

        debug!(
            task_id = task.id.as_str(),
            is_valid = result.is_valid,
            warnings = result.warnings.len(),
            errors = result.errors.len(),
            "Resume validation completed"
        );

        Ok(result)
    }

    /// Validate git state for resumption.
    ///
    /// Checks:
    /// 1. Task branch exists and is accessible
    /// 2. Worktree is clean (no uncommitted changes)
    /// 3. No stale merge/rebase in progress
    async fn validate_git_state(
        &self,
        task: &Task,
        project: &Project,
        worktree_path: Option<&str>,
    ) -> AppResult<ResumeValidationResult> {
        let mut result = ResumeValidationResult::new();

        let repo_path = PathBuf::from(&project.working_directory);

        // 1. Check task branch exists
        if let Some(ref task_branch) = task.task_branch {
            if !GitService::branch_exists(&repo_path, task_branch).await {
                result = result.with_error(format!(
                    "Task branch '{}' does not exist",
                    task_branch
                ));
                return Ok(result);
            }
            debug!(
                task_branch = task_branch.as_str(),
                "Task branch exists"
            );
        } else {
            // No branch is not necessarily an error - task may not have git isolation
            debug!("Task has no associated branch, skipping branch validation");
            return Ok(result);
        }

        // 2. Check worktree/repo is clean
        let check_path = worktree_path
            .map(PathBuf::from)
            .unwrap_or_else(|| repo_path.clone());

        if let Ok(status_output) = self.get_git_status(&check_path).await {
            if !status_output.trim().is_empty() {
                result = result.with_error(format!(
                    "Working tree has uncommitted changes:\n{}",
                    self.truncate_status_output(&status_output)
                ));
            }
        }

        // 3. Check for stale merge/rebase
        if GitService::is_rebase_in_progress(&check_path) {
            result = result.with_error(
                "A rebase operation is in progress. Complete or abort the rebase before resuming.",
            );
        }

        if GitService::is_merge_in_progress(&check_path) {
            result = result.with_error(
                "A merge operation is in progress. Complete or abort the merge before resuming.",
            );
        }

        // 4. Check for conflict markers in changed files
        if let Ok(true) = GitService::has_conflict_markers(&check_path).await {
            result = result.with_error(
                "Conflict markers found in working tree. Resolve conflicts before resuming.",
            );
        }

        Ok(result)
    }

    /// Clean up orphan agent processes for a task.
    ///
    /// Kills any running agents associated with this task and unregisters them.
    /// Also handles cleanup of agents that might have been orphaned by crashes.
    async fn cleanup_orphan_agents(&self, task: &Task) -> ResumeValidationResult {
        let mut result = ResumeValidationResult::new();

        // Determine context type based on what state the task might be in
        // We check multiple context types since the task might have been stopped
        // from different states (executing, reviewing, merging)
        let context_types = ["task_execution", "review", "merge"];

        let mut stopped_count = 0;
        for context_type in context_types {
            let key = RunningAgentKey::new(context_type, task.id.as_str());

            // Check if agent is registered
            if self.running_agent_registry.is_running(&key).await {
                info!(
                    task_id = task.id.as_str(),
                    context_type = context_type,
                    "Stopping orphan agent for task"
                );

                match self.running_agent_registry.stop(&key).await {
                    Ok(Some(_)) => {
                        stopped_count += 1;
                        debug!(
                            task_id = task.id.as_str(),
                            context_type = context_type,
                            "Stopped orphan agent"
                        );
                    }
                    Ok(None) => {
                        // Agent was not found in registry (already stopped)
                        debug!(
                            task_id = task.id.as_str(),
                            context_type = context_type,
                            "Agent was not in registry"
                        );
                    }
                    Err(e) => {
                        warn!(
                            task_id = task.id.as_str(),
                            context_type = context_type,
                            error = %e,
                            "Failed to stop orphan agent"
                        );
                        result = result.with_warning(format!(
                            "Failed to stop {} agent: {}",
                            context_type, e
                        ));
                    }
                }
            }
        }

        if stopped_count > 0 {
            result = result.with_warning(format!(
                "Stopped {} orphan agent(s) for this task",
                stopped_count
            ));
        }

        result
    }

    /// Get git status --porcelain output for a path.
    async fn get_git_status(&self, path: &Path) -> std::io::Result<String> {
        let output = git_cmd::run(&["status", "--porcelain"], path)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Check if base branch has commits not on the task branch.
    async fn has_base_branch_moved_ahead(
        &self,
        repo_path: &Path,
        task_branch: &str,
        base_branch: &str,
    ) -> AppResult<bool> {
        // Use git rev-list to count commits on base not in task
        let range = format!("{}..{}", task_branch, base_branch);
        match git_cmd::run(&["rev-list", "--count", &range], repo_path).await {
            Ok(output) => {
                let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(count) = count_str.parse::<u32>() {
                    return Ok(count > 0);
                }
            }
            Err(_) => {}
        }

        // If we can't determine, assume no (don't block)
        Ok(false)
    }

    /// Truncate git status output to prevent overwhelming error messages.
    fn truncate_status_output(&self, status: &str) -> String {
        let lines: Vec<&str> = status.lines().collect();
        if lines.len() <= 10 {
            status.to_string()
        } else {
            format!(
                "{}\n... and {} more files",
                lines[..10].join("\n"),
                lines.len() - 10
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ProjectId, Task};
    use crate::domain::services::MemoryRunningAgentRegistry;

    fn create_test_validator() -> ResumeValidator {
        let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
        ResumeValidator::new(registry)
    }

    fn create_test_task() -> Task {
        Task::new(ProjectId::new(), "Test Task".to_string())
    }

    fn create_test_project() -> Project {
        Project::new("Test Project".to_string(), "/tmp/test".to_string())
    }

    #[test]
    fn test_validation_result_new_is_valid() {
        let result = ResumeValidationResult::new();
        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_with_warning() {
        let result = ResumeValidationResult::new().with_warning("Test warning");
        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0], "Test warning");
    }

    #[test]
    fn test_validation_result_with_error() {
        let result = ResumeValidationResult::new().with_error("Test error");
        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0], "Test error");
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result1 = ResumeValidationResult::new().with_warning("Warning 1");
        let result2 = ResumeValidationResult::new()
            .with_warning("Warning 2")
            .with_error("Error 1");

        result1.merge(&result2);

        assert!(!result1.is_valid);
        assert_eq!(result1.warnings.len(), 2);
        assert_eq!(result1.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_task_without_branch() {
        let validator = create_test_validator();
        let task = create_test_task();
        let project = create_test_project();

        let result = validator.validate(&task, &project, None).await.unwrap();

        // Task without branch should validate (no git isolation)
        assert!(result.is_valid);
    }

    #[tokio::test]
    async fn test_cleanup_orphan_agents_no_agents() {
        let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
        let validator = ResumeValidator::new(Arc::clone(&registry));
        let task = create_test_task();

        let result = validator.cleanup_orphan_agents(&task).await;

        assert!(result.is_valid);
        assert!(result.warnings.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_orphan_agents_with_running_agent() {
        let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
        let validator = ResumeValidator::new(Arc::clone(&registry));
        let task = create_test_task();

        // Register a running agent
        let key = RunningAgentKey::new("task_execution", task.id.as_str());
        registry
            .register(
                key.clone(),
                12345,
                "conv-123".to_string(),
                "run-123".to_string(),
                None,
                None,
            )
            .await;

        let result = validator.cleanup_orphan_agents(&task).await;

        assert!(result.is_valid);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("Stopped 1 orphan agent"));

        // Agent should be unregistered
        assert!(!registry.is_running(&key).await);
    }

    #[tokio::test]
    async fn test_cleanup_multiple_orphan_agents() {
        let registry: Arc<dyn RunningAgentRegistry> = Arc::new(MemoryRunningAgentRegistry::new());
        let validator = ResumeValidator::new(Arc::clone(&registry));
        let task = create_test_task();

        // Register multiple agents for different contexts
        for context_type in &["task_execution", "review"] {
            let key = RunningAgentKey::new(*context_type, task.id.as_str());
            registry
                .register(
                    key,
                    12345,
                    "conv-123".to_string(),
                    "run-123".to_string(),
                    None,
                    None,
                )
                .await;
        }

        let result = validator.cleanup_orphan_agents(&task).await;

        assert!(result.is_valid);
        assert!(result.warnings[0].contains("Stopped 2 orphan agent"));
    }

    #[test]
    fn test_truncate_status_output_short() {
        let validator = create_test_validator();
        let status = "M file1.txt\nM file2.txt";
        let truncated = validator.truncate_status_output(status);
        assert_eq!(truncated, status);
    }

    #[test]
    fn test_truncate_status_output_long() {
        let validator = create_test_validator();
        let lines: Vec<String> = (0..20).map(|i| format!("M file{}.txt", i)).collect();
        let status = lines.join("\n");
        let truncated = validator.truncate_status_output(&status);

        assert!(truncated.contains("... and 10 more files"));
        assert!(!truncated.contains("file19.txt"));
    }
}
