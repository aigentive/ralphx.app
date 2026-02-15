use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use tracing::error;

use super::*;
use crate::domain::entities::{
    StepProgressSummary, Task, TaskId, TaskStep, TaskStepId, TaskStepStatus,
};

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
    step.parent_step_id = req.parent_step_id.map(TaskStepId::from_string);
    step.scope_context = req.scope_context;

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

pub async fn get_sub_steps_http(
    State(state): State<HttpServerState>,
    Path(parent_step_id): Path<String>,
) -> Result<Json<Vec<StepResponse>>, StatusCode> {
    let parent_step_id_typed = TaskStepId::from_string(parent_step_id.clone());

    // First verify the parent step exists
    let parent_step = state
        .app_state
        .task_step_repo
        .get_by_id(&parent_step_id_typed)
        .await
        .map_err(|e| {
            error!("Failed to get parent step {}: {}", parent_step_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get all steps for the task
    let all_steps = state
        .app_state
        .task_step_repo
        .get_by_task(&parent_step.task_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get steps for task {}: {}",
                parent_step.task_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Filter to sub-steps of this parent, ordered by sort_order
    let mut sub_steps: Vec<TaskStep> = all_steps
        .into_iter()
        .filter(|s| s.parent_step_id.as_ref() == Some(&parent_step_id_typed))
        .collect();
    sub_steps.sort_by_key(|s| s.sort_order);

    Ok(Json(
        sub_steps.into_iter().map(StepResponse::from).collect(),
    ))
}

fn build_step_context_hints(step: &TaskStep, task: &Task, siblings: &[TaskStep]) -> Vec<String> {
    let mut hints = Vec::new();
    if step.scope_context.is_some() {
        hints.push("STRICT SCOPE assigned — only modify files listed in scope_context".to_string());
    }
    hints.push(format!(
        "SCOPE: You are executing step \"{}\" for task \"{}\". Do not work on other steps.",
        step.title, task.title
    ));
    if !siblings.is_empty() {
        let names: Vec<_> = siblings.iter().map(|s| s.title.as_str()).collect();
        hints.push(format!(
            "Parallel sibling steps (other coders — DO NOT do their work): {}",
            names.join(", ")
        ));
    }
    hints
}

pub async fn get_step_context_http(
    State(state): State<HttpServerState>,
    Path(step_id): Path<String>,
) -> Result<Json<StepContextResponse>, StatusCode> {
    let step_id = TaskStepId::from_string(step_id);

    // Fetch the step
    let step = state
        .app_state
        .task_step_repo
        .get_by_id(&step_id)
        .await
        .map_err(|e| {
            error!("Failed to get step {}: {}", step_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch parent step if this is a sub-step
    let parent_step = if let Some(ref parent_id) = step.parent_step_id {
        state
            .app_state
            .task_step_repo
            .get_by_id(parent_id)
            .await
            .map_err(|e| {
                error!("Failed to get parent step {}: {}", parent_id.as_str(), e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        None
    };

    // Fetch the task
    let task = state
        .app_state
        .task_repo
        .get_by_id(&step.task_id)
        .await
        .map_err(|e| {
            error!("Failed to get task {}: {}", step.task_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Fetch sibling steps (same parent_step_id)
    let all_steps = state
        .app_state
        .task_step_repo
        .get_by_task(&step.task_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get steps for task {}: {}",
                step.task_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let sibling_steps: Vec<TaskStep> = all_steps
        .into_iter()
        .filter(|s| s.id != step.id && s.parent_step_id == step.parent_step_id)
        .collect();

    // Compute step progress for the task
    let all_task_steps = state
        .app_state
        .task_step_repo
        .get_by_task(&step.task_id)
        .await
        .map_err(|e| {
            error!("Failed to get steps for progress: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let step_progress = StepProgressSummary::from_steps(&step.task_id, &all_task_steps);

    // Build context hints
    let context_hints = build_step_context_hints(&step, &task, &sibling_steps);

    // Build response
    let response = StepContextResponse {
        step: StepResponse::from(step.clone()),
        parent_step: parent_step.map(StepResponse::from),
        task_summary: TaskSummaryForStep {
            id: task.id.as_str().to_string(),
            title: task.title.clone(),
            description: task.description.clone(),
            internal_status: task.internal_status.as_str().to_string(),
        },
        scope_context: step.scope_context.clone(),
        sibling_steps: sibling_steps.into_iter().map(StepResponse::from).collect(),
        step_progress,
        context_hints,
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::commands::ExecutionState;
    use crate::domain::entities::{ProjectId, Task};
    use std::sync::Arc;

    async fn setup_test_state() -> HttpServerState {
        let app_state = Arc::new(AppState::new_test());
        let execution_state = Arc::new(ExecutionState::new());

        HttpServerState {
            app_state,
            execution_state,
            team_tracker: crate::application::TeamStateTracker::new(),
        }
    }

    #[tokio::test]
    async fn test_add_step_with_parent() {
        let state = setup_test_state().await;

        // Create a project
        let project_id = ProjectId::new();

        // Create a task
        let task = Task::new(project_id, "Test Task".to_string());
        let task_id = task.id.clone();
        state.app_state.task_repo.create(task).await.unwrap();

        // Create a parent step
        let parent_req = AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Parent Step".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        };
        let parent_response = add_step_http(State(state.clone()), Json(parent_req))
            .await
            .unwrap();
        let parent_id = parent_response.0.id.clone();

        // Create a sub-step
        let sub_req = AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Sub Step".to_string(),
            description: Some("A sub-step".to_string()),
            after_step_id: None,
            parent_step_id: Some(parent_id.clone()),
            scope_context: Some(r#"{"files":["test.rs"]}"#.to_string()),
        };

        let response = add_step_http(State(state.clone()), Json(sub_req))
            .await
            .unwrap();

        assert_eq!(response.0.parent_step_id, Some(parent_id));
        assert_eq!(
            response.0.scope_context,
            Some(r#"{"files":["test.rs"]}"#.to_string())
        );
    }

    #[tokio::test]
    async fn test_get_step_context() {
        let state = setup_test_state().await;

        // Create a project
        let project_id = ProjectId::new();

        // Create a task
        let task = Task::new(project_id, "Test Task".to_string());
        let task_id = task.id.clone();
        state.app_state.task_repo.create(task).await.unwrap();

        // Create parent and sub-steps
        let parent_step =
            TaskStep::new(task_id.clone(), "Parent".to_string(), 0, "test".to_string());
        let parent_id = parent_step.id.clone();
        state
            .app_state
            .task_step_repo
            .create(parent_step)
            .await
            .unwrap();

        let mut sub_step = TaskStep::new(task_id.clone(), "Sub".to_string(), 0, "test".to_string());
        sub_step.parent_step_id = Some(parent_id.clone());
        sub_step.scope_context = Some(r#"{"files":["test.rs"]}"#.to_string());
        let sub_id = sub_step.id.clone();
        state
            .app_state
            .task_step_repo
            .create(sub_step)
            .await
            .unwrap();

        // Get step context
        let response =
            get_step_context_http(State(state.clone()), Path(sub_id.as_str().to_string()))
                .await
                .unwrap();

        assert_eq!(response.0.step.id, sub_id.as_str());
        assert_eq!(response.0.parent_step.unwrap().id, parent_id.as_str());
        assert_eq!(response.0.task_summary.id, task_id.as_str());
        assert!(response.0.scope_context.is_some());
        assert!(!response.0.context_hints.is_empty());
    }

    #[tokio::test]
    async fn test_get_sub_steps() {
        let state = setup_test_state().await;

        // Create a project
        let project_id = ProjectId::new();

        // Create a task
        let task = Task::new(project_id, "Test Task".to_string());
        let task_id = task.id.clone();
        state.app_state.task_repo.create(task).await.unwrap();

        // Create parent step
        let parent_step =
            TaskStep::new(task_id.clone(), "Parent".to_string(), 0, "test".to_string());
        let parent_id = parent_step.id.clone();
        state
            .app_state
            .task_step_repo
            .create(parent_step)
            .await
            .unwrap();

        // Create 2 sub-steps
        let mut sub1 = TaskStep::new(task_id.clone(), "Sub 1".to_string(), 0, "test".to_string());
        sub1.parent_step_id = Some(parent_id.clone());
        state.app_state.task_step_repo.create(sub1).await.unwrap();

        let mut sub2 = TaskStep::new(task_id.clone(), "Sub 2".to_string(), 1, "test".to_string());
        sub2.parent_step_id = Some(parent_id.clone());
        state.app_state.task_step_repo.create(sub2).await.unwrap();

        // Get sub-steps
        let response =
            get_sub_steps_http(State(state.clone()), Path(parent_id.as_str().to_string()))
                .await
                .unwrap();

        assert_eq!(response.0.len(), 2);
        assert_eq!(response.0[0].title, "Sub 1");
        assert_eq!(response.0[1].title, "Sub 2");
    }
}
