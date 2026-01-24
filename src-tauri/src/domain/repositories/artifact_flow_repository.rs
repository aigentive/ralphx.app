// Artifact flow repository trait - domain layer abstraction
//
// This trait defines the contract for artifact flow persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::{ArtifactFlow, ArtifactFlowId};
use crate::error::AppResult;

/// Repository trait for ArtifactFlow persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ArtifactFlowRepository: Send + Sync {
    /// Create a new artifact flow
    async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow>;

    /// Get artifact flow by ID
    async fn get_by_id(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>>;

    /// Get all artifact flows
    async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>>;

    /// Get all active artifact flows (is_active = true)
    async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>>;

    /// Update an artifact flow
    async fn update(&self, flow: &ArtifactFlow) -> AppResult<()>;

    /// Delete an artifact flow
    async fn delete(&self, id: &ArtifactFlowId) -> AppResult<()>;

    /// Set the active state of a flow
    async fn set_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()>;

    /// Check if a flow exists
    async fn exists(&self, id: &ArtifactFlowId) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        ArtifactBucketId, ArtifactFlowFilter, ArtifactFlowStep, ArtifactFlowTrigger, ArtifactType,
    };
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockArtifactFlowRepository {
        return_flow: Option<ArtifactFlow>,
        return_flows: Vec<ArtifactFlow>,
    }

    impl MockArtifactFlowRepository {
        fn new() -> Self {
            Self {
                return_flow: None,
                return_flows: vec![],
            }
        }

        fn with_flow(flow: ArtifactFlow) -> Self {
            Self {
                return_flow: Some(flow.clone()),
                return_flows: vec![flow],
            }
        }

        fn with_flows(flows: Vec<ArtifactFlow>) -> Self {
            Self {
                return_flow: flows.first().cloned(),
                return_flows: flows,
            }
        }
    }

    #[async_trait]
    impl ArtifactFlowRepository for MockArtifactFlowRepository {
        async fn create(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
            Ok(flow)
        }

        async fn get_by_id(&self, _id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
            Ok(self.return_flow.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<ArtifactFlow>> {
            Ok(self.return_flows.clone())
        }

        async fn get_active(&self) -> AppResult<Vec<ArtifactFlow>> {
            Ok(self
                .return_flows
                .iter()
                .filter(|f| f.is_active)
                .cloned()
                .collect())
        }

        async fn update(&self, _flow: &ArtifactFlow) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ArtifactFlowId) -> AppResult<()> {
            Ok(())
        }

        async fn set_active(&self, _id: &ArtifactFlowId, _is_active: bool) -> AppResult<()> {
            Ok(())
        }

        async fn exists(&self, _id: &ArtifactFlowId) -> AppResult<bool> {
            Ok(self.return_flow.is_some())
        }
    }

    fn create_test_flow() -> ArtifactFlow {
        ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created())
            .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
                "test-bucket",
            )))
    }

    fn create_flow_with_filter() -> ArtifactFlow {
        ArtifactFlow::new(
            "Filtered Flow",
            ArtifactFlowTrigger::on_artifact_created().with_filter(
                ArtifactFlowFilter::new()
                    .with_artifact_types(vec![ArtifactType::Recommendations])
                    .with_source_bucket(ArtifactBucketId::from_string("research-outputs")),
            ),
        )
        .with_step(ArtifactFlowStep::copy(ArtifactBucketId::from_string(
            "prd-library",
        )))
        .with_step(ArtifactFlowStep::spawn_process(
            "task_decomposition",
            "orchestrator",
        ))
    }

    #[test]
    fn test_artifact_flow_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn ArtifactFlowRepository> = Arc::new(MockArtifactFlowRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_create() {
        let repo = MockArtifactFlowRepository::new();
        let flow = create_test_flow();

        let result = repo.create(flow.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, flow.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_by_id_returns_none() {
        let repo = MockArtifactFlowRepository::new();
        let flow_id = ArtifactFlowId::new();

        let result = repo.get_by_id(&flow_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_by_id_returns_flow() {
        let flow = create_test_flow();
        let repo = MockArtifactFlowRepository::with_flow(flow.clone());

        let result = repo.get_by_id(&flow.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, flow.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_all_empty() {
        let repo = MockArtifactFlowRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_all_with_flows() {
        let flow1 = create_test_flow();
        let flow2 = create_flow_with_filter();
        let repo = MockArtifactFlowRepository::with_flows(vec![flow1.clone(), flow2.clone()]);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let flows = result.unwrap();
        assert_eq!(flows.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_active_filters_inactive() {
        let active_flow = create_test_flow();
        let inactive_flow = create_flow_with_filter().set_active(false);
        let repo =
            MockArtifactFlowRepository::with_flows(vec![active_flow.clone(), inactive_flow.clone()]);

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, active_flow.id);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_get_active_returns_all_active() {
        let flow1 = create_test_flow();
        let flow2 = create_flow_with_filter();
        let repo = MockArtifactFlowRepository::with_flows(vec![flow1.clone(), flow2.clone()]);

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_update() {
        let repo = MockArtifactFlowRepository::new();
        let flow = create_test_flow();

        let result = repo.update(&flow).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_delete() {
        let repo = MockArtifactFlowRepository::new();
        let flow_id = ArtifactFlowId::new();

        let result = repo.delete(&flow_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_set_active() {
        let repo = MockArtifactFlowRepository::new();
        let flow_id = ArtifactFlowId::new();

        let result = repo.set_active(&flow_id, true).await;
        assert!(result.is_ok());

        let result = repo.set_active(&flow_id, false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_exists_true() {
        let flow = create_test_flow();
        let repo = MockArtifactFlowRepository::with_flow(flow.clone());

        let result = repo.exists(&flow.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_artifact_flow_repository_exists_false() {
        let repo = MockArtifactFlowRepository::new();
        let flow_id = ArtifactFlowId::new();

        let result = repo.exists(&flow_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_artifact_flow_repository_trait_object_in_arc() {
        let flow = create_test_flow();
        let repo: Arc<dyn ArtifactFlowRepository> =
            Arc::new(MockArtifactFlowRepository::with_flow(flow.clone()));

        // Use through trait object
        let result = repo.get_by_id(&flow.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_artifact_flow_with_trigger_filter_preserved() {
        let flow = create_flow_with_filter();
        let repo = MockArtifactFlowRepository::with_flow(flow.clone());

        let result = repo.get_by_id(&flow.id).await.unwrap().unwrap();

        // Verify trigger and filter are preserved
        assert!(result.trigger.filter.is_some());
        let filter = result.trigger.filter.as_ref().unwrap();
        assert!(filter.artifact_types.is_some());
        assert!(filter.source_bucket.is_some());
    }

    #[tokio::test]
    async fn test_artifact_flow_with_multiple_steps_preserved() {
        let flow = create_flow_with_filter();
        let repo = MockArtifactFlowRepository::with_flow(flow.clone());

        let result = repo.get_by_id(&flow.id).await.unwrap().unwrap();

        assert_eq!(result.steps.len(), 2);
        assert!(result.steps[0].is_copy());
        assert!(result.steps[1].is_spawn_process());
    }
}
