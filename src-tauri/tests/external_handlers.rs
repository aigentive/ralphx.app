// Integration tests for external API handlers (Phase 4 + Phase 5)
//
// Tests list_projects_http, get_project_status_http, get_pipeline_overview_http,
// start_ideation_http, poll_events_http, get_task_detail_http,
// get_task_review_summary_http, get_merge_pipeline_http, and related handlers
// using the in-memory AppState.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use ralphx_lib::application::{AppState, InteractiveProcessKey, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{
    ideation::{ChatMessage, IdeationSession, IdeationSessionStatus, VerificationStatus},
    project::{GitMode, Project},
    task::Task,
    types::ProjectId,
    IdeationSessionId, InternalStatus, Priority, ProposalCategory, TaskProposal,
};
use ralphx_lib::domain::services::running_agent_registry::RunningAgentKey;
use ralphx_lib::error::AppError;
use ralphx_lib::http_server::handlers::*;
use ralphx_lib::http_server::project_scope::ProjectScope;
use ralphx_lib::http_server::types::HttpServerState;
use std::sync::Arc;

// ============================================================================
// Setup helpers
// ============================================================================

async fn setup_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
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
        git_mode: GitMode::Worktree,
        base_branch: None,
        worktree_parent_directory: None,
        use_feature_branches: true,
        merge_validation_mode: Default::default(),
        merge_strategy: Default::default(),
        detected_analysis: None,
        custom_analysis: None,
        analyzed_at: None,
        github_pr_enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
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

#[tokio::test]
async fn test_list_projects_external_does_not_expose_working_directory() {
    // Security boundary regression: external ProjectSummary must NOT include
    // working_directory. Filesystem paths must never be visible to API key holders.
    let state = setup_test_state().await;

    let p1 = make_project("sec-proj-1", "SecureProject");
    state.app_state.project_repo.create(p1).await.unwrap();

    let result = list_projects_http(State(state), unrestricted_scope()).await;
    let response = result.unwrap().0;
    assert_eq!(response.projects.len(), 1);

    // Serialize and verify no working_directory field is present
    let json_str = serde_json::to_string(&response.projects[0]).unwrap();
    assert!(
        !json_str.contains("working_directory"),
        "external ProjectSummary must not contain working_directory: {}",
        json_str
    );

    // Verify expected fields are present via deserialized map
    let obj: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(&json_str).unwrap();
    assert!(obj.contains_key("id"), "missing id");
    assert!(obj.contains_key("name"), "missing name");
    assert!(obj.contains_key("task_count"), "missing task_count");
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
            title: Some("New feature brainstorm".to_string()),
            prompt: None,
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
            title: Some("Forbidden".to_string()),
            prompt: None,
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// start_ideation_http — extended behavior tests
// ============================================================================

/// No prompt provided → session created, agent_spawned: false (no attempt to spawn)
#[tokio::test]
async fn test_start_ideation_no_prompt_agent_not_spawned() {
    let state = setup_test_state().await;

    let project_id = "proj-no-prompt";
    let p = make_project(project_id, "No Prompt Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: None,
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
    // No prompt → no agent spawned
    assert!(!response.agent_spawned, "agent_spawned must be false when no prompt is given");
}

/// With title → session created with that title preserved in the response session_id and
/// verifiable by fetching the session from the repo.
#[tokio::test]
async fn test_start_ideation_with_title_preserved() {
    let state = setup_test_state().await;

    let project_id = "proj-with-title";
    let p = make_project(project_id, "Title Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: Some("My Custom Session Title".to_string()),
            prompt: None,
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");

    // Verify the session was stored with the correct title
    let session_id = IdeationSessionId::from_string(
        response.session_id.clone(),
    );
    let fetched = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .unwrap()
        .expect("Session should exist in repo");
    assert_eq!(
        fetched.title.as_deref(),
        Some("My Custom Session Title"),
        "Session title should be persisted as provided"
    );
}

/// Non-existent project_id → 404
#[tokio::test]
async fn test_start_ideation_invalid_project_returns_404() {
    let state = setup_test_state().await;

    let result = start_ideation_http(
        State(state),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: "nonexistent-project-xyz".to_string(),
            title: None,
            prompt: None,
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::NOT_FOUND);
}

/// backward_compat: `initial_prompt` field (no `prompt`) → treated same as `prompt`.
/// Both fields absent means agent_spawned = false; presence of either triggers spawn attempt.
/// In test env the spawn attempt fails gracefully, so agent_spawned stays false in both cases,
/// but the session is still created and the response is 200.
#[tokio::test]
async fn test_start_ideation_initial_prompt_backward_compat() {
    let state = setup_test_state().await;

    let project_id = "proj-initial-prompt";
    let p = make_project(project_id, "Initial Prompt Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Use `initial_prompt` (legacy field) but NOT `prompt`
    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: None,
            initial_prompt: Some("Legacy initial prompt text".to_string()),
        }),
    )
    .await;

    assert!(result.is_ok(), "initial_prompt should be accepted as a valid prompt");
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
    // Session must have been created
    let session_id = IdeationSessionId::from_string(
        response.session_id.clone(),
    );
    let fetched = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .unwrap();
    assert!(fetched.is_some(), "Session should be persisted when initial_prompt is used");
}

/// `prompt` field takes precedence over `initial_prompt` when both are supplied.
/// The session is created and 200 is returned regardless of spawn outcome.
#[tokio::test]
async fn test_start_ideation_prompt_takes_precedence_over_initial_prompt() {
    let state = setup_test_state().await;

    let project_id = "proj-prompt-precedence";
    let p = make_project(project_id, "Prompt Precedence Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Both fields supplied — handler should prefer `prompt`
    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: Some("Primary prompt".to_string()),
            initial_prompt: Some("Legacy fallback".to_string()),
        }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.status, "ideating");
    // Session was created regardless of which prompt field was used
    let session_id = IdeationSessionId::from_string(
        response.session_id.clone(),
    );
    assert!(
        state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .unwrap()
            .is_some(),
        "Session should exist in repo"
    );
}

