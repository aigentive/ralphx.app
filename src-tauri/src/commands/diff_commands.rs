//! Diff Commands - Tauri commands for the DiffViewer
//!
//! Provides file change and diff data for reviewing task execution results.

use crate::application::{AppState, DiffService, FileChange, FileDiff};
use crate::error::AppResult;
use crate::domain::entities::TaskId;
use tauri::State;
use std::sync::Arc;

/// Get all files changed by the agent for a task
#[tauri::command]
pub async fn get_task_file_changes(
    app_state: State<'_, AppState>,
    task_id: String,
    project_path: String,
) -> AppResult<Vec<FileChange>> {
    let task_id = TaskId::from_string(task_id);
    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_task_file_changes(&task_id, &project_path).await
}

/// Get the diff content for a specific file
#[tauri::command]
pub async fn get_file_diff(
    app_state: State<'_, AppState>,
    file_path: String,
    project_path: String,
) -> AppResult<FileDiff> {
    let diff_service = DiffService::new(Arc::clone(&app_state.activity_event_repo));
    diff_service.get_file_diff(&file_path, &project_path)
}
