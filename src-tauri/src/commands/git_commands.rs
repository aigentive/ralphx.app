// Git commands for task commits, diff, merge, and cleanup
// Thin layer that delegates to GitService and TaskTransitionService

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::application::git_service::{CommitInfo, DiffStats, GitService};
use crate::application::{AppState, TaskTransitionService};
use crate::commands::ExecutionState;
use crate::domain::entities::{GitMode, InternalStatus, ProjectId, TaskId};

/// Response for get_task_commits command
#[derive(Debug, Serialize)]
pub struct TaskCommitsResponse {
    pub commits: Vec<CommitInfoResponse>,
}

/// Individual commit info for response
#[derive(Debug, Serialize)]
pub struct CommitInfoResponse {
    pub sha: String,
    pub short_sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

impl From<CommitInfo> for CommitInfoResponse {
    fn from(info: CommitInfo) -> Self {
        Self {
            sha: info.sha,
            short_sha: info.short_sha,
            message: info.message,
            author: info.author,
            timestamp: info.timestamp,
        }
    }
}

/// Response for get_task_diff_stats command
#[derive(Debug, Serialize)]
pub struct TaskDiffStatsResponse {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub changed_files: Vec<String>,
}

impl From<DiffStats> for TaskDiffStatsResponse {
    fn from(stats: DiffStats) -> Self {
        Self {
            files_changed: stats.files_changed,
            insertions: stats.insertions,
            deletions: stats.deletions,
            changed_files: stats.changed_files,
        }
    }
}

/// Get commits on task branch since it diverged from base
///
/// Returns the list of commits made on this task's branch.
/// Used by UI to show commit history in task detail views.
#[tauri::command]
pub async fn get_task_commits(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskCommitsResponse, String> {
    let task_id = TaskId::from_string(task_id);

    // Get task
    let task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    // Task must have a branch
    let _task_branch = task
        .task_branch
        .as_ref()
        .ok_or_else(|| "Task has no branch assigned".to_string())?;

    // Get project for base branch and working directory
    let project = state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", task.project_id.as_str()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // Determine working path based on git mode
    // For worktree mode, the worktree is already checked out to the task branch
    // For local mode, the repo should be on the task branch when executing
    let working_path = match project.git_mode {
        GitMode::Worktree => task
            .worktree_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&project.working_directory)),
        GitMode::Local => PathBuf::from(&project.working_directory),
    };

    // Get commits since base (from HEAD of the working path)
    let commits = GitService::get_commits_since(&working_path, base_branch)
        .map_err(|e| e.to_string())?;

    Ok(TaskCommitsResponse {
        commits: commits.into_iter().map(CommitInfoResponse::from).collect(),
    })
}

/// Get diff statistics for task branch compared to base
///
/// Returns stats about files changed, lines added/deleted.
/// Used by UI to show change summary in task detail views.
#[tauri::command]
pub async fn get_task_diff_stats(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<TaskDiffStatsResponse, String> {
    let task_id = TaskId::from_string(task_id);

    // Get task
    let task = state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id.as_str()))?;

    // Get project for base branch and working directory
    let project = state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", task.project_id.as_str()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // Determine working path based on git mode
    let working_path = match project.git_mode {
        GitMode::Worktree => task
            .worktree_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&project.working_directory)),
        GitMode::Local => PathBuf::from(&project.working_directory),
    };

    // Get diff stats
    let stats = GitService::get_diff_stats(&working_path, base_branch)
        .map_err(|e| e.to_string())?;

    Ok(TaskDiffStatsResponse::from(stats))
}