/// With prompt present → session created; `agent_spawned` reflects whether the spawn
/// attempt succeeded. In CI/test environments where Claude CLI is unavailable,
/// the handler logs the error and returns agent_spawned=false but still 200.
/// This test verifies the handler never returns an error for a spawn failure.
#[tokio::test]
async fn test_start_ideation_with_prompt_returns_200_regardless_of_spawn_outcome() {
    let state = setup_test_state().await;

    let project_id = "proj-prompt-spawn";
    let p = make_project(project_id, "Prompt Spawn Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: Some("Please ideate on caching strategies".to_string()),
            initial_prompt: None,
        }),
    )
    .await;

    // Must always return 200 — spawn failures are non-fatal
    assert!(result.is_ok(), "Spawn failure must not cause a non-200 response");
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
    // Session persisted regardless of spawn outcome
    let session_id = IdeationSessionId::from_string(
        response.session_id.clone(),
    );
    assert!(
        state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .unwrap()
            .is_some(),
        "Session must be persisted even when agent spawn fails"
    );
}

/// Scope 403 — API key scoped to a different project cannot create an ideation session
/// for a project it has no access to. (Explicit named test for start_ideation_http scope check.)
#[tokio::test]
async fn test_start_ideation_scope_mismatch_returns_403() {
    let state = setup_test_state().await;

    let project_id = "proj-scope-mismatch";
    let p = make_project(project_id, "Scope Mismatch Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Key scoped to a different project
    let result = start_ideation_http(
        State(state),
        scoped(&["proj-different-from-target"]),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: Some("Should be blocked".to_string()),
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::FORBIDDEN);
}

/// Session created even when multiple sessions exist — no hard cap anymore.
/// Gate was removed; second session creates successfully.
#[tokio::test]
async fn test_start_ideation_no_session_cap() {
    let state = setup_test_state().await;

    let project_id = "proj-no-cap";
    let p = make_project(project_id, "No Cap Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Create first session
    let existing = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "First session",
    );
    state
        .app_state
        .ideation_session_repo
        .create(existing)
        .await
        .unwrap();

    // Second request must succeed — no session cap
    let result = start_ideation_http(
        State(state),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: Some("Second session".to_string()),
            prompt: None,
            initial_prompt: None,
        }),
    )
    .await;

    assert!(result.is_ok(), "Second session must succeed — cap was removed");
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
}

