use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use tracing::error;

use super::*;
use crate::domain::entities::{StepProgressSummary, TaskId, TaskStep, TaskStepId, TaskStepStatus};

pub async fn get_task_steps_http(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<Vec<StepResponse>>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
        .app_state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get steps for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(steps.into_iter().map(StepResponse::from).collect()))
}

pub async fn start_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<StartStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|e| {
            error!("Failed to get step {}: {}", step_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is Pending
    if step.status != TaskStepStatus::Pending {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::InProgress;
    step.started_at = Some(chrono::Utc::now());
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|e| {
            error!("Failed to update step {}: {}", step.id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": &response,
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

pub async fn complete_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<CompleteStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|e| {
            error!("Failed to get step {}: {}", step_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is InProgress
    if step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Completed;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = req.note;
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|e| {
            error!("Failed to update step {}: {}", step.id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": &response,
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

pub async fn skip_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<SkipStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|e| {
            error!("Failed to get step {}: {}", step_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is Pending or InProgress
    if step.status != TaskStepStatus::Pending && step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Skipped;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = Some(req.reason);
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|e| {
            error!("Failed to update step {}: {}", step.id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": &response,
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

pub async fn fail_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<FailStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(req.step_id);

    // Get existing step
    let mut step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|e| {
            error!("Failed to get step {}: {}", step_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Validate step is InProgress
    if step.status != TaskStepStatus::InProgress {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status
    step.status = TaskStepStatus::Failed;
    step.completed_at = Some(chrono::Utc::now());
    step.completion_note = Some(req.error);
    step.touch();

    // Save
    state
        .app_state
        .task_step_repo
        .update(&step)
        .await
        .map_err(|e| {
            error!("Failed to update step {}: {}", step.id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:updated",
            serde_json::json!({
                "step": &response,
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

pub async fn add_step_http(
    State(state): State<HttpServerState>,
    Json(req): Json<AddStepRequest>,
) -> Result<Json<StepResponse>, StatusCode> {
    let task_id = TaskId::from_string(req.task_id);

    // Determine sort_order
    let sort_order = if let Some(after_step_id_str) = req.after_step_id {
        // Insert after specified step
        let after_step_id = TaskStepId::from_string(after_step_id_str);
        let after_step = state
            .app_state
            .task_step_repo
            .get_by_id(&after_step_id)
            .await
            .map_err(|e| {
                error!("Failed to get step {}: {}", after_step_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .ok_or(StatusCode::NOT_FOUND)?;
        after_step.sort_order + 1
    } else {
        // Append to end - find max sort_order
        let steps = state
            .app_state
            .task_step_repo
            .get_by_task(&task_id)
            .await
            .map_err(|e| {
                error!("Failed to get steps for task {}: {}", task_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        steps.iter().map(|s| s.sort_order).max().unwrap_or(-1) + 1
    };

    // Create new step
    let mut step = TaskStep::new(task_id.clone(), req.title, sort_order, "agent".to_string());
    step.description = req.description;

    // Save to repository
    let step = state
        .app_state
        .task_step_repo
        .create(step)
        .await
        .map_err(|e| {
            error!("Failed to create step for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = StepResponse::from(step);

    // Emit event to frontend
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "step:created",
            serde_json::json!({
                "step": &response,
                "task_id": &response.task_id
            }),
        );
    }

    Ok(Json(response))
}

pub async fn get_step_progress_http(
    State(state): State<HttpServerState>,
    Path(task_id): Path<String>,
) -> Result<Json<StepProgressSummary>, StatusCode> {
    let task_id = TaskId::from_string(task_id);
    let steps = state
        .app_state
        .task_step_repo
        .get_by_task(&task_id)
        .await
        .map_err(|e| {
            error!("Failed to get steps for task {}: {}", task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(StepProgressSummary::from_steps(&task_id, &steps)))
}
