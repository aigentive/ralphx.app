// ResearchService - domain service for research process management
//
// Provides business logic for:
// - Starting and stopping research processes
// - Pausing and resuming research
// - Creating checkpoints
// - Tracking progress
// - Converting presets to configurations

use std::sync::Arc;

use crate::domain::entities::research::{
    CustomDepth, ResearchBrief, ResearchDepth, ResearchDepthPreset, ResearchOutput,
    ResearchProcess, ResearchProcessId, ResearchProcessStatus, RESEARCH_PRESETS,
};
use crate::domain::entities::ArtifactId;
use crate::domain::repositories::ProcessRepository;
use crate::error::{AppError, AppResult};

/// Service for research process management
pub struct ResearchService<R: ProcessRepository> {
    process_repo: Arc<R>,
}

impl<R: ProcessRepository> ResearchService<R> {
    /// Create a new ResearchService with the given repository
    pub fn new(process_repo: Arc<R>) -> Self {
        Self { process_repo }
    }

    /// Start a new research process
    pub async fn start_research(
        &self,
        name: impl Into<String>,
        brief: ResearchBrief,
        agent_profile_id: impl Into<String>,
        depth: ResearchDepth,
        output: Option<ResearchOutput>,
    ) -> AppResult<ResearchProcess> {
        let mut process = ResearchProcess::new(name, brief, agent_profile_id)
            .with_depth(depth);

        if let Some(output_config) = output {
            process = process.with_output(output_config);
        }

        // Start the process
        process.start();

        // Persist and return
        self.process_repo.create(process).await
    }

    /// Start a research process with a preset depth
    pub async fn start_research_with_preset(
        &self,
        name: impl Into<String>,
        brief: ResearchBrief,
        agent_profile_id: impl Into<String>,
        preset: ResearchDepthPreset,
    ) -> AppResult<ResearchProcess> {
        self.start_research(
            name,
            brief,
            agent_profile_id,
            ResearchDepth::Preset(preset),
            None,
        )
        .await
    }

    /// Start a research process with custom depth configuration
    pub async fn start_research_with_custom_depth(
        &self,
        name: impl Into<String>,
        brief: ResearchBrief,
        agent_profile_id: impl Into<String>,
        custom_depth: CustomDepth,
    ) -> AppResult<ResearchProcess> {
        self.start_research(
            name,
            brief,
            agent_profile_id,
            ResearchDepth::Custom(custom_depth),
            None,
        )
        .await
    }

    /// Pause a running research process
    pub async fn pause_research(&self, id: &ResearchProcessId) -> AppResult<ResearchProcess> {
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.status() != ResearchProcessStatus::Running {
            return Err(AppError::Validation(format!(
                "Cannot pause process in state '{}', must be 'running'",
                process.status()
            )));
        }

        process.pause();
        self.process_repo.update(&process).await?;
        Ok(process)
    }

    /// Resume a paused research process
    pub async fn resume_research(&self, id: &ResearchProcessId) -> AppResult<ResearchProcess> {
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.status() != ResearchProcessStatus::Paused {
            return Err(AppError::Validation(format!(
                "Cannot resume process in state '{}', must be 'paused'",
                process.status()
            )));
        }

        process.resume();
        self.process_repo.update(&process).await?;
        Ok(process)
    }

    /// Create a checkpoint for a research process
    pub async fn checkpoint(
        &self,
        id: &ResearchProcessId,
        artifact_id: ArtifactId,
    ) -> AppResult<ResearchProcess> {
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.is_terminal() {
            return Err(AppError::Validation(format!(
                "Cannot checkpoint a terminal process (status: {})",
                process.status()
            )));
        }

        process.checkpoint(artifact_id);
        self.process_repo.update_progress(&process).await?;
        Ok(process)
    }

    /// Advance the iteration counter for a research process
    pub async fn advance_iteration(
        &self,
        id: &ResearchProcessId,
    ) -> AppResult<ResearchProcess> {
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.status() != ResearchProcessStatus::Running {
            return Err(AppError::Validation(format!(
                "Cannot advance iteration for process in state '{}', must be 'running'",
                process.status()
            )));
        }

        process.advance();
        self.process_repo.update_progress(&process).await?;
        Ok(process)
    }

