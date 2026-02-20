use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{ProjectId, Task};
use std::sync::Arc;

async fn setup_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = crate::application::TeamStateTracker::new();
    let team_service = Arc::new(crate::application::TeamService::new_without_events(
        Arc::new(tracker.clone()),
    ));

    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
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
