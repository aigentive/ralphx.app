//! Diff Commands - Tauri commands for the DiffViewer
//!
//! Provides file change and diff data for reviewing task execution results.

use crate::application::{AppState, DiffService, FileChange, FileDiff};
use crate::domain::entities::{GitMode, TaskId};
use crate::error::{AppError, AppResult};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

/// Determine the working path for a task based on git mode
/// - Worktree mode: use task.worktree_path (falls back to project.working_directory)
/// - Local mode: use project.working_directory
async fn get_task_working_path(
    app_state: &AppState,
    task_id: &TaskId,
) -> AppResult<(PathBuf, String)> {
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

    // Determine working path based on git mode
    let working_path = match project.git_mode {
        GitMode::Worktree => task
            .worktree_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&project.working_directory)),
        GitMode::Local => PathBuf::from(&project.working_directory),
    };

    let working_path_str = working_path.to_string_lossy().to_string();
    Ok((working_path, working_path_str))
}

/// Get all files changed by the agent for a task
#[tauri::command]
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);

    // Get the correct working path for this task
    let (_, working_path_str) = get_task_working_path(&app_state, &task_id).await?;

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service
        .get_task_file_changes(&task_id, &working_path_str)
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

    // Get the correct working path for this task
    let (_, working_path_str) = get_task_working_path(&app_state, &task_id).await?;

    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_file_diff(&file_path, &working_path_str)
}
