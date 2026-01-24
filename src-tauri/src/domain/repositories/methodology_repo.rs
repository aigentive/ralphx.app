// Methodology repository trait - domain layer abstraction
//
// This trait defines the contract for MethodologyExtension persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::methodology::{MethodologyExtension, MethodologyId};
use crate::error::AppResult;

/// Repository trait for MethodologyExtension persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait MethodologyRepository: Send + Sync {
    /// Create a new methodology extension
    async fn create(&self, methodology: MethodologyExtension) -> AppResult<MethodologyExtension>;

    /// Get methodology by ID
    async fn get_by_id(&self, id: &MethodologyId) -> AppResult<Option<MethodologyExtension>>;

    /// Get all methodologies
    async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>>;

    /// Get the currently active methodology (if any)
    async fn get_active(&self) -> AppResult<Option<MethodologyExtension>>;

    /// Activate a methodology (deactivates any currently active one)
    async fn activate(&self, id: &MethodologyId) -> AppResult<()>;

    /// Deactivate a methodology
    async fn deactivate(&self, id: &MethodologyId) -> AppResult<()>;

    /// Update a methodology
    async fn update(&self, methodology: &MethodologyExtension) -> AppResult<()>;

    /// Delete a methodology
    async fn delete(&self, id: &MethodologyId) -> AppResult<()>;

    /// Check if a methodology exists
    async fn exists(&self, id: &MethodologyId) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::methodology::{MethodologyPhase, MethodologyTemplate};
    use crate::domain::entities::status::InternalStatus;
    use crate::domain::entities::workflow::{WorkflowColumn, WorkflowSchema};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockMethodologyRepository {
        return_methodology: Option<MethodologyExtension>,
        return_methodologies: Vec<MethodologyExtension>,
    }

    impl MockMethodologyRepository {
        fn new() -> Self {
            Self {
                return_methodology: None,
                return_methodologies: vec![],
            }
        }

        fn with_methodology(methodology: MethodologyExtension) -> Self {
            Self {
                return_methodology: Some(methodology.clone()),
                return_methodologies: vec![methodology],
            }
        }

        fn with_methodologies(methodologies: Vec<MethodologyExtension>) -> Self {
            Self {
                return_methodology: methodologies.first().cloned(),
                return_methodologies: methodologies,
            }
        }
    }

    #[async_trait]
    impl MethodologyRepository for MockMethodologyRepository {
        async fn create(
            &self,
            methodology: MethodologyExtension,
        ) -> AppResult<MethodologyExtension> {
            Ok(methodology)
        }

        async fn get_by_id(
            &self,
            _id: &MethodologyId,
        ) -> AppResult<Option<MethodologyExtension>> {
            Ok(self.return_methodology.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<MethodologyExtension>> {
            Ok(self.return_methodologies.clone())
        }

        async fn get_active(&self) -> AppResult<Option<MethodologyExtension>> {
            Ok(self
                .return_methodologies
                .iter()
                .find(|m| m.is_active)
                .cloned())
        }

        async fn activate(&self, _id: &MethodologyId) -> AppResult<()> {
            Ok(())
        }

        async fn deactivate(&self, _id: &MethodologyId) -> AppResult<()> {
            Ok(())
        }

        async fn update(&self, _methodology: &MethodologyExtension) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &MethodologyId) -> AppResult<()> {
            Ok(())
        }

        async fn exists(&self, _id: &MethodologyId) -> AppResult<bool> {
            Ok(self.return_methodology.is_some())
        }
    }

    fn create_test_workflow() -> WorkflowSchema {
        WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("in_progress", "In Progress", InternalStatus::Executing),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        )
    }

    fn create_test_methodology() -> MethodologyExtension {
        let workflow = create_test_workflow();
        MethodologyExtension::new("Test Method", workflow)
            .with_description("A test methodology")
            .with_agent_profiles(["analyst", "developer"])
            .with_phase(
                MethodologyPhase::new("analysis", "Analysis", 0)
                    .with_agent_profile("analyst"),
            )
    }

    fn create_active_methodology() -> MethodologyExtension {
        let workflow = create_test_workflow();
        let mut methodology = MethodologyExtension::new("Active Method", workflow);
        methodology.activate();
        methodology
    }

    #[test]
    fn test_methodology_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn MethodologyRepository> =
            Arc::new(MockMethodologyRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_create() {
        let repo = MockMethodologyRepository::new();
        let methodology = create_test_methodology();

        let result = repo.create(methodology.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, methodology.id);
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_by_id_returns_none() {
        let repo = MockMethodologyRepository::new();
        let id = MethodologyId::new();

        let result = repo.get_by_id(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_by_id_returns_methodology() {
        let methodology = create_test_methodology();
        let repo = MockMethodologyRepository::with_methodology(methodology.clone());

        let result = repo.get_by_id(&methodology.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, methodology.id);
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_all_empty() {
        let repo = MockMethodologyRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_all_with_methodologies() {
        let methodology1 = create_test_methodology();
        let methodology2 = create_active_methodology();
        let repo = MockMethodologyRepository::with_methodologies(vec![
            methodology1.clone(),
            methodology2.clone(),
        ]);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let methodologies = result.unwrap();
        assert_eq!(methodologies.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_active_none() {
        let methodology = create_test_methodology(); // not active
        let repo = MockMethodologyRepository::with_methodology(methodology);

        let result = repo.get_active().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_get_active_some() {
        let inactive = create_test_methodology();
        let active = create_active_methodology();
        let repo = MockMethodologyRepository::with_methodologies(vec![inactive, active.clone()]);

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Active Method");
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_activate() {
        let repo = MockMethodologyRepository::new();
        let id = MethodologyId::new();

        let result = repo.activate(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_deactivate() {
        let repo = MockMethodologyRepository::new();
        let id = MethodologyId::new();

        let result = repo.deactivate(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_update() {
        let repo = MockMethodologyRepository::new();
        let methodology = create_test_methodology();

        let result = repo.update(&methodology).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_delete() {
        let repo = MockMethodologyRepository::new();
        let id = MethodologyId::new();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_exists_true() {
        let methodology = create_test_methodology();
        let repo = MockMethodologyRepository::with_methodology(methodology.clone());

        let result = repo.exists(&methodology.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_methodology_repository_exists_false() {
        let repo = MockMethodologyRepository::new();
        let id = MethodologyId::new();

        let result = repo.exists(&id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_methodology_repository_trait_object_in_arc() {
        let methodology = create_test_methodology();
        let repo: Arc<dyn MethodologyRepository> =
            Arc::new(MockMethodologyRepository::with_methodology(methodology.clone()));

        // Use through trait object
        let result = repo.get_by_id(&methodology.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_methodology_with_phases_preserved() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Phases Test", workflow)
            .with_phase(
                MethodologyPhase::new("analysis", "Analysis", 0)
                    .with_description("Analyze requirements")
                    .with_agent_profiles(["analyst", "researcher"]),
            )
            .with_phase(
                MethodologyPhase::new("planning", "Planning", 1)
                    .with_agent_profile("pm"),
            )
            .with_phase(
                MethodologyPhase::new("execution", "Execution", 2)
                    .with_agent_profile("developer"),
            );

        let repo = MockMethodologyRepository::with_methodology(methodology.clone());
        let result = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(result.phases.len(), 3);
        assert_eq!(result.phase_at_order(0).unwrap().name, "Analysis");
        assert_eq!(result.phase_at_order(1).unwrap().name, "Planning");
    }

    #[tokio::test]
    async fn test_methodology_with_templates_preserved() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Templates Test", workflow)
            .with_template(
                MethodologyTemplate::new("prd", "templates/prd.md")
                    .with_name("PRD Template")
                    .with_description("Product Requirements Document"),
            )
            .with_template(MethodologyTemplate::new("design_doc", "templates/design.md"));

        let repo = MockMethodologyRepository::with_methodology(methodology.clone());
        let result = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(result.templates.len(), 2);
        assert_eq!(result.templates[0].artifact_type, "prd");
        assert_eq!(result.templates[0].name, Some("PRD Template".to_string()));
    }

    #[tokio::test]
    async fn test_methodology_with_skills_preserved() {
        let workflow = create_test_workflow();
        let methodology = MethodologyExtension::new("Skills Test", workflow)
            .with_skills(["skills/prd-creation", "skills/code-review", "skills/architecture"]);

        let repo = MockMethodologyRepository::with_methodology(methodology.clone());
        let result = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert_eq!(result.skills.len(), 3);
        assert!(result.skills.contains(&"skills/prd-creation".to_string()));
    }

    #[tokio::test]
    async fn test_methodology_with_hooks_config_preserved() {
        let workflow = create_test_workflow();
        let hooks = serde_json::json!({
            "pre_commit": ["validate_prd"],
            "post_review": ["notify_pm"],
            "phase_gate": { "analysis": ["checklist_complete"] }
        });
        let methodology =
            MethodologyExtension::new("Hooks Test", workflow).with_hooks_config(hooks.clone());

        let repo = MockMethodologyRepository::with_methodology(methodology.clone());
        let result = repo.get_by_id(&methodology.id).await.unwrap().unwrap();

        assert!(result.hooks_config.is_some());
        let hooks_result = result.hooks_config.unwrap();
        assert!(hooks_result.get("pre_commit").is_some());
    }
}
