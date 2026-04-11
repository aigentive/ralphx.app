use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{extract::State, Json};
use ralphx_lib::application::{AppState, TeamService, TeamStateTracker};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::agents::AgentHarnessKind;
use ralphx_lib::domain::entities::{IdeationSession, IdeationSessionId, Project};
use ralphx_lib::http_server::handlers::{cancel_delegate, start_delegate, wait_delegate};
use ralphx_lib::http_server::types::{
    DelegateCancelRequest, DelegateStartRequest, DelegateWaitRequest, HttpServerState,
};
use ralphx_lib::infrastructure::agents::mock::{MockAgenticClient, MockCallType};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri has a repo-root parent")
        .to_path_buf()
}

fn build_state(app_state: Arc<AppState>) -> HttpServerState {
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(TeamService::new_without_events(Arc::new(tracker.clone())));
    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
        delegation_service: Default::default(),
    }
}

async fn create_parent_session(state: &HttpServerState) -> IdeationSession {
    let project = Project::new(
        "Delegation Test Project".to_string(),
        repo_root().display().to_string(),
    );
    let project_id = project.id.clone();
    state.app_state.project_repo.create(project).await.unwrap();

    let session = IdeationSession::builder()
        .project_id(project_id)
        .title("Delegation Parent")
        .cross_project_checked(true)
        .build();
    state
        .app_state
        .ideation_session_repo
        .create(session)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_delegate_start_creates_child_session_and_completes_with_mock_client() {
    let codex_mock = Arc::new(MockAgenticClient::new());
    let app_state = Arc::new(
        AppState::new_sqlite_test()
            .with_harness_agent_client(AgentHarnessKind::Codex, codex_mock.clone()),
    );
    let state = build_state(app_state);
    let parent = create_parent_session(&state).await;

    let start = start_delegate(
        State(state.clone()),
        Json(DelegateStartRequest {
            parent_session_id: parent.id.as_str().to_string(),
            parent_turn_id: Some("turn-42".to_string()),
            parent_message_id: Some("msg-99".to_string()),
            child_session_id: None,
            agent_name: "ralphx-ideation".to_string(),
            prompt: "Review the proposal set and summarize the main implementation risks."
                .to_string(),
            title: Some("Delegated Risk Review".to_string()),
            inherit_context: true,
            harness: Some(AgentHarnessKind::Codex),
            model: None,
            logical_effort: None,
            approval_policy: None,
            sandbox_mode: None,
        }),
    )
    .await
    .unwrap()
    .0;

    assert_eq!(start.parent_session_id, parent.id.as_str());
    assert_eq!(start.parent_turn_id.as_deref(), Some("turn-42"));
    assert_eq!(start.parent_message_id.as_deref(), Some("msg-99"));
    assert_eq!(start.agent_name, "ralphx-ideation");
    assert_eq!(start.harness, "codex");
    assert_eq!(start.status, "running");
    assert_ne!(start.child_session_id, parent.id.as_str());
    assert_eq!(start.history.len(), 1);
    assert_eq!(start.history[0].status, "running");

    let child_id = IdeationSessionId::from_string(start.child_session_id.clone());
    let child = state
        .app_state
        .ideation_session_repo
        .get_by_id(&child_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        child.parent_session_id.as_ref().map(|id| id.as_str()),
        Some(parent.id.as_str())
    );

    tokio::time::sleep(Duration::from_millis(25)).await;

    let waited = wait_delegate(
        State(state),
        Json(DelegateWaitRequest {
            job_id: start.job_id.clone(),
            include_child_status: Some(true),
            include_messages: Some(false),
            message_limit: None,
        }),
    )
    .await
    .unwrap()
    .0;

    assert_eq!(waited.job_id, start.job_id);
    assert_eq!(waited.status, "completed");
    assert_eq!(waited.content.as_deref(), Some("MOCK_COMPLETION"));
    assert!(waited.error.is_none());
    assert_eq!(waited.parent_turn_id.as_deref(), Some("turn-42"));
    assert_eq!(waited.parent_message_id.as_deref(), Some("msg-99"));
    assert_eq!(
        waited.history.iter().map(|entry| entry.status.as_str()).collect::<Vec<_>>(),
        vec!["running", "completed"]
    );
    let child_status = waited.child_status.expect("child status should be hydrated");
    assert_eq!(child_status.session.id, waited.child_session_id);
    assert_eq!(
        child_status.session.parent_session_id.as_deref(),
        Some(parent.id.as_str())
    );
    assert_eq!(child_status.agent_state.estimated_status, "idle");
    assert!(child_status.recent_messages.is_none());

    let spawn_calls = codex_mock.get_spawn_calls().await;
    assert_eq!(spawn_calls.len(), 1);
    match &spawn_calls[0].call_type {
        MockCallType::Spawn { prompt, .. } => {
            assert!(prompt.contains("Delegated Risk Review") || prompt.contains("Delegated task:"));
            assert!(prompt.contains(parent.id.as_str()));
            assert!(prompt.contains("Parent turn id: `turn-42`"));
            assert!(prompt.contains("Parent message id: `msg-99`"));
            assert!(prompt.contains(waited.child_session_id.as_str()));
            assert!(prompt.contains("summarize the main implementation risks"));
        }
        other => panic!("expected spawn call, got {other:?}"),
    }
}

#[tokio::test]
async fn test_delegate_start_rejects_unknown_agent_name() {
    let state = build_state(Arc::new(AppState::new_sqlite_test()));
    let parent = create_parent_session(&state).await;

    let error = start_delegate(
        State(state),
        Json(DelegateStartRequest {
            parent_session_id: parent.id.as_str().to_string(),
            parent_turn_id: Some("turn-bad".to_string()),
            parent_message_id: Some("msg-bad".to_string()),
            child_session_id: None,
            agent_name: "ralphx-does-not-exist".to_string(),
            prompt: "noop".to_string(),
            title: None,
            inherit_context: true,
            harness: Some(AgentHarnessKind::Codex),
            model: None,
            logical_effort: None,
            approval_policy: None,
            sandbox_mode: None,
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::BAD_REQUEST);
    assert!(
        error.1 .0["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Unknown canonical agent")
    );
}

#[tokio::test]
async fn test_delegate_cancel_rejects_unknown_job() {
    let state = build_state(Arc::new(AppState::new_sqlite_test()));

    let error = cancel_delegate(
        State(state),
        Json(DelegateCancelRequest {
            job_id: "missing-job".to_string(),
        }),
    )
    .await
    .unwrap_err();

    assert_eq!(error.0, axum::http::StatusCode::NOT_FOUND);
}
