use ralphx_lib::application::chat_service::{ChatService, ClaudeChatService, SendMessageOptions};
use ralphx_lib::application::AppState;
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::domain::entities::{ChatContextType, InternalStatus, Project, Task};
use ralphx_lib::domain::services::RunningAgentKey;
use ralphx_lib::http_server::types::HttpServerState;
use std::sync::Arc;

async fn setup_test_state() -> HttpServerState {
    HttpServerState {
        app_state: Arc::new(AppState::new_test()),
        execution_state: Arc::new(ExecutionState::new()),
        team_tracker: ralphx_lib::application::TeamStateTracker::new(),
        team_service: Arc::new(ralphx_lib::application::TeamService::new_without_events(
            Arc::new(ralphx_lib::application::TeamStateTracker::new()),
        )),
    }
}

fn build_chat_service(state: &HttpServerState) -> ClaudeChatService<tauri::Wry> {
    let app = &state.app_state;
    ClaudeChatService::new(
        Arc::clone(&app.chat_message_repo),
        Arc::clone(&app.chat_attachment_repo),
        Arc::clone(&app.artifact_repo),
        Arc::clone(&app.chat_conversation_repo),
        Arc::clone(&app.agent_run_repo),
        Arc::clone(&app.project_repo),
        Arc::clone(&app.task_repo),
        Arc::clone(&app.task_dependency_repo),
        Arc::clone(&app.ideation_session_repo),
        Arc::clone(&app.activity_event_repo),
        Arc::clone(&app.message_queue),
        Arc::clone(&app.running_agent_registry),
        Arc::clone(&app.memory_event_repo),
    )
    .with_execution_state(Arc::clone(&state.execution_state))
    .with_execution_settings_repo(Arc::clone(&app.execution_settings_repo))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry))
}

async fn create_task(state: &HttpServerState, status: InternalStatus) -> String {
    let project = Project::new("Pause Flow Project".to_string(), "/tmp/pause-flow".to_string());
    let project_id = project.id.clone();
    state.app_state.project_repo.create(project).await.unwrap();

    let mut task = Task::new(project_id, "Pause Flow Task".to_string());
    task.internal_status = status;
    let task = state.app_state.task_repo.create(task).await.unwrap();
    task.id.as_str().to_string()
}

#[tokio::test]
async fn test_paused_task_execution_send_is_queued_not_spawned() {
    let state = setup_test_state().await;
    let task_id = create_task(&state, InternalStatus::Executing).await;
    state.execution_state.pause();

    let result = build_chat_service(&state)
        .send_message(
            ChatContextType::TaskExecution,
            &task_id,
            "Queue worker follow-up during pause",
            SendMessageOptions::default(),
        )
        .await
        .expect("paused task execution send should queue");

    assert!(result.was_queued);
    assert_eq!(
        state
            .app_state
            .message_queue
            .get_queued(ChatContextType::TaskExecution, &task_id)
            .len(),
        1
    );
    let key = RunningAgentKey::new("task_execution", &task_id);
    assert!(!state.app_state.running_agent_registry.is_running(&key).await);
}

#[tokio::test]
async fn test_paused_review_send_is_queued_not_spawned() {
    let state = setup_test_state().await;
    let task_id = create_task(&state, InternalStatus::Reviewing).await;
    state.execution_state.pause();

    let result = build_chat_service(&state)
        .send_message(
            ChatContextType::Review,
            &task_id,
            "Queue review follow-up during pause",
            SendMessageOptions::default(),
        )
        .await
        .expect("paused review send should queue");

    assert!(result.was_queued);
    assert_eq!(
        state
            .app_state
            .message_queue
            .get_queued(ChatContextType::Review, &task_id)
            .len(),
        1
    );
    let key = RunningAgentKey::new("review", &task_id);
    assert!(!state.app_state.running_agent_registry.is_running(&key).await);
}

#[tokio::test]
async fn test_paused_merge_send_is_queued_not_spawned() {
    let state = setup_test_state().await;
    let task_id = create_task(&state, InternalStatus::Merging).await;
    state.execution_state.pause();

    let result = build_chat_service(&state)
        .send_message(
            ChatContextType::Merge,
            &task_id,
            "Queue merge follow-up during pause",
            SendMessageOptions::default(),
        )
        .await
        .expect("paused merge send should queue");

    assert!(result.was_queued);
    assert_eq!(
        state
            .app_state
            .message_queue
            .get_queued(ChatContextType::Merge, &task_id)
            .len(),
        1
    );
    let key = RunningAgentKey::new("merge", &task_id);
    assert!(!state.app_state.running_agent_registry.is_running(&key).await);
}
