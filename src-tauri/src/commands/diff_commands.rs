//! Diff Commands - Tauri commands for the DiffViewer
//!
//! Provides file change and diff data for reviewing task execution results.

use crate::application::{AppState, ConflictDiff, DiffService, FileChange, FileDiff};
use crate::domain::entities::{Project, Task, TaskId};
use crate::error::{AppError, AppResult};
use std::path::PathBuf;
use tauri::State;

/// Determine the working path for a task.
///
/// Uses task.worktree_path if available and exists, falls back to project.working_directory.
/// Also returns the project for access to base_branch.
async fn get_task_context(
    app_state: &AppState,
    task_id: &TaskId,
) -> AppResult<(Task, PathBuf, String, Project)> {
    // Get task
    let task = app_state
        .task_repo
        .get_by_id(task_id)
        .await?
        .ok_or_else(|| AppError::TaskNotFound(task_id.as_str().to_string()))?;

    // Get project
    let project = app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await?
        .ok_or_else(|| AppError::ProjectNotFound(task.project_id.as_str().to_string()))?;

    // Determine working path — worktree path if available, else project dir
    let working_path = task
        .worktree_path
        .as_ref()
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(&project.working_directory));

    let working_path_str = working_path.to_string_lossy().to_string();
    Ok((task, working_path, working_path_str, project))
}

/// Get all files changed by the agent for a task
#[tauri::command]
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path and project for this task
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    if task.internal_status == crate::domain::entities::InternalStatus::Merged {
        if let Some(ref merge_sha) = task.merge_commit_sha {
            return diff_service.get_merged_task_file_changes(
                &working_path_str,
                base_branch,
                merge_sha,
            );
        }
    }

    diff_service
        .get_task_file_changes(&task_id, &working_path_str, base_branch)
        .await
}

/// Get the diff content for a specific file
#[tauri::command]
pub async fn get_file_diff(
    app_state: State<'_, AppState>,
    task_id: String,
    file_path: String,
) -> AppResult<FileDiff> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path and project for this task
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    if task.internal_status == crate::domain::entities::InternalStatus::Merged {
        if let Some(ref merge_sha) = task.merge_commit_sha {
            return diff_service.get_merged_task_file_diff(
                &file_path,
                &working_path_str,
                base_branch,
                merge_sha,
            );
        }
    }

    diff_service.get_file_diff(&file_path, &working_path_str, base_branch)
}

/// Get files changed in a specific commit
#[tauri::command]
pub async fn get_commit_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
    commit_sha: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path for this task
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    if task.internal_status == crate::domain::entities::InternalStatus::Merged {
        if let Some(ref merge_sha) = task.merge_commit_sha {
            if merge_sha == &commit_sha
                && diff_service.is_merge_commit(&working_path_str, merge_sha)
            {
                let from_ref =
                    diff_service.get_merged_base_ref(&working_path_str, base_branch, merge_sha);
                return diff_service.get_file_changes_between_refs(
                    &working_path_str,
                    &from_ref,
                    merge_sha,
                );
            }
        }
    }

    diff_service.get_commit_file_changes(&commit_sha, &working_path_str)
}

/// Get diff for a file in a specific commit (comparing to its parent)
#[tauri::command]
pub async fn get_commit_file_diff(
    app_state: State<'_, AppState>,
    task_id: String,
    commit_sha: String,
    file_path: String,
) -> AppResult<FileDiff> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path for this task
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;
    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    if task.internal_status == crate::domain::entities::InternalStatus::Merged {
        if let Some(ref merge_sha) = task.merge_commit_sha {
            if merge_sha == &commit_sha
                && diff_service.is_merge_commit(&working_path_str, merge_sha)
            {
                let from_ref =
                    diff_service.get_merged_base_ref(&working_path_str, base_branch, merge_sha);
                return diff_service.get_file_diff_between_refs(
                    &file_path,
                    &working_path_str,
                    &from_ref,
                    merge_sha,
                );
            }
        }
    }

    diff_service.get_commit_file_diff(&commit_sha, &file_path, &working_path_str)
}

/// Detect merge conflicts for a task.
///
/// Uses two strategies based on the current git state:
/// 1. **Active merge** (MERGE_HEAD exists): Returns files with conflict markers.
/// 2. **Pre-merge preview** (no active merge): Simulates merge using `git merge-tree --write-tree`.
///
/// Returns an empty vector if no conflicts are detected.
///
/// # Arguments
/// * `task_id` - The task to check for conflicts
///
/// # Returns
/// * `Vec<String>` - List of file paths with merge conflicts
#[tauri::command]
pub async fn detect_merge_conflicts(
    app_state: State<'_, AppState>,
    task_id: String,
) -> AppResult<Vec<String>> {
    let task_id = TaskId::from_string(task_id);

    // Get task context (task, working_path, project)
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;

    // Get the task branch - required for conflict detection
    let task_branch = task
        .task_branch
        .as_deref()
        .ok_or_else(|| AppError::Validation("Task has no branch assigned".to_string()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    diff_service.detect_conflicts(&working_path_str, task_branch, base_branch).await
}

/// Get 3-way diff data for a file with merge conflicts.
///
/// Returns the content from all three sides of the merge (base, ours, theirs)
/// plus the current file content with conflict markers for inline rendering.
///
/// # Arguments
/// * `task_id` - The task with conflicts
/// * `file_path` - Path to the conflicting file (relative to project root)
///
/// # Returns
/// * `ConflictDiff` - 3-way diff data with conflict markers
#[tauri::command]
pub async fn get_conflict_file_diff(
    app_state: State<'_, AppState>,
    task_id: String,
    file_path: String,
) -> AppResult<ConflictDiff> {
    let task_id = TaskId::from_string(task_id);

    // Get task context (task, working_path, project)
    let (task, _, working_path_str, project) = get_task_context(&app_state, &task_id).await?;

    // Get the task branch - required for 3-way diff
    let task_branch = task
        .task_branch
        .as_deref()
        .ok_or_else(|| AppError::Validation("Task has no branch assigned".to_string()))?;

    let base_branch = project.base_branch.as_deref().unwrap_or("main");

    let diff_service = DiffService::new();
    diff_service.get_conflict_diff(&file_path, &working_path_str, task_branch, base_branch)
}
