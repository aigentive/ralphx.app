// Type definitions for IdeationService

use crate::domain::entities::{
    ChatMessage, IdeationSession, Priority, TaskCategory, TaskProposal,
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

/// Options for creating a new proposal
#[derive(Debug, Clone)]
pub struct CreateProposalOptions {
    /// Title for the proposal
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Task category
    pub category: TaskCategory,
    /// Suggested priority
    pub suggested_priority: Priority,
    /// Optional implementation steps (JSON array)
    pub steps: Option<String>,
    /// Optional acceptance criteria (JSON array)
    pub acceptance_criteria: Option<String>,
}

/// Options for updating a proposal
#[derive(Debug, Clone, Default)]
pub struct UpdateProposalOptions {
    /// New title (if provided)
    pub title: Option<String>,
    /// New description (if provided)
    pub description: Option<Option<String>>,
    /// New category (if provided)
    pub category: Option<TaskCategory>,
    /// New steps (if provided)
    pub steps: Option<Option<String>>,
    /// New acceptance criteria (if provided)
    pub acceptance_criteria: Option<Option<String>>,
    /// User priority override (if provided)
    pub user_priority: Option<Priority>,
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