/// Mark merge conflict as resolved after manual user resolution
///
/// User has resolved conflicts externally (in IDE), this command:
/// 1. Commits the resolved state
/// 2. Transitions task from MergeConflict to Merged
/// 3. Cleans up branch/worktree
#[tauri::command]
pub async fn resolve_merge_conflict(
    task_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
) -> Result<(), String> {
    let task_id_parsed = TaskId::from_string(task_id);

    // Get task
    let task = state
        .task_repo
        .get_by_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id_parsed.as_str()))?;

    // Validate task is in MergeConflict state
    if task.internal_status != InternalStatus::MergeConflict {
        return Err(format!(
            "Task is not in MergeConflict state (current: {:?})",
            task.internal_status
        ));
    }

    // Get project
    let project = state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", task.project_id.as_str()))?;

    // Determine working path
    let working_path = match project.git_mode {
        GitMode::Worktree => task
            .worktree_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&project.working_directory)),
        GitMode::Local => PathBuf::from(&project.working_directory),
    };

    // Commit the resolved merge
    let commit_message = format!("Merge resolution for task: {}", task.title);
    let commit_sha = GitService::commit_all(&working_path, &commit_message)
        .map_err(|e| format!("Failed to commit resolved merge: {}", e))?;

    // Update task with merge commit SHA if commit was made
    if let Some(sha) = &commit_sha {
        let mut updated_task = task.clone();
        updated_task.merge_commit_sha = Some(sha.clone());
        state
            .task_repo
            .update(&updated_task)
            .await
            .map_err(|e| e.to_string())?;
    }

    // Create transition service and transition to Merged
    let transition_service = create_transition_service(&state, &execution_state);

    transition_service
        .transition_task(&task_id_parsed, InternalStatus::Merged)
        .await
        .map_err(|e| e.to_string())?;

    // Cleanup branch/worktree
    cleanup_task_git_resources(&task, &project).await?;

    Ok(())
}

/// Re-attempt merge after user made changes
///
/// Transitions task back to PendingMerge to trigger programmatic merge attempt.
/// Used when user wants to retry after resolving issues.
#[tauri::command]
pub async fn retry_merge(
    task_id: String,
    state: State<'_, AppState>,
    execution_state: State<'_, Arc<ExecutionState>>,
) -> Result<(), String> {
    let task_id_parsed = TaskId::from_string(task_id);

    // Get task
    let task = state
        .task_repo
        .get_by_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id_parsed.as_str()))?;

    // Validate task is in a mergeable retry state
    let valid_retry_states = [
        InternalStatus::MergeConflict,
        InternalStatus::Merging,
    ];
    if !valid_retry_states.contains(&task.internal_status) {
        return Err(format!(
            "Task is not in a state that allows merge retry (current: {:?})",
            task.internal_status
        ));
    }

    // Create transition service and transition to PendingMerge
    // This will trigger the programmatic merge attempt via on_enter(PendingMerge)
    let transition_service = create_transition_service(&state, &execution_state);

    transition_service
        .transition_task(&task_id_parsed, InternalStatus::PendingMerge)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Manual cleanup for task branch/worktree
