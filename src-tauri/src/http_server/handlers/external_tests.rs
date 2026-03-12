// Integration tests for external API handlers (Phase 4 + Phase 5)
//
// Tests list_projects_http, get_project_status_http, get_pipeline_overview_http,
// start_ideation_http, poll_events_http, get_task_detail_http,
// get_task_review_summary_http, get_merge_pipeline_http, and related handlers
// using the in-memory AppState.

use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{
    ideation::IdeationSession,
    project::Project,
    task::Task,
    types::ProjectId,
    InternalStatus,
};
use crate::http_server::project_scope::ProjectScope;
use std::sync::Arc;

// ============================================================================
// Setup helpers
// ============================================================================

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

fn make_project(id: &str, name: &str) -> Project {
    Project {
        id: ProjectId::from_string(id.to_string()),
        name: name.to_string(),
        working_directory: "/tmp".to_string(),
        git_mode: crate::domain::entities::project::GitMode::Worktree,
        base_branch: None,
        worktree_parent_directory: None,
        use_feature_branches: true,
        merge_validation_mode: Default::default(),
        merge_strategy: Default::default(),
        detected_analysis: None,
        custom_analysis: None,
        analyzed_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn unrestricted_scope() -> ProjectScope {
    ProjectScope(None)
}

fn scoped(ids: &[&str]) -> ProjectScope {
    let vec: Vec<ProjectId> = ids
        .iter()
        .map(|s| ProjectId::from_string(s.to_string()))
        .collect();
    ProjectScope(Some(vec))
}

// ============================================================================
// list_projects_http
// ============================================================================

#[tokio::test]
async fn test_list_projects_no_scope() {
    let state = setup_test_state().await;

    // Create two projects
    let p1 = make_project("proj-alpha", "Alpha");
    let p2 = make_project("proj-beta", "Beta");
    state.app_state.project_repo.create(p1).await.unwrap();
    state.app_state.project_repo.create(p2).await.unwrap();

    let result = list_projects_http(State(state), unrestricted_scope()).await;
    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.projects.len(), 2);

    let names: Vec<&str> = response.projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"Alpha"));
    assert!(names.contains(&"Beta"));
}

#[tokio::test]
async fn test_list_projects_with_scope() {
    let state = setup_test_state().await;

    let p1 = make_project("proj-alpha", "Alpha");
    let p2 = make_project("proj-beta", "Beta");
    state.app_state.project_repo.create(p1).await.unwrap();
    state.app_state.project_repo.create(p2).await.unwrap();

    // Scope to proj-alpha only
    let result = list_projects_http(State(state), scoped(&["proj-alpha"])).await;
    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.projects.len(), 1);
    assert_eq!(response.projects[0].name, "Alpha");
    assert_eq!(response.projects[0].id, "proj-alpha");
}

#[tokio::test]
async fn test_list_projects_empty_scope_returns_nothing() {
    let state = setup_test_state().await;

    let p1 = make_project("proj-alpha", "Alpha");
    state.app_state.project_repo.create(p1).await.unwrap();

    // Scope to an unrelated project
    let result = list_projects_http(State(state), scoped(&["proj-other"])).await;
    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.projects.len(), 0);
}

// ============================================================================
// get_project_status_http
// ============================================================================

#[tokio::test]
async fn test_get_project_status_returns_task_counts() {
    let state = setup_test_state().await;

    let project_id = "proj-status-test";
    let p = make_project(project_id, "Status Test");
    state.app_state.project_repo.create(p).await.unwrap();

    // Create tasks with various statuses
    let task_backlog = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Backlog task".to_string(),
    );
    let mut task_executing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Executing task".to_string(),
    );
    task_executing.internal_status = InternalStatus::Executing;
    let mut task_merged = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merged task".to_string(),
    );
    task_merged.internal_status = InternalStatus::Merged;

    state
        .app_state
        .task_repo
        .create(task_backlog)
        .await
        .unwrap();
    state
        .app_state
        .task_repo
        .create(task_executing)
        .await
        .unwrap();
    state
        .app_state
        .task_repo
        .create(task_merged)
        .await
        .unwrap();

    let result = get_project_status_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.project.id, project_id);
    assert_eq!(response.project.name, "Status Test");
    assert_eq!(response.task_counts.total, 3);
    assert_eq!(response.task_counts.backlog, 1);
    assert_eq!(response.task_counts.executing, 1);
    assert_eq!(response.task_counts.merged, 1);
}

