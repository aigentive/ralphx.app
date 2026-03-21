// Request/Response types for HTTP server endpoints

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::{AppState, TeamService, TeamStateTracker};
use crate::commands::ExecutionState;
use crate::domain::entities::{
    Artifact, ArtifactContent, AuditLogEntry, MemoryEntry, StepProgressSummary, TaskProposal,
    TaskStep,
};
use crate::http_server::handlers::artifacts::EditError;

// ============================================================================
// HTTP Server State
// ============================================================================

/// Combined state for HTTP server handlers
/// Includes both AppState and ExecutionState for task transitions
#[derive(Clone)]
pub struct HttpServerState {
    pub app_state: Arc<AppState>,
    pub execution_state: Arc<ExecutionState>,
    pub team_tracker: TeamStateTracker,
    pub team_service: Arc<TeamService>,
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
// Request/Response Types - Child Session Status + Messaging
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChildSessionStatusParams {
    pub include_messages: Option<bool>,
    pub message_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdeationSessionSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub session_purpose: Option<String>,
    pub parent_session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStateInfo {
    pub is_running: bool,
    pub started_at: Option<String>,
    pub last_active_at: Option<String>,
    pub pid: Option<u32>,
    pub estimated_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationInfo {
    pub status: String,
    pub generation: i32,
    pub current_round: Option<u32>,
    pub gap_score: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatMessageSummary {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildSessionStatusResponse {
    pub session: IdeationSessionSummary,
    pub agent_state: AgentStateInfo,
    pub verification: Option<VerificationInfo>,
    pub recent_messages: Option<Vec<ChatMessageSummary>>,
}

#[derive(Debug, Deserialize)]
pub struct SendSessionMessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendSessionMessageResponse {
    pub delivery_status: String,
    pub conversation_id: Option<String>,
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
    /// Optional list of proposal IDs this proposal depends on
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Optional target project ID for cross-project proposal execution
    pub target_project: Option<String>,
    /// Expected total number of proposals for this session (set-once gating)
    pub expected_proposal_count: Option<u32>,
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
    /// Additive: proposal IDs this proposal should depend on
    #[serde(default)]
    pub add_depends_on: Vec<String>,
    /// Additive: proposal IDs this proposal should block (reverse direction)
    #[serde(default)]
    pub add_blocks: Vec<String>,
    /// Optional target project ID for cross-project proposal execution
    pub target_project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FinalizeProposalsRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct FinalizeProposalsResponse {
    pub created_task_ids: Vec<String>,
    pub dependencies_created: u32,
    pub session_status: String,
    pub execution_plan_id: Option<String>,
    pub warnings: Vec<String>,
    pub project_id: String,
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
    /// Partial failure contract: non-fatal dependency errors encountered during create/update
    pub dependency_errors: Vec<String>,
    /// Optional target project ID for cross-project proposal execution
    pub target_project: Option<String>,
    /// Whether the auto-accept pipeline was triggered for this session (always false — kept for backward compat)
    pub auto_accept_triggered: bool,
    /// Whether the session is ready to finalize (expected proposal count reached)
    pub ready_to_finalize: bool,
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
            dependency_errors: Vec::new(),
            target_project: proposal.target_project.clone(),
            auto_accept_triggered: false,
            ready_to_finalize: false,
        }
    }
}

/// Lightweight proposal summary for list endpoint
#[derive(Debug, Serialize)]
pub struct ProposalSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub priority: String,
    pub depends_on: Vec<String>,
    pub plan_artifact_id: Option<String>,
    /// Optional target project ID for cross-project proposal execution
    pub target_project: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListProposalsResponse {
    pub proposals: Vec<ProposalSummary>,
    pub count: usize,
}

/// Full proposal details for get endpoint
#[derive(Debug, Serialize)]
pub struct ProposalDetailResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub priority: String,
    pub steps: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub depends_on: Vec<String>,
    pub plan_artifact_id: Option<String>,
    pub created_at: String,
    /// Optional target project ID for cross-project proposal execution
    pub target_project: Option<String>,
}

// ============================================================================
// Request/Response Types - Dependency Analysis
// ============================================================================

/// Node in dependency analysis response
#[derive(Debug, Serialize)]
pub struct DependencyNodeResponse {
    pub id: String,
    pub title: String,
    pub in_degree: usize,
    pub out_degree: usize,
    pub is_root: bool,
    pub is_blocker: bool,
}

/// Edge in dependency analysis response
#[derive(Debug, Serialize)]
pub struct DependencyEdgeResponse {
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
}

/// Summary statistics for dependency analysis
#[derive(Debug, Serialize)]
pub struct DependencyAnalysisSummary {
    pub total_proposals: usize,
    pub root_count: usize,
    pub leaf_count: usize,
    pub max_depth: usize,
}

/// Response for analyze_session_dependencies endpoint
#[derive(Debug, Serialize)]
pub struct AnalyzeDependenciesResponse {
    pub nodes: Vec<DependencyNodeResponse>,
    pub edges: Vec<DependencyEdgeResponse>,
    pub critical_path: Vec<String>,
    pub critical_path_length: usize,
    pub has_cycles: bool,
    pub cycles: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub summary: DependencyAnalysisSummary,
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

/// Request body for POST /api/external/projects
/// Registers a directory as a RalphX project (creates dir + git if needed).
#[derive(Debug, Deserialize)]
pub struct RegisterProjectExternalRequest {
    pub working_directory: String,
    pub name: Option<String>,
}

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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReviewIssue {
    pub severity: String, // "critical" | "major" | "minor" | "suggestion"
    pub file: Option<String>,
    pub line: Option<u32>,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteReviewRequest {
    pub task_id: String,
    pub decision: String, // "approved" | "needs_changes" | "escalate"
    pub summary: Option<String>,
    pub feedback: Option<String>,
    pub issues: Option<Vec<ReviewIssue>>,
    pub escalation_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReviewNoteResponse {
    pub id: String,
    pub reviewer: String,
    pub outcome: String,
    pub summary: Option<String>,
    pub notes: Option<String>,
    pub issues: Option<Vec<ReviewIssue>>,
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
    #[serde(default)]
    pub tool_input: serde_json::Value,
    pub context: Option<String>,
    // Agent identity fields (optional for backward compat)
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub context_type: Option<String>,
    #[serde(default)]
    pub context_id: Option<String>,
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
    #[serde(default)]
    pub caller_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EditPlanArtifactRequest {
    pub artifact_id: String,
    pub edits: Vec<PlanEdit>,
    #[serde(default)]
    pub caller_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlanEdit {
    pub old_text: String,
    pub new_text: String,
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
    /// The ideation session this plan belongs to (for scoped notifications)
    pub session_id: Option<String>,
    /// Whether proposals were already re-linked to the new artifact ID server-side.
    /// When true, the UI only needs to refresh — no client-side re-linking is needed.
    pub proposals_relinked: bool,
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
    /// The artifact ID that was replaced (only set on update responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_artifact_id: Option<String>,
    /// The ideation session this artifact belongs to (only set on update responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Whether this plan was inherited from a parent session (only set on get_session_plan responses).
    /// When true, the plan is read-only — use create_plan_artifact to create a session-specific plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_inherited: Option<bool>,
    /// The working directory of the project this session belongs to (only set on get_session_plan responses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_working_directory: Option<String>,
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
            previous_artifact_id: None,
            session_id: None,
            is_inherited: None,
            project_working_directory: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchArtifactsRequest {
    pub project_id: String,
    pub query: String,
    pub artifact_types: Option<Vec<String>>,
}

/// Summary of an artifact version for history display
#[derive(Debug, Serialize)]
pub struct ArtifactVersionSummaryResponse {
    pub id: String,
    pub version: u32,
    pub name: String,
    pub created_at: String,
}

impl From<crate::domain::repositories::ArtifactVersionSummary> for ArtifactVersionSummaryResponse {
    fn from(summary: crate::domain::repositories::ArtifactVersionSummary) -> Self {
        Self {
            id: summary.id.to_string(),
            version: summary.version,
            name: summary.name,
            created_at: summary.created_at.to_rfc3339(),
        }
    }
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
    pub parent_step_id: Option<String>, // NEW: create as sub-step
    pub scope_context: Option<String>,  // NEW: STRICT SCOPE JSON
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
    pub parent_step_id: Option<String>,
    pub scope_context: Option<String>,
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
            parent_step_id: step.parent_step_id.map(|id| id.as_str().to_string()),
            scope_context: step.scope_context,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TaskSummaryForStep {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub internal_status: String,
}

#[derive(Debug, Serialize)]
pub struct StepContextResponse {
    pub step: StepResponse,
    pub parent_step: Option<StepResponse>,
    pub task_summary: TaskSummaryForStep,
    pub scope_context: Option<String>,
    pub sibling_steps: Vec<StepResponse>,
    pub step_progress: StepProgressSummary,
    pub context_hints: Vec<String>,
}

// ============================================================================
// Request/Response Types - Review Issues
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MarkIssueInProgressRequest {
    pub issue_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MarkIssueAddressedRequest {
    pub issue_id: String,
    pub resolution_notes: String,
    pub attempt_number: i32,
}

// ============================================================================
// Request/Response Types - Questions (AskUserQuestion)
// ============================================================================

/// Option in a question request
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QuestionOptionInput {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QuestionRequestInput {
    pub session_id: String,
    pub question: String,
    pub header: Option<String>,
    #[serde(default)]
    pub options: Vec<QuestionOptionInput>,
    #[serde(default)]
    pub multi_select: bool,
}

#[derive(Debug, Serialize)]
pub struct QuestionRequestResponse {
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveQuestionInput {
    pub request_id: String,
    pub selected_options: Vec<String>,
    pub text: Option<String>,
}

// ============================================================================
// Request/Response Types - Memory (read + write tools)
// ============================================================================

#[derive(Debug, Serialize)]
pub struct MemoryEntryResponse {
    pub id: String,
    pub project_id: String,
    pub bucket: String,
    pub title: String,
    pub summary: String,
    pub details_markdown: String,
    pub scope_paths: Vec<String>,
    pub source_context_type: Option<String>,
    pub source_context_id: Option<String>,
    pub source_conversation_id: Option<String>,
    pub source_rule_file: Option<String>,
    pub quality_score: Option<f64>,
    pub status: String,
    pub content_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MemoryEntry> for MemoryEntryResponse {
    fn from(entry: MemoryEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            project_id: entry.project_id.to_string(),
            bucket: entry.bucket.to_string(),
            title: entry.title,
            summary: entry.summary,
            details_markdown: entry.details_markdown,
            scope_paths: entry.scope_paths,
            source_context_type: entry.source_context_type,
            source_context_id: entry.source_context_id,
            source_conversation_id: entry.source_conversation_id,
            source_rule_file: entry.source_rule_file,
            quality_score: entry.quality_score,
            status: entry.status.to_string(),
            content_hash: entry.content_hash,
            created_at: entry.created_at.to_rfc3339(),
            updated_at: entry.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchMemoriesRequest {
    pub project_id: String,
    pub query: Option<String>,
    pub bucket: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchMemoriesResponse {
    pub memories: Vec<MemoryEntryResponse>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct GetMemoryRequest {
    pub memory_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetMemoryResponse {
    pub memory: Option<MemoryEntryResponse>,
}

#[derive(Debug, Deserialize)]
pub struct GetMemoriesForPathsRequest {
    pub project_id: String,
    pub paths: Vec<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct GetMemoriesForPathsResponse {
    pub memories: Vec<MemoryEntryResponse>,
    pub count: usize,
}

/// Single memory entry to upsert
#[derive(Debug, Deserialize)]
pub struct MemoryEntryInput {
    pub bucket: String, // architecture_patterns | implementation_discoveries | operational_playbooks
    pub title: String,
    pub summary: String,
    pub details_markdown: String,
    pub scope_paths: Vec<String>, // glob patterns for path scoping
    pub source_context_type: Option<String>,
    pub source_context_id: Option<String>,
    pub source_conversation_id: Option<String>,
    pub quality_score: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertMemoriesRequest {
    pub project_id: String,
    pub memories: Vec<MemoryEntryInput>,
}

#[derive(Debug, Serialize)]
pub struct UpsertMemoriesResponse {
    pub inserted: usize,
    pub skipped: usize,
    pub failed: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct MarkMemoryObsoleteRequest {
    pub memory_id: String,
}

#[derive(Debug, Serialize)]
pub struct MarkMemoryObsoleteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshMemoryRuleIndexRequest {
    pub project_id: String,
    pub scope_key: Option<String>, // if None, refresh all
}

#[derive(Debug, Serialize)]
pub struct RefreshMemoryRuleIndexResponse {
    pub files_refreshed: usize,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct IngestRuleFileRequest {
    pub project_id: String,
    pub rule_file_path: String, // relative to project root (e.g., ".claude/rules/task-state-machine.md")
}

#[derive(Debug, Serialize)]
pub struct IngestRuleFileResponse {
    pub memories_created: usize,
    pub memories_updated: usize,
    pub file_rewritten: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct RebuildArchiveSnapshotsRequest {
    pub project_id: String,
}

#[derive(Debug, Serialize)]
pub struct RebuildArchiveSnapshotsResponse {
    pub job_id: String,
    pub message: String,
}

// ============================================================================
// Request/Response Types - Session Linking
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateChildSessionRequest {
    pub parent_session_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_inherit_context")]
    pub inherit_context: bool,
    pub initial_prompt: Option<String>,
    /// Team mode override: "solo", "research", or "debate"
    /// If omitted and inherit_context=true, inherits from parent session
    pub team_mode: Option<String>,
    /// Team constraints override (max_teammates, model_ceiling, etc.)
    /// If omitted and inherit_context=true, inherits from parent session
    pub team_config: Option<TeamConfigInput>,
    /// Purpose of the child session: "general" (default) or "verification"
    pub purpose: Option<String>,
}

/// Team configuration input for create_child_session
#[derive(Debug, Deserialize, Serialize)]
pub struct TeamConfigInput {
    pub max_teammates: Option<i32>,
    pub model_ceiling: Option<String>,
    pub budget_limit: Option<f64>,
    pub composition_mode: Option<String>,
}

fn default_inherit_context() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct CreateChildSessionResponse {
    pub session_id: String,
    pub parent_session_id: String,
    pub title: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherited_plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_context: Option<ParentContextResponse>,
    /// Whether an orchestrator job was enqueued (true when description is provided)
    pub orchestration_triggered: bool,
    /// Resolved team mode: inherited from parent or explicitly set via request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_mode: Option<String>,
    /// Resolved team configuration: inherited from parent or explicitly set via request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_config: Option<TeamConfigInput>,
    /// Verification generation number; only set when purpose == "verification" and initialization succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ParentSessionSummary {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ParentProposalSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub priority: String,
    pub status: String,
    pub acceptance_criteria: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParentContextResponse {
    pub parent_session: ParentSessionSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_content: Option<String>,
    pub proposals: Vec<ParentProposalSummary>,
}

// ============================================================================
// Request/Response Types - Conversation Transcript
// ============================================================================

/// Single message in a transcript
#[derive(Debug, Serialize, Clone)]
pub struct TranscriptMessage {
    pub role: String, // "user", "assistant", etc.
    pub content: String,
    pub created_at: String, // RFC3339 timestamp
}

#[derive(Debug, Deserialize)]
pub struct GetConversationTranscriptRequest {
    pub conversation_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetConversationTranscriptResponse {
    pub conversation_id: String,
    pub messages: Vec<TranscriptMessage>,
    pub message_count: usize,
}

// ============================================================================
// Request/Response Types - Session Messages (Ideation Agent Context Recovery)
// ============================================================================

/// Default limit for session messages retrieval
fn default_session_messages_limit() -> usize {
    50
}

#[derive(Debug, Deserialize)]
pub struct GetSessionMessagesRequest {
    pub session_id: String,
    #[serde(default = "default_session_messages_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub include_tool_calls: bool,
}

/// Single message in a session messages response
#[derive(Debug, Serialize, Clone)]
pub struct SessionMessageResponse {
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct GetSessionMessagesResponse {
    pub messages: Vec<SessionMessageResponse>,
    pub count: usize,
    pub truncated: bool,
    pub total_available: usize,
}

// ============================================================================
// Request/Response Types - Team Endpoints
// ============================================================================

/// POST /api/team/plan/request — Phase 1: register team plan and return plan_id immediately
#[derive(Debug, Serialize)]
pub struct TeamPlanRegisterResponse {
    pub success: bool,
    pub plan_id: String,
    pub message: String,
    /// Whether the plan was auto-approved (spawning happened in Phase 1, no Phase 2 needed)
    pub auto_approved: bool,
    /// Teammates spawned during auto-approve (empty when auto_approved is false)
    pub teammates_spawned: Vec<SpawnedTeammateInfo>,
}

/// GET /api/team/plan/pending/:context_id — frontend reconciliation response
#[derive(Debug, Serialize)]
pub struct GetPendingPlanResponse {
    pub has_pending: bool,
    pub plan_id: Option<String>,
    pub context_id: String,
    pub process: Option<String>,
    pub teammate_count: Option<usize>,
    pub created_at: Option<String>,
}

/// POST /api/team/plan — request approval for a team plan
#[derive(Debug, Deserialize)]
pub struct RequestTeamPlanRequest {
    pub context_type: String,
    pub context_id: String,
    pub process: String,
    pub teammates: Vec<TeamPlanTeammate>,
    /// Team name from the lead agent's TeamCreate call.
    /// Must match the Claude Code team registry name so teammates join the right team.
    pub team_name: String,
    /// Lead agent's Claude Code session ID (from RALPHX_LEAD_SESSION_ID env var).
    /// When present, used as parent-session-id for teammate spawns instead of
    /// reading from the team config file.
    pub lead_session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamPlanTeammate {
    pub role: String,
    pub tools: Vec<String>,
    pub mcp_tools: Vec<String>,
    pub model: String,
    pub preset: Option<String>,
    pub prompt_summary: String,
    /// Full prompt for the teammate — required for batch-spawn on plan approval.
    /// When present, approve_team_plan can spawn without further MCP calls.
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RequestTeamPlanResponse {
    pub success: bool,
    pub plan_id: String,
    /// When approved: team name
    pub team_name: Option<String>,
    /// When approved: list of spawned teammates
    pub teammates_spawned: Vec<SpawnedTeammateInfo>,
    pub message: String,
}

/// POST /api/team/plan/approve — approve a validated team plan and batch-spawn
#[derive(Debug, Deserialize)]
pub struct ApproveTeamPlanRequest {
    pub plan_id: String,
    /// Context for the team (e.g., ideation session context_type + context_id).
    /// Required so the backend can create the team and attach teammates.
    pub context_type: String,
    pub context_id: String,
}

/// POST /api/team/plan/reject — reject a team plan
#[derive(Debug, Deserialize)]
pub struct RejectTeamPlanRequest {
    pub plan_id: String,
}

#[derive(Debug, Serialize)]
pub struct ApproveTeamPlanResponse {
    pub success: bool,
    pub team_name: String,
    pub teammates_spawned: Vec<SpawnedTeammateInfo>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpawnedTeammateInfo {
    pub name: String,
    pub role: String,
    pub model: String,
    pub color: String,
}

/// POST /api/team/spawn — request to spawn a single teammate
#[derive(Debug, Deserialize)]
pub struct RequestTeammateSpawnRequest {
    pub role: String,
    pub prompt: String,
    pub model: String,
    pub tools: Vec<String>,
    pub mcp_tools: Vec<String>,
    pub preset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RequestTeammateSpawnResponse {
    pub success: bool,
    pub message: String,
    pub teammate_name: String,
}

/// POST /api/team/artifact — create a team artifact
#[derive(Debug, Deserialize)]
pub struct CreateTeamArtifactRequest {
    pub session_id: String,
    pub title: String,
    pub content: String,
    pub artifact_type: String,
    pub related_artifact_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateTeamArtifactResponse {
    pub artifact_id: String,
}

/// GET /api/team/artifacts/:session_id response
#[derive(Debug, Serialize)]
pub struct TeamArtifactSummary {
    pub id: String,
    pub name: String,
    pub artifact_type: String,
    pub version: u32,
    pub content_preview: String,
    pub created_at: String,
    pub author_teammate: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetTeamArtifactsResponse {
    pub artifacts: Vec<TeamArtifactSummary>,
    pub count: usize,
}

/// GET /api/team/session_state/:session_id response
#[derive(Debug, Serialize)]
pub struct TeamSessionStateResponse {
    pub session_id: String,
    pub team_name: Option<String>,
    pub phase: String,
    pub team_composition: Vec<TeamCompositionEntry>,
    pub artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeamCompositionEntry {
    pub name: String,
    pub role: String,
    pub prompt: String,
    pub model: String,
}

/// POST /api/team/session_state — save team session state
#[derive(Debug, Deserialize)]
pub struct SaveTeamSessionStateRequest {
    pub session_id: String,
    pub team_composition: Vec<TeamCompositionEntry>,
    pub phase: String,
    pub artifact_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SaveTeamSessionStateResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Request/Response Types - Active Streaming State
// ============================================================================

/// Response for GET /api/conversations/:id/active-state
///
/// Returns the current streaming state for a conversation, used by frontend
/// to hydrate streaming UI when navigating to an active agent execution.
#[derive(Debug, Serialize)]
pub struct ActiveStateResponse {
    /// Whether an agent is currently running for this conversation
    pub is_active: bool,
    /// Tool calls currently in progress or recently completed
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ActiveToolCall>,
    /// Streaming tasks (subagents) currently running or completed
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub streaming_tasks: Vec<ActiveStreamingTask>,
    /// Partial text content accumulated from agent:chunk events
    #[serde(skip_serializing_if = "String::is_empty")]
    pub partial_text: String,
}

/// A tool call in the active state response.
///
/// Mirrors CachedToolCall from streaming_state_cache.rs for HTTP serialization.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveToolCall {
    /// Unique tool call ID (e.g., "toolu_01A...")
    pub id: String,
    /// Tool name (e.g., "bash", "read", "edit")
    pub name: String,
    /// Current arguments (may be partial during streaming)
    pub arguments: serde_json::Value,
    /// Result if completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Diff context for Edit/Write tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_context: Option<serde_json::Value>,
    /// Parent tool use ID for nested tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_use_id: Option<String>,
}

impl From<crate::application::chat_service::CachedToolCall> for ActiveToolCall {
    fn from(cached: crate::application::chat_service::CachedToolCall) -> Self {
        Self {
            id: cached.id,
            name: cached.name,
            arguments: cached.arguments,
            result: cached.result,
            diff_context: cached.diff_context,
            parent_tool_use_id: cached.parent_tool_use_id,
        }
    }
}

/// A streaming task in the active state response.
///
/// Mirrors CachedStreamingTask from streaming_state_cache.rs for HTTP serialization.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveStreamingTask {
    /// Tool use ID that started this task
    pub tool_use_id: String,
    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Subagent type (e.g., "ralphx:coder")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent_type: Option<String>,
    /// Model being used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current status: "running" or "completed"
    pub status: String,
    /// Teammate name if this is a team member task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teammate_name: Option<String>,
    /// Total tokens used by this task (from TaskCompleted stats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// Total tool uses count (from TaskCompleted stats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tool_uses: Option<u64>,
    /// Duration in milliseconds (from TaskCompleted stats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl From<crate::application::chat_service::CachedStreamingTask> for ActiveStreamingTask {
    fn from(cached: crate::application::chat_service::CachedStreamingTask) -> Self {
        Self {
            tool_use_id: cached.tool_use_id,
            description: cached.description,
            subagent_type: cached.subagent_type,
            model: cached.model,
            status: cached.status,
            teammate_name: cached.teammate_name,
            total_tokens: cached.total_tokens,
            total_tool_uses: cached.total_tool_uses,
            duration_ms: cached.duration_ms,
        }
    }
}

// ============================================================================
// Request/Response Types - Execution Complete
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExecutionCompleteRequest {
    pub summary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExecutionCompleteResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// Common Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

// ============================================================================
// HTTP Error Type
// ============================================================================

/// HTTP handler error that preserves validation messages in the response body.
///
/// `From<StatusCode>` allows existing `?` operators on `Result<T, StatusCode>`
/// to compile unchanged when handler return types use `HttpError` as the error type.
#[derive(Debug)]
pub struct HttpError {
    pub status: StatusCode,
    pub message: Option<String>,
}

impl HttpError {
    /// 422 Unprocessable Entity with an actionable message body.
    pub fn validation(message: String) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            message: Some(message),
        }
    }
}

impl From<StatusCode> for HttpError {
    fn from(status: StatusCode) -> Self {
        Self {
            status,
            message: None,
        }
    }
}

impl From<EditError> for HttpError {
    fn from(e: EditError) -> Self {
        match e {
            EditError::AnchorNotFound {
                edit_index,
                old_text_preview,
            } => HttpError::validation(format!(
                "Edit #{} failed: old_text not found in plan content. Preview: '{}'",
                edit_index, old_text_preview
            )),
            EditError::AmbiguousAnchor {
                edit_index,
                old_text_preview,
            } => HttpError::validation(format!(
                "Edit #{} failed: old_text matches multiple locations. Use a longer/more unique anchor. Preview: '{}'",
                edit_index, old_text_preview
            )),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        match self.message {
            Some(msg) => {
                (self.status, Json(serde_json::json!({"error": msg}))).into_response()
            }
            None => self.status.into_response(),
        }
    }
}

// ============================================================================
// Request/Response Types - API Key Management
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<i32>,
    pub project_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key: String,
    pub key_prefix: String,
    pub permissions: i32,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub permissions: i32,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
    pub project_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ListApiKeysResponse {
    pub keys: Vec<ApiKeyInfo>,
    pub count: usize,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyProjectsRequest {
    pub project_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RotateApiKeyResponse {
    pub id: String,
    pub new_key: String,
    pub key_prefix: String,
    pub old_key_grace_expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub entries: Vec<AuditLogEntry>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsRequest {
    pub permissions: i64,
}

// ============================================================================
// Request/Response Types - Plan Verification
// ============================================================================

/// A gap identified by the critic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationGapRequest {
    pub severity: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub why_it_matters: Option<String>,
    /// Which critic layer identified this gap: "layer1" | "layer2"
    #[serde(default)]
    pub source: Option<String>,
}

/// Request to update verification state (from MCP orchestrator)
#[derive(Debug, Deserialize)]
pub struct UpdateVerificationRequest {
    pub status: String, // "reviewing" | "needs_revision" | "verified" | "skipped"
    #[serde(default)]
    pub in_progress: bool,
    #[serde(default)]
    pub round: Option<u32>,
    #[serde(default)]
    pub gaps: Option<Vec<VerificationGapRequest>>,
    #[serde(default)]
    pub convergence_reason: Option<String>,
    #[serde(default)]
    pub max_rounds: Option<u32>,
    /// True if the critic output could not be parsed this round (parse failure tracking)
    #[serde(default)]
    pub parse_failed: Option<bool>,
    /// Generation counter for zombie protection — must match session's current generation
    /// when setting in_progress=true
    #[serde(default)]
    pub generation: Option<i32>,
}

/// A single verification gap in the API response (mirrors domain VerificationGap)
#[derive(Debug, Serialize)]
pub struct VerificationGapResponse {
    pub severity: String,
    pub category: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_it_matters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Per-round summary in the API response (round number derived from array index + 1)
#[derive(Debug, Serialize)]
pub struct VerificationRoundSummary {
    /// 1-based round number (derived from array index)
    pub round: u32,
    pub gap_score: u32,
    /// Deduplicated unique gap count (fingerprints.len() for historical rounds)
    pub gap_count: u32,
}

/// Response for GET/POST verification status
#[derive(Debug, Serialize)]
pub struct VerificationResponse {
    pub session_id: String,
    pub status: String,
    pub in_progress: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_round: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rounds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_score: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub convergence_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_round_index: Option<u32>,
    /// Full gap objects for the latest round (empty if no metadata)
    #[serde(default)]
    pub current_gaps: Vec<VerificationGapResponse>,
    /// Round history summaries — last 10 rounds in chronological order (empty if no metadata)
    #[serde(default)]
    pub rounds: Vec<VerificationRoundSummary>,
    /// Plan artifact version when verification ran — null if session has no linked plan
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_version: Option<u32>,
    /// Current verification generation counter
    pub verification_generation: i32,
}

/// Request to atomically revert plan + skip verification
#[derive(Debug, Deserialize)]
pub struct RevertAndSkipRequest {
    /// The plan artifact version (artifact_id) to restore content from
    pub plan_version_to_restore: String,
}

#[cfg(test)]
mod http_error_tests {
    use super::*;

    #[test]
    fn test_validation_error_has_422_status_and_message() {
        let err = HttpError::validation(
            "Cannot modify accepted session. Reopen it first.".to_string(),
        );
        assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            err.message.as_deref(),
            Some("Cannot modify accepted session. Reopen it first.")
        );
    }

    #[test]
    fn test_validation_error_message_is_not_sensitive() {
        // Verify the message is a user-actionable string, not a raw DB error
        let err = HttpError::validation("Validation error: Cannot modify archived session. Reopen it first.".to_string());
        let msg = err.message.unwrap();
        assert!(msg.contains("Reopen it first"), "Message should guide the user");
        assert!(!msg.contains("SQLITE"), "Should not leak DB internals");
        assert!(!msg.contains("rusqlite"), "Should not leak internal library names");
    }

    #[test]
    fn test_from_status_code_has_no_message() {
        let err = HttpError::from(StatusCode::NOT_FOUND);
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert!(err.message.is_none(), "StatusCode errors should have no body message");
    }

    #[test]
    fn test_from_internal_server_error_has_no_message() {
        let err = HttpError::from(StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(err.message.is_none(), "Internal errors should not expose messages");
    }

    #[tokio::test]
    async fn test_validation_error_into_response_status() {
        use axum::response::IntoResponse;
        let err = HttpError::validation("Cannot modify archived session.".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_status_only_error_into_response() {
        use axum::response::IntoResponse;
        let err = HttpError::from(StatusCode::NOT_FOUND);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