///
/// Used for failed/cancelled tasks that have git resources to clean up.
/// Does not change task status.
#[tauri::command]
pub async fn cleanup_task_branch(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let task_id_parsed = TaskId::from_string(task_id);

    // Get task
    let task = state
        .task_repo
        .get_by_id(&task_id_parsed)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Task not found: {}", task_id_parsed.as_str()))?;

    // Task should have a branch to clean up
    if task.task_branch.is_none() {
        return Err("Task has no branch to clean up".to_string());
    }

    // Get project
    let project = state
        .project_repo
        .get_by_id(&task.project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", task.project_id.as_str()))?;

    // Cleanup git resources
    cleanup_task_git_resources(&task, &project).await?;

    // Clear git fields on task
    let mut updated_task = task.clone();
    updated_task.task_branch = None;
    updated_task.worktree_path = None;
    state
        .task_repo
        .update(&updated_task)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Input for changing project git mode
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeGitModeInput {
    pub git_mode: String,
    pub worktree_parent_directory: Option<String>,
}

/// Change project git mode between Local and Worktree
///
/// Allows switching modes after project creation.
/// In-progress tasks continue in their current mode.
#[tauri::command]
pub async fn change_project_git_mode(
    project_id: String,
    input: ChangeGitModeInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let project_id = ProjectId::from_string(project_id);

    // Get project
    let mut project = state
        .project_repo
        .get_by_id(&project_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Project not found: {}", project_id.as_str()))?;

    // Parse git mode
    let new_mode: GitMode = input.git_mode.parse().map_err(|_| {
        format!(
            "Invalid git mode: {}. Valid values: 'local', 'worktree'",
            input.git_mode
        )
    })?;

    // Update project
    project.git_mode = new_mode;
    if let Some(worktree_parent) = input.worktree_parent_directory {
        project.worktree_path = Some(worktree_parent);
    }
    project.touch();

    state
        .project_repo
        .update(&project)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// Helper functions
// ============================================================================

/// Create a TaskTransitionService with required dependencies
fn create_transition_service(
    state: &AppState,
    execution_state: &Arc<ExecutionState>,
) -> TaskTransitionService<tauri::Wry> {
    TaskTransitionService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_dependency_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.chat_message_repo),
        Arc::clone(&state.chat_conversation_repo),
        Arc::clone(&state.agent_run_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.activity_event_repo),
        Arc::clone(&state.message_queue),
        Arc::clone(&state.running_agent_registry),
        Arc::clone(execution_state),
        state.app_handle.clone(),
    )
}

/// Clean up git resources (branch/worktree) for a task
async fn cleanup_task_git_resources(
    task: &crate::domain::entities::Task,
    project: &crate::domain::entities::Project,
) -> Result<(), String> {
    use tracing::warn;

    let repo_path = PathBuf::from(&project.working_directory);
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    // Get task branch
    let task_branch = match &task.task_branch {
        Some(branch) => branch.clone(),
        None => return Ok(()), // Nothing to clean up
    };

    match project.git_mode {
        GitMode::Worktree => {
            // Delete worktree first if it exists
            if let Some(worktree_path) = &task.worktree_path {
                let worktree_path_buf = PathBuf::from(worktree_path);
                if let Err(e) = GitService::delete_worktree(&repo_path, &worktree_path_buf) {
                    warn!("Failed to delete worktree {}: {} (non-fatal)", worktree_path, e);
                }
            }

            // Checkout base branch in main repo before deleting the task branch
            if let Err(e) = GitService::checkout_branch(&repo_path, base_branch) {
                warn!(
                    "Failed to checkout base branch {} after merge: {} (non-fatal)",
                    base_branch, e
                );
            }

            // Delete task branch
            if let Err(e) = GitService::delete_branch(&repo_path, &task_branch, true) {
                warn!("Failed to delete branch {}: {} (non-fatal)", task_branch, e);
            }
        }
        GitMode::Local => {
            // For local mode, just delete the branch (already on base after merge)
            if let Err(e) = GitService::delete_branch(&repo_path, &task_branch, true) {
                warn!("Failed to delete branch {}: {} (non-fatal)", task_branch, e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_info_response_conversion() {
        let info = CommitInfo {
            sha: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
            short_sha: "abcdef1".to_string(),
            message: "Test commit".to_string(),
            author: "Test Author".to_string(),
            timestamp: "2026-02-02T12:00:00+00:00".to_string(),
        };

        let response = CommitInfoResponse::from(info);
        assert_eq!(response.short_sha, "abcdef1");
        assert_eq!(response.message, "Test commit");
    }

    #[test]
    fn test_diff_stats_response_conversion() {
        let stats = DiffStats {
            files_changed: 5,
            insertions: 100,
            deletions: 50,
            changed_files: vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()],
        };

        let response = TaskDiffStatsResponse::from(stats);
        assert_eq!(response.files_changed, 5);
        assert_eq!(response.insertions, 100);
        assert_eq!(response.deletions, 50);
        assert_eq!(response.changed_files.len(), 2);
    }
}
