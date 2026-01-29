use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use super::*;
use crate::domain::entities::{InternalStatus, ProjectId};

pub async fn list_tasks(
    State(state): State<HttpServerState>,
    Json(req): Json<ListTasksRequest>,
) -> Result<Json<ListTasksResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    // Parse optional status filter
    let status_filter = req
        .status
        .as_ref()
        .map(|s| parse_internal_status(s.as_str()))
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Get all tasks for project
    let mut tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Apply status filter if provided
    if let Some(status) = status_filter {
        tasks.retain(|t| t.internal_status == status);
    }

    // Convert to response
    let task_responses: Vec<_> = tasks.iter().map(task_to_response).collect();

    Ok(Json(ListTasksResponse {
        tasks: task_responses,
    }))
}

pub async fn suggest_task(
    State(state): State<HttpServerState>,
    Json(req): Json<SuggestTaskRequest>,
) -> Result<Json<SuggestTaskResponse>, StatusCode> {
    let project_id = ProjectId::from_string(req.project_id);

    // Get all backlog tasks for the project
    let tasks = state
        .app_state
        .task_repo
        .get_by_project(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let backlog_tasks: Vec<_> = tasks
        .into_iter()
        .filter(|t| t.internal_status == InternalStatus::Backlog)
        .collect();

    if backlog_tasks.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Find highest priority task (higher i32 = higher priority)
    let suggested = backlog_tasks
        .iter()
        .max_by_key(|t| t.priority)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SuggestTaskResponse {
        task: task_to_response(suggested),
    }))
}
