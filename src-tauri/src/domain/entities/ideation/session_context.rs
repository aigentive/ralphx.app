//! Parent session context value object for context inheritance

use serde::{Deserialize, Serialize};

use super::types::{ProposalStatus, TaskCategory};
use crate::domain::entities::{IdeationSessionId, TaskProposalId};

/// Summary of a single proposal for context inheritance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProposalSummary {
    /// Proposal ID
    pub id: TaskProposalId,
    /// Proposal title
    pub title: String,
    /// Task category
    pub category: TaskCategory,
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
mod tests {
    use super::*;

    #[test]
    fn context_proposal_summary_serializes_to_json() {
        let summary = ContextProposalSummary {
            id: TaskProposalId::from_string("prop-123"),
            title: "Test proposal".to_string(),
            category: TaskCategory::Feature,
            priority_score: 75,
            status: ProposalStatus::Accepted,
            acceptance_criteria: Some("[\"Criterion 1\"]".to_string()),
        };

        let json = serde_json::to_value(&summary).expect("Should serialize");
        assert_eq!(json["id"], "prop-123");
        assert_eq!(json["title"], "Test proposal");
        assert_eq!(json["category"], "feature");
        assert_eq!(json["priority_score"], 75);
        assert_eq!(json["status"], "accepted");
    }

    #[test]
    fn parent_session_context_new_creates_minimal_context() {
        let session_id = IdeationSessionId::from_string("session-123");
        let context = ParentSessionContext::new(
            session_id.clone(),
            "Parent Session",
            "active",
        );

        assert_eq!(context.session_id, session_id);
        assert_eq!(context.session_title, "Parent Session");
        assert_eq!(context.session_status, "active");
        assert!(context.plan_content.is_none());
        assert_eq!(context.proposals.len(), 0);
        assert!(!context.has_plan());
        assert_eq!(context.proposal_count(), 0);
    }

    #[test]
    fn parent_session_context_with_plan_content_sets_plan() {
        let session_id = IdeationSessionId::from_string("session-123");
        let context = ParentSessionContext::new(session_id, "Parent", "active")
            .with_plan_content("# Plan\nSome content");

        assert!(context.has_plan());
        assert_eq!(context.plan_content.as_deref(), Some("# Plan\nSome content"));
    }

    #[test]
    fn parent_session_context_with_proposals_sets_list() {
        let session_id = IdeationSessionId::from_string("session-123");
        let proposals = vec![
            ContextProposalSummary {
                id: TaskProposalId::from_string("prop-1"),
                title: "Proposal 1".to_string(),
                category: TaskCategory::Feature,
                priority_score: 80,
                status: ProposalStatus::Accepted,
                acceptance_criteria: None,
            },
            ContextProposalSummary {
                id: TaskProposalId::from_string("prop-2"),
                title: "Proposal 2".to_string(),
                category: TaskCategory::Fix,
                priority_score: 60,
                status: ProposalStatus::Pending,
                acceptance_criteria: None,
            },
        ];

        let context = ParentSessionContext::new(session_id, "Parent", "active")
            .with_proposals(proposals);

        assert_eq!(context.proposal_count(), 2);
        assert_eq!(context.proposals[0].title, "Proposal 1");
        assert_eq!(context.proposals[1].title, "Proposal 2");
    }

    #[test]
    fn parent_session_context_proposals_by_status_filters_correctly() {
        let session_id = IdeationSessionId::from_string("session-123");
        let proposals = vec![
            ContextProposalSummary {
                id: TaskProposalId::from_string("prop-1"),
                title: "Accepted".to_string(),
                category: TaskCategory::Feature,
                priority_score: 80,
                status: ProposalStatus::Accepted,
                acceptance_criteria: None,
            },
            ContextProposalSummary {
                id: TaskProposalId::from_string("prop-2"),
                title: "Pending".to_string(),
                category: TaskCategory::Fix,
                priority_score: 60,
                status: ProposalStatus::Pending,
                acceptance_criteria: None,
            },
            ContextProposalSummary {
                id: TaskProposalId::from_string("prop-3"),
                title: "Also Accepted".to_string(),
                category: TaskCategory::Refactor,
                priority_score: 70,
                status: ProposalStatus::Accepted,
                acceptance_criteria: None,
            },
        ];

        let context = ParentSessionContext::new(session_id, "Parent", "active")
            .with_proposals(proposals);

        let accepted = context.proposals_by_status(ProposalStatus::Accepted);
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].title, "Accepted");
        assert_eq!(accepted[1].title, "Also Accepted");

        let pending = context.proposals_by_status(ProposalStatus::Pending);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].title, "Pending");
    }

    #[test]
    fn parent_session_context_serializes_to_json() {
        let session_id = IdeationSessionId::from_string("session-123");
        let proposals = vec![ContextProposalSummary {
            id: TaskProposalId::from_string("prop-1"),
            title: "Test".to_string(),
            category: TaskCategory::Feature,
            priority_score: 50,
            status: ProposalStatus::Accepted,
            acceptance_criteria: None,
        }];

        let context = ParentSessionContext::new(session_id, "Parent", "active")
            .with_plan_content("# Plan")
            .with_proposals(proposals);

        let json = serde_json::to_value(&context).expect("Should serialize");
        assert_eq!(json["session_id"], "session-123");
        assert_eq!(json["session_title"], "Parent");
        assert_eq!(json["session_status"], "active");
        assert_eq!(json["plan_content"], "# Plan");
        assert_eq!(json["proposals"].as_array().unwrap().len(), 1);
    }
}
