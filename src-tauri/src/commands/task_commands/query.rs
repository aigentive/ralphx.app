// Query (read-only) handlers for task_commands module

use tauri::State;
use crate::application::AppState;
use crate::domain::entities::{InternalStatus, ProjectId, TaskId};
use super::types::{TaskResponse, TaskListResponse, StatusTransition, StateTransitionResponse};
use super::helpers::status_to_label;

/// List tasks for a project with pagination support
///
/// # Arguments
/// * `project_id` - The project ID
/// * `statuses` - Optional status filter (array of status strings)
/// * `offset` - Pagination offset (default 0)
/// * `limit` - Page size (default 20)
/// * `include_archived` - Whether to include archived tasks (default false)
///
/// # Returns
/// * `TaskListResponse` - Contains tasks, total count, has_more flag, and offset
#[tauri::command]
pub async fn list_tasks(
    project_id: String,
    statuses: Option<Vec<String>>,
    offset: Option<u32>,
    limit: Option<u32>,
    include_archived: Option<bool>,
    state: State<'_, AppState>,
) -> Result<TaskListResponse, String> {
    let project_id = ProjectId::from_string(project_id);
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(20);
    let include_archived = include_archived.unwrap_or(false);

    // Parse statuses if provided
    let internal_statuses = if let Some(status_vec) = statuses {
        let mut parsed = Vec::new();
        for status_str in status_vec {
            let status = status_str
                .parse::<InternalStatus>()
                .map_err(|_| format!("Invalid status: {}", status_str))?;
            parsed.push(status);
        }
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    } else {
        None
    };

    // Get paginated tasks
    let tasks = state
        .task_repo
        .list_paginated(&project_id, internal_statuses, offset, limit, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Get total count
    let total = state
        .task_repo
        .count_tasks(&project_id, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Calculate has_more
    let has_more = (offset + tasks.len() as u32) < total;

    // Convert to response
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(TaskListResponse {
        tasks: task_responses,
        total,
        has_more,
        offset,
    })
}

/// Get a single task by ID
#[tauri::command]
pub async fn get_task(id: String, state: State<'_, AppState>) -> Result<Option<TaskResponse>, String> {
    let task_id = TaskId::from_string(id);
    state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map(|opt| opt.map(TaskResponse::from))
        .map_err(|e| e.to_string())
}

/// Get the count of archived tasks for a project
///
/// This count is used by the frontend to show an archive access button
/// when archived tasks exist.
///
/// # Arguments
/// * `project_id` - The project ID
///
/// # Returns
/// * `u32` - The count of archived tasks
#[tauri::command]
pub async fn get_archived_count(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<u32, String> {
    let project_id_obj = ProjectId::from_string(project_id);
    state
        .task_repo
        .get_archived_count(&project_id_obj)
        .await
        .map_err(|e| e.to_string())
}

/// Search tasks by title and description (case-insensitive)
///
/// Searches in both title AND description fields for the query string.
/// Uses server-side search for reliable results across all tasks.
///
/// # Arguments
/// * `project_id` - The project ID to search within
/// * `query` - The search query string
/// * `include_archived` - Whether to include archived tasks in search results (default: false)
///
/// # Returns
/// * `Vec<TaskResponse>` - All matching tasks (no pagination - results should be small)
///
/// # Examples
/// ```ignore
/// // Search for "authentication" in title or description
/// search_tasks("proj-123", "authentication", None)
///
/// // Search including archived tasks
/// search_tasks("proj-123", "old feature", Some(true))
/// ```
#[tauri::command]
pub async fn search_tasks(
    project_id: String,
    query: String,
    include_archived: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskResponse>, String> {
    let project_id_obj = ProjectId::from_string(project_id);
    let include_archived = include_archived.unwrap_or(false);

    // Call repository search method
    let tasks = state
        .task_repo
        .search(&project_id_obj, &query, include_archived)
        .await
        .map_err(|e| e.to_string())?;

    // Convert to response
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(task_responses)
}

/// Get state transition history for a task
///
/// Returns a chronological list of all status transitions a task has gone through.
/// Used by the StateTimelineNav component for displaying task state history.
///
/// # Arguments
/// * `task_id` - The task ID to get state history for
///
/// # Returns
/// * `Vec<StateTransitionResponse>` - Chronologically ordered list of state transitions
///
/// # Examples
/// ```ignore
/// // Get state history for a completed task
/// // Returns transitions like:
/// // [
/// //   { from_status: null, to_status: "backlog", trigger: "user", timestamp: "..." },
/// //   { from_status: "backlog", to_status: "ready", trigger: "user", timestamp: "..." },
/// //   { from_status: "ready", to_status: "executing", trigger: "agent", timestamp: "..." },
/// //   { from_status: "executing", to_status: "approved", trigger: "reviewer", timestamp: "..." }
/// // ]
/// get_task_state_transitions("task-123")
/// ```
#[tauri::command]
pub async fn get_task_state_transitions(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StateTransitionResponse>, String> {
    let task_id_obj = TaskId::from_string(task_id);

    // Get status history from repository
    let transitions = state
        .task_repo
        .get_status_history(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?;

    // Convert domain StatusTransition to StateTransitionResponse
    let responses: Vec<StateTransitionResponse> = transitions
        .into_iter()
        .map(|t| StateTransitionResponse {
            from_status: Some(t.from.as_str().to_string()),
            to_status: t.to.as_str().to_string(),
            trigger: t.trigger,
            timestamp: t.timestamp.to_rfc3339(),
        })
        .collect();

    Ok(responses)
}

/// Get valid status transitions for a task
///
/// Queries the state machine for valid transitions from the task's current status
/// and maps them to user-friendly labels for display in the status dropdown.
///
/// # Arguments
/// * `task_id` - The task ID to get valid transitions for
///
/// # Returns
/// * `Vec<StatusTransition>` - List of valid transitions with status string and label
///
/// # Examples
/// ```ignore
/// // Get valid transitions for a task in "backlog" status
/// // Returns: [
/// //   { status: "ready", label: "Ready for Work" },
/// //   { status: "cancelled", label: "Cancel" }
/// // ]
/// get_valid_transitions("task-123")
/// ```
#[tauri::command]
pub async fn get_valid_transitions(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<StatusTransition>, String> {
    // Get the task to check its current status
    let task_id_obj = TaskId::from_string(task_id);
    let task = state
        .task_repo
        .get_by_id(&task_id_obj)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Task not found".to_string())?;

    // Get valid transitions from the state machine
    let valid_transitions = task.internal_status.valid_transitions();

    // Map to user-friendly labels
    let transitions = valid_transitions
        .iter()
        .map(|status| {
            let status_str = status.as_str().to_string();
            let label = status_to_label(*status);
            StatusTransition {
                status: status_str,
                label,
            }
        })
        .collect();

    Ok(transitions)
}

/// Get tasks awaiting review for a project
///
/// Returns tasks in review-related statuses that are awaiting either
/// AI review or human review decision.
///
/// # Arguments
/// * `project_id` - The project ID
///
/// # Returns
/// * `Vec<TaskResponse>` - Tasks in pending_review, reviewing, review_passed, or escalated states
///
/// # Review Status Meanings
/// - `pending_review`: Queued for AI review
/// - `reviewing`: AI review in progress
/// - `review_passed`: AI approved, awaiting human approval
/// - `escalated`: AI escalated, awaiting human decision
#[tauri::command]
pub async fn get_tasks_awaiting_review(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskResponse>, String> {
    let project_id = ProjectId::from_string(project_id);

    // Define the review-related statuses
    let review_statuses = vec![
        InternalStatus::PendingReview,
        InternalStatus::Reviewing,
        InternalStatus::ReviewPassed,
        InternalStatus::Escalated,
    ];

    // Get tasks in review statuses using the existing list_paginated method
    // Use a high limit to get all tasks (no pagination needed for this view)
    let tasks = state
        .task_repo
        .list_paginated(&project_id, Some(review_statuses), 0, 1000, false)
        .await
        .map_err(|e| e.to_string())?;

    // Convert to response
    let task_responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();

    Ok(task_responses)
}