    /// Complete a research process successfully
    pub async fn complete(&self, id: &ResearchProcessId) -> AppResult<ResearchProcess> {
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.is_terminal() {
            return Err(AppError::Validation(format!(
                "Process is already in terminal state: {}",
                process.status()
            )));
        }

        process.complete();
        self.process_repo.update(&process).await?;
        Ok(process)
    }

    /// Fail a research process with an error message
    pub async fn fail(
        &self,
        id: &ResearchProcessId,
        error: impl Into<String>,
    ) -> AppResult<ResearchProcess> {
        let error_msg = error.into();
        let mut process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        if process.is_terminal() {
            return Err(AppError::Validation(format!(
                "Process is already in terminal state: {}",
                process.status()
            )));
        }

        process.fail(&error_msg);
        self.process_repo.update(&process).await?;
        Ok(process)
    }

    /// Stop a research process (mark as completed if running, or fail if pending)
    pub async fn stop_research(&self, id: &ResearchProcessId) -> AppResult<ResearchProcess> {
        let process = self
            .process_repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Research process not found: {}", id)))?;

        match process.status() {
            ResearchProcessStatus::Running | ResearchProcessStatus::Paused => {
                self.complete(id).await
            }
            ResearchProcessStatus::Pending => {
                self.fail(id, "Research stopped before starting").await
            }
            _ => Err(AppError::Validation(format!(
                "Process is already in terminal state: {}",
                process.status()
            ))),
        }
    }

    /// Get a research process by ID
    pub async fn get_process(
        &self,
        id: &ResearchProcessId,
    ) -> AppResult<Option<ResearchProcess>> {
        self.process_repo.get_by_id(id).await
    }

    /// Get all research processes
    pub async fn get_all_processes(&self) -> AppResult<Vec<ResearchProcess>> {
        self.process_repo.get_all().await
    }

    /// Get active research processes (pending or running)
    pub async fn get_active_processes(&self) -> AppResult<Vec<ResearchProcess>> {
        self.process_repo.get_active().await
    }

    /// Get research processes by status
    pub async fn get_processes_by_status(
        &self,
        status: ResearchProcessStatus,
    ) -> AppResult<Vec<ResearchProcess>> {
        self.process_repo.get_by_status(status).await
    }

    /// Delete a research process
    pub async fn delete_process(&self, id: &ResearchProcessId) -> AppResult<()> {
        self.process_repo.delete(id).await
    }

    /// Check if a research process exists
    pub async fn process_exists(&self, id: &ResearchProcessId) -> AppResult<bool> {
        self.process_repo.exists(id).await
    }

    /// Convert a preset to its custom depth configuration
    pub fn preset_to_config(preset: ResearchDepthPreset) -> CustomDepth {
        RESEARCH_PRESETS[&preset]
    }

    /// Get all available presets with their configurations
    pub fn get_all_presets() -> Vec<(ResearchDepthPreset, CustomDepth)> {
        ResearchDepthPreset::all()
            .iter()
            .map(|p| (*p, Self::preset_to_config(*p)))
            .collect()
    }

    /// Check if a process should create a checkpoint at current iteration
    pub fn should_checkpoint(process: &ResearchProcess) -> bool {
        process.should_checkpoint()
    }

    /// Check if a process has reached max iterations
    pub fn is_max_iterations_reached(process: &ResearchProcess) -> bool {
        process.is_max_iterations_reached()
    }

