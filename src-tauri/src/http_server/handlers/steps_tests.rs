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
    let parent_step = TaskStep::new(task_id.clone(), "Parent".to_string(), 0, "test".to_string());
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
    let response = get_step_context_http(State(state.clone()), Path(sub_id.as_str().to_string()))
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
    let parent_step = TaskStep::new(task_id.clone(), "Parent".to_string(), 0, "test".to_string());
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
    let response = get_sub_steps_http(State(state.clone()), Path(parent_id.as_str().to_string()))
        .await
        .unwrap();

    assert_eq!(response.0.len(), 2);
    assert_eq!(response.0[0].title, "Sub 1");
    assert_eq!(response.0[1].title, "Sub 2");
}

// ============================================================================
// IPR (Interactive Process Registry) exit signal tests
// ============================================================================

/// Helper: spawn a cat process to get a live ChildStdin for IPR registration.
/// The caller is responsible for killing the child after the test.
async fn spawn_test_stdin() -> (tokio::process::Child, tokio::process::ChildStdin) {
    let mut child = tokio::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn cat for IPR test");
    let stdin = child.stdin.take().expect("cat stdin handle");
    (child, stdin)
}

/// execution_complete happy path: task exists, IPR entry registered → handler removes IPR.
#[tokio::test]
async fn test_execution_complete_removes_ipr_entry() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "Exec complete test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    // Register a live IPR entry
    let (mut child, stdin) = spawn_test_stdin().await;
    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "task_execution",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    assert!(
        state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR entry should be present before handler call"
    );

    let req = ExecutionCompleteRequest {
        summary: Some("All done".to_string()),
    };
    let result = execution_complete_http(
        State(state.clone()),
        Path(task_id.as_str().to_string()),
        Json(req),
    )
    .await;

    assert!(result.is_ok(), "handler should succeed: {:?}", result.err());
    assert!(result.unwrap().0.success, "response success flag must be true");

    assert!(
        !state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR entry must be removed after execution_complete"
    );

    let _ = child.kill().await;
}

/// execution_complete with non-existent task_id returns 404.
#[tokio::test]
async fn test_execution_complete_task_not_found_returns_404() {
    let state = setup_test_state().await;

    let result = execution_complete_http(
        State(state.clone()),
        Path("non-existent-task-id".to_string()),
        Json(ExecutionCompleteRequest { summary: None }),
    )
    .await;

    match result {
        Err(status) => assert_eq!(
            status,
            axum::http::StatusCode::NOT_FOUND,
            "expected 404 for non-existent task"
        ),
        Ok(_) => panic!("expected 404 for non-existent task"),
    }
}

/// execution_complete without an IPR entry succeeds (agent already exited or never registered).
#[tokio::test]
async fn test_execution_complete_no_ipr_entry_is_idempotent() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "No IPR test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    // No IPR entry registered — handler should still succeed
    let result = execution_complete_http(
        State(state.clone()),
        Path(task_id.as_str().to_string()),
        Json(ExecutionCompleteRequest { summary: None }),
    )
    .await;

    assert!(
        result.is_ok(),
        "handler must succeed even without IPR entry: {:?}",
        result.err()
    );
}

/// execution_complete called twice: second call also succeeds (idempotent).
#[tokio::test]
async fn test_execution_complete_double_call_idempotent() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "Double call test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    let (mut child, stdin) = spawn_test_stdin().await;
    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "task_execution",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    // First call removes the IPR entry
    let result1 = execution_complete_http(
        State(state.clone()),
        Path(task_id.as_str().to_string()),
        Json(ExecutionCompleteRequest { summary: None }),
    )
    .await;
    assert!(result1.is_ok(), "first call should succeed");

    // Second call — IPR already removed, handler must still return Ok
    let result2 = execution_complete_http(
        State(state.clone()),
        Path(task_id.as_str().to_string()),
        Json(ExecutionCompleteRequest { summary: None }),
    )
    .await;
    assert!(result2.is_ok(), "second call should also succeed (idempotent)");

    let _ = child.kill().await;
}

