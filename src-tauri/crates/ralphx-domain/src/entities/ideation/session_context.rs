//! Parent session context value object for context inheritance

use serde::{Deserialize, Serialize};

use super::types::{ProposalCategory, ProposalStatus};
use crate::entities::{IdeationSessionId, TaskProposalId};

/// Summary of a single proposal for context inheritance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProposalSummary {
    /// Proposal ID
    pub id: TaskProposalId,
    /// Proposal title
    pub title: String,
    /// Task category
    pub category: ProposalCategory,
    /// Priority score (0-100)
    pub priority_score: i32,
    /// Current status
    pub status: ProposalStatus,
    /// Acceptance criteria (JSON array of strings)
    pub acceptance_criteria: Option<String>,
}

/// Context snapshot from a parent session
/// Contains actionable information without chat history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParentSessionContext {
    /// Parent session ID
    pub session_id: IdeationSessionId,
    /// Parent session title
    pub session_title: String,
    /// Parent session status (draft, active, archived)
    pub session_status: String,
    /// Plan artifact content (markdown)
    pub plan_content: Option<String>,
    /// List of proposals from parent session
    pub proposals: Vec<ContextProposalSummary>,
}

impl ParentSessionContext {
    /// Creates a new parent session context
    pub fn new(
        session_id: IdeationSessionId,
        session_title: impl Into<String>,
        session_status: impl Into<String>,
    ) -> Self {
        Self {
            session_id,
            session_title: session_title.into(),
            session_status: session_status.into(),
            plan_content: None,
            proposals: Vec::new(),
        }
    }

    /// Sets the plan content
    pub fn with_plan_content(mut self, content: impl Into<String>) -> Self {
        self.plan_content = Some(content.into());
        self
    }

    /// Sets the proposals list
    pub fn with_proposals(mut self, proposals: Vec<ContextProposalSummary>) -> Self {
        self.proposals = proposals;
        self
    }

    /// Returns true if the parent has a plan artifact
    pub fn has_plan(&self) -> bool {
        self.plan_content.is_some()
    }

    /// Returns the number of proposals
    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }

    /// Returns proposals filtered by status
    pub fn proposals_by_status(&self, status: ProposalStatus) -> Vec<&ContextProposalSummary> {
        self.proposals
            .iter()
            .filter(|p| p.status == status)
            .collect()
    }
}

#[cfg(test)]
#[path = "session_context_tests.rs"]
mod tests;
