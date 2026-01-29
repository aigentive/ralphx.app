use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use super::*;
use crate::domain::entities::{Task, TaskId};

pub async fn update_task(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update fields
    if let Some(title) = req.title {
        task.title = title;
    }
    if let Some(description) = req.description {
        task.description = description;
    }
    if let Some(priority_str) = req.priority {
        task.priority = parse_priority(&priority_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    }

    // Save updated task
    state
        .app_state
        .task_repo
        .update(task.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(task_to_response(&task)))
}

pub async fn add_task_note(
    State(state): State<HttpServerState>,
    Json(req): Json<AddTaskNoteRequest>,
) -> Result<StatusCode, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    state
        .app_state
        .task_service
        .add_note(task_id, req.note)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_task_details(
    State(state): State<HttpServerState>,
    Json(req): Json<GetTaskDetailsRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(task_to_response(&task)))
}

pub fn task_to_response(task: &Task) -> TaskResponse {
    TaskResponse {
        task_id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: task.internal_status.to_string(),
        priority: task.priority.to_string(),
        category: task.category.to_string(),
        steps: task.steps.clone(),
        acceptance_criteria: task.acceptance_criteria.clone(),
        notes: task.notes.clone(),
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }
}
