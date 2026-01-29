// Agent run repository trait - domain layer abstraction
//
// This trait defines the contract for agent run persistence.
// Agent runs track the execution status of Claude agents for conversations.

use async_trait::async_trait;

use crate::domain::entities::{AgentRun, AgentRunId, AgentRunStatus, ChatConversationId, InterruptedConversation};
use crate::error::AppResult;

/// Repository trait for AgentRun persistence.
/// Implementations can use SQLite, PostgreSQL, in-memory, etc.
#[async_trait]
pub trait AgentRunRepository: Send + Sync {
    /// Create a new agent run
    async fn create(&self, run: AgentRun) -> AppResult<AgentRun>;

    /// Get run by ID
    async fn get_by_id(&self, id: &AgentRunId) -> AppResult<Option<AgentRun>>;

    /// Get the most recent run for a conversation (active or completed)
    async fn get_latest_for_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Option<AgentRun>>;

    /// Get the active (running) run for a conversation, if any
    async fn get_active_for_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Option<AgentRun>>;

    /// Get all runs for a conversation, ordered by started_at DESC
    async fn get_by_conversation(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Vec<AgentRun>>;

    /// Update run status
    async fn update_status(&self, id: &AgentRunId, status: AgentRunStatus) -> AppResult<()>;

    /// Complete a run (set status to Completed and completed_at timestamp)
    async fn complete(&self, id: &AgentRunId) -> AppResult<()>;

    /// Fail a run (set status to Failed, completed_at timestamp, and error message)
    async fn fail(&self, id: &AgentRunId, error_message: &str) -> AppResult<()>;

    /// Cancel a run (set status to Cancelled and completed_at timestamp)
    async fn cancel(&self, id: &AgentRunId) -> AppResult<()>;

    /// Delete a run
    async fn delete(&self, id: &AgentRunId) -> AppResult<()>;

    /// Delete all runs for a conversation
    async fn delete_by_conversation(&self, conversation_id: &ChatConversationId) -> AppResult<()>;

    /// Count runs by status for a conversation
    async fn count_by_status(
        &self,
        conversation_id: &ChatConversationId,
        status: AgentRunStatus,
    ) -> AppResult<u32>;

    /// Cancel all runs currently in "running" status
    ///
    /// Used on startup to clean up orphaned agent runs from previous sessions
    /// that didn't complete properly (e.g., app crash or force quit).
    /// Returns the number of runs cancelled.
    async fn cancel_all_running(&self) -> AppResult<u32>;

    /// Get conversations that were interrupted during app shutdown
    ///
    /// Returns conversations where:
    /// - claude_session_id is NOT NULL (can use --resume)
    /// - latest agent_run status is 'cancelled'
    /// - latest agent_run error_message is 'Orphaned on app restart'
    ///
    /// Used by ChatResumptionRunner to resume interrupted conversations on startup.
    async fn get_interrupted_conversations(&self) -> AppResult<Vec<InterruptedConversation>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{AgentRun, ChatConversationId};
    use std::sync::Arc;

    // Mock implementation for testing trait object usage
    struct MockAgentRunRepository {
        runs: Vec<AgentRun>,
    }

    impl MockAgentRunRepository {
        fn new() -> Self {
            Self { runs: vec![] }
        }

        fn with_runs(runs: Vec<AgentRun>) -> Self {
            Self { runs }
        }
    }

    #[async_trait]
    impl AgentRunRepository for MockAgentRunRepository {
        async fn create(&self, run: AgentRun) -> AppResult<AgentRun> {
            Ok(run)
        }

        async fn get_by_id(&self, id: &AgentRunId) -> AppResult<Option<AgentRun>> {
            Ok(self.runs.iter().find(|r| r.id == *id).cloned())
        }

        async fn get_latest_for_conversation(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<Option<AgentRun>> {
            Ok(self
                .runs
                .iter()
                .filter(|r| r.conversation_id == *conversation_id)
                .max_by_key(|r| r.started_at)
                .cloned())
        }

        async fn get_active_for_conversation(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<Option<AgentRun>> {
            Ok(self
                .runs
                .iter()
                .find(|r| r.conversation_id == *conversation_id && r.is_active())
                .cloned())
        }

        async fn get_by_conversation(
            &self,
            conversation_id: &ChatConversationId,
        ) -> AppResult<Vec<AgentRun>> {
            Ok(self
                .runs
                .iter()
                .filter(|r| r.conversation_id == *conversation_id)
                .cloned()
                .collect())
        }

        async fn update_status(&self, _id: &AgentRunId, _status: AgentRunStatus) -> AppResult<()> {
            Ok(())
        }

        async fn complete(&self, _id: &AgentRunId) -> AppResult<()> {
            Ok(())
        }

        async fn fail(&self, _id: &AgentRunId, _error_message: &str) -> AppResult<()> {
            Ok(())
        }

        async fn cancel(&self, _id: &AgentRunId) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &AgentRunId) -> AppResult<()> {
            Ok(())
        }

        async fn delete_by_conversation(&self, _conversation_id: &ChatConversationId) -> AppResult<()> {
            Ok(())
        }

        async fn count_by_status(
            &self,
            conversation_id: &ChatConversationId,
            status: AgentRunStatus,
        ) -> AppResult<u32> {
            Ok(self
                .runs
                .iter()
                .filter(|r| r.conversation_id == *conversation_id && r.status == status)
                .count() as u32)
        }

        async fn cancel_all_running(&self) -> AppResult<u32> {
            // Mock just returns 0 - not needed for mock tests
            Ok(0)
        }

        async fn get_interrupted_conversations(&self) -> AppResult<Vec<InterruptedConversation>> {
            // Mock returns empty - actual filtering would need conversation data
            Ok(vec![])
        }
    }

    #[test]
    fn test_trait_object_safety() {
        let repo = MockAgentRunRepository::new();
        let _: Arc<dyn AgentRunRepository> = Arc::new(repo);
    }

    #[test]
    fn test_mock_with_runs() {
        let conversation_id = ChatConversationId::new();
        let run = AgentRun::new(conversation_id);
        let repo = MockAgentRunRepository::with_runs(vec![run.clone()]);

        assert_eq!(repo.runs.len(), 1);
        assert_eq!(repo.runs[0].id, run.id);
    }
}
