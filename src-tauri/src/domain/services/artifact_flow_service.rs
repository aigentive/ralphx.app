// ArtifactFlowService - domain service for artifact flow automation
//
// Provides business logic for:
// - Evaluating artifact flow triggers on events
// - Executing flow steps (copy, spawn process)
// - Managing flow registration and state

use std::sync::Arc;

use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactFlow, ArtifactFlowContext, ArtifactFlowEngine,
    ArtifactFlowEvaluation, ArtifactFlowId, ArtifactFlowStep,
};
use crate::domain::repositories::ArtifactFlowRepository;
use crate::error::AppResult;

/// Result of executing a flow step
#[derive(Debug, Clone)]
pub enum StepExecutionResult {
    /// Copy step executed successfully
    Copied {
        artifact_id: String,
        target_bucket: ArtifactBucketId,
    },
    /// Spawn process step queued (process creation handled externally)
    ProcessSpawned {
        process_type: String,
        agent_profile: String,
    },
    /// Emit event step - carries event name and artifact ID for frontend handling
    EventEmitted {
        event_name: String,
        artifact_id: String,
    },
    /// Find linked proposals step - returns proposal IDs linked to the artifact
    LinkedProposalsFound {
        artifact_id: String,
        /// Placeholder: actual proposal IDs will be populated by the caller
        /// since the service doesn't have access to the proposal repository
        proposal_ids: Vec<String>,
    },
}

/// Result of executing a complete flow
#[derive(Debug, Clone)]
pub struct FlowExecutionResult {
    /// The flow that was executed
    pub flow_id: ArtifactFlowId,
    /// The flow name
    pub flow_name: String,
    /// Results of each step
    pub step_results: Vec<StepExecutionResult>,
}

/// Service for artifact flow automation
pub struct ArtifactFlowService<R: ArtifactFlowRepository> {
    flow_repo: Arc<R>,
    engine: ArtifactFlowEngine,
}

impl<R: ArtifactFlowRepository> ArtifactFlowService<R> {
    /// Create a new ArtifactFlowService with the given repository
    pub fn new(flow_repo: Arc<R>) -> Self {
        Self {
            flow_repo,
            engine: ArtifactFlowEngine::new(),
        }
    }

    /// Load all active flows from the repository into the engine
    pub async fn load_active_flows(&mut self) -> AppResult<usize> {
        let flows = self.flow_repo.get_active().await?;
        let count = flows.len();
        self.engine = ArtifactFlowEngine::new();
        self.engine.register_flows(flows);
        Ok(count)
    }

    /// Register a flow with the engine (does not persist)
    pub fn register_flow(&mut self, flow: ArtifactFlow) {
        self.engine.register_flow(flow);
    }

    /// Get the current flow count in the engine
    pub fn flow_count(&self) -> usize {
        self.engine.flow_count()
    }

    /// Evaluate flows when an artifact is created
    pub fn on_artifact_created(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_artifact_created(artifact)
    }

