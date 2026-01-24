// Process repository trait - domain layer abstraction
//
// This trait defines the contract for ResearchProcess persistence.
// Implementations can use SQLite, PostgreSQL, in-memory, etc.

use async_trait::async_trait;

use crate::domain::entities::research::{ResearchProcess, ResearchProcessId, ResearchProcessStatus};
use crate::error::AppResult;

/// Repository trait for ResearchProcess persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait ProcessRepository: Send + Sync {
    /// Create a new research process
    async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess>;

    /// Get research process by ID
    async fn get_by_id(&self, id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>>;

    /// Get all research processes
    async fn get_all(&self) -> AppResult<Vec<ResearchProcess>>;

    /// Get research processes by status
    async fn get_by_status(&self, status: ResearchProcessStatus) -> AppResult<Vec<ResearchProcess>>;

    /// Get active research processes (pending or running)
    async fn get_active(&self) -> AppResult<Vec<ResearchProcess>>;

    /// Update progress on a research process (iteration count, checkpoint, etc.)
    async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()>;

    /// Update the full research process
    async fn update(&self, process: &ResearchProcess) -> AppResult<()>;

    /// Mark a process as completed
    async fn complete(&self, id: &ResearchProcessId) -> AppResult<()>;

    /// Mark a process as failed with an error message
    async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()>;

    /// Delete a research process
    async fn delete(&self, id: &ResearchProcessId) -> AppResult<()>;

    /// Check if a process exists
    async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::research::{
        CustomDepth, ResearchBrief, ResearchDepthPreset, ResearchOutput,
    };
    use crate::domain::entities::ArtifactType;
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockProcessRepository {
        return_process: Option<ResearchProcess>,
        return_processes: Vec<ResearchProcess>,
    }

    impl MockProcessRepository {
        fn new() -> Self {
            Self {
                return_process: None,
                return_processes: vec![],
            }
        }

        fn with_process(process: ResearchProcess) -> Self {
            Self {
                return_process: Some(process.clone()),
                return_processes: vec![process],
            }
        }

        fn with_processes(processes: Vec<ResearchProcess>) -> Self {
            Self {
                return_process: processes.first().cloned(),
                return_processes: processes,
            }
        }
    }

    #[async_trait]
    impl ProcessRepository for MockProcessRepository {
        async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess> {
            Ok(process)
        }

        async fn get_by_id(&self, _id: &ResearchProcessId) -> AppResult<Option<ResearchProcess>> {
            Ok(self.return_process.clone())
        }

        async fn get_all(&self) -> AppResult<Vec<ResearchProcess>> {
            Ok(self.return_processes.clone())
        }

        async fn get_by_status(
            &self,
            status: ResearchProcessStatus,
        ) -> AppResult<Vec<ResearchProcess>> {
            Ok(self
                .return_processes
                .iter()
                .filter(|p| p.status() == status)
                .cloned()
                .collect())
        }

        async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
            Ok(self
                .return_processes
                .iter()
                .filter(|p| p.is_active())
                .cloned()
                .collect())
        }

        async fn update_progress(&self, _process: &ResearchProcess) -> AppResult<()> {
            Ok(())
        }

        async fn update(&self, _process: &ResearchProcess) -> AppResult<()> {
            Ok(())
        }

        async fn complete(&self, _id: &ResearchProcessId) -> AppResult<()> {
            Ok(())
        }

        async fn fail(&self, _id: &ResearchProcessId, _error: &str) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ResearchProcessId) -> AppResult<()> {
            Ok(())
        }

        async fn exists(&self, _id: &ResearchProcessId) -> AppResult<bool> {
            Ok(self.return_process.is_some())
        }
    }

    fn create_test_process() -> ResearchProcess {
        let brief = ResearchBrief::new("What architecture should we use?")
            .with_context("Building a new web application")
            .with_constraint("Must be scalable");
        ResearchProcess::new("Architecture Research", brief, "deep-researcher")
            .with_preset(ResearchDepthPreset::Standard)
    }

    fn create_running_process() -> ResearchProcess {
        let brief = ResearchBrief::new("Which database to choose?");
        let mut process =
            ResearchProcess::new("Database Research", brief, "deep-researcher")
                .with_preset(ResearchDepthPreset::QuickScan);
        process.start();
        process.advance();
        process.advance();
        process
    }

    fn create_completed_process() -> ResearchProcess {
        let brief = ResearchBrief::new("Completed question");
        let mut process =
            ResearchProcess::new("Completed Research", brief, "deep-researcher");
        process.start();
        process.complete();
        process
    }

    #[test]
    fn test_process_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn ProcessRepository> = Arc::new(MockProcessRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_process_repository_create() {
        let repo = MockProcessRepository::new();
        let process = create_test_process();

        let result = repo.create(process.clone()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, process.id);
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_by_id_returns_none() {
        let repo = MockProcessRepository::new();
        let process_id = ResearchProcessId::new();

        let result = repo.get_by_id(&process_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_by_id_returns_process() {
        let process = create_test_process();
        let repo = MockProcessRepository::with_process(process.clone());

        let result = repo.get_by_id(&process.id).await;
        assert!(result.is_ok());
        let returned = result.unwrap();
        assert!(returned.is_some());
        assert_eq!(returned.unwrap().id, process.id);
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_all_empty() {
        let repo = MockProcessRepository::new();

        let result = repo.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_all_with_processes() {
        let process1 = create_test_process();
        let process2 = create_running_process();
        let repo = MockProcessRepository::with_processes(vec![process1.clone(), process2.clone()]);

        let result = repo.get_all().await;
        assert!(result.is_ok());
        let processes = result.unwrap();
        assert_eq!(processes.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_by_status() {
        let pending = create_test_process();
        let running = create_running_process();
        let completed = create_completed_process();
        let repo = MockProcessRepository::with_processes(vec![
            pending.clone(),
            running.clone(),
            completed.clone(),
        ]);

        // Get pending
        let result = repo.get_by_status(ResearchProcessStatus::Pending).await;
        assert!(result.is_ok());
        let pending_processes = result.unwrap();
        assert_eq!(pending_processes.len(), 1);

        // Get running
        let result = repo.get_by_status(ResearchProcessStatus::Running).await;
        assert!(result.is_ok());
        let running_processes = result.unwrap();
        assert_eq!(running_processes.len(), 1);

        // Get completed
        let result = repo.get_by_status(ResearchProcessStatus::Completed).await;
        assert!(result.is_ok());
        let completed_processes = result.unwrap();
        assert_eq!(completed_processes.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_process_repository_get_active() {
        let pending = create_test_process();
        let running = create_running_process();
        let completed = create_completed_process();
        let repo = MockProcessRepository::with_processes(vec![
            pending.clone(),
            running.clone(),
            completed.clone(),
        ]);

        let result = repo.get_active().await;
        assert!(result.is_ok());
        let active = result.unwrap();
        // Pending and running are both considered active
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_mock_process_repository_update_progress() {
        let repo = MockProcessRepository::new();
        let process = create_running_process();

        let result = repo.update_progress(&process).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_process_repository_update() {
        let repo = MockProcessRepository::new();
        let process = create_test_process();

        let result = repo.update(&process).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_process_repository_complete() {
        let repo = MockProcessRepository::new();
        let process_id = ResearchProcessId::new();

        let result = repo.complete(&process_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_process_repository_fail() {
        let repo = MockProcessRepository::new();
        let process_id = ResearchProcessId::new();

        let result = repo.fail(&process_id, "Something went wrong").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_process_repository_delete() {
        let repo = MockProcessRepository::new();
        let process_id = ResearchProcessId::new();

        let result = repo.delete(&process_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_process_repository_exists_true() {
        let process = create_test_process();
        let repo = MockProcessRepository::with_process(process.clone());

        let result = repo.exists(&process.id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_process_repository_exists_false() {
        let repo = MockProcessRepository::new();
        let process_id = ResearchProcessId::new();

        let result = repo.exists(&process_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_process_repository_trait_object_in_arc() {
        let process = create_test_process();
        let repo: Arc<dyn ProcessRepository> =
            Arc::new(MockProcessRepository::with_process(process.clone()));

        // Use through trait object
        let result = repo.get_by_id(&process.id).await;
        assert!(result.is_ok());

        let all = repo.get_all().await;
        assert!(all.is_ok());
        assert_eq!(all.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_process_with_custom_depth_preserved() {
        let brief = ResearchBrief::new("Custom depth question");
        let process = ResearchProcess::new("Custom Research", brief, "researcher")
            .with_custom_depth(CustomDepth::new(150, 5.0, 30));
        let repo = MockProcessRepository::with_process(process.clone());

        let result = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert!(result.depth.is_custom());
        let resolved = result.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
        assert_eq!(resolved.timeout_hours, 5.0);
    }

    #[tokio::test]
    async fn test_process_with_brief_preserved() {
        let brief = ResearchBrief::new("Main question")
            .with_context("Context info")
            .with_scope("Backend only")
            .with_constraints(["Constraint 1", "Constraint 2"]);
        let process = ResearchProcess::new("Full Brief Research", brief.clone(), "researcher");
        let repo = MockProcessRepository::with_process(process.clone());

        let result = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert_eq!(result.brief.question, "Main question");
        assert_eq!(result.brief.context, Some("Context info".to_string()));
        assert_eq!(result.brief.scope, Some("Backend only".to_string()));
        assert_eq!(result.brief.constraints.len(), 2);
    }

    #[tokio::test]
    async fn test_process_with_output_config_preserved() {
        let brief = ResearchBrief::new("Question");
        let output = ResearchOutput::new("custom-bucket")
            .with_artifact_type(ArtifactType::Findings)
            .with_artifact_type(ArtifactType::Recommendations);
        let process = ResearchProcess::new("Output Research", brief, "researcher")
            .with_output(output);
        let repo = MockProcessRepository::with_process(process.clone());

        let result = repo.get_by_id(&process.id).await.unwrap().unwrap();

        assert_eq!(result.output.target_bucket, "custom-bucket");
        assert_eq!(result.output.artifact_types.len(), 2);
    }
}
