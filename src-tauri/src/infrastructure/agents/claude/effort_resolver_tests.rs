use crate::domain::ideation::EffortBucket;
use crate::domain::repositories::IdeationEffortSettingsRepository;
use crate::infrastructure::memory::MemoryIdeationEffortSettingsRepository;

use super::{effort_bucket_for_agent, resolve_ideation_effort};

// --- effort_bucket_for_agent ---

#[test]
fn test_effort_bucket_mapping_primary_agents() {
    for name in &[
        "orchestrator-ideation",
        "ideation-team-lead",
        "ideation-team-member",
        "orchestrator-ideation-readonly",
    ] {
        assert_eq!(
            effort_bucket_for_agent(name),
            Some(EffortBucket::Primary),
            "expected Primary for agent '{}'",
            name
        );
    }
}

#[test]
fn test_effort_bucket_mapping_verifier() {
    assert_eq!(
        effort_bucket_for_agent("plan-verifier"),
        Some(EffortBucket::Verifier)
    );
    assert_eq!(
        effort_bucket_for_agent("ralphx:plan-verifier"),
        Some(EffortBucket::Verifier)
    );
}

#[test]
fn test_effort_bucket_mapping_primary_agents_fully_qualified() {
    assert_eq!(
        effort_bucket_for_agent("ralphx:orchestrator-ideation"),
        Some(EffortBucket::Primary)
    );
    assert_eq!(
        effort_bucket_for_agent("ralphx:ideation-team-lead"),
        Some(EffortBucket::Primary)
    );
}

#[test]
fn test_effort_bucket_mapping_non_ideation() {
    assert_eq!(effort_bucket_for_agent("ralphx-worker"), None);
}

// --- resolve_ideation_effort ---

#[tokio::test]
async fn test_resolve_effort_non_ideation_agent() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    // Non-ideation agent bypasses DB; result comes from YAML. Just verify it
    // doesn't panic and returns a non-empty string.
    let result = resolve_ideation_effort("ralphx-worker", None, &repo).await;
    assert!(!result.is_empty(), "expected non-empty effort for ralphx-worker");
}

#[tokio::test]
async fn test_resolve_effort_project_override() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    // Seed project row with high primary effort
    repo.upsert(Some("proj-abc"), "high", "low").await.unwrap();

    let result =
        resolve_ideation_effort("orchestrator-ideation", Some("proj-abc"), &repo).await;
    assert_eq!(result, "high");
}

#[tokio::test]
async fn test_resolve_effort_global_override() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    // Seed global row with medium verifier effort
    repo.upsert(None, "low", "medium").await.unwrap();

    let result = resolve_ideation_effort("plan-verifier", None, &repo).await;
    assert_eq!(result, "medium");
}

#[tokio::test]
async fn test_resolve_effort_fully_qualified_verifier_uses_verifier_bucket() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    repo.upsert(None, "low", "medium").await.unwrap();

    let result = resolve_ideation_effort("ralphx:plan-verifier", None, &repo).await;
    assert_eq!(result, "medium");
}

#[tokio::test]
async fn test_resolve_effort_fully_qualified_primary_uses_primary_bucket() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    repo.upsert(None, "high", "low").await.unwrap();

    let result = resolve_ideation_effort("ralphx:orchestrator-ideation", None, &repo).await;
    assert_eq!(result, "high");
}

#[tokio::test]
async fn test_resolve_effort_inherit_falls_through_to_yaml() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    // Both rows set to inherit → should fall through to YAML
    repo.upsert(Some("proj-x"), "inherit", "inherit")
        .await
        .unwrap();
    repo.upsert(None, "inherit", "inherit").await.unwrap();

    let result =
        resolve_ideation_effort("orchestrator-ideation", Some("proj-x"), &repo).await;
    assert!(!result.is_empty(), "expected non-empty effort from YAML fallback");
    assert_ne!(result, "inherit", "inherit should not be returned as the final value");
}

#[tokio::test]
async fn test_resolve_effort_project_inherit_falls_to_global() {
    let repo = MemoryIdeationEffortSettingsRepository::new();
    // Project row inherits; global row has "high"
    repo.upsert(Some("proj-y"), "inherit", "inherit")
        .await
        .unwrap();
    repo.upsert(None, "high", "inherit").await.unwrap();

    let result =
        resolve_ideation_effort("orchestrator-ideation", Some("proj-y"), &repo).await;
    assert_eq!(result, "high");
}
