// Chat Resumption Runner
//
// Handles automatic resumption of interrupted chat conversations on app startup.
// Conversations that were interrupted during app shutdown (Ideation, Task, Project,
// TaskExecution, Review) are automatically resumed, respecting pause state and
// deduplicating against StartupJobRunner for task-based chats.
//
// Usage:
// - Called once during app initialization after StartupJobRunner completes
// - Queries for interrupted conversations (orphaned agent runs with claude_session_id)
// - Prioritizes by context type: TaskExecution > Review > Task > Ideation > Project
// - Skips TaskExecution/Review if task is in AGENT_ACTIVE_STATUSES (handled by StartupJobRunner)
// - Sends "Continue where you left off." message to resume Claude session

use std::sync::Arc;
use tauri::{AppHandle, Runtime};
use tracing::{info, warn};

use crate::application::{ChatService, ClaudeChatService};
use crate::commands::execution_commands::{ExecutionState, AGENT_ACTIVE_STATUSES};
use crate::domain::entities::{ChatContextType, InterruptedConversation, TaskId};
use crate::domain::repositories::{
    ActivityEventRepository, AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, PlanBranchRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};

/// Runs chat resumption on startup.
///
/// Finds all conversations that were interrupted when the app shut down
/// and resumes them by sending a message with --resume to continue the Claude session.
pub struct ChatResumptionRunner<R: Runtime = tauri::Wry> {
    agent_run_repo: Arc<dyn AgentRunRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    task_repo: Arc<dyn TaskRepository>,
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    activity_event_repo: Arc<dyn ActivityEventRepository>,
    message_queue: Arc<MessageQueue>,
    running_agent_registry: Arc<dyn RunningAgentRegistry>,
    execution_state: Arc<ExecutionState>,
    plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    app_handle: Option<AppHandle<R>>,
}

