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
#[path = "research_service_tests.rs"]
mod tests;
