use super::*;
use crate::domain::entities::research::{ResearchBrief, ResearchDepthPreset};

fn create_test_process() -> ResearchProcess {
    let brief = ResearchBrief::new("What architecture should we use?");
    ResearchProcess::new("Test Research", brief, "deep-researcher")
        .with_preset(ResearchDepthPreset::Standard)
}

fn create_running_process() -> ResearchProcess {
    let brief = ResearchBrief::new("Running question");
    let mut process = ResearchProcess::new("Running Research", brief, "deep-researcher");
    process.start();
    process
}

#[tokio::test]
async fn test_create_and_get_process() {
    let repo = MemoryProcessRepository::new();
    let process = create_test_process();

    repo.create(process.clone()).await.unwrap();
    let found = repo.get_by_id(&process.id).await.unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, process.id);
}

#[tokio::test]
async fn test_get_all_processes() {
    let repo = MemoryProcessRepository::new();
    let process1 = create_test_process();
    let process2 = create_running_process();

    repo.create(process1).await.unwrap();
    repo.create(process2).await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_by_status() {
    let repo = MemoryProcessRepository::new();
    let pending = create_test_process();
    let running = create_running_process();

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();

    let pending_processes = repo
        .get_by_status(ResearchProcessStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending_processes.len(), 1);

    let running_processes = repo
        .get_by_status(ResearchProcessStatus::Running)
        .await
        .unwrap();
    assert_eq!(running_processes.len(), 1);
}

#[tokio::test]
async fn test_get_active() {
    let repo = MemoryProcessRepository::new();
    let pending = create_test_process();
    let running = create_running_process();

    repo.create(pending).await.unwrap();
    repo.create(running).await.unwrap();

    let active = repo.get_active().await.unwrap();
    assert_eq!(active.len(), 2); // Both pending and running are active
}

#[tokio::test]
async fn test_complete_process() {
    let repo = MemoryProcessRepository::new();
    let process = create_running_process();

    repo.create(process.clone()).await.unwrap();
    repo.complete(&process.id).await.unwrap();

    let found = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Completed);
}

#[tokio::test]
async fn test_fail_process() {
    let repo = MemoryProcessRepository::new();
    let process = create_running_process();

    repo.create(process.clone()).await.unwrap();
    repo.fail(&process.id, "Test error").await.unwrap();

    let found = repo.get_by_id(&process.id).await.unwrap().unwrap();
    assert_eq!(found.status(), ResearchProcessStatus::Failed);
}

#[tokio::test]
async fn test_delete_process() {
    let repo = MemoryProcessRepository::new();
    let process = create_test_process();

    repo.create(process.clone()).await.unwrap();
    repo.delete(&process.id).await.unwrap();
    let found = repo.get_by_id(&process.id).await.unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn test_exists() {
    let repo = MemoryProcessRepository::new();
    let process = create_test_process();

    assert!(!repo.exists(&process.id).await.unwrap());
    repo.create(process.clone()).await.unwrap();
    assert!(repo.exists(&process.id).await.unwrap());
}
