// Request/Response types for HTTP server endpoints

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{
    Artifact, ArtifactContent, TaskProposal, TaskStep,
};

// ============================================================================
// HTTP Server State
// ============================================================================

/// Combined state for HTTP server handlers
/// Includes both AppState and ExecutionState for task transitions
#[derive(Clone)]
pub struct HttpServerState {
    pub app_state: Arc<AppState>,
    pub execution_state: Arc<ExecutionState>,
}

// ============================================================================
// Request/Response Types - Ideation (Sessions)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateSessionTitleRequest {
    pub session_id: String,
    pub title: String,
}

// ============================================================================
// Request/Response Types - Ideation (Proposals)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateProposalRequest {
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProposalRequest {
    pub proposal_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub user_priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteProposalRequest {
    pub proposal_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AddDependencyRequest {
    pub proposal_id: String,
    pub depends_on_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: String,
    pub steps: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub created_at: String,
}

impl From<TaskProposal> for ProposalResponse {
    fn from(proposal: TaskProposal) -> Self {
        Self {
            id: proposal.id.to_string(),
            session_id: proposal.session_id.to_string(),
            title: proposal.title,
            description: proposal.description,
            category: proposal.category.to_string(),
            priority: proposal.suggested_priority.to_string(),
            steps: proposal.steps,
            acceptance_criteria: proposal.acceptance_criteria,
            created_at: proposal.created_at.to_rfc3339(),
        }
    }
}

// ============================================================================
// Request/Response Types - Tasks
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub task_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AddTaskNoteRequest {
    pub task_id: String,
    pub note: String,
}

#[derive(Debug, Deserialize)]
pub struct GetTaskDetailsRequest {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================================
// Request/Response Types - Projects
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ListTasksRequest {
    pub project_id: String,
    pub status: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskResponse>,
}

#[derive(Debug, Deserialize)]
pub struct SuggestTaskRequest {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub priority: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SuggestTaskResponse {
    pub task: TaskResponse,
}

// ============================================================================
// Request/Response Types - Reviews
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CompleteReviewRequest {
    pub task_id: String,
    pub decision: String, // "approved" | "needs_changes" | "escalate"
    pub comments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewNoteResponse {
    pub id: String,
    pub reviewer: String,
    pub outcome: String,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewNotesResponse {
    pub task_id: String,
    pub revision_count: u32,
    pub max_revisions: u32,
    pub reviews: Vec<ReviewNoteResponse>,
}

#[derive(Debug, Serialize)]
pub struct CompleteReviewResponse {
    pub success: bool,
    pub message: String,
    pub new_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveTaskRequest {
    pub task_id: String,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RequestTaskChangesRequest {
    pub task_id: String,
    pub feedback: String,
}

// ============================================================================
// Request/Response Types - Permissions
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct PermissionRequestInput {
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PermissionRequestResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolvePermissionInput {
    pub request_id: String,
    pub decision: String, // "allow" or "deny"
    pub message: Option<String>,
}

// ============================================================================
// Request/Response Types - Plan Artifacts
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreatePlanArtifactRequest {
    pub session_id: String,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlanArtifactRequest {
    pub artifact_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkProposalsToPlanRequest {
    pub proposal_ids: Vec<String>,
    pub artifact_id: String,
}

/// Payload for the plan:proposals_may_need_update event
/// Emitted when a plan artifact is updated and has linked proposals
#[derive(Debug, Clone, Serialize)]
pub struct PlanProposalsSyncPayload {
    /// The new artifact ID (new version)
    pub artifact_id: String,
    /// The previous artifact ID (the one that was updated)
    pub previous_artifact_id: String,
    /// IDs of proposals linked to the original plan
    pub proposal_ids: Vec<String>,
    /// The new version number
    pub new_version: u32,
}

#[derive(Debug, Serialize)]
pub struct ArtifactResponse {
    pub id: String,
    pub artifact_type: String,
    pub name: String,
    pub content: String,
    pub version: u32,
    pub created_at: String,
    pub created_by: String,
}

impl From<Artifact> for ArtifactResponse {
    fn from(artifact: Artifact) -> Self {
        let content = match &artifact.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };

        Self {
            id: artifact.id.to_string(),
            artifact_type: format!("{:?}", artifact.artifact_type),
            name: artifact.name,
            content,
            version: artifact.metadata.version,
            created_at: artifact.metadata.created_at.to_rfc3339(),
            created_by: artifact.metadata.created_by.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchArtifactsRequest {
    pub project_id: String,
    pub query: String,
    pub artifact_types: Option<Vec<String>>,
}

// ============================================================================
// Request/Response Types - Task Steps
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartStepRequest {
    pub step_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteStepRequest {
    pub step_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkipStepRequest {
    pub step_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct FailStepRequest {
    pub step_id: String,
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct AddStepRequest {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub after_step_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepResponse {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub completion_note: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<TaskStep> for StepResponse {
    fn from(step: TaskStep) -> Self {
        Self {
            id: step.id.as_str().to_string(),
            task_id: step.task_id.as_str().to_string(),
            title: step.title,
            description: step.description,
            status: step.status.to_db_string().to_string(),
            sort_order: step.sort_order,
            completion_note: step.completion_note,
            started_at: step.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: step.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

// ============================================================================
// Common Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}