impl<R: Runtime> ChatResumptionRunner<R> {
    /// Create a new ChatResumptionRunner with all required dependencies.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_run_repo: Arc<dyn AgentRunRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        execution_state: Arc<ExecutionState>,
    ) -> Self {
        Self {
            agent_run_repo,
            conversation_repo,
            task_repo,
            task_dependency_repo,
            chat_message_repo,
            project_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            execution_state,
            plan_branch_repo: None,
            app_handle: None,
        }
    }

    pub fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }

    /// Set the Tauri app handle (builder pattern).
    pub fn with_app_handle(mut self, app_handle: AppHandle<R>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// Run chat resumption, resuming interrupted conversations.
    ///
    /// Skips if execution is paused. For TaskExecution/Review contexts,
    /// skips if the task is in an AGENT_ACTIVE_STATUS (handled by StartupJobRunner).
    pub async fn run(&self) {
        info!("[CHAT_RESUMPTION] ChatResumptionRunner::run() called");

        // 1. Skip if paused
        if self.execution_state.is_paused() {
            info!("[CHAT_RESUMPTION] Execution paused, skipping chat resumption");
            return;
        }

        // 2. Get interrupted conversations
        let interrupted = match self.agent_run_repo.get_interrupted_conversations().await {
            Ok(convs) => convs,
            Err(e) => {
                warn!(error = %e, "[CHAT_RESUMPTION] Failed to get interrupted conversations");
                return;
            }
        };

        if interrupted.is_empty() {
            info!("[CHAT_RESUMPTION] No interrupted conversations to resume");
            return;
        }

        info!(
            count = interrupted.len(),
            "[CHAT_RESUMPTION] Found interrupted conversations"
        );

        // 3. Sort by priority
        let sorted = self.prioritize_resumptions(interrupted);

        // 4. Resume each (skip if handled by task resumption)
        let mut resumed = 0u32;
        for conv in sorted {
            if self.is_handled_by_task_resumption(&conv).await {
                info!(
                    conversation_id = conv.conversation.id.as_str(),
                    context_type = %conv.conversation.context_type,
                    "[CHAT_RESUMPTION] Skipping - handled by task resumption"
                );
                continue;
            }

            info!(
                conversation_id = conv.conversation.id.as_str(),
                context_type = %conv.conversation.context_type,
                context_id = %conv.conversation.context_id,
                "[CHAT_RESUMPTION] Resuming conversation"
            );

            // Create ChatService and send resume message
            let chat_service = self.create_chat_service();
            match chat_service
                .send_message(
                    conv.conversation.context_type,
                    &conv.conversation.context_id,
                    "Continue where you left off.",
                )
                .await
            {
                Ok(_result) => {
                    info!(
                        conversation_id = conv.conversation.id.as_str(),
                        "[CHAT_RESUMPTION] Successfully resumed conversation"
                    );
                    resumed += 1;
                }
                Err(e) => {
                    warn!(
                        conversation_id = conv.conversation.id.as_str(),
                        error = %e,
                        "[CHAT_RESUMPTION] Failed to resume conversation"
                    );
                }
            }
        }

        info!(count = resumed, "[CHAT_RESUMPTION] Chat resumption complete");
    }

    /// Sort interrupted conversations by priority.
    ///
    /// Priority order: TaskExecution > Review > Task > Ideation > Project
    fn prioritize_resumptions(
        &self,
        mut conversations: Vec<InterruptedConversation>,
    ) -> Vec<InterruptedConversation> {
        conversations.sort_by_key(|conv| context_type_priority(conv.conversation.context_type));
        conversations
    }

    /// Check if this conversation is handled by StartupJobRunner.
    ///
    /// TaskExecution and Review contexts with tasks in AGENT_ACTIVE_STATUSES
    /// are already handled by StartupJobRunner via entry actions.
    async fn is_handled_by_task_resumption(&self, conv: &InterruptedConversation) -> bool {
        match conv.conversation.context_type {
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge => {
                // Check if the task is in an agent-active status
                let task_id = TaskId::from_string(conv.conversation.context_id.clone());
                match self.task_repo.get_by_id(&task_id).await {
                    Ok(Some(task)) => {
                        let is_agent_active = AGENT_ACTIVE_STATUSES.contains(&task.internal_status);
                        if is_agent_active {
                            info!(
                                task_id = task.id.as_str(),
                                status = ?task.internal_status,
                                "[CHAT_RESUMPTION] Task in agent-active status, handled by StartupJobRunner"
                            );
                            return true;
                        }
                        if task.internal_status.is_terminal() {
                            info!(
                                task_id = task.id.as_str(),
                                status = ?task.internal_status,
                                "[CHAT_RESUMPTION] Task in terminal state, skipping"
                            );
                            return true;
                        }
                        false
                    }
                    Ok(None) => {
                        // Task doesn't exist, skip this conversation
                        warn!(
                            task_id = %task_id,
                            "[CHAT_RESUMPTION] Task not found, skipping conversation"
                        );
                        true // Treat as "handled" to skip it
                    }
                    Err(e) => {
                        warn!(
                            error = %e,
                            "[CHAT_RESUMPTION] Failed to get task, skipping conversation"
                        );
                        true // Treat as "handled" to skip it on error
                    }
                }
            }
            // Other context types are not handled by StartupJobRunner
            ChatContextType::Task | ChatContextType::Ideation | ChatContextType::Project => false,
        }
    }

    /// Create a ChatService instance for resumption.
    fn create_chat_service(&self) -> ClaudeChatService<R> {
        let mut service = ClaudeChatService::new(
            Arc::clone(&self.chat_message_repo),
            Arc::clone(&self.conversation_repo),
            Arc::clone(&self.agent_run_repo),
            Arc::clone(&self.project_repo),
            Arc::clone(&self.task_repo),
            Arc::clone(&self.task_dependency_repo),
            Arc::clone(&self.ideation_session_repo),
            Arc::clone(&self.activity_event_repo),
            Arc::clone(&self.message_queue),
            Arc::clone(&self.running_agent_registry),
        )
        .with_execution_state(Arc::clone(&self.execution_state));

        if let Some(ref handle) = self.app_handle {
            service = service.with_app_handle(handle.clone());
        }
        if let Some(ref repo) = self.plan_branch_repo {
            service = service.with_plan_branch_repo(Arc::clone(repo));
        }

        service
    }
}

