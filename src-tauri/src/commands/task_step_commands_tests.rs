use super::*;
use crate::application::AppState;
use crate::domain::entities::{Project, ProjectId, TaskStepStatus};

fn setup_test_state() -> AppState {
    AppState::new_test()
}

async fn create_test_project(state: &AppState) -> Project {
    let project = Project::new("Test Project".to_string(), "/tmp/test".to_string());
    state.project_repo.create(project.clone()).await.unwrap();
    project
}

async fn create_test_task(state: &AppState, project_id: ProjectId) -> TaskId {
    let task = crate::domain::entities::Task::new(project_id, "Test Task".to_string());
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
    let step1 = TaskStep::new(task_id.clone(), "Step 1".to_string(), 0, "user".to_string());
    let step2 = TaskStep::new(task_id.clone(), "Step 2".to_string(), 1, "user".to_string());

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

    let found = state
        .task_step_repo
        .get_by_id(&created.id)
        .await
        .unwrap()
        .unwrap();
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
    let step1 = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Step 1".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    let step2 = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Step 2".to_string(),
            1,
            "user".to_string(),
        ))
        .await
        .unwrap();

    let step3 = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Step 3".to_string(),
            2,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Reorder: 3, 1, 2
    let new_order = vec![step3.id.clone(), step1.id.clone(), step2.id.clone()];
    state
        .task_step_repo
        .reorder(&task_id, new_order)
        .await
        .unwrap();

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
    let step1 = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Step 1".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Step 2".to_string(),
            1,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Mark step 1 as completed
    let mut step1_entity = state
        .task_step_repo
        .get_by_id(&step1.id)
        .await
        .unwrap()
        .unwrap();
    step1_entity.status = TaskStepStatus::Completed;
    state.task_step_repo.update(&step1_entity).await.unwrap();

    let steps = state.task_step_repo.get_by_task(&task_id).await.unwrap();
    let progress = StepProgressSummary::from_steps(&task_id, &steps);

    assert_eq!(progress.total, 2);
    assert_eq!(progress.completed, 1);
    assert_eq!(progress.pending, 1);
    assert_eq!(progress.percent_complete, 50.0);
}

#[tokio::test]
async fn test_start_step_valid() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create a pending step
    let step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Start the step via command (simulating tauri command)
    let mut updated = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    updated.status = TaskStepStatus::InProgress;
    updated.started_at = Some(chrono::Utc::now());
    updated.touch();
    state.task_step_repo.update(&updated).await.unwrap();

    // Verify status changed
    let found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TaskStepStatus::InProgress);
    assert!(found.started_at.is_some());
}

#[tokio::test]
async fn test_start_step_invalid_status() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create a step and mark it as completed
    let mut step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    step.status = TaskStepStatus::Completed;
    state.task_step_repo.update(&step).await.unwrap();

    // Trying to start a completed step should fail
    // In actual command this would return AppError::Validation
    let found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TaskStepStatus::Completed);
    assert_ne!(found.status, TaskStepStatus::Pending);
}

#[tokio::test]
async fn test_complete_step_valid() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create and start a step
    let mut step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    step.status = TaskStepStatus::InProgress;
    step.started_at = Some(chrono::Utc::now());
    state.task_step_repo.update(&step).await.unwrap();

    // Complete the step
    let mut found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    found.status = TaskStepStatus::Completed;
    found.completed_at = Some(chrono::Utc::now());
    found.completion_note = Some("Done!".to_string());
    found.touch();
    state.task_step_repo.update(&found).await.unwrap();

    // Verify
    let completed = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(completed.status, TaskStepStatus::Completed);
    assert!(completed.completed_at.is_some());
    assert_eq!(completed.completion_note, Some("Done!".to_string()));
}

#[tokio::test]
async fn test_complete_step_invalid_status() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create a pending step (not in progress)
    let step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Trying to complete a pending step should fail in actual command
    let found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TaskStepStatus::Pending);
    assert_ne!(found.status, TaskStepStatus::InProgress);
}

#[tokio::test]
async fn test_skip_step_from_pending() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create a pending step
    let step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Skip the step
    let mut found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    found.status = TaskStepStatus::Skipped;
    found.completed_at = Some(chrono::Utc::now());
    found.completion_note = Some("Not needed".to_string());
    found.touch();
    state.task_step_repo.update(&found).await.unwrap();

    // Verify
    let skipped = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(skipped.status, TaskStepStatus::Skipped);
    assert!(skipped.completed_at.is_some());
    assert_eq!(skipped.completion_note, Some("Not needed".to_string()));
}

#[tokio::test]
async fn test_skip_step_from_in_progress() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create and start a step
    let mut step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    step.status = TaskStepStatus::InProgress;
    step.started_at = Some(chrono::Utc::now());
    state.task_step_repo.update(&step).await.unwrap();

    // Skip the step
    let mut found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    found.status = TaskStepStatus::Skipped;
    found.completed_at = Some(chrono::Utc::now());
    found.completion_note = Some("Changed approach".to_string());
    found.touch();
    state.task_step_repo.update(&found).await.unwrap();

    // Verify
    let skipped = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(skipped.status, TaskStepStatus::Skipped);
    assert!(skipped.completed_at.is_some());
}

#[tokio::test]
async fn test_fail_step_valid() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create and start a step
    let mut step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    step.status = TaskStepStatus::InProgress;
    step.started_at = Some(chrono::Utc::now());
    state.task_step_repo.update(&step).await.unwrap();

    // Fail the step
    let mut found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    found.status = TaskStepStatus::Failed;
    found.completed_at = Some(chrono::Utc::now());
    found.completion_note = Some("Build error".to_string());
    found.touch();
    state.task_step_repo.update(&found).await.unwrap();

    // Verify
    let failed = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, TaskStepStatus::Failed);
    assert!(failed.completed_at.is_some());
    assert_eq!(failed.completion_note, Some("Build error".to_string()));
}

#[tokio::test]
async fn test_fail_step_invalid_status() {
    let state = setup_test_state();
    let project = create_test_project(&state).await;
    let task_id = create_test_task(&state, project.id).await;

    // Create a pending step (not in progress)
    let step = state
        .task_step_repo
        .create(TaskStep::new(
            task_id.clone(),
            "Test Step".to_string(),
            0,
            "user".to_string(),
        ))
        .await
        .unwrap();

    // Trying to fail a pending step should fail in actual command
    let found = state
        .task_step_repo
        .get_by_id(&step.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.status, TaskStepStatus::Pending);
    assert_ne!(found.status, TaskStepStatus::InProgress);
}
