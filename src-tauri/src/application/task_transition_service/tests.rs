use super::*;
use crate::application::AppState;
use crate::domain::entities::{InternalStatus, Project, Task};

#[test]
fn test_tauri_event_emitter_creation() {
    let emitter: TauriEventEmitter<tauri::Wry> = TauriEventEmitter::new(None);
    assert!(emitter.app_handle.is_none());
}

#[test]
fn test_logging_notifier() {
    let _notifier = LoggingNotifier;
    // Just verify it can be created
}

#[test]
fn test_no_op_review_starter() {
    let _starter = NoOpReviewStarter;
    // Just verify it can be created
}

fn build_dependency_manager(app_state: &AppState) -> RepoBackedDependencyManager<tauri::Wry> {
    RepoBackedDependencyManager::new(
        Arc::clone(&app_state.task_dependency_repo),
        Arc::clone(&app_state.task_repo),
        None,
    )
}

#[tokio::test]
async fn test_dependency_manager_treats_paused_blocker_as_incomplete() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Paused Blocker".to_string());
    blocker.internal_status = InternalStatus::Paused;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut blocked = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&blocked.id, &blocker.id)
        .await
        .unwrap();

    let has_blockers = manager.has_unresolved_blockers(blocked.id.as_str()).await;
    assert!(
        has_blockers,
        "Paused blockers should be treated as unresolved"
    );
}

#[tokio::test]
async fn test_dependency_manager_treats_stopped_blocker_as_incomplete() {
    let app_state = AppState::new_test();
    let manager = build_dependency_manager(&app_state);

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());

    let mut blocker = Task::new(project.id.clone(), "Stopped Blocker".to_string());
    blocker.internal_status = InternalStatus::Stopped;
    app_state.task_repo.create(blocker.clone()).await.unwrap();

    let mut blocked = Task::new(project.id.clone(), "Blocked Task".to_string());
    blocked.internal_status = InternalStatus::Blocked;
    app_state.task_repo.create(blocked.clone()).await.unwrap();

    app_state
        .task_dependency_repo
        .add_dependency(&blocked.id, &blocker.id)
        .await
        .unwrap();

    let has_blockers = manager.has_unresolved_blockers(blocked.id.as_str()).await;
    assert!(
        has_blockers,
        "Stopped blockers should be treated as unresolved"
    );
}
