// Request/Response types for HTTP server endpoints

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::application::{AppState, TeamStateTracker};
use crate::commands::ExecutionState;
use crate::domain::entities::{
    Artifact, ArtifactContent, MemoryEntry, StepProgressSummary, TaskProposal, TaskStep,
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
    pub team_tracker: TeamStateTracker,
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

/// Single dependency suggestion from AI
#[derive(Debug, Deserialize)]
pub struct DependencySuggestion {
    pub proposal_id: String,
    pub depends_on_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Request to apply AI-suggested dependencies (replaces all existing)
#[derive(Debug, Deserialize)]
pub struct ApplyDependencySuggestionsRequest {
    pub session_id: String,
    pub dependencies: Vec<DependencySuggestion>,
}

/// Response for apply_proposal_dependencies
#[derive(Debug, Serialize)]
pub struct ApplyDependenciesResponse {
    pub success: bool,
    pub applied_count: usize,
    pub skipped_count: usize,
    pub message: String,
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

/// Lightweight proposal summary for list endpoint
#[derive(Debug, Serialize)]
pub struct ProposalSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub priority: String,
    pub depends_on: Vec<String>,
    pub plan_artifact_id: Option<String>,
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
    pub analysis_in_progress: bool,
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
// Request/Response Types - Team Endpoints
// ============================================================================

/// POST /api/team/plan — request approval for a team plan
#[derive(Debug, Deserialize)]
pub struct RequestTeamPlanRequest {
    pub process: String,
    pub teammates: Vec<TeamPlanTeammate>,
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
// Common Response Types
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}
