// IdeationService
// Application service for orchestrating ideation workflow
//
// This service manages the ideation flow:
// - Creating and managing ideation sessions
// - Creating and updating task proposals
// - Adding chat messages
// - Retrieving session data with proposals and messages

use crate::domain::entities::{
    ChatMessage, IdeationSession, IdeationSessionId, IdeationSessionStatus, MethodologyExtension,
    Priority, ProjectId, ProposalStatus, TaskCategory, TaskProposal, TaskProposalId,
};
use crate::domain::repositories::{
    ChatMessageRepository, IdeationSessionRepository, ProposalDependencyRepository,
    TaskProposalRepository,
};
use crate::error::{AppError, AppResult};
use chrono::Utc;
use std::sync::Arc;

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

/// Service for orchestrating ideation workflow
pub struct IdeationService<
    S: IdeationSessionRepository,
    P: TaskProposalRepository,
    M: ChatMessageRepository,
    D: ProposalDependencyRepository,
> {
    /// Repository for ideation sessions
    session_repo: Arc<S>,
    /// Repository for task proposals
    proposal_repo: Arc<P>,
    /// Repository for chat messages
    message_repo: Arc<M>,
    /// Repository for proposal dependencies
    dependency_repo: Arc<D>,
}

impl<S, P, M, D> IdeationService<S, P, M, D>
where
    S: IdeationSessionRepository,
    P: TaskProposalRepository,
    M: ChatMessageRepository,
    D: ProposalDependencyRepository,
{
    /// Create a new ideation service
    pub fn new(
        session_repo: Arc<S>,
        proposal_repo: Arc<P>,
        message_repo: Arc<M>,
        dependency_repo: Arc<D>,
    ) -> Self {
        Self {
            session_repo,
            proposal_repo,
            message_repo,
            dependency_repo,
        }
    }

    /// Create a new ideation session with optional auto-generated title
    pub async fn create_session(
        &self,
        project_id: ProjectId,
        title: Option<String>,
    ) -> AppResult<IdeationSession> {
        let session = match title {
            Some(t) => IdeationSession::new_with_title(project_id, t),
            None => {
                // Generate a default title based on timestamp
                let default_title = format!("Ideation Session {}", Utc::now().format("%Y-%m-%d %H:%M"));
                IdeationSession::new_with_title(project_id, default_title)
            }
        };

        self.session_repo.create(session).await
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
        self.session_repo.get_by_id(session_id).await
    }

    /// Get a session with all its proposals and messages
    pub async fn get_session_with_data(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Option<SessionWithData>> {
        let session = self.session_repo.get_by_id(session_id).await?;

        match session {
            Some(session) => {
                let proposals = self.proposal_repo.get_by_session(session_id).await?;
                let messages = self.message_repo.get_by_session(session_id).await?;

                Ok(Some(SessionWithData {
                    session,
                    proposals,
                    messages,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all sessions for a project
    pub async fn get_sessions_by_project(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        self.session_repo.get_by_project(project_id).await
    }

    /// Get active sessions for a project
    pub async fn get_active_sessions(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<IdeationSession>> {
        self.session_repo.get_active_by_project(project_id).await
    }

    /// Archive a session
    pub async fn archive_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        self.session_repo
            .update_status(session_id, IdeationSessionStatus::Archived)
            .await
    }

    /// Update session title
    pub async fn update_session_title(
        &self,
        session_id: &IdeationSessionId,
        title: Option<String>,
    ) -> AppResult<()> {
        self.session_repo.update_title(session_id, title).await
    }

    /// Delete a session and all its data
    pub async fn delete_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
        self.session_repo.delete(session_id).await
    }

    /// Create a new task proposal in a session
    pub async fn create_proposal(
        &self,
        session_id: IdeationSessionId,
        options: CreateProposalOptions,
    ) -> AppResult<TaskProposal> {
        // Verify session exists and is active
        let session = self.session_repo.get_by_id(&session_id).await?;
        match session {
            None => return Err(AppError::NotFound(format!("Session {} not found", session_id))),
            Some(s) if s.status != IdeationSessionStatus::Active => {
                return Err(AppError::Validation(format!(
                    "Cannot add proposal to {} session",
                    s.status
                )));
            }
            _ => {}
        }

        // Get current proposal count for sort_order
        let count = self.proposal_repo.count_by_session(&session_id).await?;

        let mut proposal = TaskProposal::new(
            session_id,
            options.title,
            options.category,
            options.suggested_priority,
        );
        proposal.description = options.description;
        proposal.steps = options.steps;
        proposal.acceptance_criteria = options.acceptance_criteria;
        proposal.sort_order = count as i32;

        self.proposal_repo.create(proposal).await
    }

    /// Update an existing proposal
    pub async fn update_proposal(
        &self,
        proposal_id: &TaskProposalId,
        options: UpdateProposalOptions,
    ) -> AppResult<TaskProposal> {
        let proposal = self.proposal_repo.get_by_id(proposal_id).await?;

        match proposal {
            None => Err(AppError::NotFound(format!("Proposal {} not found", proposal_id))),
            Some(mut proposal) => {
                let mut modified = false;

                if let Some(title) = options.title {
                    proposal.title = title;
                    modified = true;
                }

                if let Some(description) = options.description {
                    proposal.description = description;
                    modified = true;
                }

                if let Some(category) = options.category {
                    proposal.category = category;
                    modified = true;
                }

                if let Some(steps) = options.steps {
                    proposal.steps = steps;
                    modified = true;
                }

                if let Some(acceptance_criteria) = options.acceptance_criteria {
                    proposal.acceptance_criteria = acceptance_criteria;
                    modified = true;
                }

                if let Some(user_priority) = options.user_priority {
                    proposal.user_priority = Some(user_priority);
                    proposal.status = ProposalStatus::Modified;
                    modified = true;
                }

                if modified {
                    proposal.user_modified = true;
                    proposal.touch();
                    self.proposal_repo.update(&proposal).await?;
                }

                Ok(proposal)
            }
        }
    }

    /// Delete a proposal
    pub async fn delete_proposal(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
        // First clear any dependencies
        self.dependency_repo.clear_dependencies(proposal_id).await?;
        // Then delete the proposal
        self.proposal_repo.delete(proposal_id).await
    }

    /// Toggle proposal selection state
    pub async fn toggle_proposal_selection(
        &self,
        proposal_id: &TaskProposalId,
    ) -> AppResult<bool> {
        let proposal = self.proposal_repo.get_by_id(proposal_id).await?;

        match proposal {
            None => Err(AppError::NotFound(format!("Proposal {} not found", proposal_id))),
            Some(proposal) => {
                let new_selected = !proposal.selected;
                self.proposal_repo
                    .update_selection(proposal_id, new_selected)
                    .await?;
                Ok(new_selected)
            }
        }
    }

    /// Set proposal selection state
    pub async fn set_proposal_selection(
        &self,
        proposal_id: &TaskProposalId,
        selected: bool,
    ) -> AppResult<()> {
        self.proposal_repo
            .update_selection(proposal_id, selected)
            .await
    }

    /// Get proposals for a session
    pub async fn get_proposals(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>> {
        self.proposal_repo.get_by_session(session_id).await
    }

    /// Get selected proposals for a session
    pub async fn get_selected_proposals(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<TaskProposal>> {
        self.proposal_repo.get_selected_by_session(session_id).await
    }

    /// Add a user message to a session
    pub async fn add_user_message(
        &self,
        session_id: IdeationSessionId,
        content: impl Into<String>,
    ) -> AppResult<ChatMessage> {
        let message = ChatMessage::user_in_session(session_id, content);
        self.message_repo.create(message).await
    }

    /// Add an orchestrator message to a session
    pub async fn add_orchestrator_message(
        &self,
        session_id: IdeationSessionId,
        content: impl Into<String>,
    ) -> AppResult<ChatMessage> {
        let message = ChatMessage::orchestrator_in_session(session_id, content);
        self.message_repo.create(message).await
    }

    /// Add a system message to a session
    pub async fn add_system_message(
        &self,
        session_id: IdeationSessionId,
        content: impl Into<String>,
    ) -> AppResult<ChatMessage> {
        let message = ChatMessage::system_in_session(session_id, content);
        self.message_repo.create(message).await
    }

    /// Get all messages for a session
    pub async fn get_session_messages(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<Vec<ChatMessage>> {
        self.message_repo.get_by_session(session_id).await
    }

    /// Get recent messages for a session
    pub async fn get_recent_messages(
        &self,
        session_id: &IdeationSessionId,
        limit: u32,
    ) -> AppResult<Vec<ChatMessage>> {
        self.message_repo
            .get_recent_by_session(session_id, limit)
            .await
    }

    /// Reorder proposals within a session
    pub async fn reorder_proposals(
        &self,
        session_id: &IdeationSessionId,
        proposal_ids: Vec<TaskProposalId>,
    ) -> AppResult<()> {
        self.proposal_repo.reorder(session_id, proposal_ids).await
    }

    /// Select all proposals in a session
    pub async fn select_all_proposals(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let proposals = self.proposal_repo.get_by_session(session_id).await?;
        let mut count = 0;

        for proposal in proposals {
            if !proposal.selected {
                self.proposal_repo
                    .update_selection(&proposal.id, true)
                    .await?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Deselect all proposals in a session
    pub async fn deselect_all_proposals(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
        let proposals = self.proposal_repo.get_by_session(session_id).await?;
        let mut count = 0;

        for proposal in proposals {
            if proposal.selected {
                self.proposal_repo
                    .update_selection(&proposal.id, false)
                    .await?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get session statistics
    pub async fn get_session_stats(
        &self,
        session_id: &IdeationSessionId,
    ) -> AppResult<SessionStats> {
        let total_proposals = self.proposal_repo.count_by_session(session_id).await?;
        let selected_proposals = self.proposal_repo.count_selected_by_session(session_id).await?;
        let message_count = self.message_repo.count_by_session(session_id).await?;

        Ok(SessionStats {
            total_proposals,
            selected_proposals,
            message_count,
        })
    }

    /// Get plan artifact configuration
    ///
    /// Returns the artifact type and bucket to use for ideation plans.
    /// If a methodology is active and provides custom configuration, that is used.
    /// Otherwise, returns default configuration (Specification type, prd-library bucket).
    ///
    /// This is infrastructure for future methodology integration - currently always returns defaults.
    pub fn get_plan_artifact_config(
        active_methodology: Option<&MethodologyExtension>,
    ) -> PlanArtifactConfig {
        match active_methodology.and_then(|m| m.plan_artifact_config.as_ref()) {
            Some(config) => PlanArtifactConfig {
                artifact_type: config.artifact_type.clone(),
                bucket_id: config.bucket_id.clone(),
            },
            None => PlanArtifactConfig {
                artifact_type: "specification".to_string(),
                bucket_id: "prd-library".to_string(),
            },
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::domain::entities::{ArtifactId, ChatConversationId, ChatMessageId, MessageRole, PriorityAssessment, TaskId};
    use std::collections::HashMap;
    use std::sync::Mutex;

    // ========================================================================
    // MOCK REPOSITORIES
    // ========================================================================

    struct MockSessionRepository {
        sessions: Mutex<HashMap<String, IdeationSession>>,
    }

    impl MockSessionRepository {
        fn new() -> Self {
            Self {
                sessions: Mutex::new(HashMap::new()),
            }
        }

        fn with_session(session: IdeationSession) -> Self {
            let repo = Self::new();
            repo.sessions
                .lock()
                .unwrap()
                .insert(session.id.to_string(), session);
            repo
        }
    }

    #[async_trait]
    impl IdeationSessionRepository for MockSessionRepository {
        async fn create(&self, session: IdeationSession) -> AppResult<IdeationSession> {
            self.sessions
                .lock()
                .unwrap()
                .insert(session.id.to_string(), session.clone());
            Ok(session)
        }

        async fn get_by_id(&self, id: &IdeationSessionId) -> AppResult<Option<IdeationSession>> {
            Ok(self.sessions.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id)
                .cloned()
                .collect())
        }

        async fn update_status(
            &self,
            id: &IdeationSessionId,
            status: IdeationSessionStatus,
        ) -> AppResult<()> {
            if let Some(session) = self.sessions.lock().unwrap().get_mut(&id.to_string()) {
                session.status = status;
                session.updated_at = Utc::now();
                if status == IdeationSessionStatus::Archived {
                    session.archived_at = Some(Utc::now());
                }
                if status == IdeationSessionStatus::Converted {
                    session.converted_at = Some(Utc::now());
                }
            }
            Ok(())
        }

        async fn update_title(&self, id: &IdeationSessionId, title: Option<String>) -> AppResult<()> {
            if let Some(session) = self.sessions.lock().unwrap().get_mut(&id.to_string()) {
                session.title = title;
                session.updated_at = Utc::now();
            }
            Ok(())
        }

        async fn update_plan_artifact_id(&self, id: &IdeationSessionId, plan_artifact_id: Option<String>) -> AppResult<()> {
            if let Some(session) = self.sessions.lock().unwrap().get_mut(&id.to_string()) {
                session.plan_artifact_id = plan_artifact_id.map(|s| crate::domain::entities::ArtifactId::from_string(s));
                session.updated_at = Utc::now();
            }
            Ok(())
        }

        async fn delete(&self, id: &IdeationSessionId) -> AppResult<()> {
            self.sessions.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn get_active_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<IdeationSession>> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id && s.status == IdeationSessionStatus::Active)
                .cloned()
                .collect())
        }

        async fn count_by_status(
            &self,
            project_id: &ProjectId,
            status: IdeationSessionStatus,
        ) -> AppResult<u32> {
            Ok(self
                .sessions
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.project_id == project_id && s.status == status)
                .count() as u32)
        }
    }

    struct MockProposalRepository {
        proposals: Mutex<HashMap<String, TaskProposal>>,
    }

    impl MockProposalRepository {
        fn new() -> Self {
            Self {
                proposals: Mutex::new(HashMap::new()),
            }
        }

        fn with_proposals(proposals: Vec<TaskProposal>) -> Self {
            let repo = Self::new();
            for p in proposals {
                repo.proposals
                    .lock()
                    .unwrap()
                    .insert(p.id.to_string(), p);
            }
            repo
        }
    }

    #[async_trait]
    impl TaskProposalRepository for MockProposalRepository {
        async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
            self.proposals
                .lock()
                .unwrap()
                .insert(proposal.id.to_string(), proposal.clone());
            Ok(proposal)
        }

        async fn get_by_id(&self, id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
            Ok(self.proposals.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<TaskProposal>> {
            let mut proposals: Vec<_> = self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id)
                .cloned()
                .collect();
            proposals.sort_by_key(|p| p.sort_order);
            Ok(proposals)
        }

        async fn update(&self, proposal: &TaskProposal) -> AppResult<()> {
            self.proposals
                .lock()
                .unwrap()
                .insert(proposal.id.to_string(), proposal.clone());
            Ok(())
        }

        async fn update_priority(
            &self,
            id: &TaskProposalId,
            assessment: &PriorityAssessment,
        ) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.suggested_priority = assessment.suggested_priority;
                p.priority_score = assessment.priority_score;
            }
            Ok(())
        }

        async fn update_selection(&self, id: &TaskProposalId, selected: bool) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.selected = selected;
            }
            Ok(())
        }

        async fn set_created_task_id(&self, id: &TaskProposalId, task_id: &TaskId) -> AppResult<()> {
            if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                p.created_task_id = Some(task_id.clone());
            }
            Ok(())
        }

        async fn delete(&self, id: &TaskProposalId) -> AppResult<()> {
            self.proposals.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn reorder(
            &self,
            _session_id: &IdeationSessionId,
            proposal_ids: Vec<TaskProposalId>,
        ) -> AppResult<()> {
            for (i, id) in proposal_ids.iter().enumerate() {
                if let Some(p) = self.proposals.lock().unwrap().get_mut(&id.to_string()) {
                    p.sort_order = i as i32;
                }
            }
            Ok(())
        }

        async fn get_selected_by_session(
            &self,
            session_id: &IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            let mut proposals: Vec<_> = self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id && p.selected)
                .cloned()
                .collect();
            proposals.sort_by_key(|p| p.sort_order);
            Ok(proposals)
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id)
                .count() as u32)
        }

        async fn count_selected_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| &p.session_id == session_id && p.selected)
                .count() as u32)
        }

        async fn get_by_plan_artifact_id(&self, artifact_id: &ArtifactId) -> AppResult<Vec<TaskProposal>> {
            Ok(self
                .proposals
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.plan_artifact_id.as_ref() == Some(artifact_id))
                .cloned()
                .collect())
        }
    }

    struct MockMessageRepository {
        messages: Mutex<HashMap<String, ChatMessage>>,
    }

    impl MockMessageRepository {
        fn new() -> Self {
            Self {
                messages: Mutex::new(HashMap::new()),
            }
        }

        fn with_messages(messages: Vec<ChatMessage>) -> Self {
            let repo = Self::new();
            for m in messages {
                repo.messages
                    .lock()
                    .unwrap()
                    .insert(m.id.to_string(), m);
            }
            repo
        }
    }

    #[async_trait]
    impl ChatMessageRepository for MockMessageRepository {
        async fn create(&self, message: ChatMessage) -> AppResult<ChatMessage> {
            self.messages
                .lock()
                .unwrap()
                .insert(message.id.to_string(), message.clone());
            Ok(message)
        }

        async fn get_by_id(&self, id: &ChatMessageId) -> AppResult<Option<ChatMessage>> {
            Ok(self.messages.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_session(&self, session_id: &IdeationSessionId) -> AppResult<Vec<ChatMessage>> {
            let mut messages: Vec<_> = self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .cloned()
                .collect();
            messages.sort_by_key(|m| m.created_at);
            Ok(messages)
        }

        async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<ChatMessage>> {
            let mut messages: Vec<_> = self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.project_id.as_ref() == Some(project_id) && m.session_id.is_none())
                .cloned()
                .collect();
            messages.sort_by_key(|m| m.created_at);
            Ok(messages)
        }

        async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<ChatMessage>> {
            let mut messages: Vec<_> = self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.task_id.as_ref() == Some(task_id))
                .cloned()
                .collect();
            messages.sort_by_key(|m| m.created_at);
            Ok(messages)
        }

        async fn get_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<Vec<ChatMessage>> {
            let mut messages: Vec<_> = self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.conversation_id.as_ref() == Some(conversation_id))
                .cloned()
                .collect();
            messages.sort_by_key(|m| m.created_at);
            Ok(messages)
        }

        async fn delete_by_session(&self, session_id: &IdeationSessionId) -> AppResult<()> {
            self.messages
                .lock()
                .unwrap()
                .retain(|_, m| m.session_id.as_ref() != Some(session_id));
            Ok(())
        }

        async fn delete_by_project(&self, project_id: &ProjectId) -> AppResult<()> {
            self.messages
                .lock()
                .unwrap()
                .retain(|_, m| m.project_id.as_ref() != Some(project_id));
            Ok(())
        }

        async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
            self.messages
                .lock()
                .unwrap()
                .retain(|_, m| m.task_id.as_ref() != Some(task_id));
            Ok(())
        }

        async fn delete(&self, id: &ChatMessageId) -> AppResult<()> {
            self.messages.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn count_by_session(&self, session_id: &IdeationSessionId) -> AppResult<u32> {
            Ok(self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .count() as u32)
        }

        async fn get_recent_by_session(
            &self,
            session_id: &IdeationSessionId,
            limit: u32,
        ) -> AppResult<Vec<ChatMessage>> {
            let mut messages: Vec<_> = self
                .messages
                .lock()
                .unwrap()
                .values()
                .filter(|m| m.session_id.as_ref() == Some(session_id))
                .cloned()
                .collect();
            messages.sort_by_key(|m| std::cmp::Reverse(m.created_at));
            messages.truncate(limit as usize);
            messages.reverse();
            Ok(messages)
        }
    }

    struct MockDependencyRepository {
        dependencies: Mutex<Vec<(String, String)>>,
    }

    impl MockDependencyRepository {
        fn new() -> Self {
            Self {
                dependencies: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl ProposalDependencyRepository for MockDependencyRepository {
        async fn add_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            self.dependencies
                .lock()
                .unwrap()
                .push((proposal_id.to_string(), depends_on_id.to_string()));
            Ok(())
        }

        async fn remove_dependency(
            &self,
            proposal_id: &TaskProposalId,
            depends_on_id: &TaskProposalId,
        ) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(p, d)| {
                p != &proposal_id.to_string() || d != &depends_on_id.to_string()
            });
            Ok(())
        }

        async fn get_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _)| p == &proposal_id.to_string())
                .map(|(_, d)| TaskProposalId::from_string(d.clone()))
                .collect())
        }

        async fn get_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<Vec<TaskProposalId>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &proposal_id.to_string())
                .map(|(p, _)| TaskProposalId::from_string(p.clone()))
                .collect())
        }

        async fn get_all_for_session(
            &self,
            _session_id: &IdeationSessionId,
        ) -> AppResult<Vec<(TaskProposalId, TaskProposalId)>> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .map(|(p, d)| {
                    (
                        TaskProposalId::from_string(p.clone()),
                        TaskProposalId::from_string(d.clone()),
                    )
                })
                .collect())
        }

        async fn would_create_cycle(
            &self,
            _proposal_id: &TaskProposalId,
            _depends_on_id: &TaskProposalId,
        ) -> AppResult<bool> {
            Ok(false)
        }

        async fn clear_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<()> {
            self.dependencies.lock().unwrap().retain(|(p, d)| {
                p != &proposal_id.to_string() && d != &proposal_id.to_string()
            });
            Ok(())
        }

        async fn count_dependencies(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(p, _)| p == &proposal_id.to_string())
                .count() as u32)
        }

        async fn count_dependents(&self, proposal_id: &TaskProposalId) -> AppResult<u32> {
            Ok(self
                .dependencies
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, d)| d == &proposal_id.to_string())
                .count() as u32)
        }
    }

    // ========================================================================
    // HELPER FUNCTIONS
    // ========================================================================

    fn create_service() -> IdeationService<
        MockSessionRepository,
        MockProposalRepository,
        MockMessageRepository,
        MockDependencyRepository,
    > {
        IdeationService::new(
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockProposalRepository::new()),
            Arc::new(MockMessageRepository::new()),
            Arc::new(MockDependencyRepository::new()),
        )
    }

    fn create_service_with_session(
        session: IdeationSession,
    ) -> IdeationService<
        MockSessionRepository,
        MockProposalRepository,
        MockMessageRepository,
        MockDependencyRepository,
    > {
        IdeationService::new(
            Arc::new(MockSessionRepository::with_session(session)),
            Arc::new(MockProposalRepository::new()),
            Arc::new(MockMessageRepository::new()),
            Arc::new(MockDependencyRepository::new()),
        )
    }

    fn create_proposal_options() -> CreateProposalOptions {
        CreateProposalOptions {
            title: "Test Proposal".to_string(),
            description: Some("A test proposal".to_string()),
            category: TaskCategory::Feature,
            suggested_priority: Priority::Medium,
            steps: None,
            acceptance_criteria: None,
        }
    }

    // ========================================================================
    // SESSION TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_create_session_with_title() {
        let service = create_service();
        let project_id = ProjectId::new();

        let session = service
            .create_session(project_id.clone(), Some("My Session".to_string()))
            .await
            .unwrap();

        assert_eq!(session.project_id, project_id);
        assert_eq!(session.title, Some("My Session".to_string()));
        assert_eq!(session.status, IdeationSessionStatus::Active);
    }

    #[tokio::test]
    async fn test_create_session_without_title_generates_default() {
        let service = create_service();
        let project_id = ProjectId::new();

        let session = service.create_session(project_id.clone(), None).await.unwrap();

        assert_eq!(session.project_id, project_id);
        assert!(session.title.is_some());
        assert!(session.title.unwrap().starts_with("Ideation Session"));
    }

    #[tokio::test]
    async fn test_get_session_returns_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new_with_title(project_id.clone(), "Test");
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let result = service.get_session(&session_id).await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, session_id);
    }

    #[tokio::test]
    async fn test_get_session_returns_none_for_nonexistent() {
        let service = create_service();
        let session_id = IdeationSessionId::new();

        let result = service.get_session(&session_id).await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_archive_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        service.archive_session(&session_id).await.unwrap();

        let updated = service.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(updated.status, IdeationSessionStatus::Archived);
        assert!(updated.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_update_session_title() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new_with_title(project_id.clone(), "Original");
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        service
            .update_session_title(&session_id, Some("Updated".to_string()))
            .await
            .unwrap();

        let updated = service.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(updated.title, Some("Updated".to_string()));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        service.delete_session(&session_id).await.unwrap();

        let result = service.get_session(&session_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_sessions_by_project() {
        let project_id = ProjectId::new();
        let session1 = IdeationSession::new_with_title(project_id.clone(), "Session 1");
        let session2 = IdeationSession::new_with_title(project_id.clone(), "Session 2");

        let service = IdeationService::new(
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockProposalRepository::new()),
            Arc::new(MockMessageRepository::new()),
            Arc::new(MockDependencyRepository::new()),
        );

        // Create sessions
        service
            .session_repo
            .create(session1.clone())
            .await
            .unwrap();
        service
            .session_repo
            .create(session2.clone())
            .await
            .unwrap();

        let sessions = service.get_sessions_by_project(&project_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_get_active_sessions() {
        let project_id = ProjectId::new();
        let mut session1 = IdeationSession::new(project_id.clone());
        let session2 = IdeationSession::new(project_id.clone());
        session1.status = IdeationSessionStatus::Archived;

        let service = IdeationService::new(
            Arc::new(MockSessionRepository::new()),
            Arc::new(MockProposalRepository::new()),
            Arc::new(MockMessageRepository::new()),
            Arc::new(MockDependencyRepository::new()),
        );

        service
            .session_repo
            .create(session1.clone())
            .await
            .unwrap();
        service
            .session_repo
            .create(session2.clone())
            .await
            .unwrap();

        let active = service.get_active_sessions(&project_id).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].status, IdeationSessionStatus::Active);
    }

    // ========================================================================
    // PROPOSAL TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_create_proposal_in_active_session() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        assert_eq!(proposal.session_id, session_id);
        assert_eq!(proposal.title, "Test Proposal");
        assert_eq!(proposal.category, TaskCategory::Feature);
        assert_eq!(proposal.suggested_priority, Priority::Medium);
    }

    #[tokio::test]
    async fn test_create_proposal_fails_for_nonexistent_session() {
        let service = create_service();
        let session_id = IdeationSessionId::new();

        let result = service
            .create_proposal(session_id, create_proposal_options())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_proposal_fails_for_archived_session() {
        let project_id = ProjectId::new();
        let mut session = IdeationSession::new(project_id.clone());
        session.archive();
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let result = service
            .create_proposal(session_id, create_proposal_options())
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_proposal_title() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        let updated = service
            .update_proposal(
                &proposal.id,
                UpdateProposalOptions {
                    title: Some("Updated Title".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert!(updated.user_modified);
    }

    #[tokio::test]
    async fn test_update_proposal_user_priority() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        let updated = service
            .update_proposal(
                &proposal.id,
                UpdateProposalOptions {
                    user_priority: Some(Priority::Critical),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.user_priority, Some(Priority::Critical));
        assert_eq!(updated.status, ProposalStatus::Modified);
    }

    #[tokio::test]
    async fn test_update_proposal_not_found() {
        let service = create_service();
        let proposal_id = TaskProposalId::new();

        let result = service
            .update_proposal(
                &proposal_id,
                UpdateProposalOptions {
                    title: Some("New Title".to_string()),
                    ..Default::default()
                },
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_proposal() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        service.delete_proposal(&proposal.id).await.unwrap();

        let proposals = service.get_proposals(&session_id).await.unwrap();
        assert!(proposals.is_empty());
    }

    #[tokio::test]
    async fn test_toggle_proposal_selection() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        // Initially selected (default)
        assert!(proposal.selected);

        // Toggle off
        let new_state = service.toggle_proposal_selection(&proposal.id).await.unwrap();
        assert!(!new_state);

        // Toggle back on
        let new_state = service.toggle_proposal_selection(&proposal.id).await.unwrap();
        assert!(new_state);
    }

    #[tokio::test]
    async fn test_get_selected_proposals() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal1 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        let proposal2 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        // Deselect proposal2
        service.set_proposal_selection(&proposal2.id, false).await.unwrap();

        let selected = service.get_selected_proposals(&session_id).await.unwrap();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, proposal1.id);
    }

    #[tokio::test]
    async fn test_select_all_proposals() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let proposal1 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        let proposal2 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        // Deselect both
        service.set_proposal_selection(&proposal1.id, false).await.unwrap();
        service.set_proposal_selection(&proposal2.id, false).await.unwrap();

        let count = service.select_all_proposals(&session_id).await.unwrap();
        assert_eq!(count, 2);

        let selected = service.get_selected_proposals(&session_id).await.unwrap();
        assert_eq!(selected.len(), 2);
    }

    #[tokio::test]
    async fn test_deselect_all_proposals() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        let count = service.deselect_all_proposals(&session_id).await.unwrap();
        assert_eq!(count, 2);

        let selected = service.get_selected_proposals(&session_id).await.unwrap();
        assert!(selected.is_empty());
    }

    // ========================================================================
    // MESSAGE TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_add_user_message() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let message = service
            .add_user_message(session_id.clone(), "Hello!")
            .await
            .unwrap();

        assert_eq!(message.session_id, Some(session_id));
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello!");
    }

    #[tokio::test]
    async fn test_add_orchestrator_message() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let message = service
            .add_orchestrator_message(session_id.clone(), "I can help with that.")
            .await
            .unwrap();

        assert_eq!(message.session_id, Some(session_id));
        assert_eq!(message.role, MessageRole::Orchestrator);
    }

    #[tokio::test]
    async fn test_add_system_message() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let message = service
            .add_system_message(session_id.clone(), "Session started")
            .await
            .unwrap();

        assert_eq!(message.role, MessageRole::System);
    }

    #[tokio::test]
    async fn test_get_session_messages() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        service
            .add_user_message(session_id.clone(), "Message 1")
            .await
            .unwrap();
        service
            .add_orchestrator_message(session_id.clone(), "Message 2")
            .await
            .unwrap();

        let messages = service.get_session_messages(&session_id).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_recent_messages() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        for i in 1..=5 {
            service
                .add_user_message(session_id.clone(), format!("Message {}", i))
                .await
                .unwrap();
        }

        let recent = service.get_recent_messages(&session_id, 3).await.unwrap();
        assert_eq!(recent.len(), 3);
    }

    // ========================================================================
    // SESSION WITH DATA TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_get_session_with_data() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        // Add some proposals and messages
        service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        service
            .add_user_message(session_id.clone(), "Hello")
            .await
            .unwrap();
        service
            .add_orchestrator_message(session_id.clone(), "Hi there")
            .await
            .unwrap();

        let data = service
            .get_session_with_data(&session_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(data.session.id, session_id);
        assert_eq!(data.proposals.len(), 1);
        assert_eq!(data.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_session_with_data_returns_none_for_nonexistent() {
        let service = create_service();
        let session_id = IdeationSessionId::new();

        let result = service.get_session_with_data(&session_id).await.unwrap();
        assert!(result.is_none());
    }

    // ========================================================================
    // STATS TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_get_session_stats() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        // Add 2 proposals, deselect 1
        let proposal1 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        service.set_proposal_selection(&proposal1.id, false).await.unwrap();

        // Add 3 messages
        service
            .add_user_message(session_id.clone(), "1")
            .await
            .unwrap();
        service
            .add_user_message(session_id.clone(), "2")
            .await
            .unwrap();
        service
            .add_user_message(session_id.clone(), "3")
            .await
            .unwrap();

        let stats = service.get_session_stats(&session_id).await.unwrap();

        assert_eq!(stats.total_proposals, 2);
        assert_eq!(stats.selected_proposals, 1);
        assert_eq!(stats.message_count, 3);
    }

    // ========================================================================
    // REORDER TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_reorder_proposals() {
        let project_id = ProjectId::new();
        let session = IdeationSession::new(project_id.clone());
        let session_id = session.id.clone();
        let service = create_service_with_session(session);

        let p1 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        let p2 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();
        let p3 = service
            .create_proposal(session_id.clone(), create_proposal_options())
            .await
            .unwrap();

        // Reorder: p3, p1, p2
        service
            .reorder_proposals(&session_id, vec![p3.id.clone(), p1.id.clone(), p2.id.clone()])
            .await
            .unwrap();

        let proposals = service.get_proposals(&session_id).await.unwrap();
        assert_eq!(proposals[0].id, p3.id);
        assert_eq!(proposals[1].id, p1.id);
        assert_eq!(proposals[2].id, p2.id);
    }
}
