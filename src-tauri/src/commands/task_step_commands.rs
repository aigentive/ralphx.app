// Tauri commands for TaskStep CRUD operations
// Thin layer that delegates to TaskStepRepository

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::AppState;
use crate::domain::entities::{StepProgressSummary, TaskId, TaskStep, TaskStepId};
use crate::error::AppResult;

/// Input for creating a new task step
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskStepInput {
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// Input for updating a task step
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStepInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// Response wrapper for task step operations
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStepResponse {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub depends_on: Option<String>,
    pub created_by: String,
    pub completion_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<TaskStep> for TaskStepResponse {
    fn from(step: TaskStep) -> Self {
        Self {
            id: step.id.as_str().to_string(),
            task_id: step.task_id.as_str().to_string(),
            title: step.title,
            description: step.description,
            status: step.status.to_db_string().to_string(),
            sort_order: step.sort_order,
            depends_on: step.depends_on.map(|id| id.as_str().to_string()),
            created_by: step.created_by,
            completion_note: step.completion_note,
            created_at: step.created_at.to_rfc3339(),
            updated_at: step.updated_at.to_rfc3339(),
            started_at: step.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: step.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Create a new task step
#[tauri::command]
pub async fn create_task_step(
    task_id: String,
    input: CreateTaskStepInput,
    state: State<'_, AppState>,
) -> AppResult<TaskStepResponse> {
    let task_id = TaskId::from_string(task_id);

    // Determine sort_order: use provided, or default to 0
    let sort_order = input.sort_order.unwrap_or(0);

    // Create new step
    let mut step = TaskStep::new(
        task_id,
        input.title,
        sort_order,
        "user".to_string(),
    );

    // Set description if provided
    if let Some(desc) = input.description {
        step.description = Some(desc);
    }

    // Save to repository
    let step = state.task_step_repo.create(step).await?;

    Ok(TaskStepResponse::from(step))
}

/// Get all steps for a task
#[tauri::command]
pub async fn get_task_steps(
    task_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<TaskStepResponse>> {
    let task_id = TaskId::from_string(task_id);
    let steps = state.task_step_repo.get_by_task(&task_id).await?;
    Ok(steps.into_iter().map(TaskStepResponse::from).collect())
}

/// Update a task step
#[tauri::command]
pub async fn update_task_step(
    step_id: String,
    input: UpdateTaskStepInput,
    state: State<'_, AppState>,
) -> AppResult<TaskStepResponse> {
    let step_id = TaskStepId::from_string(step_id);

    // Get existing step
    let mut step = state
        .task_step_repo
        .get_by_id(&step_id)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound(format!("Step {} not found", step_id.as_str())))?;

    // Update fields
    if let Some(title) = input.title {
        step.title = title;
    }
    if let Some(description) = input.description {
        step.description = Some(description);
    }
    if let Some(sort_order) = input.sort_order {
        step.sort_order = sort_order;
    }

    // Update timestamp
    step.touch();

    // Save
    state.task_step_repo.update(&step).await?;

    Ok(TaskStepResponse::from(step))
}

/// Delete a task step
#[tauri::command]
pub async fn delete_task_step(
    step_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let step_id = TaskStepId::from_string(step_id);
    state.task_step_repo.delete(&step_id).await
}

/// Reorder task steps
#[tauri::command]
pub async fn reorder_task_steps(
    task_id: String,
    step_ids: Vec<String>,
    state: State<'_, AppState>,
) -> AppResult<Vec<TaskStepResponse>> {
    let task_id = TaskId::from_string(task_id);
    let step_ids: Vec<TaskStepId> = step_ids
        .into_iter()
        .map(TaskStepId::from_string)
        .collect();

    state.task_step_repo.reorder(&task_id, step_ids).await?;

    // Return updated steps
    let steps = state.task_step_repo.get_by_task(&task_id).await?;
    Ok(steps.into_iter().map(TaskStepResponse::from).collect())
}

/// Get step progress summary for a task
#[tauri::command]
pub async fn get_step_progress(
    task_id: String,
    state: State<'_, AppState>,
) -> AppResult<StepProgressSummary> {
    let task_id = TaskId::from_string(task_id);
    let steps = state.task_step_repo.get_by_task(&task_id).await?;
    Ok(StepProgressSummary::from_steps(&task_id, &steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{Project, ProjectId, TaskStepStatus};

    fn setup_test_state() -> AppState {
        AppState::new_test()
    }

    async fn create_test_project(state: &AppState) -> Project {
        let project = Project::new(
            "Test Project".to_string(),
            "/tmp/test".to_string(),
        );
        state.project_repo.create(project.clone()).await.unwrap();
        project
    }

    async fn create_test_task(state: &AppState, project_id: ProjectId) -> TaskId {
        let task = crate::domain::entities::Task::new(
            project_id,
            "Test Task".to_string(),
        );
        state.task_repo.create(task.clone()).await.unwrap();
        task.id
    }

    #[tokio::test]
    async fn test_create_task_step() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        // Test repository directly
        let step = TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        );

        let created = state.task_step_repo.create(step).await.unwrap();

        assert_eq!(created.title, "Test Step");
        assert_eq!(created.sort_order, 0);
        assert_eq!(created.status, TaskStepStatus::Pending);
        assert_eq!(created.created_by, "user");
    }

    #[tokio::test]
    async fn test_get_task_steps() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        // Create two steps
        let step1 = TaskStep::new(
            task_id.clone(),
            "Step 1".to_string(),
            0,
            "user".to_string(),
        );
        let step2 = TaskStep::new(
            task_id.clone(),
            "Step 2".to_string(),
            1,
            "user".to_string(),
        );

        state.task_step_repo.create(step1).await.unwrap();
        state.task_step_repo.create(step2).await.unwrap();

        let steps = state.task_step_repo.get_by_task(&task_id).await.unwrap();

        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].title, "Step 1");
        assert_eq!(steps[1].title, "Step 2");
    }

    #[tokio::test]
    async fn test_update_task_step() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        let step = TaskStep::new(
            task_id.clone(),
            "Original Title".to_string(),
            0,
            "user".to_string(),
        );

        let created = state.task_step_repo.create(step).await.unwrap();

        let mut updated = created.clone();
        updated.title = "Updated Title".to_string();
        updated.description = Some("Updated Description".to_string());

        state.task_step_repo.update(&updated).await.unwrap();

        let found = state.task_step_repo.get_by_id(&created.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Updated Title");
        assert_eq!(found.description, Some("Updated Description".to_string()));
        assert_eq!(found.sort_order, 0); // Unchanged
    }

    #[tokio::test]
    async fn test_delete_task_step() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        let step = TaskStep::new(
            task_id.clone(),
            "To Delete".to_string(),
            0,
            "user".to_string(),
        );

        let created = state.task_step_repo.create(step).await.unwrap();

        state.task_step_repo.delete(&created.id).await.unwrap();

        let steps = state.task_step_repo.get_by_task(&task_id).await.unwrap();

        assert_eq!(steps.len(), 0);
    }

    #[tokio::test]
    async fn test_reorder_task_steps() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        // Create three steps
        let step1 = state.task_step_repo.create(
            TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string())
        ).await.unwrap();

        let step2 = state.task_step_repo.create(
            TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string())
        ).await.unwrap();

        let step3 = state.task_step_repo.create(
            TaskStep::new(task_id.clone(), "Step 3".to_string(), 2, "user".to_string())
        ).await.unwrap();

        // Reorder: 3, 1, 2
        let new_order = vec![step3.id.clone(), step1.id.clone(), step2.id.clone()];
        state.task_step_repo.reorder(&task_id, new_order).await.unwrap();

        let reordered = state.task_step_repo.get_by_task(&task_id).await.unwrap();

        assert_eq!(reordered.len(), 3);
        assert_eq!(reordered[0].title, "Step 3");
        assert_eq!(reordered[0].sort_order, 0);
        assert_eq!(reordered[1].title, "Step 1");
        assert_eq!(reordered[1].sort_order, 1);
        assert_eq!(reordered[2].title, "Step 2");
        assert_eq!(reordered[2].sort_order, 2);
    }

    #[tokio::test]
    async fn test_get_step_progress() {
        let state = setup_test_state();
        let project = create_test_project(&state).await;
        let task_id = create_test_task(&state, project.id).await;

        // Create steps with different statuses
        let step1 = state.task_step_repo.create(
            TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string())
        ).await.unwrap();

        state.task_step_repo.create(
            TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string())
        ).await.unwrap();

        // Mark step 1 as completed
        let mut step1_entity = state.task_step_repo.get_by_id(&step1.id).await.unwrap().unwrap();
        step1_entity.status = TaskStepStatus::Completed;
        state.task_step_repo.update(&step1_entity).await.unwrap();

        let steps = state.task_step_repo.get_by_task(&task_id).await.unwrap();
        let progress = StepProgressSummary::from_steps(&task_id, &steps);

        assert_eq!(progress.total, 2);
        assert_eq!(progress.completed, 1);
        assert_eq!(progress.pending, 1);
        assert_eq!(progress.percent_complete, 50.0);
    }
}