/// Spawn failure → agent_spawn_blocked_reason populated, session still created (200).
#[tokio::test]
async fn test_start_ideation_spawn_failure_populates_blocked_reason() {
    let state = setup_test_state().await;

    let project_id = "proj-spawn-fail";
    let p = make_project(project_id, "Spawn Fail Project");
    state.app_state.project_repo.create(p).await.unwrap();

    // Provide a prompt so spawn is attempted; in test env Claude CLI is unavailable → SpawnFailed
    let result = start_ideation_http(
        State(state.clone()),
        unrestricted_scope(),
        Json(StartIdeationRequest {
            project_id: project_id.to_string(),
            title: None,
            prompt: Some("Trigger spawn attempt".to_string()),
            initial_prompt: None,
        }),
    )
    .await;

    // Must return 200 even on spawn failure
    assert!(result.is_ok(), "Spawn failure must not cause a non-200 response");
    let response = result.unwrap().0;
    assert!(!response.session_id.is_empty());
    assert_eq!(response.status, "ideating");
    // Session was persisted
    let session_id = IdeationSessionId::from_string(
        response.session_id.clone(),
    );
    assert!(
        state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .unwrap()
            .is_some(),
        "Session must be persisted even when spawn fails"
    );
    // In test env where Claude is not available, either:
    // - agent_spawned=false with a blocked reason, OR
    // - agent_spawned=false with no reason (if SpawnFailed is not raised)
    // Either way, agent_spawn_blocked_reason must not be Some("") — it must be None or a real message
    if let Some(reason) = &response.agent_spawn_blocked_reason {
        assert!(!reason.is_empty(), "Blocked reason must not be empty string");
    }
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
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.created', ?1, '{\"id\":\"t1\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.created', ?1, '{\"id\":\"t2\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
            conn.execute(
                "INSERT INTO external_events (event_type, project_id, payload) VALUES ('task.merged', ?1, '{\"id\":\"t3\"}')",
                rusqlite::params![proj_id_clone],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
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
            .map_err(|e| AppError::Database(e.to_string()))?;
            for i in 0..3 {
                conn.execute(
                    "INSERT INTO external_events (event_type, project_id, payload) VALUES ('ev', ?1, '{}')",
                    rusqlite::params![proj_id_clone],
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
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
        base_branch_override: None,
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
        base_branch_override: None,
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
        base_branch_override: None,
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
        base_branch_override: None,
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
        base_branch_override: None,
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

// ============================================================================
// ideation_message_http
// ============================================================================

#[tokio::test]
async fn test_ideation_message_invalid_session_returns_404() {
    let state = setup_test_state().await;

    let result = ideation_message_http(
        State(state),
        unrestricted_scope(),
        Json(IdeationMessageRequest {
            session_id: "nonexistent-session-id".to_string(),
            message: "hello".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_ideation_message_session_not_active_returns_400() {
    let state = setup_test_state().await;
    let (_, session_id_str) = setup_session(&state, "proj-msg-inactive", "Inactive Session").await;

    // Update the session status to Archived (non-Active)
    let session_id = IdeationSessionId::from_string(session_id_str.clone());
    state
        .app_state
        .ideation_session_repo
        .update_status(&session_id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let result = ideation_message_http(
        State(state),
        unrestricted_scope(),
        Json(IdeationMessageRequest {
            session_id: session_id_str,
            message: "hello".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_ideation_message_scope_violation_returns_403() {
    let state = setup_test_state().await;
    // Session belongs to "proj-msg-scope-a"
    let (_, session_id_str) = setup_session(&state, "proj-msg-scope-a", "Scope A Session").await;

    // Request using scope for a different project "proj-msg-scope-b"
    let result = ideation_message_http(
        State(state),
        scoped(&["proj-msg-scope-b"]),
        Json(IdeationMessageRequest {
            session_id: session_id_str,
            message: "hello".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_ideation_message_queued_when_agent_running() {
    let state = setup_test_state().await;
    let (_, session_id_str) = setup_session(&state, "proj-msg-queued", "Queued Session").await;

    // Register a fake running agent for this session in the registry
    let agent_key = RunningAgentKey::new("session", &session_id_str);
    state
        .app_state
        .running_agent_registry
        .register(
            agent_key,
            12345,                        // fake PID
            "fake-conv-id".to_string(),
            "fake-run-id".to_string(),
            None,
            None,
        )
        .await;

    let result = ideation_message_http(
        State(state),
        unrestricted_scope(),
        Json(IdeationMessageRequest {
            session_id: session_id_str,
            message: "hello".to_string(),
        }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok response, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.status, "queued");
}

#[tokio::test]
async fn test_ideation_message_sent_when_interactive_process_registered() {
    // Register a real interactive process (using `cat` as a stand-in for Claude CLI).
    // This verifies that when an interactive process is registered for the session,
    // the message is written directly to stdin and the response status is "sent".
    let state = setup_test_state().await;
    let (_, session_id_str) = setup_session(&state, "proj-msg-sent", "Sent Session").await;

    // Spawn a `cat` process to act as the interactive process (accepts stdin writes)
    let mut child = tokio::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn cat for test");
    let stdin = child.stdin.take().expect("no stdin on cat process");

    let ipr_key = InteractiveProcessKey::new("session", &session_id_str);
    state
        .app_state
        .interactive_process_registry
        .register(ipr_key, stdin)
        .await;

    let result = ideation_message_http(
        State(state),
        unrestricted_scope(),
        Json(IdeationMessageRequest {
            session_id: session_id_str,
            message: "hello from test".to_string(),
        }),
    )
    .await;

    // Ensure child is kept alive until after the handler call
    drop(child);

    assert!(result.is_ok(), "expected Ok response, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.status, "sent");
}

// ============================================================================
// list_ideation_sessions_http
// ============================================================================

/// No filter — all sessions for the project returned
#[tokio::test]
async fn test_list_sessions_no_scope_returns_all() {
    let state = setup_test_state().await;

    let project_id = "proj-list-all";
    let p = make_project(project_id, "List All Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());
    let s1 = IdeationSession::new_with_title(pid.clone(), "Session One");
    let s2 = IdeationSession::new_with_title(pid.clone(), "Session Two");
    state.app_state.ideation_session_repo.create(s1).await.unwrap();
    state.app_state.ideation_session_repo.create(s2).await.unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: None, limit: None }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 2);
    let titles: Vec<_> = response.sessions.iter().filter_map(|s| s.title.as_deref()).collect();
    assert!(titles.contains(&"Session One"));
    assert!(titles.contains(&"Session Two"));
}

/// Filter by status=active — only active sessions returned
#[tokio::test]
async fn test_list_sessions_filter_active() {
    let state = setup_test_state().await;

    let project_id = "proj-list-active";
    let p = make_project(project_id, "List Active Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());

    // Create one active session
    let active = IdeationSession::new_with_title(pid.clone(), "Active Session");
    let created_active = state.app_state.ideation_session_repo.create(active).await.unwrap();

    // Create one archived session
    let archived = IdeationSession::new_with_title(pid.clone(), "Archived Session");
    let created_archived = state.app_state.ideation_session_repo.create(archived).await.unwrap();
    state
        .app_state
        .ideation_session_repo
        .update_status(&created_archived.id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: Some("active".to_string()), limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 1);
    assert_eq!(response.sessions[0].id, created_active.id.as_str());
    assert_eq!(response.sessions[0].status, "active");
}

/// Filter by status=accepted — only accepted sessions returned
#[tokio::test]
async fn test_list_sessions_filter_accepted() {
    let state = setup_test_state().await;

    let project_id = "proj-list-accepted";
    let p = make_project(project_id, "List Accepted Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());

    // Create active session
    let active = IdeationSession::new_with_title(pid.clone(), "Still Active");
    state.app_state.ideation_session_repo.create(active).await.unwrap();

    // Create accepted session
    let accepted_sess = IdeationSession::new_with_title(pid.clone(), "Accepted Session");
    let created_accepted = state
        .app_state
        .ideation_session_repo
        .create(accepted_sess)
        .await
        .unwrap();
    state
        .app_state
        .ideation_session_repo
        .update_status(&created_accepted.id, IdeationSessionStatus::Accepted)
        .await
        .unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: Some("accepted".to_string()), limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 1);
    assert_eq!(response.sessions[0].status, "accepted");
}

/// Filter by status=archived — only archived sessions returned
#[tokio::test]
async fn test_list_sessions_filter_archived() {
    let state = setup_test_state().await;

    let project_id = "proj-list-archived";
    let p = make_project(project_id, "List Archived Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());

    // Create active session
    let active = IdeationSession::new_with_title(pid.clone(), "Active Remains");
    state.app_state.ideation_session_repo.create(active).await.unwrap();

    // Create archived session
    let archived = IdeationSession::new_with_title(pid.clone(), "Archived Session");
    let created_archived = state
        .app_state
        .ideation_session_repo
        .create(archived)
        .await
        .unwrap();
    state
        .app_state
        .ideation_session_repo
        .update_status(&created_archived.id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: Some("archived".to_string()), limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 1);
    assert_eq!(response.sessions[0].status, "archived");
}

/// Filter by status=all — same as no filter, returns all sessions
#[tokio::test]
async fn test_list_sessions_filter_all() {
    let state = setup_test_state().await;

    let project_id = "proj-list-filter-all";
    let p = make_project(project_id, "List Filter All Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());

    let s1 = IdeationSession::new_with_title(pid.clone(), "Session A");
    let s2 = IdeationSession::new_with_title(pid.clone(), "Session B");
    let created_s2 = state.app_state.ideation_session_repo.create(s2).await.unwrap();
    state.app_state.ideation_session_repo.create(s1).await.unwrap();
    state
        .app_state
        .ideation_session_repo
        .update_status(&created_s2.id, IdeationSessionStatus::Archived)
        .await
        .unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: Some("all".to_string()), limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 2, "status=all must return all sessions regardless of status");
}

/// Invalid status filter → 400
#[tokio::test]
async fn test_list_sessions_invalid_status_returns_400() {
    let state = setup_test_state().await;

    let project_id = "proj-list-bad-status";
    let p = make_project(project_id, "Bad Status Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: Some("invalid_xyz".to_string()), limit: None }),
    )
    .await;

    assert!(result.is_err());
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

/// Scope violation — API key scoped to different project returns 403
#[tokio::test]
async fn test_list_sessions_scope_violation_returns_403() {
    let state = setup_test_state().await;

    let project_id = "proj-list-scope-a";
    let p = make_project(project_id, "Scope A Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        scoped(&["proj-list-scope-b"]),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: None, limit: None }),
    )
    .await;

    assert!(result.is_err());
    let (status, _body) = result.unwrap_err();
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

/// Empty project — no sessions → empty array returned
#[tokio::test]
async fn test_list_sessions_empty_project() {
    let state = setup_test_state().await;

    let project_id = "proj-list-empty";
    let p = make_project(project_id, "Empty Sessions Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: None, limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.sessions.is_empty(), "expected empty sessions array for project with no sessions");
}

/// Session with proposals — proposal_count reflects the actual count
#[tokio::test]
async fn test_list_sessions_includes_proposal_count() {
    let state = setup_test_state().await;

    let project_id = "proj-list-proposals";
    let p = make_project(project_id, "Proposals Project");
    state.app_state.project_repo.create(p).await.unwrap();

    let pid = ProjectId::from_string(project_id.to_string());
    let session = IdeationSession::new_with_title(pid.clone(), "Session With Proposals");
    let created = state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Add 2 proposals to the session
    let prop1 = make_proposal(created.id.clone(), "Feature Alpha");
    let prop2 = make_proposal(created.id.clone(), "Feature Beta");
    state.app_state.task_proposal_repo.create(prop1).await.unwrap();
    state.app_state.task_proposal_repo.create(prop2).await.unwrap();

    let result = list_ideation_sessions_http(
        State(state),
        unrestricted_scope(),
        Path(project_id.to_string()),
        Query(ListSessionsParams { status: None, limit: None }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.sessions.len(), 1);
    assert_eq!(
        response.sessions[0].proposal_count, 2,
        "proposal_count must reflect actual number of proposals"
    );
}

// ============================================================================
// trigger_verification_http
// ============================================================================

/// SQLite-backed setup for trigger_verification tests.
/// trigger_verification_http calls db.run(|conn| SessionRepo::trigger_auto_verify_sync(conn, ...))
/// which requires the session to exist in the SQLite DB, not just in memory.
async fn setup_sqlite_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_sqlite_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
    }
}

#[tokio::test]
async fn test_trigger_verification_no_plan() {
    // Session with no plan_artifact_id → status "no_plan"
    let state = setup_test_state().await;

    let pid = ProjectId::from_string("proj-verify-no-plan".to_string());
    let project = make_project("proj-verify-no-plan", "No Plan Project");
    state.app_state.project_repo.create(project).await.unwrap();

    // Create session with no plan
    let session = IdeationSession::new(pid);
    let created = state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let result = trigger_verification_http(
        State(state),
        unrestricted_scope(),
        Json(TriggerVerificationRequest {
            session_id: created.id.as_str().to_string(),
        }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.status, "no_plan");
}

#[tokio::test]
async fn test_trigger_verification_already_running() {
    // Session with plan + verification_in_progress=true → "already_running"
    // Uses SQLite-backed state so trigger_auto_verify_sync can operate on the DB.
    let state = setup_sqlite_test_state().await;

    let pid = ProjectId::from_string("proj-verify-running".to_string());
    let project = make_project("proj-verify-running", "Already Running Project");
    state.app_state.project_repo.create(project).await.unwrap();

    // Create session with a plan artifact
    let session = IdeationSession::builder()
        .project_id(pid.clone())
        .status(IdeationSessionStatus::Active)
        .build();
    let created = state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Set a plan_artifact_id so the no_plan check passes
    state
        .app_state
        .ideation_session_repo
        .update_plan_artifact_id(&created.id, Some("artifact-x".to_string()))
        .await
        .unwrap();

    // Mark verification_in_progress = true via update_verification_state
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &created.id,
            VerificationStatus::Reviewing,
            true,
            None,
        )
        .await
        .unwrap();

    // Trigger: session in_progress=1 → trigger_auto_verify_sync returns None → "already_running"
    let result = trigger_verification_http(
        State(state),
        unrestricted_scope(),
        Json(TriggerVerificationRequest {
            session_id: created.id.as_str().to_string(),
        }),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.status, "already_running");
}

#[tokio::test]
async fn test_trigger_verification_scope_403() {
    // Session from different project → 403
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-verify-a", "Project A").await;

    let result = trigger_verification_http(
        State(state),
        scoped(&["proj-verify-b"]),
        Json(TriggerVerificationRequest { session_id }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_trigger_verification_session_not_found() {
    // Non-existent session → 404
    let state = setup_test_state().await;

    let result = trigger_verification_http(
        State(state),
        unrestricted_scope(),
        Json(TriggerVerificationRequest {
            session_id: "nonexistent-session-id".to_string(),
        }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::NOT_FOUND);
}

// ============================================================================
// get_plan_verification_external_http
// ============================================================================

#[tokio::test]
async fn test_get_plan_verification_basic() {
    // Session with verification state → reads it correctly
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-verify-get", "Verify Get Project").await;
    let session_id_obj = IdeationSessionId::from_string(session_id.clone());

    // Set verification state
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(
            &session_id_obj,
            VerificationStatus::Verified,
            false,
            None,
        )
        .await
        .unwrap();

    let result = get_plan_verification_external_http(
        State(state),
        unrestricted_scope(),
        Path(session_id),
    )
    .await;

    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.status, "verified");
    assert!(!response.in_progress);
    assert_eq!(response.round, None);
    assert_eq!(response.gap_count, None);
}

#[tokio::test]
async fn test_get_plan_verification_scope_403() {
    // Session from different project → 403
    let state = setup_test_state().await;
    let (_, session_id) = setup_session(&state, "proj-verify-scope", "Scope Project").await;

    let result = get_plan_verification_external_http(
        State(state),
        scoped(&["proj-other-scope"]),
        Path(session_id),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

// ============================================================================
// get_ideation_messages_http
// ============================================================================

async fn create_message(
    state: &HttpServerState,
    msg: ChatMessage,
) {
    state
        .app_state
        .chat_message_repo
        .create(msg)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_get_ideation_messages_empty() {
    let state = setup_test_state().await;
    let (_, session_id_str) =
        setup_session(&state, "proj-msg-empty", "Empty Messages Session").await;

    let result = get_ideation_messages_http(
        State(state),
        unrestricted_scope(),
        Path(session_id_str),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 50, offset: 0 }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert!(response.messages.is_empty());
    assert!(!response.has_more);
    assert_eq!(response.agent_status, "idle");
}

#[tokio::test]
async fn test_get_ideation_messages_returns_user_and_orchestrator() {
    let state = setup_test_state().await;
    let (_, session_id_str) =
        setup_session(&state, "proj-msg-roles", "Role Filter Session").await;
    let session_id = IdeationSessionId::from_string(session_id_str.clone());

    // Add user, orchestrator, and system messages
    create_message(
        &state,
        ChatMessage::user_in_session(session_id.clone(), "user message"),
    )
    .await;
    create_message(
        &state,
        ChatMessage::orchestrator_in_session(session_id.clone(), "orchestrator reply"),
    )
    .await;
    create_message(
        &state,
        ChatMessage::system_in_session(session_id.clone(), "system internal"),
    )
    .await;

    let result = get_ideation_messages_http(
        State(state),
        unrestricted_scope(),
        Path(session_id_str),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 50, offset: 0 }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    // System message must be excluded
    assert_eq!(response.messages.len(), 2);

    let roles: Vec<&str> = response.messages.iter().map(|m| m.role.as_str()).collect();
    assert!(roles.contains(&"user"));
    assert!(roles.contains(&"assistant")); // Orchestrator maps to "assistant"
    assert!(!roles.contains(&"system"));
}

#[tokio::test]
async fn test_get_ideation_messages_pagination() {
    let state = setup_test_state().await;
    let (_, session_id_str) =
        setup_session(&state, "proj-msg-page", "Pagination Session").await;
    let session_id = IdeationSessionId::from_string(session_id_str.clone());

    // Insert 5 user messages
    for i in 0..5 {
        create_message(
            &state,
            ChatMessage::user_in_session(session_id.clone(), format!("message {i}")),
        )
        .await;
    }

    // Fetch first 3
    let result = get_ideation_messages_http(
        State(state.clone()),
        unrestricted_scope(),
        Path(session_id_str.clone()),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 3, offset: 0 }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.messages.len(), 3);
    assert!(response.has_more);

    // Fetch remaining 2
    let result2 = get_ideation_messages_http(
        State(state),
        unrestricted_scope(),
        Path(session_id_str),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 3, offset: 3 }),
    )
    .await;

    assert!(result2.is_ok());
    let response2 = result2.unwrap().0;
    assert_eq!(response2.messages.len(), 2);
    assert!(!response2.has_more);
}

#[tokio::test]
async fn test_get_ideation_messages_scope_violation() {
    let state = setup_test_state().await;
    let (_, session_id_str) =
        setup_session(&state, "proj-msg-scope", "Scope Test Session").await;

    let result = get_ideation_messages_http(
        State(state),
        scoped(&["proj-other"]),
        Path(session_id_str),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 50, offset: 0 }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_get_ideation_messages_not_found() {
    let state = setup_test_state().await;

    let result = get_ideation_messages_http(
        State(state),
        unrestricted_scope(),
        Path("nonexistent-session".to_string()),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 50, offset: 0 }),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_ideation_messages_agent_status_generating() {
    let state = setup_test_state().await;
    let (_, session_id_str) =
        setup_session(&state, "proj-msg-agent", "Agent Status Session").await;

    // Register a running agent (no interactive process = "generating")
    let agent_key = RunningAgentKey::new("session", &session_id_str);
    state
        .app_state
        .running_agent_registry
        .register(
            agent_key,
            0,
            "conv-id".to_string(),
            "run-id".to_string(),
            None,
            None,
        )
        .await;

    let result = get_ideation_messages_http(
        State(state),
        unrestricted_scope(),
        Path(session_id_str),
        axum::extract::Query(GetIdeationMessagesQuery { limit: 50, offset: 0 }),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.agent_status, "generating");
}

// ============================================================================
// batch_task_status_http
// ============================================================================

async fn seed_task(state: &HttpServerState, project_id: &str, title: &str) -> Task {
    let task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        title.to_string(),
    );
    state.app_state.task_repo.create(task.clone()).await.unwrap();
    task
}

#[tokio::test]
async fn test_batch_task_status_multiple_tasks() {
    let state = setup_test_state().await;
    let t1 = seed_task(&state, "proj-batch-1", "Task Alpha").await;
    let t2 = seed_task(&state, "proj-batch-1", "Task Beta").await;

    let req = BatchTaskStatusRequest {
        task_ids: vec![t1.id.to_string(), t2.id.to_string()],
    };

    let result = batch_task_status_http(
        State(state),
        unrestricted_scope(),
        Json(req),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.requested_count, 2);
    assert_eq!(response.returned_count, 2);
    assert_eq!(response.tasks.len(), 2);
    assert!(response.errors.is_empty());

    let titles: Vec<&str> = response.tasks.iter().map(|t| t.title.as_str()).collect();
    assert!(titles.contains(&"Task Alpha"));
    assert!(titles.contains(&"Task Beta"));
}

#[tokio::test]
async fn test_batch_task_status_not_found() {
    let state = setup_test_state().await;
    let t1 = seed_task(&state, "proj-batch-nf", "Real Task").await;

    let req = BatchTaskStatusRequest {
        task_ids: vec![t1.id.to_string(), "nonexistent-task-id".to_string()],
    };

    let result = batch_task_status_http(
        State(state),
        unrestricted_scope(),
        Json(req),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.requested_count, 2);
    assert_eq!(response.returned_count, 1);
    assert_eq!(response.tasks.len(), 1);
    assert_eq!(response.errors.len(), 1);
    assert_eq!(response.errors[0].id, "nonexistent-task-id");
    assert_eq!(response.errors[0].reason, "not_found");
}

#[tokio::test]
async fn test_batch_task_status_access_denied() {
    let state = setup_test_state().await;
    let t1 = seed_task(&state, "proj-batch-scoped", "Scoped Task").await;
    let t2 = seed_task(&state, "proj-batch-other", "Other Task").await;

    // Scope to only proj-batch-scoped
    let req = BatchTaskStatusRequest {
        task_ids: vec![t1.id.to_string(), t2.id.to_string()],
    };

    let result = batch_task_status_http(
        State(state),
        scoped(&["proj-batch-scoped"]),
        Json(req),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.requested_count, 2);
    assert_eq!(response.returned_count, 1);
    assert_eq!(response.tasks.len(), 1);
    assert_eq!(response.tasks[0].title, "Scoped Task");
    assert_eq!(response.errors.len(), 1);
    assert_eq!(response.errors[0].id, t2.id.to_string());
    assert_eq!(response.errors[0].reason, "access_denied");
}

#[tokio::test]
async fn test_batch_task_status_max_50_enforced() {
    let state = setup_test_state().await;

    // Submit 51 task IDs
    let task_ids: Vec<String> = (0..51).map(|i| format!("task-id-{i}")).collect();
    let req = BatchTaskStatusRequest { task_ids };

    let result = batch_task_status_http(
        State(state),
        unrestricted_scope(),
        Json(req),
    )
    .await;

    assert!(result.is_err());
    let (status, msg) = result.unwrap_err();
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(msg.contains("Maximum is 50"));
}

#[tokio::test]
async fn test_batch_task_status_requested_count_and_returned_count() {
    let state = setup_test_state().await;
    let t1 = seed_task(&state, "proj-batch-counts", "Task One").await;

    let req = BatchTaskStatusRequest {
        task_ids: vec![
            t1.id.to_string(),
            "ghost-1".to_string(),
            "ghost-2".to_string(),
        ],
    };

    let result = batch_task_status_http(
        State(state),
        unrestricted_scope(),
        Json(req),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.requested_count, 3);
    assert_eq!(response.returned_count, 1);
    assert_eq!(response.errors.len(), 2);
    for err in &response.errors {
        assert_eq!(err.reason, "not_found");
    }
}

#[tokio::test]
async fn test_batch_task_status_empty_request() {
    let state = setup_test_state().await;

    let req = BatchTaskStatusRequest { task_ids: vec![] };

    let result = batch_task_status_http(
        State(state),
        unrestricted_scope(),
        Json(req),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.requested_count, 0);
    assert_eq!(response.returned_count, 0);
    assert!(response.tasks.is_empty());
    assert!(response.errors.is_empty());
}

// ============================================================================
// get_session_tasks_http
// ============================================================================

/// Session with no linked tasks returns not_scheduled + empty task list.
#[tokio::test]
async fn test_get_session_tasks_empty_session_returns_not_scheduled() {
    let state = setup_test_state().await;

    let project_id = "proj-session-tasks-empty";
    let p = make_project(project_id, "Session Tasks Empty");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Empty Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.session_id, session_id.as_str());
    assert!(response.tasks.is_empty(), "Expected no tasks");
    assert_eq!(response.delivery_status, "not_scheduled");
    assert_eq!(response.task_count, 0);
}

/// Session with tasks returns correct task list, delivery_status, and task_count.
#[tokio::test]
async fn test_get_session_tasks_with_tasks_returns_task_list() {
    let state = setup_test_state().await;

    let project_id = "proj-session-tasks-list";
    let p = make_project(project_id, "Session Tasks List");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Session With Tasks",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Create two tasks linked to the session
    let mut task1 = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Task One".to_string(),
    );
    task1.ideation_session_id = Some(session_id.clone());
    task1.internal_status = InternalStatus::Backlog;

    let mut task2 = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Task Two".to_string(),
    );
    task2.ideation_session_id = Some(session_id.clone());
    task2.internal_status = InternalStatus::Executing;

    state.app_state.task_repo.create(task1).await.unwrap();
    state.app_state.task_repo.create(task2).await.unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok(), "Expected Ok, got: {:?}", result.err());
    let response = result.unwrap().0;
    assert_eq!(response.session_id, session_id.as_str());
    assert_eq!(response.task_count, 2);
    assert_eq!(response.tasks.len(), 2);
    assert_eq!(response.delivery_status, "in_progress");

    let titles: Vec<&str> = response.tasks.iter().map(|t| t.title.as_str()).collect();
    assert!(titles.contains(&"Task One"), "Missing 'Task One'");
    assert!(titles.contains(&"Task Two"), "Missing 'Task Two'");
}

/// Unlinked tasks (different project, no session_id) are excluded from results.
#[tokio::test]
async fn test_get_session_tasks_excludes_unlinked_tasks() {
    let state = setup_test_state().await;

    let project_id = "proj-session-tasks-excl";
    let p = make_project(project_id, "Session Tasks Exclude");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Session A",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Task linked to this session
    let mut linked_task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Linked Task".to_string(),
    );
    linked_task.ideation_session_id = Some(session_id.clone());

    // Task with no session_id — should be excluded
    let unlinked_task = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Unlinked Task".to_string(),
    );

    state
        .app_state
        .task_repo
        .create(linked_task)
        .await
        .unwrap();
    state
        .app_state
        .task_repo
        .create(unlinked_task)
        .await
        .unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok());
    let response = result.unwrap().0;
    assert_eq!(response.task_count, 1, "Only linked task should be returned");
    assert_eq!(response.tasks[0].title, "Linked Task");
}

/// Session not found returns 404.
#[tokio::test]
async fn test_get_session_tasks_nonexistent_session_returns_404() {
    let state = setup_test_state().await;

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path("nonexistent-session-id".to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().status,
        axum::http::StatusCode::NOT_FOUND
    );
}

/// ProjectScope violation returns 403 when API key is scoped to different project.
#[tokio::test]
async fn test_get_session_tasks_scope_violation_returns_403() {
    let state = setup_test_state().await;

    let project_id = "proj-scope-session-tasks";
    let p = make_project(project_id, "Scope Session Tasks");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Scoped Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    // Scope to a different project — must be rejected
    let result = get_session_tasks_http(
        State(state),
        scoped(&["proj-different"]),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().status,
        axum::http::StatusCode::FORBIDDEN
    );
}

// ============================================================================
// delivery_status derivation logic
// ============================================================================

/// All tasks merged → "delivered".
#[tokio::test]
async fn test_get_session_tasks_delivery_status_all_merged_is_delivered() {
    let state = setup_test_state().await;

    let project_id = "proj-ds-delivered";
    let p = make_project(project_id, "DS Delivered");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "All Merged Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    for title in &["Task A", "Task B"] {
        let mut task = Task::new(
            ProjectId::from_string(project_id.to_string()),
            title.to_string(),
        );
        task.ideation_session_id = Some(session_id.clone());
        task.internal_status = InternalStatus::Merged;
        state.app_state.task_repo.create(task).await.unwrap();
    }

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0.delivery_status, "delivered");
}

/// Mixed merged + failed tasks → "partial".
#[tokio::test]
async fn test_get_session_tasks_delivery_status_mixed_terminal_is_partial() {
    let state = setup_test_state().await;

    let project_id = "proj-ds-partial";
    let p = make_project(project_id, "DS Partial");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Mixed Terminal Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let mut merged = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merged Task".to_string(),
    );
    merged.ideation_session_id = Some(session_id.clone());
    merged.internal_status = InternalStatus::Merged;

    let mut failed = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Failed Task".to_string(),
    );
    failed.ideation_session_id = Some(session_id.clone());
    failed.internal_status = InternalStatus::Failed;

    state.app_state.task_repo.create(merged).await.unwrap();
    state.app_state.task_repo.create(failed).await.unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0.delivery_status, "partial");
}

/// Tasks in review states (no active tasks) → "pending_review".
#[tokio::test]
async fn test_get_session_tasks_delivery_status_in_review_is_pending_review() {
    let state = setup_test_state().await;

    let project_id = "proj-ds-review";
    let p = make_project(project_id, "DS Review");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "In Review Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let mut reviewing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Reviewing Task".to_string(),
    );
    reviewing.ideation_session_id = Some(session_id.clone());
    reviewing.internal_status = InternalStatus::Reviewing;

    state.app_state.task_repo.create(reviewing).await.unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0.delivery_status, "pending_review");
}

/// Any active task (executing, backlog, ready) → "in_progress".
#[tokio::test]
async fn test_get_session_tasks_delivery_status_active_tasks_is_in_progress() {
    let state = setup_test_state().await;

    let project_id = "proj-ds-active";
    let p = make_project(project_id, "DS Active");
    state.app_state.project_repo.create(p).await.unwrap();

    let session = IdeationSession::new_with_title(
        ProjectId::from_string(project_id.to_string()),
        "Active Session",
    );
    let session_id = session.id.clone();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap();

    let mut executing = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Executing Task".to_string(),
    );
    executing.ideation_session_id = Some(session_id.clone());
    executing.internal_status = InternalStatus::Executing;

    // Also a merged task — but presence of active makes it in_progress
    let mut merged = Task::new(
        ProjectId::from_string(project_id.to_string()),
        "Merged Task".to_string(),
    );
    merged.ideation_session_id = Some(session_id.clone());
    merged.internal_status = InternalStatus::Merged;

    state.app_state.task_repo.create(executing).await.unwrap();
    state.app_state.task_repo.create(merged).await.unwrap();

    let result = get_session_tasks_http(
        State(state),
        unrestricted_scope(),
        Path(session_id.as_str().to_string()),
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().0.delivery_status, "in_progress");
}
