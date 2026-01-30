// Type definitions for Ideation commands
// Input types, response types, and conversions

use serde::{Deserialize, Serialize};

use crate::domain::entities::{
    DependencyGraph, IdeationSession, TaskProposal,
};

// Re-export shared ChatMessageResponse
pub use crate::commands::chat_responses::ChatMessageResponse;

// ============================================================================
// Session Types
// ============================================================================

/// Input for creating a new ideation session
#[derive(Debug, Deserialize)]
pub struct CreateSessionInput {
    pub project_id: String,
    pub title: Option<String>,
    pub seed_task_id: Option<String>,
}

/// Response wrapper for ideation session operations
#[derive(Debug, Serialize)]
pub struct IdeationSessionResponse {
    pub id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: String,
    pub plan_artifact_id: Option<String>,
    pub seed_task_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub converted_at: Option<String>,
}

impl From<IdeationSession> for IdeationSessionResponse {
    fn from(session: IdeationSession) -> Self {
        Self {
            id: session.id.as_str().to_string(),
            project_id: session.project_id.as_str().to_string(),
            title: session.title,
            status: session.status.to_string(),
            plan_artifact_id: session.plan_artifact_id.map(|id| id.as_str().to_string()),
            seed_task_id: session.seed_task_id.map(|id| id.as_str().to_string()),
            created_at: session.created_at.to_rfc3339(),
            updated_at: session.updated_at.to_rfc3339(),
            archived_at: session.archived_at.map(|dt| dt.to_rfc3339()),
            converted_at: session.converted_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

/// Response for session with proposals and messages
#[derive(Debug, Serialize)]
pub struct SessionWithDataResponse {
    pub session: IdeationSessionResponse,
    pub proposals: Vec<TaskProposalResponse>,
    pub messages: Vec<ChatMessageResponse>,
}

// ============================================================================
// Proposal Types
// ============================================================================

/// Input for creating a new task proposal
#[derive(Debug, Deserialize)]
pub struct CreateProposalInput {
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub priority: Option<String>,
    pub complexity: Option<String>,
}

/// Input for updating a task proposal
#[derive(Debug, Deserialize)]
pub struct UpdateProposalInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub steps: Option<Vec<String>>,
    pub acceptance_criteria: Option<Vec<String>>,
    pub user_priority: Option<String>,
    pub complexity: Option<String>,
}

/// Response for TaskProposal
#[derive(Debug, Serialize)]
pub struct TaskProposalResponse {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub steps: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub suggested_priority: String,
    pub priority_score: i32,
    pub priority_reason: Option<String>,
    pub estimated_complexity: String,
    pub user_priority: Option<String>,
    pub user_modified: bool,
    pub status: String,
    pub selected: bool,
    pub created_task_id: Option<String>,
    pub plan_artifact_id: Option<String>,
    pub plan_version_at_creation: Option<u32>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TaskProposal> for TaskProposalResponse {
    fn from(proposal: TaskProposal) -> Self {
        // Parse JSON strings to Vec<String>, defaulting to empty vec
        let steps: Vec<String> = proposal
            .steps
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let acceptance_criteria: Vec<String> = proposal
            .acceptance_criteria
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Self {
            id: proposal.id.as_str().to_string(),
            session_id: proposal.session_id.as_str().to_string(),
            title: proposal.title,
            description: proposal.description,
            category: proposal.category.to_string(),
            steps,
            acceptance_criteria,
            suggested_priority: proposal.suggested_priority.to_string(),
            priority_score: proposal.priority_score,
            priority_reason: proposal.priority_reason,
            estimated_complexity: proposal.estimated_complexity.to_string(),
            user_priority: proposal.user_priority.map(|p| p.to_string()),
            user_modified: proposal.user_modified,
            status: proposal.status.to_string(),
            selected: proposal.selected,
            created_task_id: proposal.created_task_id.map(|id| id.as_str().to_string()),
            plan_artifact_id: proposal.plan_artifact_id.map(|id| id.as_str().to_string()),
            plan_version_at_creation: proposal.plan_version_at_creation,
            sort_order: proposal.sort_order,
            created_at: proposal.created_at.to_rfc3339(),
            updated_at: proposal.updated_at.to_rfc3339(),
        }
    }
}

/// Response for priority assessment
#[derive(Debug, Serialize)]
pub struct PriorityAssessmentResponse {
    pub proposal_id: String,
    pub priority: String,
    pub score: i32,
    pub reason: String,
}

// ============================================================================
// Dependency and Graph Types
// ============================================================================

/// Response for DependencyGraph
#[derive(Debug, Serialize)]
pub struct DependencyGraphResponse {
    pub nodes: Vec<DependencyGraphNodeResponse>,
    pub edges: Vec<DependencyGraphEdgeResponse>,
    pub critical_path: Vec<String>,
    pub has_cycles: bool,
    pub cycles: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Serialize)]
pub struct DependencyGraphNodeResponse {
    pub proposal_id: String,
    pub title: String,
    pub in_degree: usize,
    pub out_degree: usize,
}

#[derive(Debug, Serialize)]
pub struct DependencyGraphEdgeResponse {
    pub from: String,
    pub to: String,
}

impl From<DependencyGraph> for DependencyGraphResponse {
    fn from(graph: DependencyGraph) -> Self {
        Self {
            nodes: graph
                .nodes
                .into_iter()
                .map(|n| DependencyGraphNodeResponse {
                    proposal_id: n.proposal_id.as_str().to_string(),
                    title: n.title,
                    in_degree: n.in_degree,
                    out_degree: n.out_degree,
                })
                .collect(),
            edges: graph
                .edges
                .into_iter()
                .map(|e| DependencyGraphEdgeResponse {
                    from: e.from.as_str().to_string(),
                    to: e.to.as_str().to_string(),
                })
                .collect(),
            critical_path: graph
                .critical_path
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            has_cycles: graph.has_cycles,
            cycles: graph.cycles.map(|cs| {
                cs.into_iter()
                    .map(|c| c.into_iter().map(|id| id.as_str().to_string()).collect())
                    .collect()
            }),
        }
    }
}

/// Input for apply proposals
#[derive(Debug, Deserialize)]
pub struct ApplyProposalsInput {
    pub session_id: String,
    pub proposal_ids: Vec<String>,
    pub target_column: String,
    pub preserve_dependencies: bool,
}

/// Response for apply proposals
#[derive(Debug, Serialize)]
pub struct ApplyProposalsResultResponse {
    pub created_task_ids: Vec<String>,
    pub dependencies_created: usize,
    pub warnings: Vec<String>,
    pub session_converted: bool,
}

// ============================================================================
// Chat Message Types
// ============================================================================

/// Input for sending a chat message
#[derive(Debug, Deserialize)]
pub struct SendChatMessageInput {
    /// Session ID for session context (optional, mutually exclusive with project_id and task_id)
    pub session_id: Option<String>,
    /// Project ID for project context (optional, only used if session_id is None)
    pub project_id: Option<String>,
    /// Task ID for task context (optional, only used if session_id and project_id are None)
    pub task_id: Option<String>,
    /// Role of the message sender
    pub role: String,
    /// Message content (supports Markdown)
    pub content: String,
    /// Optional metadata (JSON)
    pub metadata: Option<String>,
    /// Optional parent message ID for threading
    pub parent_message_id: Option<String>,
}

// ============================================================================
// Orchestrator Types
// ============================================================================

/// Input for sending a message to the orchestrator
#[derive(Debug, Deserialize)]
pub struct SendOrchestratorMessageInput {
    pub session_id: String,
    pub content: String,
}

/// Response from the orchestrator
#[derive(Debug, Serialize)]
pub struct OrchestratorMessageResponse {
    pub response_text: String,
    pub proposals_created: Vec<TaskProposalResponse>,
    pub tool_calls: Vec<ToolCallResultResponse>,
}

/// Tool call result response
#[derive(Debug, Serialize)]
pub struct ToolCallResultResponse {
    pub tool_name: String,
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}