    /// Get progress percentage for a process
    pub fn progress_percentage(process: &ResearchProcess) -> f32 {
        process.progress_percentage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    // ==================== Mock Process Repository ====================

    struct MockProcessRepository {
        processes: Mutex<HashMap<String, ResearchProcess>>,
    }

    impl MockProcessRepository {
        fn new() -> Self {
            Self {
                processes: Mutex::new(HashMap::new()),
            }
        }

        async fn add_process(&self, process: ResearchProcess) {
            let mut processes = self.processes.lock().await;
            processes.insert(process.id.as_str().to_string(), process);
        }
    }

    #[async_trait]
    impl ProcessRepository for MockProcessRepository {
        async fn create(&self, process: ResearchProcess) -> AppResult<ResearchProcess> {
            self.add_process(process.clone()).await;
            Ok(process)
        }

        async fn get_by_id(
            &self,
            id: &ResearchProcessId,
        ) -> AppResult<Option<ResearchProcess>> {
            let processes = self.processes.lock().await;
            Ok(processes.get(id.as_str()).cloned())
        }

        async fn get_all(&self) -> AppResult<Vec<ResearchProcess>> {
            let processes = self.processes.lock().await;
            Ok(processes.values().cloned().collect())
        }

        async fn get_by_status(
            &self,
            status: ResearchProcessStatus,
        ) -> AppResult<Vec<ResearchProcess>> {
            let processes = self.processes.lock().await;
            Ok(processes
                .values()
                .filter(|p| p.status() == status)
                .cloned()
                .collect())
        }

        async fn get_active(&self) -> AppResult<Vec<ResearchProcess>> {
            let processes = self.processes.lock().await;
            Ok(processes.values().filter(|p| p.is_active()).cloned().collect())
        }

        async fn update_progress(&self, process: &ResearchProcess) -> AppResult<()> {
            let mut processes = self.processes.lock().await;
            processes.insert(process.id.as_str().to_string(), process.clone());
            Ok(())
        }

        async fn update(&self, process: &ResearchProcess) -> AppResult<()> {
            let mut processes = self.processes.lock().await;
            processes.insert(process.id.as_str().to_string(), process.clone());
            Ok(())
        }

        async fn complete(&self, id: &ResearchProcessId) -> AppResult<()> {
            let mut processes = self.processes.lock().await;
            if let Some(process) = processes.get_mut(id.as_str()) {
                process.complete();
            }
            Ok(())
        }

        async fn fail(&self, id: &ResearchProcessId, error: &str) -> AppResult<()> {
            let mut processes = self.processes.lock().await;
            if let Some(process) = processes.get_mut(id.as_str()) {
                process.fail(error);
            }
            Ok(())
        }

        async fn delete(&self, id: &ResearchProcessId) -> AppResult<()> {
            let mut processes = self.processes.lock().await;
            processes.remove(id.as_str());
            Ok(())
        }

        async fn exists(&self, id: &ResearchProcessId) -> AppResult<bool> {
            let processes = self.processes.lock().await;
            Ok(processes.contains_key(id.as_str()))
        }
    }

    // ==================== Test Helpers ====================

    fn create_service() -> (ResearchService<MockProcessRepository>, Arc<MockProcessRepository>) {
        let process_repo = Arc::new(MockProcessRepository::new());
        let service = ResearchService::new(process_repo.clone());
        (service, process_repo)
    }

    fn create_test_brief() -> ResearchBrief {
        ResearchBrief::new("What architecture should we use?")
            .with_context("Building a web application")
    }

    // ==================== start_research Tests ====================

    #[tokio::test]
    async fn start_research_creates_and_starts_process() {
        let (service, _) = create_service();

        let result = service
            .start_research(
                "Architecture Research",
                create_test_brief(),
                "deep-researcher",
                ResearchDepth::preset(ResearchDepthPreset::Standard),
                None,
            )
            .await;

        assert!(result.is_ok());
        let process = result.unwrap();
        assert_eq!(process.name, "Architecture Research");
        assert_eq!(process.agent_profile_id, "deep-researcher");
        assert_eq!(process.status(), ResearchProcessStatus::Running);
        assert!(process.started_at.is_some());
    }

    #[tokio::test]
    async fn start_research_with_custom_output() {
        let (service, _) = create_service();

        let output = ResearchOutput::new("custom-bucket")
            .with_artifact_type(crate::domain::entities::ArtifactType::Findings);

        let result = service
            .start_research(
                "Test",
                create_test_brief(),
                "agent",
                ResearchDepth::default(),
                Some(output),
            )
            .await;

        assert!(result.is_ok());
        let process = result.unwrap();
        assert_eq!(process.output.target_bucket, "custom-bucket");
    }

    #[tokio::test]
    async fn start_research_with_preset() {
        let (service, _) = create_service();

        let result = service
            .start_research_with_preset(
                "Quick Research",
                create_test_brief(),
                "agent",
                ResearchDepthPreset::QuickScan,
            )
            .await;

        assert!(result.is_ok());
        let process = result.unwrap();
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 10);
    }

