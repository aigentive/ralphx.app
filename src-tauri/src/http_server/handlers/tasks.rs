use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use tracing::error;

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
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update fields
    if let Some(title) = req.title {
        task.title = title;
    }
    if let Some(description) = req.description {
        task.description = Some(description);
    }
    if let Some(priority) = req.priority {
        task.priority = priority;
    }

    // Save updated task
    state
        .app_state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| {
            error!("Failed to update task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(task_to_response(&task)))
}

pub async fn add_task_note(
    State(state): State<HttpServerState>,
    Json(req): Json<AddTaskNoteRequest>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Get existing task
    let mut task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Add note to description (append with newline separator)
    let note_text = format!("\n\n---\n**Note:** {}", req.note);
    task.description = Some(match task.description {
        Some(existing) => format!("{}{}", existing, note_text),
        None => note_text,
    });

    // Save updated task
    state
        .app_state
        .task_repo
        .update(&task)
        .await
        .map_err(|e| {
            error!("Failed to update task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(task_to_response(&task)))
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
        .map_err(|e| {
            error!("Failed to get task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(task_to_response(&task)))
}

pub fn task_to_response(task: &Task) -> TaskResponse {
    TaskResponse {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone(),
        status: format!("{:?}", task.internal_status),
        priority: task.priority.to_string(),
        category: task.category.clone(),
        created_at: task.created_at.to_rfc3339(),
        updated_at: task.updated_at.to_rfc3339(),
    }
}