#[tokio::test]
async fn test_get_project_status_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-secret";
    let p = make_project(project_id, "Secret Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Request scoped to a different project
    let result = get_project_status_http(
        State(state),
        scoped(&["proj-other"]),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_project_status_not_found() {
    let state = setup_test_state().await;

    let result = get_project_status_http(
        State(state),
        unrestricted_scope(),
        Path("nonexistent-proj".to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// get_pipeline_overview_http
// ============================================================================

#[tokio::test]
async fn test_get_pipeline_overview_counts_stages() {
    let state = setup_test_state().await;

    let project_id = "proj-pipeline";
    let p = make_project(project_id, "Pipeline Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task_pending = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Pending".to_string(),
    );
    task_pending.internal_status = InternalStatus::Ready;

    let mut task_exec = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Executing".to_string(),
    );
    task_exec.internal_status = InternalStatus::Executing;

    let mut task_merged = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merged".to_string(),
    );
    task_merged.internal_status = InternalStatus::Merged;

    state
        .app_state
        .task_repo
        .create(task_pending)
        .await
        .unwrap();
    state.app_state.task_repo.create(task_exec).await.unwrap();
    state
        .app_state
        .task_repo
        .create(task_merged)
        .await
        .unwrap();

    let result = get_pipeline_overview_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.project_id, project_id);
    assert_eq!(response.stages.pending, 1);
    assert_eq!(response.stages.executing, 1);
    assert_eq!(response.stages.merged, 1);
}

#[tokio::test]
async fn test_get_pipeline_overview_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-pipeline-secret";
    let p = make_project(project_id, "Secret Pipeline");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = get_pipeline_overview_http(
        State(state),
        scoped(&["proj-different"]),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// start_ideation_http
// ============================================================================

#[tokio::test]
async fn test_start_ideation_creates_session() {
    let state = setup_test_state().await;

    let project_id = "proj-ideation";
    let p = make_project(project_id, "Ideation Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: "New feature brainstorm".to_string(),
            initial_prompt: Some("Let's ideate on authentication".to_string()),
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
}

#[tokio::test]
async fn test_start_ideation_rate_limit() {
    let state = setup_test_state().await;

    let project_id = "proj-rate-limit";
    let p = make_project(project_id, "Rate Limit Test");
    state.app_state.project_repo.create(p).await.unwrap();

    // Create an active ideation session directly (max_sessions=1)
    let existing_session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Existing active session",
    );
    state
        .app_state
        .ideation_session_repo
        .create(existing_session)
        .await
        .unwrap();

    // Second attempt should be rate limited
    let result = start_ideation_http(
        State(state),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: "Another session".to_string(),
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        axum::http::StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn test_start_ideation_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-ideation-secret";
    let p = make_project(project_id, "Secret Ideation");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = start_ideation_http(
        State(state),
        scoped(&["proj-other"]),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: "Forbidden".to_string(),
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// poll_events_http
// ============================================================================

#[tokio::test]
async fn test_poll_events_cursor_based() {
    let state = setup_test_state().await;

    let project_id = "proj-events";
    let p = make_project(project_id, "Events Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Create the external_events table and seed data using the in-memory db
    let proj_id_clone = project_id.to_string();
    state
        .app_state
        .db
        .run(move |conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS external_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    event_type TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    payload TEXT NOT NULL DEFAULT '{}',
                    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
                );",
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.created', ?1, '{\"id\":\"t1\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.created', ?1, '{\"id\":\"t2\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.merged', ?1, '{\"id\":\"t3\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            Ok(())
        })
        .await
        .unwrap();

    // Poll without cursor — should return all 3 events
    let result = poll_events_http(
        State(state.clone()),
        unrestricted_scope(),
        Query(PollEventsQuery {
            project_id: project_id.to_string(),
            cursor: None,
            limit: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.events.len(), 3);
    assert!(!response.has_more);
    assert!(response.next_cursor.is_none());

    // Poll with cursor after first event — should return only events 2 and 3
    let first_id = response.events[0].id;
    let result2 = poll_events_http(
        State(state),
        unrestricted_scope(),
        Query(PollEventsQuery {
            project_id: project_id.to_string(),
            cursor: Some(first_id),
            limit: None,
        }),
    )
    .await;

    assert!(result2.is_ok());
    let response2 = result2.unwrap().0;
    assert_eq!(response2.events.len(), 2);
    assert!(!response2.has_more);
}

#[tokio::test]
async fn test_poll_events_limit_and_has_more() {
    let state = setup_test_state().await;

    let project_id = "proj-events-limit";
    let p = make_project(project_id, "Events Limit Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Create table and insert 3 events
    let proj_id_clone = project_id.to_string();
    state
        .app_state
        .db
        .run(move |conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS external_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    event_type TEXT NOT NULL,
                    project_id TEXT NOT NULL,
                    payload TEXT NOT NULL DEFAULT '{}',
                    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
                );",
            )
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
            for i in 0..3 {
                conn.execute(
                    "INSERT INTO external_events (event_type, project_id, payload) VALUES ('ev', ?1, '{}')",
                    rusqlite::params![proj_id_clone],
                )
                .map_err(|e| crate::error::AppError::Database(e.to_string()))?;
                let _ = i;
            }
            Ok(())
        })
        .await
        .unwrap();

    // Request with limit=2 — should indicate has_more=true
    let result = poll_events_http(
        State(state),
        unrestricted_scope(),
        Query(PollEventsQuery {
            project_id: project_id.to_string(),
            cursor: None,
            limit: Some(2),
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.events.len(), 2);
    assert!(response.has_more);
    assert!(response.next_cursor.is_some());
}

#[tokio::test]
async fn test_poll_events_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-events-secret";
    let p = make_project(project_id, "Secret Events");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = poll_events_http(
        State(state),
        scoped(&["proj-other"]),
        Query(PollEventsQuery {
            project_id: project_id.to_string(),
            cursor: None,
            limit: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// external_task_transition_http — pause, cancel, retry
// ============================================================================

#[tokio::test]
async fn test_task_transition_pause() {
    let state = setup_test_state().await;

    let project_id = "proj-transition";
    let p = make_project(project_id, "Transition Test");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Running task".to_string(),
    );
    task.internal_status = InternalStatus::Executing;
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = external_task_transition_http(
        State(state),
        unrestricted_scope(),
        Json(TaskTransitionRequest {
            task_id: task.id.to_string(),
            action: TransitionAction::Pause,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.success);
    assert_eq!(response.task_id, task.id.to_string());
}

#[tokio::test]
async fn test_task_transition_cancel() {
    let state = setup_test_state().await;

    let project_id = "proj-cancel";
    let p = make_project(project_id, "Cancel Test");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Task to cancel".to_string(),
    );
    task.internal_status = InternalStatus::Ready;
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = external_task_transition_http(
        State(state),
        unrestricted_scope(),
        Json(TaskTransitionRequest {
            task_id: task.id.to_string(),
            action: TransitionAction::Cancel,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.success);
}

#[tokio::test]
async fn test_task_transition_retry_from_terminal() {
    let state = setup_test_state().await;

    let project_id = "proj-retry";
    let p = make_project(project_id, "Retry Test");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Stopped task".to_string(),
    );
    task.internal_status = InternalStatus::Stopped;
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = external_task_transition_http(
        State(state),
        unrestricted_scope(),
        Json(TaskTransitionRequest {
            task_id: task.id.to_string(),
            action: TransitionAction::Retry,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.success);
}

#[tokio::test]
async fn test_task_transition_retry_non_terminal_fails() {
    let state = setup_test_state().await;

    let project_id = "proj-retry-fail";
    let p = make_project(project_id, "Retry Fail Test");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Executing task".to_string(),
    );
    task.internal_status = InternalStatus::Executing;
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = external_task_transition_http(
        State(state),
        unrestricted_scope(),
        Json(TaskTransitionRequest {
            task_id: task.id.to_string(),
            action: TransitionAction::Retry,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_task_transition_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-transition-secret";
    let p = make_project(project_id, "Secret Transition");
    state.app_state.project_repo.create(p).await.unwrap();

    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Protected task".to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = external_task_transition_http(
        State(state),
        scoped(&["proj-other"]),
        Json(TaskTransitionRequest {
            task_id: task.id.to_string(),
            action: TransitionAction::Pause,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// get_task_detail_http
// ============================================================================

#[tokio::test]
async fn test_get_task_detail_returns_task_and_steps() {
    let state = setup_test_state().await;

    let project_id = "proj-detail";
    let p = make_project(project_id, "Detail Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Detail task".to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = get_task_detail_http(
        State(state),
        unrestricted_scope(),
        Path(task.id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.id, task.id.to_string());
    assert_eq!(response.title, "Detail task");
    assert_eq!(response.project_id, project_id);
    assert!(response.steps.is_empty());
}

#[tokio::test]
async fn test_get_task_detail_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-detail-secret";
    let p = make_project(project_id, "Secret Detail");
    state.app_state.project_repo.create(p).await.unwrap();

    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Secret task".to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = get_task_detail_http(
        State(state),
        scoped(&["proj-other"]),
        Path(task.id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_task_detail_not_found() {
    let state = setup_test_state().await;

    let result = get_task_detail_http(
        State(state),
        unrestricted_scope(),
        Path("nonexistent-task".to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// get_task_review_summary_http
// ============================================================================

#[tokio::test]
async fn test_get_review_summary_empty() {
    let state = setup_test_state().await;

    let project_id = "proj-review-sum";
    let p = make_project(project_id, "Review Summary Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Review task".to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = get_task_review_summary_http(
        State(state),
        unrestricted_scope(),
        Path(task.id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.task_id, task.id.to_string());
    assert!(response.review_notes.is_empty());
    assert_eq!(response.revision_count, 0);
}

#[tokio::test]
async fn test_get_review_summary_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-review-sum-secret";
    let p = make_project(project_id, "Secret Review");
    state.app_state.project_repo.create(p).await.unwrap();

    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Secret review task".to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();

    let result = get_task_review_summary_http(
        State(state),
        scoped(&["proj-other"]),
        Path(task.id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// get_merge_pipeline_http
// ============================================================================

#[tokio::test]
async fn test_get_merge_pipeline_filters_correctly() {
    let state = setup_test_state().await;

    let project_id = "proj-merge-pipe";
    let p = make_project(project_id, "Merge Pipeline Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut task_pending_merge = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Pending merge task".to_string(),
    );
    task_pending_merge.internal_status = InternalStatus::PendingMerge;

    let mut task_merging = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merging task".to_string(),
    );
    task_merging.internal_status = InternalStatus::Merging;

    let task_executing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Not in merge".to_string(),
    );

    state.app_state.task_repo.create(task_pending_merge).await.unwrap();
    state.app_state.task_repo.create(task_merging).await.unwrap();
    state.app_state.task_repo.create(task_executing).await.unwrap();

    let result = get_merge_pipeline_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.project_id, project_id);
    assert_eq!(response.tasks.len(), 2);
}

#[tokio::test]
async fn test_get_merge_pipeline_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-merge-secret";
    let p = make_project(project_id, "Secret Merge");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = get_merge_pipeline_http(
        State(state),
        scoped(&["proj-other"]),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// get_attention_items_http
// ============================================================================

#[tokio::test]
async fn test_get_attention_items_groups_by_category() {
    let state = setup_test_state().await;

    let project_id = "proj-attention";
    let p = make_project(project_id, "Attention Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut escalated = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Escalated review".to_string(),
    );
    escalated.internal_status = InternalStatus::Escalated;

    let mut failed = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Failed task".to_string(),
    );
    failed.internal_status = InternalStatus::Failed;

    let mut conflict = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merge conflict".to_string(),
    );
    conflict.internal_status = InternalStatus::MergeConflict;

    let healthy = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Healthy task".to_string(),
    );

    state.app_state.task_repo.create(escalated).await.unwrap();
    state.app_state.task_repo.create(failed).await.unwrap();
    state.app_state.task_repo.create(conflict).await.unwrap();
    state.app_state.task_repo.create(healthy).await.unwrap();

    let result = get_attention_items_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.escalated_reviews.len(), 1);
    assert_eq!(response.failed_tasks.len(), 1);
    assert_eq!(response.merge_conflicts.len(), 1);
}

#[tokio::test]
async fn test_get_attention_items_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-attention-secret";
    let p = make_project(project_id, "Secret Attention");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = get_attention_items_http(
        State(state),
        scoped(&["proj-other"]),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_attention_items_empty_project() {
    let state = setup_test_state().await;

    let project_id = "proj-attention-empty";
    let p = make_project(project_id, "Empty Attention");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = get_attention_items_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.escalated_reviews.is_empty());
    assert!(response.failed_tasks.is_empty());
    assert!(response.merge_conflicts.is_empty());
}

// ============================================================================
// get_execution_capacity_http
// ============================================================================

#[tokio::test]
async fn test_get_execution_capacity_running_and_queued() {
    let state = setup_test_state().await;

    let project_id = "proj-capacity";
    let p = make_project(project_id, "Capacity Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let mut executing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Executing task".to_string(),
    );
    executing.internal_status = InternalStatus::Executing;

    let mut reviewing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Reviewing task".to_string(),
    );
    reviewing.internal_status = InternalStatus::Reviewing;

    let mut pending_review = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Pending review task".to_string(),
    );
    pending_review.internal_status = InternalStatus::PendingReview;

    let healthy = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Blocked task".to_string(),
    );

    state.app_state.task_repo.create(executing).await.unwrap();
    state.app_state.task_repo.create(reviewing).await.unwrap();
    state.app_state.task_repo.create(pending_review).await.unwrap();
    state.app_state.task_repo.create(healthy).await.unwrap();

    let result = get_execution_capacity_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    // executing + reviewing = 2 running
    assert_eq!(response.project_running, 2);
    // pending_review = 1 queued
    assert_eq!(response.project_queued, 1);
}

#[tokio::test]
async fn test_get_execution_capacity_scope_violation() {
    let state = setup_test_state().await;

    let project_id = "proj-capacity-secret";
    let p = make_project(project_id, "Secret Capacity");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = get_execution_capacity_http(
        State(state),
        scoped(&["proj-other"]),
        Path(project_id.to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// external_apply_proposals (POST /api/external/apply_proposals)
// ============================================================================

use crate::domain::entities::{IdeationSessionId, Priority, ProposalCategory, TaskProposal};

fn make_proposal(session_id: IdeationSessionId, title: &str) -> TaskProposal {
    TaskProposal::new(session_id, title, ProposalCategory::Feature, Priority::Medium)
}

/// Creates a project + active ideation session. Returns (project_id_str, session_id_str).
async fn setup_session(
    state: &HttpServerState,
    project_id: &str,
    project_name: &str,
) -> (String, String) {
    let project = make_project(project_id, project_name);
    state.app_state.project_repo.create(project).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());
    let session = IdeationSession::new(pid);
    let created = state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    (project_id.to_string(), created.id.as_str().to_string())
}

#[tokio::test]
async fn test_external_apply_proposals_session_not_found() {
    let state = setup_test_state().await;

    let req = ExternalApplyProposalsRequest {
        session_id: "nonexistent-session".to_string(),
        proposal_ids: vec![],
        target_column: "auto".to_string(),
        use_feature_branch: Some(false),
    };

    let result = external_apply_proposals(State(state), unrestricted_scope(), Json(req)).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_external_apply_proposals_project_scope_enforced() {
    // External agent scoped to "proj-other" cannot apply proposals to session in "proj-apply"
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-apply", "Apply Test").await;

    let req = ExternalApplyProposalsRequest {
        session_id,
        proposal_ids: vec![],
        target_column: "auto".to_string(),
        use_feature_branch: Some(false),
    };

    let result = external_apply_proposals(
        State(state),
        scoped(&["proj-other"]), // wrong project
        Json(req),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().status,
        axum::http::StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn test_external_apply_proposals_unrestricted_scope_allowed() {
    // Unrestricted scope (no X-RalphX-Project-Scope header) allows all projects
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-unrestricted", "Unrestricted").await;

    let req = ExternalApplyProposalsRequest {
        session_id,
        proposal_ids: vec![],
        target_column: "auto".to_string(),
        use_feature_branch: Some(false),
    };

    let result = external_apply_proposals(State(state), unrestricted_scope(), Json(req)).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_external_apply_proposals_correct_scope_allowed() {
    // Scoped key can apply proposals when it has access to the session's project
    let state = setup_test_state().await;
    let (project_id, session_id) = setup_session(&state, "proj-scoped-ok", "Scoped OK").await;

    let req = ExternalApplyProposalsRequest {
        session_id,
        proposal_ids: vec![],
        target_column: "auto".to_string(),
        use_feature_branch: Some(false),
    };

    let result = external_apply_proposals(
        State(state),
        scoped(&[&project_id]), // correct project scope
        Json(req),
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_external_apply_proposals_creates_tasks_from_proposals() {
    // Full apply: session with proposals → tasks created, session_converted = true
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-full-apply", "Full Apply").await;

    let session_id_typed = IdeationSessionId::from_string(session_id.clone());

    let p1 = make_proposal(session_id_typed.clone(), "Task Alpha");
    let p2 = make_proposal(session_id_typed.clone(), "Task Beta");
    let created_p1 = state
        .app_state
        .task_proposal_repo
        .create(p1)
        .await
        .unwrap();
    let created_p2 = state
        .app_state
        .task_proposal_repo
        .create(p2)
        .await
        .unwrap();

    let req = ExternalApplyProposalsRequest {
        session_id,
        proposal_ids: vec![
            created_p1.id.as_str().to_string(),
            created_p2.id.as_str().to_string(),
        ],
        target_column: "auto".to_string(),
        use_feature_branch: Some(false), // no feature branch for test simplicity
    };

    let result = external_apply_proposals(State(state), unrestricted_scope(), Json(req)).await;

    assert!(
        result.is_ok(),
        "apply should succeed: {:?}",
        result.err().map(|e| e.status)
    );
    let response = result.unwrap().0;
    assert_eq!(response.created_task_ids.len(), 2);
    assert!(response.session_converted, "all proposals applied");
    assert!(response.execution_plan_id.is_some());
    assert!(response.warnings.is_empty());
}

// Note: Tests for "blocked when unverified", "allowed when verified", "allowed when skipped"
// require Wave 1 schema migration (`v57_plan_verification.rs`) to add verification_status
// to ideation_sessions. check_verification_gate() is currently a stub (allows all sessions).
// See: src-tauri/src/domain/services/verification_gate.rs