    #[tokio::test]
    async fn start_research_with_custom_depth() {
        let (service, _) = create_service();

        let custom = CustomDepth::new(150, 5.0, 30);
        let result = service
            .start_research_with_custom_depth("Custom Research", create_test_brief(), "agent", custom)
            .await;

        assert!(result.is_ok());
        let process = result.unwrap();
        let resolved = process.resolved_depth();
        assert_eq!(resolved.max_iterations, 150);
        assert_eq!(resolved.timeout_hours, 5.0);
    }

    // ==================== pause_research Tests ====================

    #[tokio::test]
    async fn pause_research_pauses_running_process() {
        let (service, process_repo) = create_service();

        // Create a running process
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.pause_research(&id).await;

        assert!(result.is_ok());
        let paused = result.unwrap();
        assert_eq!(paused.status(), ResearchProcessStatus::Paused);
    }

    #[tokio::test]
    async fn pause_research_fails_for_non_running() {
        let (service, process_repo) = create_service();

        // Create a pending process
        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.pause_research(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be 'running'"));
    }

    #[tokio::test]
    async fn pause_research_fails_for_not_found() {
        let (service, _) = create_service();

        let id = ResearchProcessId::new();
        let result = service.pause_research(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // ==================== resume_research Tests ====================

    #[tokio::test]
    async fn resume_research_resumes_paused_process() {
        let (service, process_repo) = create_service();

        // Create a paused process
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.pause();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.resume_research(&id).await;

        assert!(result.is_ok());
        let resumed = result.unwrap();
        assert_eq!(resumed.status(), ResearchProcessStatus::Running);
    }

    #[tokio::test]
    async fn resume_research_fails_for_non_paused() {
        let (service, process_repo) = create_service();

        // Create a running process
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.resume_research(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be 'paused'"));
    }

    // ==================== checkpoint Tests ====================

    #[tokio::test]
    async fn checkpoint_saves_artifact_id() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let artifact_id = ArtifactId::from_string("checkpoint-artifact");
        let result = service.checkpoint(&id, artifact_id.clone()).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.progress.last_checkpoint, Some(artifact_id));
    }

    #[tokio::test]
    async fn checkpoint_fails_for_terminal_process() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.complete();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let artifact_id = ArtifactId::new();
        let result = service.checkpoint(&id, artifact_id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("terminal"));
    }

    // ==================== advance_iteration Tests ====================

    #[tokio::test]
    async fn advance_iteration_increments_counter() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.advance_iteration(&id).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.progress.current_iteration, 1);

