use std::sync::Arc;

use crate::application::ideation_effort_bootstrap::seed_ideation_effort_defaults;
use crate::domain::ideation::EffortLevel;
use crate::infrastructure::memory::MemoryIdeationEffortSettingsRepository;
use ralphx_domain::repositories::IdeationEffortSettingsRepository;

#[tokio::test]
async fn test_seeds_when_empty() {
    let repo = Arc::new(MemoryIdeationEffortSettingsRepository::new());
    let result = seed_ideation_effort_defaults(Arc::clone(&repo) as _)
        .await
        .unwrap();
    assert!(result.seeded_global);

    let row = repo.get_by_project_id(None).await.unwrap().unwrap();
    assert_eq!(row.primary_effort, EffortLevel::Inherit);
    assert_eq!(row.verifier_effort, EffortLevel::Inherit);
}

#[tokio::test]
async fn test_idempotent_when_already_seeded() {
    let repo = Arc::new(MemoryIdeationEffortSettingsRepository::new());
    // Pre-seed with user values
    repo.upsert(None, "high", "medium").await.unwrap();

    let result = seed_ideation_effort_defaults(Arc::clone(&repo) as _)
        .await
        .unwrap();
    assert!(!result.seeded_global);

    // User values preserved
    let row = repo.get_by_project_id(None).await.unwrap().unwrap();
    assert_eq!(row.primary_effort, EffortLevel::High);
    assert_eq!(row.verifier_effort, EffortLevel::Medium);
}

#[tokio::test]
async fn test_idempotent_on_repeated_calls() {
    let repo = Arc::new(MemoryIdeationEffortSettingsRepository::new());

    let r1 = seed_ideation_effort_defaults(Arc::clone(&repo) as _)
        .await
        .unwrap();
    let r2 = seed_ideation_effort_defaults(Arc::clone(&repo) as _)
        .await
        .unwrap();

    assert!(r1.seeded_global);
    assert!(!r2.seeded_global);
}
