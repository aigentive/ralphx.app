use crate::domain::ideation::model_settings::ModelBucket;
use crate::domain::repositories::IdeationModelSettingsRepository;
use crate::infrastructure::memory::MemoryIdeationModelSettingsRepository;

use super::{resolve_ideation_model, ResolvedModel};

// --- model_bucket_for_agent (via model_resolver context) ---

#[tokio::test]
async fn test_resolve_model_non_ideation_agent() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    // Non-ideation agent bypasses DB; result comes from YAML. Verify it doesn't
    // panic, returns a non-empty string, and source is "yaml" or "yaml_default".
    let result = resolve_ideation_model("ralphx-worker", None, &repo).await;
    assert!(!result.model.is_empty(), "expected non-empty model for ralphx-worker");
    assert!(
        result.source == "yaml" || result.source == "yaml_default",
        "expected yaml source, got: {}",
        result.source
    );
}

#[tokio::test]
async fn test_resolve_model_project_override_primary() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_for_project("proj-abc", "opus", "sonnet")
        .await
        .unwrap();

    let result = resolve_ideation_model("orchestrator-ideation", Some("proj-abc"), &repo).await;
    assert_eq!(
        result,
        ResolvedModel {
            model: "opus".to_string(),
            source: "user"
        }
    );
}

#[tokio::test]
async fn test_resolve_model_project_override_verifier() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_for_project("proj-abc", "sonnet", "haiku")
        .await
        .unwrap();

    let result = resolve_ideation_model("plan-verifier", Some("proj-abc"), &repo).await;
    assert_eq!(
        result,
        ResolvedModel {
            model: "haiku".to_string(),
            source: "user"
        }
    );
}

#[tokio::test]
async fn test_resolve_model_global_override() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("opus", "haiku").await.unwrap();

    let result = resolve_ideation_model("orchestrator-ideation", None, &repo).await;
    assert_eq!(
        result,
        ResolvedModel {
            model: "opus".to_string(),
            source: "global"
        }
    );
}

#[tokio::test]
async fn test_resolve_model_global_override_verifier() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("sonnet", "opus").await.unwrap();

    let result = resolve_ideation_model("plan-verifier", None, &repo).await;
    assert_eq!(
        result,
        ResolvedModel {
            model: "opus".to_string(),
            source: "global"
        }
    );
}

#[tokio::test]
async fn test_resolve_model_project_inherit_falls_to_global() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    // Project row inherits; global row has opus primary
    repo.upsert_for_project("proj-y", "inherit", "inherit")
        .await
        .unwrap();
    repo.upsert_global("opus", "inherit").await.unwrap();

    let result = resolve_ideation_model("orchestrator-ideation", Some("proj-y"), &repo).await;
    assert_eq!(result.model, "opus");
    assert_eq!(result.source, "global");
}

#[tokio::test]
async fn test_resolve_model_both_inherit_falls_through_to_yaml() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    // Both rows set to inherit → should fall through to YAML config
    repo.upsert_for_project("proj-x", "inherit", "inherit")
        .await
        .unwrap();
    repo.upsert_global("inherit", "inherit").await.unwrap();

    let result = resolve_ideation_model("orchestrator-ideation", Some("proj-x"), &repo).await;
    assert!(!result.model.is_empty(), "expected non-empty model from YAML fallback");
    assert_ne!(result.model, "inherit", "inherit should not be returned as final value");
    assert!(
        result.source == "yaml" || result.source == "yaml_default",
        "expected yaml source, got: {}",
        result.source
    );
}

#[tokio::test]
async fn test_resolve_model_no_db_rows_falls_through_to_yaml() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    // No rows at all — should fall through to YAML config (zero-change regression)
    let result = resolve_ideation_model("orchestrator-ideation", None, &repo).await;
    assert!(!result.model.is_empty(), "expected non-empty model with no DB rows");
    assert_ne!(result.model, "inherit");
    assert!(
        result.source == "yaml" || result.source == "yaml_default",
        "expected yaml source for no-override case, got: {}",
        result.source
    );
}

#[tokio::test]
async fn test_resolve_model_project_id_none_skips_project_level() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    // Seed a global row but pass project_id = None
    repo.upsert_global("haiku", "sonnet").await.unwrap();

    // ideation-team-lead primary bucket
    let result = resolve_ideation_model("ideation-team-lead", None, &repo).await;
    assert_eq!(result.model, "haiku");
    assert_eq!(result.source, "global");
}

#[tokio::test]
async fn test_resolve_model_all_primary_agents_use_primary_bucket() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("opus", "haiku").await.unwrap();

    for agent in &[
        "orchestrator-ideation",
        "ideation-team-lead",
        "ideation-team-member",
        "orchestrator-ideation-readonly",
    ] {
        let result = resolve_ideation_model(agent, None, &repo).await;
        assert_eq!(
            result.model, "opus",
            "expected primary (opus) for agent '{}'",
            agent
        );
        assert_eq!(result.source, "global");
    }
}

#[tokio::test]
async fn test_resolve_model_verifier_agent_uses_verifier_bucket() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("sonnet", "haiku").await.unwrap();

    let result = resolve_ideation_model("plan-verifier", None, &repo).await;
    assert_eq!(result.model, "haiku");
    assert_eq!(result.source, "global");
}

#[tokio::test]
async fn test_resolve_model_fully_qualified_verifier_agent_uses_verifier_bucket() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("sonnet", "haiku").await.unwrap();

    let result = resolve_ideation_model("ralphx:plan-verifier", None, &repo).await;
    assert_eq!(result.model, "haiku");
    assert_eq!(result.source, "global");
}

#[tokio::test]
async fn test_resolve_model_fully_qualified_primary_agent_uses_primary_bucket() {
    let repo = MemoryIdeationModelSettingsRepository::new();
    repo.upsert_global("opus", "haiku").await.unwrap();

    let result = resolve_ideation_model("ralphx:orchestrator-ideation", None, &repo).await;
    assert_eq!(result.model, "opus");
    assert_eq!(result.source, "global");
}

// Ensure the ModelBucket type is accessible (used by other tasks)
#[test]
fn test_model_bucket_variants_exist() {
    let _primary = ModelBucket::Primary;
    let _verifier = ModelBucket::Verifier;
}
