use crate::domain::ideation::EffortLevel;
use crate::domain::repositories::IdeationEffortSettingsRepository;
use crate::infrastructure::memory::MemoryIdeationEffortSettingsRepository;

#[tokio::test]
async fn test_upsert_and_get_global() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    assert!(repo.get_by_project_id(None).await.unwrap().is_none());

    let result = repo.upsert(None, "high", "medium").await.unwrap();
    assert_eq!(result.primary_effort, EffortLevel::High);
    assert_eq!(result.verifier_effort, EffortLevel::Medium);
    assert!(result.project_id.is_none());

    let fetched = repo.get_by_project_id(None).await.unwrap().unwrap();
    assert_eq!(fetched.primary_effort, EffortLevel::High);
    assert_eq!(fetched.verifier_effort, EffortLevel::Medium);
}

#[tokio::test]
async fn test_upsert_and_get_project() {
    let repo = MemoryIdeationEffortSettingsRepository::new();

    let result = repo.upsert(Some("proj-123"), "max", "low").await.unwrap();
    assert_eq!(result.primary_effort, EffortLevel::Max);
    assert!(result.project_id.is_some());

    let fetched = repo
        .get_by_project_id(Some("proj-123"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.primary_effort, EffortLevel::Max);
    assert_eq!(fetched.verifier_effort, EffortLevel::Low);

    // Global is still empty
    assert!(repo.get_by_project_id(None).await.unwrap().is_none());
}

#[tokio::test]
async fn test_upsert_overwrites_existing() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    repo.upsert(None, "high", "medium").await.unwrap();
    repo.upsert(None, "low", "inherit").await.unwrap();

    let fetched = repo.get_by_project_id(None).await.unwrap().unwrap();
    assert_eq!(fetched.primary_effort, EffortLevel::Low);
    assert_eq!(fetched.verifier_effort, EffortLevel::Inherit);
}

#[tokio::test]
async fn test_upsert_project_overwrites_existing() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    repo.upsert(Some("proj-abc"), "high", "medium")
        .await
        .unwrap();
    repo.upsert(Some("proj-abc"), "max", "inherit")
        .await
        .unwrap();

    let fetched = repo
        .get_by_project_id(Some("proj-abc"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.primary_effort, EffortLevel::Max);
    assert_eq!(fetched.verifier_effort, EffortLevel::Inherit);
}

#[tokio::test]
async fn test_get_missing_returns_none() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    assert!(repo
        .get_by_project_id(Some("nonexistent"))
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_project_and_global_are_independent() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    repo.upsert(None, "high", "high").await.unwrap();
    repo.upsert(Some("proj-1"), "low", "low").await.unwrap();

    let global = repo.get_by_project_id(None).await.unwrap().unwrap();
    let project = repo
        .get_by_project_id(Some("proj-1"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(global.primary_effort, EffortLevel::High);
    assert_eq!(project.primary_effort, EffortLevel::Low);
    assert!(global.project_id.is_none());
    assert!(project.project_id.is_some());
}

#[tokio::test]
async fn test_invalid_effort_level_returns_error() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    let result = repo.upsert(None, "turbo", "medium").await;
    assert!(result.is_err());
}