/// Get priority value for a context type (lower = higher priority).
fn context_type_priority(context_type: ChatContextType) -> u8 {
    match context_type {
        ChatContextType::TaskExecution => 0, // Highest priority
        ChatContextType::Review => 1,
        ChatContextType::Merge => 2, // Same priority as review (agent-active)
        ChatContextType::Task => 3,
        ChatContextType::Ideation => 4,
        ChatContextType::Project => 5, // Lowest priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::AppState;
    use crate::domain::entities::{AgentRun, ChatConversation, InternalStatus, Project, Task};

    /// Helper to create test state
    async fn setup_test_state() -> (Arc<ExecutionState>, AppState) {
        let execution_state = Arc::new(ExecutionState::new());
        let app_state = AppState::new_test();
        (execution_state, app_state)
    }

    /// Helper to build a ChatResumptionRunner from test state
    fn build_runner(
        app_state: &AppState,
        execution_state: &Arc<ExecutionState>,
    ) -> ChatResumptionRunner<tauri::Wry> {
        ChatResumptionRunner::new(
            Arc::clone(&app_state.agent_run_repo),
            Arc::clone(&app_state.chat_conversation_repo),
            Arc::clone(&app_state.task_repo),
            Arc::clone(&app_state.task_dependency_repo),
            Arc::clone(&app_state.chat_message_repo),
            Arc::clone(&app_state.project_repo),
            Arc::clone(&app_state.ideation_session_repo),
            Arc::clone(&app_state.activity_event_repo),
            Arc::clone(&app_state.message_queue),
            Arc::clone(&app_state.running_agent_registry),
            Arc::clone(execution_state),
        )
    }

    #[test]
    fn test_context_type_priority_ordering() {
        // TaskExecution should have highest priority (lowest number)
        assert!(context_type_priority(ChatContextType::TaskExecution) < context_type_priority(ChatContextType::Review));
        assert!(context_type_priority(ChatContextType::Review) < context_type_priority(ChatContextType::Task));
        assert!(context_type_priority(ChatContextType::Task) < context_type_priority(ChatContextType::Ideation));
        assert!(context_type_priority(ChatContextType::Ideation) < context_type_priority(ChatContextType::Project));
    }

    #[test]
    fn test_prioritize_resumptions_sorts_correctly() {
        // Create test conversations with different context types
        let create_interrupted = |context_type: ChatContextType| -> InterruptedConversation {
            let mut conv = ChatConversation::new_ideation(crate::domain::entities::IdeationSessionId::new());
            // Override context_type for testing (normally set by constructor)
            conv.context_type = context_type;
            conv.context_id = "test-id".to_string();
            conv.claude_session_id = Some("test-session".to_string());

            let run = AgentRun::new(conv.id);

            InterruptedConversation {
                conversation: conv,
                last_run: run,
            }
        };

        let conversations = vec![
            create_interrupted(ChatContextType::Project),    // Lowest priority
            create_interrupted(ChatContextType::TaskExecution), // Highest priority
            create_interrupted(ChatContextType::Ideation),
            create_interrupted(ChatContextType::Review),
            create_interrupted(ChatContextType::Task),
        ];

        // Use a temporary runner just for the sort function
        let sorted = {
            let mut convs = conversations;
            convs.sort_by_key(|conv| context_type_priority(conv.conversation.context_type));
            convs
        };

        // Verify order: TaskExecution, Review, Task, Ideation, Project
        assert_eq!(sorted[0].conversation.context_type, ChatContextType::TaskExecution);
        assert_eq!(sorted[1].conversation.context_type, ChatContextType::Review);
        assert_eq!(sorted[2].conversation.context_type, ChatContextType::Task);
        assert_eq!(sorted[3].conversation.context_type, ChatContextType::Ideation);
        assert_eq!(sorted[4].conversation.context_type, ChatContextType::Project);
    }

    #[tokio::test]
    async fn test_resumption_skipped_when_paused() {
        let (execution_state, app_state) = setup_test_state().await;

        // Pause execution
        execution_state.pause();

        let runner = build_runner(&app_state, &execution_state);

        // Run should skip because paused - just verify it doesn't panic
        runner.run().await;

        // Verify no conversations were created (nothing resumed)
        // The mock repo returns empty for get_interrupted_conversations, so this is a no-op
    }

    #[tokio::test]
    async fn test_is_handled_by_task_resumption_for_agent_active_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project and task in Executing state
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Executing Task".to_string());
        task.internal_status = InternalStatus::Executing;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Create an interrupted conversation for TaskExecution
        let mut conv = ChatConversation::new_task_execution(task_id.clone());
        conv.claude_session_id = Some("test-session".to_string());

        let run = AgentRun::new(conv.id);

        let interrupted = InterruptedConversation {
            conversation: conv,
            last_run: run,
        };

        let runner = build_runner(&app_state, &execution_state);

        // Should be handled by task resumption (task is in Executing status)
        let is_handled = runner.is_handled_by_task_resumption(&interrupted).await;
        assert!(is_handled, "TaskExecution with Executing task should be handled by StartupJobRunner");
    }

    #[tokio::test]
    async fn test_is_handled_by_task_resumption_for_non_agent_active_task() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create a project and task in Ready state (NOT agent-active)
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), "Ready Task".to_string());
        task.internal_status = InternalStatus::Ready;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        // Create an interrupted conversation for TaskExecution
        let mut conv = ChatConversation::new_task_execution(task_id.clone());
        conv.claude_session_id = Some("test-session".to_string());

        let run = AgentRun::new(conv.id);

        let interrupted = InterruptedConversation {
            conversation: conv,
            last_run: run,
        };

        let runner = build_runner(&app_state, &execution_state);

        // Should NOT be handled by task resumption (task is in Ready status)
        let is_handled = runner.is_handled_by_task_resumption(&interrupted).await;
        assert!(!is_handled, "TaskExecution with Ready task should NOT be handled by StartupJobRunner");
    }

    #[tokio::test]
    async fn test_is_handled_by_task_resumption_for_ideation() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create an interrupted conversation for Ideation
        let session_id = crate::domain::entities::IdeationSessionId::new();
        let mut conv = ChatConversation::new_ideation(session_id);
        conv.claude_session_id = Some("test-session".to_string());

        let run = AgentRun::new(conv.id);

        let interrupted = InterruptedConversation {
            conversation: conv,
            last_run: run,
        };

        let runner = build_runner(&app_state, &execution_state);

        // Ideation should NOT be handled by task resumption
        let is_handled = runner.is_handled_by_task_resumption(&interrupted).await;
        assert!(!is_handled, "Ideation should NOT be handled by StartupJobRunner");
    }

    #[tokio::test]
    async fn test_is_handled_by_task_resumption_for_project() {
        let (execution_state, app_state) = setup_test_state().await;

        // Create an interrupted conversation for Project
        let project_id = crate::domain::entities::ProjectId::new();
        let mut conv = ChatConversation::new_project(project_id);
        conv.claude_session_id = Some("test-session".to_string());

        let run = AgentRun::new(conv.id);

        let interrupted = InterruptedConversation {
            conversation: conv,
            last_run: run,
        };

        let runner = build_runner(&app_state, &execution_state);

        // Project should NOT be handled by task resumption
        let is_handled = runner.is_handled_by_task_resumption(&interrupted).await;
        assert!(!is_handled, "Project should NOT be handled by StartupJobRunner");
    }

    async fn create_terminal_state_test(status: InternalStatus) -> bool {
        let (execution_state, app_state) = setup_test_state().await;

        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        app_state.project_repo.create(project.clone()).await.unwrap();

        let mut task = Task::new(project.id.clone(), format!("{:?} Task", status));
        task.internal_status = status;
        let task_id = task.id.clone();
        app_state.task_repo.create(task).await.unwrap();

        let mut conv = ChatConversation::new_task_execution(task_id);
        conv.claude_session_id = Some("test-session".to_string());

        let run = AgentRun::new(conv.id);

        let interrupted = InterruptedConversation {
            conversation: conv,
            last_run: run,
        };

        let runner = build_runner(&app_state, &execution_state);
        runner.is_handled_by_task_resumption(&interrupted).await
    }

    #[tokio::test]
    async fn test_is_handled_for_merged_task() {
        let is_handled = create_terminal_state_test(InternalStatus::Merged).await;
        assert!(is_handled, "Merged task should be skipped (terminal state)");
    }

    #[tokio::test]
    async fn test_is_handled_for_failed_task() {
        let is_handled = create_terminal_state_test(InternalStatus::Failed).await;
        assert!(is_handled, "Failed task should be skipped (terminal state)");
    }

    #[tokio::test]
    async fn test_is_handled_for_cancelled_task() {
        let is_handled = create_terminal_state_test(InternalStatus::Cancelled).await;
        assert!(is_handled, "Cancelled task should be skipped (terminal state)");
    }

    #[tokio::test]
    async fn test_is_handled_for_stopped_task() {
        let is_handled = create_terminal_state_test(InternalStatus::Stopped).await;
        assert!(is_handled, "Stopped task should be skipped (terminal state)");
    }
}
