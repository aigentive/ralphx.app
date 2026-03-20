// Type definitions for IdeationService

use crate::domain::entities::{
    ChatMessage, IdeationSession, Priority, ProposalCategory, TaskProposal,
};

/// Configuration for plan artifacts in ideation flow
#[derive(Debug, Clone)]
pub struct PlanArtifactConfig {
    /// Artifact type to use for plans
    pub artifact_type: String,
    /// Bucket ID to store plans in
    pub bucket_id: String,
}

/// Data returned when fetching a session with all related data
#[derive(Debug, Clone)]
pub struct SessionWithData {
    /// The ideation session
    pub session: IdeationSession,
    /// All proposals in this session
    pub proposals: Vec<TaskProposal>,
    /// All messages in this session
    pub messages: Vec<ChatMessage>,
}

/// Tracks caller origin for field-level modification tracking.
///
/// Used by `update_proposal_impl()` to determine whether to set `user_modified = true`
/// and call `proposal.touch()` on changed fields.
///
/// Distinct from `ProposalOperation` (future) which will gate verification checks
/// based on the type of mutation (create/update/delete).
#[derive(Debug, Clone, Copy, Default)]
pub enum UpdateSource {
    /// API (HTTP/MCP) — agent-originated. No `user_modified` tracking.
    #[default]
    Api,
    /// Tauri IPC — user-originated. Sets `user_modified = true` per changed field + calls `touch()`.
    TauriIpc,
}

/// Options for creating a new proposal
#[derive(Debug, Clone)]
pub struct CreateProposalOptions {
    /// Title for the proposal
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Task category
    pub category: ProposalCategory,
    /// Suggested priority
    pub suggested_priority: Priority,
    /// Optional implementation steps (JSON array)
    pub steps: Option<String>,
    /// Optional acceptance criteria (JSON array)
    pub acceptance_criteria: Option<String>,
    /// Optional estimated complexity string (parsed to Complexity enum in impl)
    pub estimated_complexity: Option<String>,
    /// Optional target project ID for cross-project proposals
    pub target_project: Option<String>,
    /// List of proposal IDs to add as dependencies after creation
    pub depends_on: Vec<String>,
    /// Expected total number of proposals for this session (set-once gating)
    pub expected_proposal_count: Option<u32>,
}

/// Options for updating a proposal
#[derive(Debug, Clone, Default)]
pub struct UpdateProposalOptions {
    /// New title (if provided)
    pub title: Option<String>,
    /// New description (if provided)
    pub description: Option<Option<String>>,
    /// New category (if provided)
    pub category: Option<ProposalCategory>,
    /// New steps (if provided)
    pub steps: Option<Option<String>>,
    /// New acceptance criteria (if provided)
    pub acceptance_criteria: Option<Option<String>>,
    /// User priority override (if provided)
    pub user_priority: Option<Priority>,
    /// Estimated complexity string (parsed to Complexity enum in impl, if provided)
    pub estimated_complexity: Option<String>,
    /// Optional target project ID for cross-project proposals (None=not provided, Some(None)=clear, Some(Some(v))=set)
    pub target_project: Option<Option<String>>,
    /// Source of the update — controls `user_modified` tracking and `touch()` call
    pub source: UpdateSource,
    /// Additive: proposal IDs this proposal should depend on
    pub add_depends_on: Vec<String>,
    /// Additive: proposal IDs this proposal should block (reverse direction)
    pub add_blocks: Vec<String>,
}

/// Statistics for an ideation session
#[derive(Debug, Clone)]
pub struct SessionStats {
    /// Total number of proposals in the session
    pub total_proposals: u32,
    /// Number of selected proposals
    pub selected_proposals: u32,
    /// Total number of messages in the session
    pub message_count: u32,
}