    /// Evaluate flows when an artifact is updated
    pub fn on_artifact_updated(&self, artifact: &Artifact) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_artifact_updated(artifact)
    }

    /// Evaluate flows when a task is completed
    pub fn on_task_completed(
        &self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_task_completed(task_id, artifact)
    }

    /// Evaluate flows when a process is completed
    pub fn on_process_completed(
        &self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> Vec<ArtifactFlowEvaluation> {
        self.engine.on_process_completed(process_id, artifact)
    }

    /// Evaluate flows for a given context
    pub fn evaluate_flows(&self, context: &ArtifactFlowContext) -> Vec<ArtifactFlowEvaluation> {
        self.engine.evaluate_triggers(context)
    }

    /// Execute the steps of a flow evaluation.
    /// Returns the results of each step execution.
    /// Note: The actual artifact copy, process spawn, event emission, and proposal lookup
    /// are handled by the caller, as this service does not have direct access to
    /// artifact/process/proposal repositories or the Tauri app handle.
    pub fn execute_steps(
        &self,
        evaluation: &ArtifactFlowEvaluation,
        artifact: &Artifact,
    ) -> Vec<StepExecutionResult> {
        evaluation
            .steps
            .iter()
            .map(|step| match step {
                ArtifactFlowStep::Copy { to_bucket } => StepExecutionResult::Copied {
                    artifact_id: artifact.id.as_str().to_string(),
                    target_bucket: to_bucket.clone(),
                },
                ArtifactFlowStep::SpawnProcess {
                    process_type,
                    agent_profile,
                } => StepExecutionResult::ProcessSpawned {
                    process_type: process_type.clone(),
                    agent_profile: agent_profile.clone(),
                },
                ArtifactFlowStep::EmitEvent { event_name } => StepExecutionResult::EventEmitted {
                    event_name: event_name.clone(),
                    artifact_id: artifact.id.as_str().to_string(),
                },
                ArtifactFlowStep::FindLinkedProposals => StepExecutionResult::LinkedProposalsFound {
                    artifact_id: artifact.id.as_str().to_string(),
                    // Proposal IDs will be populated by the caller who has access to the proposal repo
                    proposal_ids: vec![],
                },
            })
            .collect()
    }

    /// Execute all steps for all matching flow evaluations.
    /// Returns execution results for each flow.
    pub fn execute_all_flows(
        &self,
        evaluations: &[ArtifactFlowEvaluation],
        artifact: &Artifact,
    ) -> Vec<FlowExecutionResult> {
        evaluations
            .iter()
            .map(|eval| FlowExecutionResult {
                flow_id: eval.flow_id.clone(),
                flow_name: eval.flow_name.clone(),
                step_results: self.execute_steps(eval, artifact),
            })
            .collect()
    }

    /// Get a flow from the repository by ID
    pub async fn get_flow(&self, id: &ArtifactFlowId) -> AppResult<Option<ArtifactFlow>> {
        self.flow_repo.get_by_id(id).await
    }

    /// Get all flows from the repository
    pub async fn get_all_flows(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.flow_repo.get_all().await
    }

    /// Get all active flows from the repository
    pub async fn get_active_flows(&self) -> AppResult<Vec<ArtifactFlow>> {
        self.flow_repo.get_active().await
    }

    /// Create a new flow in the repository
    pub async fn create_flow(&self, flow: ArtifactFlow) -> AppResult<ArtifactFlow> {
        self.flow_repo.create(flow).await
    }

    /// Update a flow in the repository
    pub async fn update_flow(&self, flow: &ArtifactFlow) -> AppResult<()> {
        self.flow_repo.update(flow).await
    }

    /// Delete a flow from the repository
    pub async fn delete_flow(&self, id: &ArtifactFlowId) -> AppResult<()> {
        self.flow_repo.delete(id).await
    }

    /// Set the active state of a flow
    pub async fn set_flow_active(&self, id: &ArtifactFlowId, is_active: bool) -> AppResult<()> {
        self.flow_repo.set_active(id, is_active).await
    }

    /// Check if a flow exists in the repository
    pub async fn flow_exists(&self, id: &ArtifactFlowId) -> AppResult<bool> {
        self.flow_repo.exists(id).await
    }

    /// Process an artifact_created event: evaluate triggers and return execution plan
    pub async fn process_artifact_created(
        &mut self,
        artifact: &Artifact,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_artifact_created(artifact);

        // Execute all matching flows
        Ok(self.execute_all_flows(&evaluations, artifact))
    }

    /// Process an artifact_updated event: evaluate triggers and return execution plan
    pub async fn process_artifact_updated(
        &mut self,
        artifact: &Artifact,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_artifact_updated(artifact);

        // Execute all matching flows
        Ok(self.execute_all_flows(&evaluations, artifact))
    }

    /// Process a task_completed event: evaluate triggers and return execution plan
    pub async fn process_task_completed(
        &mut self,
        task_id: &str,
        artifact: Option<&Artifact>,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_task_completed(task_id, artifact);

        // Execute all matching flows (if artifact provided)
        if let Some(artifact) = artifact {
            Ok(self.execute_all_flows(&evaluations, artifact))
        } else {
            Ok(evaluations
                .into_iter()
                .map(|eval| FlowExecutionResult {
                    flow_id: eval.flow_id,
                    flow_name: eval.flow_name,
                    step_results: vec![],
                })
                .collect())
        }
    }

    /// Process a process_completed event: evaluate triggers and return execution plan
    pub async fn process_process_completed(
        &mut self,
        process_id: &str,
        artifact: Option<&Artifact>,
    ) -> AppResult<Vec<FlowExecutionResult>> {
        // Ensure engine has latest flows
        self.load_active_flows().await?;

        // Evaluate which flows should trigger
        let evaluations = self.on_process_completed(process_id, artifact);

        // Execute all matching flows (if artifact provided)
        if let Some(artifact) = artifact {
            Ok(self.execute_all_flows(&evaluations, artifact))
        } else {
            Ok(evaluations
                .into_iter()
                .map(|eval| FlowExecutionResult {
                    flow_id: eval.flow_id,
                    flow_name: eval.flow_name,
                    step_results: vec![],
                })
                .collect())
        }
    }
}

#[cfg(test)]
#[path = "artifact_flow_service_tests.rs"]
mod tests;