        // Advance again
        let result = service.advance_iteration(&id).await;
        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.progress.current_iteration, 2);
    }

    #[tokio::test]
    async fn advance_iteration_fails_for_non_running() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.advance_iteration(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be 'running'"));
    }

    // ==================== complete Tests ====================

    #[tokio::test]
    async fn complete_marks_process_completed() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.complete(&id).await;

        assert!(result.is_ok());
        let completed = result.unwrap();
        assert_eq!(completed.status(), ResearchProcessStatus::Completed);
        assert!(completed.completed_at.is_some());
    }

    #[tokio::test]
    async fn complete_fails_for_already_completed() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.complete();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.complete(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("terminal"));
    }

    // ==================== fail Tests ====================

    #[tokio::test]
    async fn fail_marks_process_failed() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.fail(&id, "Something went wrong").await;

        assert!(result.is_ok());
        let failed = result.unwrap();
        assert_eq!(failed.status(), ResearchProcessStatus::Failed);
        assert_eq!(
            failed.progress.error_message,
            Some("Something went wrong".to_string())
        );
    }

    #[tokio::test]
    async fn fail_fails_for_already_failed() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.fail("Original error");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.fail(&id, "New error").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("terminal"));
    }

    // ==================== stop_research Tests ====================

    #[tokio::test]
    async fn stop_research_completes_running_process() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.stop_research(&id).await;

        assert!(result.is_ok());
        let stopped = result.unwrap();
        assert_eq!(stopped.status(), ResearchProcessStatus::Completed);
    }

    #[tokio::test]
    async fn stop_research_completes_paused_process() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.pause();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.stop_research(&id).await;

        assert!(result.is_ok());
        let stopped = result.unwrap();
        assert_eq!(stopped.status(), ResearchProcessStatus::Completed);
    }

    #[tokio::test]
    async fn stop_research_fails_pending_process() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.stop_research(&id).await;

        assert!(result.is_ok());
        let stopped = result.unwrap();
        assert_eq!(stopped.status(), ResearchProcessStatus::Failed);
    }

    #[tokio::test]
    async fn stop_research_fails_for_terminal() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent");
        process.start();
        process.complete();
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.stop_research(&id).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("terminal"));
    }

    // ==================== Repository Method Tests ====================

    #[tokio::test]
    async fn get_process_found() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.get_process(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn get_process_not_found() {
        let (service, _) = create_service();

        let id = ResearchProcessId::new();
        let result = service.get_process(&id).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_all_processes_empty() {
        let (service, _) = create_service();

        let result = service.get_all_processes().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_all_processes_returns_all() {
        let (service, process_repo) = create_service();

        let brief1 = create_test_brief();
        let brief2 = ResearchBrief::new("Another question");
        let process1 = ResearchProcess::new("Test 1", brief1, "agent");
        let process2 = ResearchProcess::new("Test 2", brief2, "agent");
        process_repo.add_process(process1).await;
        process_repo.add_process(process2).await;

        let result = service.get_all_processes().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_active_processes_filters_terminal() {
        let (service, process_repo) = create_service();

        // Add running process
        let brief1 = create_test_brief();
        let mut running = ResearchProcess::new("Running", brief1, "agent");
        running.start();
        process_repo.add_process(running).await;

        // Add completed process
        let brief2 = ResearchBrief::new("Completed");
        let mut completed = ResearchProcess::new("Completed", brief2, "agent");
        completed.start();
        completed.complete();
        process_repo.add_process(completed).await;

        let result = service.get_active_processes().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn get_processes_by_status() {
        let (service, process_repo) = create_service();

        // Add pending
        let brief1 = create_test_brief();
        let pending = ResearchProcess::new("Pending", brief1, "agent");
        process_repo.add_process(pending).await;

        // Add running
        let brief2 = ResearchBrief::new("Running");
        let mut running = ResearchProcess::new("Running", brief2, "agent");
        running.start();
        process_repo.add_process(running).await;

        let result = service
            .get_processes_by_status(ResearchProcessStatus::Running)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn delete_process_removes() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        service.delete_process(&id).await.unwrap();

        let found = process_repo.get_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn process_exists_true() {
        let (service, process_repo) = create_service();

        let brief = create_test_brief();
        let process = ResearchProcess::new("Test", brief, "agent");
        let id = process.id.clone();
        process_repo.add_process(process).await;

        let result = service.process_exists(&id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn process_exists_false() {
        let (service, _) = create_service();

        let id = ResearchProcessId::new();
        let result = service.process_exists(&id).await;

        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    // ==================== Utility Method Tests ====================

    #[test]
    fn preset_to_config_quick_scan() {
        let config = ResearchService::<MockProcessRepository>::preset_to_config(
            ResearchDepthPreset::QuickScan,
        );
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.timeout_hours, 0.5);
        assert_eq!(config.checkpoint_interval, 5);
    }

    #[test]
    fn preset_to_config_standard() {
        let config = ResearchService::<MockProcessRepository>::preset_to_config(
            ResearchDepthPreset::Standard,
        );
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.timeout_hours, 2.0);
    }

    #[test]
    fn preset_to_config_deep_dive() {
        let config = ResearchService::<MockProcessRepository>::preset_to_config(
            ResearchDepthPreset::DeepDive,
        );
        assert_eq!(config.max_iterations, 200);
        assert_eq!(config.timeout_hours, 8.0);
    }

    #[test]
    fn preset_to_config_exhaustive() {
        let config = ResearchService::<MockProcessRepository>::preset_to_config(
            ResearchDepthPreset::Exhaustive,
        );
        assert_eq!(config.max_iterations, 500);
        assert_eq!(config.timeout_hours, 24.0);
    }

    #[test]
    fn get_all_presets_returns_4() {
        let presets = ResearchService::<MockProcessRepository>::get_all_presets();
        assert_eq!(presets.len(), 4);
    }

    #[test]
    fn should_checkpoint_uses_process_method() {
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // interval = 5

        process.progress.current_iteration = 4;
        assert!(!ResearchService::<MockProcessRepository>::should_checkpoint(&process));

        process.progress.current_iteration = 5;
        assert!(ResearchService::<MockProcessRepository>::should_checkpoint(&process));
    }

    #[test]
    fn is_max_iterations_reached_uses_process_method() {
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // max = 10

        process.progress.current_iteration = 9;
        assert!(!ResearchService::<MockProcessRepository>::is_max_iterations_reached(&process));

        process.progress.current_iteration = 10;
        assert!(ResearchService::<MockProcessRepository>::is_max_iterations_reached(&process));
    }

    #[test]
    fn progress_percentage_uses_process_method() {
        let brief = create_test_brief();
        let mut process = ResearchProcess::new("Test", brief, "agent")
            .with_preset(ResearchDepthPreset::QuickScan); // max = 10

        assert_eq!(
            ResearchService::<MockProcessRepository>::progress_percentage(&process),
            0.0
        );

        process.progress.current_iteration = 5;
        assert_eq!(
            ResearchService::<MockProcessRepository>::progress_percentage(&process),
            50.0
        );
    }

    // ==================== Integration Scenario Tests ====================

    #[tokio::test]
    async fn research_lifecycle_scenario() {
        let (service, _) = create_service();

        // Start research
        let process = service
            .start_research_with_preset(
                "Full Lifecycle",
                create_test_brief(),
                "deep-researcher",
                ResearchDepthPreset::QuickScan,
            )
            .await
            .unwrap();

        let id = process.id.clone();
        assert_eq!(process.status(), ResearchProcessStatus::Running);

        // Advance iterations
        service.advance_iteration(&id).await.unwrap();
        service.advance_iteration(&id).await.unwrap();

        // Create checkpoint
        let artifact_id = ArtifactId::from_string("checkpoint-1");
        let process = service.checkpoint(&id, artifact_id).await.unwrap();
        assert!(process.progress.last_checkpoint.is_some());

        // Pause
        let process = service.pause_research(&id).await.unwrap();
        assert_eq!(process.status(), ResearchProcessStatus::Paused);

        // Resume
        let process = service.resume_research(&id).await.unwrap();
        assert_eq!(process.status(), ResearchProcessStatus::Running);

        // Complete
        let process = service.complete(&id).await.unwrap();
        assert_eq!(process.status(), ResearchProcessStatus::Completed);
        assert!(process.is_terminal());
    }

    #[tokio::test]
    async fn research_failure_scenario() {
        let (service, _) = create_service();

        // Start research
        let process = service
            .start_research_with_preset(
                "Failing Research",
                create_test_brief(),
                "agent",
                ResearchDepthPreset::QuickScan,
            )
            .await
            .unwrap();

        let id = process.id.clone();

        // Fail the research
        let process = service.fail(&id, "Network timeout").await.unwrap();
        assert_eq!(process.status(), ResearchProcessStatus::Failed);
        assert_eq!(
            process.progress.error_message,
            Some("Network timeout".to_string())
        );
    }
}
