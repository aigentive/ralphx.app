use super::*;
use crate::domain::entities::{InternalStatus, Project, Task};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::memory::MemoryProjectRepository;

#[tokio::test]
async fn test_direct_cleanup_stops_task_using_current_repo_state_not_stale_snapshot() {
    let task_repo = Arc::new(crate::infrastructure::memory::MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());
    let running_registry = Arc::new(crate::domain::services::MemoryRunningAgentRegistry::new());

    let project = Project::new("Test Project".to_string(), "/tmp/test-project".to_string());
    project_repo.create(project.clone()).await.unwrap();

    let mut stored_task = Task::new(project.id.clone(), "Leaked Task".to_string());
    stored_task.internal_status = InternalStatus::Executing;
    let stored_task = task_repo.create(stored_task).await.unwrap();

    let mut stale_snapshot = stored_task.clone();
    stale_snapshot.internal_status = InternalStatus::Ready;

    running_registry
        .register(
            RunningAgentKey::new("task_execution", stored_task.id.as_str()),
            424242,
            "conv-stale".to_string(),
            "run-stale".to_string(),
            None,
            None,
        )
        .await;

    let service = TaskCleanupService::new(
        Arc::clone(&task_repo) as Arc<dyn crate::domain::repositories::TaskRepository>,
        Arc::clone(&project_repo) as Arc<dyn crate::domain::repositories::ProjectRepository>,
        Arc::clone(&running_registry) as Arc<dyn crate::domain::services::RunningAgentRegistry>,
        None,
    );

    let report = service
        .cleanup_tasks(&[stale_snapshot], StopMode::DirectStop, false)
        .await;

    assert_eq!(report.errors.len(), 0, "cleanup should not report errors");

    let key = RunningAgentKey::new("task_execution", stored_task.id.as_str());
    assert!(
        !running_registry.is_running(&key).await,
        "direct cleanup must stop the live task_execution context even when the input snapshot is stale"
    );

    let archived = task_repo
        .get_by_id(&stored_task.id)
        .await
        .unwrap()
        .expect("task should still exist after archive");
    assert!(
        archived.archived_at.is_some(),
        "cleanup should archive the task after stopping its live context"
    );
}