/// complete_step with all steps done triggers the all-steps-done IPR close fallback.
///
/// Scenario: Worker completes the final step → handler detects all steps are Completed/Skipped
/// → removes IPR entry so the agent receives EOF and exits gracefully.
#[tokio::test]
async fn test_complete_step_all_done_removes_ipr() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "All steps done test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    // Create 2 steps (Pending by default)
    let step1_resp = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step 1".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();
    let step1_id = step1_resp.0.id.clone();

    let step2_resp = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step 2".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();
    let step2_id = step2_resp.0.id.clone();

    // Transition both steps Pending → InProgress via start_step_http
    let _ = start_step_http(
        State(state.clone()),
        Json(StartStepRequest {
            step_id: step1_id.clone(),
        }),
    )
    .await
    .unwrap();
    let _ = start_step_http(
        State(state.clone()),
        Json(StartStepRequest {
            step_id: step2_id.clone(),
        }),
    )
    .await
    .unwrap();

    // Register IPR entry for the worker
    let (mut child, stdin) = spawn_test_stdin().await;
    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "task_execution",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    // Complete step 1 — step 2 still InProgress → IPR must remain
    let _ = complete_step_http(
        State(state.clone()),
        Json(CompleteStepRequest {
            step_id: step1_id,
            note: None,
        }),
    )
    .await
    .unwrap();

    assert!(
        state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must remain after completing only 1 of 2 steps"
    );

    // Complete step 2 — all steps now Completed → fallback removes IPR
    let _ = complete_step_http(
        State(state.clone()),
        Json(CompleteStepRequest {
            step_id: step2_id,
            note: None,
        }),
    )
    .await
    .unwrap();

    assert!(
        !state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must be removed after all steps are completed (all-steps-done fallback)"
    );

    let _ = child.kill().await;
}

/// skip_step with all steps done triggers the all-steps-done IPR close fallback.
///
/// Scenario: Worker skips the final step → handler detects all steps are Completed/Skipped
/// → removes IPR entry so the agent receives EOF and exits gracefully.
#[tokio::test]
async fn test_skip_step_all_done_removes_ipr() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "Skip all steps done test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    // Create 2 steps (Pending by default)
    let step1_resp = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step 1".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();
    let step1_id = step1_resp.0.id.clone();

    let step2_resp = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step 2".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();
    let step2_id = step2_resp.0.id.clone();

    // Transition step 1 Pending → InProgress so it can be completed
    let _ = start_step_http(
        State(state.clone()),
        Json(StartStepRequest {
            step_id: step1_id.clone(),
        }),
    )
    .await
    .unwrap();

    // Register IPR entry for the worker
    let (mut child, stdin) = spawn_test_stdin().await;
    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "task_execution",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    // Complete step 1 — step 2 still Pending → IPR must remain
    let _ = complete_step_http(
        State(state.clone()),
        Json(CompleteStepRequest {
            step_id: step1_id,
            note: None,
        }),
    )
    .await
    .unwrap();

    assert!(
        state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must remain after completing only 1 of 2 steps"
    );

    // Skip step 2 (still Pending — skip is valid from Pending state)
    // → all steps now Completed/Skipped → fallback removes IPR
    let _ = skip_step_http(
        State(state.clone()),
        Json(SkipStepRequest {
            step_id: step2_id,
            reason: "Not needed for this implementation".to_string(),
        }),
    )
    .await
    .unwrap();

    assert!(
        !state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must be removed after all steps done via skip (all-steps-done fallback)"
    );

    let _ = child.kill().await;
}

/// complete_step with only some steps done keeps IPR entry intact.
#[tokio::test]
async fn test_complete_step_partial_done_keeps_ipr() {
    let state = setup_test_state().await;

    let project_id = ProjectId::new();
    let task = Task::new(project_id, "Partial steps test".to_string());
    let task_id = task.id.clone();
    state.app_state.task_repo.create(task).await.unwrap();

    // Create 2 steps
    let step1_resp = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step A".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();
    let step1_id = step1_resp.0.id.clone();

    // Step 2 remains Pending (created but not started)
    let _ = add_step_http(
        State(state.clone()),
        Json(AddStepRequest {
            task_id: task_id.as_str().to_string(),
            title: "Step B".to_string(),
            description: None,
            after_step_id: None,
            parent_step_id: None,
            scope_context: None,
        }),
    )
    .await
    .unwrap();

    // Start only step 1
    let _ = start_step_http(
        State(state.clone()),
        Json(StartStepRequest {
            step_id: step1_id.clone(),
        }),
    )
    .await
    .unwrap();

    // Register IPR
    let (mut child, stdin) = spawn_test_stdin().await;
    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "task_execution",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    // Complete step 1 — step 2 is Pending, IPR must remain
    let _ = complete_step_http(
        State(state.clone()),
        Json(CompleteStepRequest {
            step_id: step1_id,
            note: None,
        }),
    )
    .await
    .unwrap();

    assert!(
        state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must remain when not all steps are done"
    );

    // Clean up: remove IPR so ChildStdin is dropped cleanly
    state
        .app_state
        .interactive_process_registry
        .remove(&key)
        .await;
    let _ = child.kill().await;
}
