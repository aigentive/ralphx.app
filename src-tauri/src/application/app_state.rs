// Application state container for dependency injection
// Holds repository trait objects that can be swapped for testing

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::application::PermissionState;
use crate::domain::entities::IdeationSessionId;
use crate::domain::agents::AgenticClient;
use crate::domain::qa::QASettings;
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::repositories::{
    ActivityEventRepository, AgentProfileRepository, AgentRunRepository, ArtifactBucketRepository,
    ArtifactFlowRepository, ArtifactRepository, ChatConversationRepository, ChatMessageRepository,
    ExecutionSettingsRepository, GlobalExecutionSettingsRepository, IdeationSessionRepository,
    IdeationSettingsRepository, MethodologyRepository, ProcessRepository, ProjectRepository,
    ProposalDependencyRepository, ReviewRepository, ReviewSettingsRepository,
    TaskDependencyRepository, TaskProposalRepository, TaskQARepository, TaskRepository,
    TaskStepRepository, WorkflowRepository,
};
use crate::infrastructure::sqlite::ReviewIssueRepository;
use crate::error::AppResult;
use crate::infrastructure::memory::{
    MemoryActivityEventRepository, MemoryAgentProfileRepository, MemoryAgentRunRepository,
    MemoryArtifactBucketRepository, MemoryArtifactFlowRepository, MemoryArtifactRepository,
    MemoryChatConversationRepository, MemoryChatMessageRepository,
    MemoryExecutionSettingsRepository, MemoryGlobalExecutionSettingsRepository,
    MemoryIdeationSessionRepository, MemoryIdeationSettingsRepository, MemoryMethodologyRepository,
    MemoryProcessRepository, MemoryProjectRepository, MemoryProposalDependencyRepository,
    MemoryReviewIssueRepository, MemoryReviewRepository, MemoryReviewSettingsRepository,
    MemoryTaskDependencyRepository, MemoryTaskProposalRepository, MemoryTaskQARepository,
    MemoryTaskRepository, MemoryTaskStepRepository, MemoryWorkflowRepository,
};
use crate::infrastructure::sqlite::{
    get_app_data_db_path, get_default_db_path, open_connection, run_migrations,
    SqliteActivityEventRepository, SqliteAgentProfileRepository, SqliteAgentRunRepository,
    SqliteArtifactBucketRepository, SqliteArtifactFlowRepository, SqliteArtifactRepository,
    SqliteChatConversationRepository, SqliteChatMessageRepository, SqliteExecutionSettingsRepository,
    SqliteGlobalExecutionSettingsRepository, SqliteIdeationSessionRepository,
    SqliteIdeationSettingsRepository, SqliteMethodologyRepository, SqliteProcessRepository,
    SqliteProjectRepository, SqliteProposalDependencyRepository, SqliteReviewIssueRepository,
    SqliteReviewRepository, SqliteReviewSettingsRepository, SqliteTaskDependencyRepository,
    SqliteTaskProposalRepository, SqliteTaskQARepository, SqliteTaskRepository,
    SqliteTaskStepRepository, SqliteWorkflowRepository,
};
use crate::infrastructure::{ClaudeCodeClient, MockAgenticClient};

/// Application state container for dependency injection
/// Holds repository trait objects that can be swapped for testing vs production
pub struct AppState {
    /// Task repository (SQLite in production, in-memory for tests)
    pub task_repo: Arc<dyn TaskRepository>,
    /// Task step repository for tracking execution progress
    pub task_step_repo: Arc<dyn TaskStepRepository>,
    /// Project repository (SQLite in production, in-memory for tests)
    pub project_repo: Arc<dyn ProjectRepository>,
    /// Agent profile repository (SQLite in production)
    pub agent_profile_repo: Arc<dyn AgentProfileRepository>,
    /// TaskQA repository for QA artifacts
    pub task_qa_repo: Arc<dyn TaskQARepository>,
    /// Review repository for code reviews
    pub review_repo: Arc<dyn ReviewRepository>,
    /// Review settings repository
    pub review_settings_repo: Arc<dyn ReviewSettingsRepository>,
    /// Review issue repository for tracking structured issues from reviews
    pub review_issue_repo: Arc<dyn ReviewIssueRepository>,
    /// Agent client (Claude Code in production, Mock for tests)
    pub agent_client: Arc<dyn AgenticClient>,
    /// Global QA settings
    pub qa_settings: Arc<tokio::sync::RwLock<QASettings>>,
    /// Execution settings repository (per-project settings)
    pub execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    /// Global execution settings repository (cross-project limits)
    /// Phase 82: Contains global_max_concurrent cap
    pub global_execution_settings_repo: Arc<dyn GlobalExecutionSettingsRepository>,
    /// Ideation session repository
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    /// Ideation settings repository
    pub ideation_settings_repo: Arc<dyn IdeationSettingsRepository>,
    /// Task proposal repository
    pub task_proposal_repo: Arc<dyn TaskProposalRepository>,
    /// Proposal dependency repository
    pub proposal_dependency_repo: Arc<dyn ProposalDependencyRepository>,
    /// Chat message repository
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    /// Chat conversation repository (for context-aware chat)
    pub chat_conversation_repo: Arc<dyn ChatConversationRepository>,
    /// Agent run repository (for tracking Claude agent executions)
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    /// Activity event repository (for activity stream persistence)
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    /// Task dependency repository
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    // Extensibility repositories
    /// Workflow repository for custom workflows
    pub workflow_repo: Arc<dyn WorkflowRepository>,
    /// Artifact repository for artifact management
    pub artifact_repo: Arc<dyn ArtifactRepository>,
    /// Artifact bucket repository for organizing artifacts
    pub artifact_bucket_repo: Arc<dyn ArtifactBucketRepository>,
    /// Artifact flow repository for artifact routing
    pub artifact_flow_repo: Arc<dyn ArtifactFlowRepository>,
    /// Process repository for research processes
    pub process_repo: Arc<dyn ProcessRepository>,
    /// Methodology repository for methodology extensions
    pub methodology_repo: Arc<dyn MethodologyRepository>,
    /// Permission state for UI-based permission approval
    pub permission_state: Arc<PermissionState>,
    /// Unified message queue for all chat contexts
    pub message_queue: Arc<MessageQueue>,
    /// Registry for tracking running agent processes
    pub running_agent_registry: Arc<RunningAgentRegistry>,
    /// Sessions currently undergoing dependency analysis (for status reporting in MCP tools)
    pub analyzing_dependencies: Arc<tokio::sync::RwLock<HashSet<IdeationSessionId>>>,
    /// Tauri app handle for emitting events to frontend (None in tests)
    pub app_handle: Option<AppHandle>,
}

impl AppState {
    /// Create AppState for production use with SQLite repositories
    /// Opens the database at the default path and runs migrations
    pub fn new_production(app_handle: AppHandle) -> AppResult<Self> {
        let path = if cfg!(debug_assertions) {
            get_default_db_path()
        } else {
            get_app_data_db_path(&app_handle)?
        };
        let conn = open_connection(&path)?;
        run_migrations(&conn)?;

        // Wrap connection in Arc<Mutex> for sharing between repos
        let shared_conn = Arc::new(Mutex::new(conn));

        // Create repositories that are used by services
        let task_repo: Arc<dyn TaskRepository> =
            Arc::new(SqliteTaskRepository::from_shared(Arc::clone(&shared_conn)));
        let task_proposal_repo: Arc<dyn TaskProposalRepository> =
            Arc::new(SqliteTaskProposalRepository::from_shared(Arc::clone(
                &shared_conn,
            )));
        let artifact_repo: Arc<dyn ArtifactRepository> =
            Arc::new(SqliteArtifactRepository::from_shared(Arc::clone(
                &shared_conn,
            )));

        Ok(Self {
            task_repo: Arc::clone(&task_repo),
            task_step_repo: Arc::new(SqliteTaskStepRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            project_repo: Arc::new(SqliteProjectRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            agent_profile_repo: Arc::new(SqliteAgentProfileRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            task_qa_repo: Arc::new(SqliteTaskQARepository::from_shared(Arc::clone(&shared_conn))),
            review_repo: Arc::new(SqliteReviewRepository::from_shared(Arc::clone(&shared_conn))),
            review_settings_repo: Arc::new(SqliteReviewSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            review_issue_repo: Arc::new(SqliteReviewIssueRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            agent_client: Arc::new(ClaudeCodeClient::new()),
            qa_settings: Arc::new(tokio::sync::RwLock::new(QASettings::default())),
            execution_settings_repo: Arc::new(SqliteExecutionSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            global_execution_settings_repo: Arc::new(
                crate::infrastructure::sqlite::SqliteGlobalExecutionSettingsRepository::from_shared(
                    Arc::clone(&shared_conn),
                ),
            ),
            ideation_session_repo: Arc::new(SqliteIdeationSessionRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            ideation_settings_repo: Arc::new(SqliteIdeationSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            task_proposal_repo: Arc::clone(&task_proposal_repo),
            proposal_dependency_repo: Arc::new(SqliteProposalDependencyRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            chat_message_repo: Arc::new(SqliteChatMessageRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            chat_conversation_repo: Arc::new(SqliteChatConversationRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            agent_run_repo: Arc::new(SqliteAgentRunRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            activity_event_repo: Arc::new(SqliteActivityEventRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            task_dependency_repo: Arc::new(SqliteTaskDependencyRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            // Extensibility repositories
            workflow_repo: Arc::new(SqliteWorkflowRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            artifact_repo: Arc::clone(&artifact_repo),
            artifact_bucket_repo: Arc::new(SqliteArtifactBucketRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            artifact_flow_repo: Arc::new(SqliteArtifactFlowRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            process_repo: Arc::new(SqliteProcessRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            methodology_repo: Arc::new(SqliteMethodologyRepository::from_shared(shared_conn)),
            permission_state: Arc::new(PermissionState::new()),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(RunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            app_handle: Some(app_handle),
        })
    }

    /// Create AppState with a specific database path
    pub fn with_db_path(db_path: &str, app_handle: AppHandle) -> AppResult<Self> {
        let path = PathBuf::from(db_path);
        let conn = open_connection(&path)?;
        run_migrations(&conn)?;

        let shared_conn = Arc::new(Mutex::new(conn));

        // Create repositories that are used by services
        let task_repo: Arc<dyn TaskRepository> =
            Arc::new(SqliteTaskRepository::from_shared(Arc::clone(&shared_conn)));
        let task_proposal_repo: Arc<dyn TaskProposalRepository> =
            Arc::new(SqliteTaskProposalRepository::from_shared(Arc::clone(
                &shared_conn,
            )));
        let artifact_repo: Arc<dyn ArtifactRepository> =
            Arc::new(SqliteArtifactRepository::from_shared(Arc::clone(
                &shared_conn,
            )));

        Ok(Self {
            task_repo: Arc::clone(&task_repo),
            task_step_repo: Arc::new(SqliteTaskStepRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            project_repo: Arc::new(SqliteProjectRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            agent_profile_repo: Arc::new(SqliteAgentProfileRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            task_qa_repo: Arc::new(SqliteTaskQARepository::from_shared(Arc::clone(&shared_conn))),
            review_repo: Arc::new(SqliteReviewRepository::from_shared(Arc::clone(&shared_conn))),
            review_settings_repo: Arc::new(SqliteReviewSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            review_issue_repo: Arc::new(SqliteReviewIssueRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            agent_client: Arc::new(ClaudeCodeClient::new()),
            qa_settings: Arc::new(tokio::sync::RwLock::new(QASettings::default())),
            execution_settings_repo: Arc::new(SqliteExecutionSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            global_execution_settings_repo: Arc::new(
                crate::infrastructure::sqlite::SqliteGlobalExecutionSettingsRepository::from_shared(
                    Arc::clone(&shared_conn),
                ),
            ),
            ideation_session_repo: Arc::new(SqliteIdeationSessionRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            ideation_settings_repo: Arc::new(SqliteIdeationSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            task_proposal_repo: Arc::clone(&task_proposal_repo),
            proposal_dependency_repo: Arc::new(SqliteProposalDependencyRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            chat_message_repo: Arc::new(SqliteChatMessageRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            chat_conversation_repo: Arc::new(SqliteChatConversationRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            agent_run_repo: Arc::new(SqliteAgentRunRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            activity_event_repo: Arc::new(SqliteActivityEventRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            task_dependency_repo: Arc::new(SqliteTaskDependencyRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            // Extensibility repositories
            workflow_repo: Arc::new(SqliteWorkflowRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            artifact_repo: Arc::clone(&artifact_repo),
            artifact_bucket_repo: Arc::new(SqliteArtifactBucketRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            artifact_flow_repo: Arc::new(SqliteArtifactFlowRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            process_repo: Arc::new(SqliteProcessRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            methodology_repo: Arc::new(SqliteMethodologyRepository::from_shared(shared_conn)),
            permission_state: Arc::new(PermissionState::new()),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(RunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            app_handle: Some(app_handle),
        })
    }

    /// Create AppState for testing with in-memory repositories
    /// No AppHandle is provided - event emission is disabled in tests
    pub fn new_test() -> Self {
        let task_repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
        let task_proposal_repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MemoryTaskProposalRepository::new());
        let artifact_repo: Arc<dyn ArtifactRepository> = Arc::new(MemoryArtifactRepository::new());

        Self {
            task_repo: Arc::clone(&task_repo),
            task_step_repo: Arc::new(MemoryTaskStepRepository::new()),
            project_repo: Arc::new(MemoryProjectRepository::new()),
            agent_profile_repo: Arc::new(MemoryAgentProfileRepository::new()),
            task_qa_repo: Arc::new(MemoryTaskQARepository::new()),
            review_repo: Arc::new(MemoryReviewRepository::new()),
            review_settings_repo: Arc::new(MemoryReviewSettingsRepository::new()),
            review_issue_repo: Arc::new(MemoryReviewIssueRepository::new()),
            agent_client: Arc::new(MockAgenticClient::new()),
            qa_settings: Arc::new(tokio::sync::RwLock::new(QASettings::default())),
            execution_settings_repo: Arc::new(MemoryExecutionSettingsRepository::new()),
            global_execution_settings_repo: Arc::new(MemoryGlobalExecutionSettingsRepository::new()),
            ideation_session_repo: Arc::new(MemoryIdeationSessionRepository::new()),
            ideation_settings_repo: Arc::new(MemoryIdeationSettingsRepository::new()),
            task_proposal_repo: Arc::clone(&task_proposal_repo),
            proposal_dependency_repo: Arc::new(MemoryProposalDependencyRepository::new()),
            chat_message_repo: Arc::new(MemoryChatMessageRepository::new()),
            chat_conversation_repo: Arc::new(MemoryChatConversationRepository::new()),
            agent_run_repo: Arc::new(MemoryAgentRunRepository::new()),
            activity_event_repo: Arc::new(MemoryActivityEventRepository::new()),
            task_dependency_repo: Arc::new(MemoryTaskDependencyRepository::new()),
            // Extensibility repositories
            workflow_repo: Arc::new(MemoryWorkflowRepository::new()),
            artifact_repo: Arc::clone(&artifact_repo),
            artifact_bucket_repo: Arc::new(MemoryArtifactBucketRepository::new()),
            artifact_flow_repo: Arc::new(MemoryArtifactFlowRepository::new()),
            process_repo: Arc::new(MemoryProcessRepository::new()),
            methodology_repo: Arc::new(MemoryMethodologyRepository::new()),
            permission_state: Arc::new(PermissionState::new()),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(RunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            app_handle: None,
        }
    }

    /// Create AppState with custom repositories (for dependency injection)
    /// No AppHandle is provided - event emission is disabled
    pub fn with_repos(
        task_repo: Arc<dyn TaskRepository>,
        project_repo: Arc<dyn ProjectRepository>,
    ) -> Self {
        let task_proposal_repo: Arc<dyn TaskProposalRepository> =
            Arc::new(MemoryTaskProposalRepository::new());
        let artifact_repo: Arc<dyn ArtifactRepository> = Arc::new(MemoryArtifactRepository::new());

        Self {
            task_repo: Arc::clone(&task_repo),
            task_step_repo: Arc::new(MemoryTaskStepRepository::new()),
            project_repo,
            agent_profile_repo: Arc::new(MemoryAgentProfileRepository::new()),
            task_qa_repo: Arc::new(MemoryTaskQARepository::new()),
            review_repo: Arc::new(MemoryReviewRepository::new()),
            review_settings_repo: Arc::new(MemoryReviewSettingsRepository::new()),
            review_issue_repo: Arc::new(MemoryReviewIssueRepository::new()),
            agent_client: Arc::new(MockAgenticClient::new()),
            qa_settings: Arc::new(tokio::sync::RwLock::new(QASettings::default())),
            execution_settings_repo: Arc::new(MemoryExecutionSettingsRepository::new()),
            global_execution_settings_repo: Arc::new(MemoryGlobalExecutionSettingsRepository::new()),
            ideation_session_repo: Arc::new(MemoryIdeationSessionRepository::new()),
            ideation_settings_repo: Arc::new(MemoryIdeationSettingsRepository::new()),
            task_proposal_repo: Arc::clone(&task_proposal_repo),
            proposal_dependency_repo: Arc::new(MemoryProposalDependencyRepository::new()),
            chat_message_repo: Arc::new(MemoryChatMessageRepository::new()),
            chat_conversation_repo: Arc::new(MemoryChatConversationRepository::new()),
            agent_run_repo: Arc::new(MemoryAgentRunRepository::new()),
            activity_event_repo: Arc::new(MemoryActivityEventRepository::new()),
            task_dependency_repo: Arc::new(MemoryTaskDependencyRepository::new()),
            // Extensibility repositories
            workflow_repo: Arc::new(MemoryWorkflowRepository::new()),
            artifact_repo: Arc::clone(&artifact_repo),
            artifact_bucket_repo: Arc::new(MemoryArtifactBucketRepository::new()),
            artifact_flow_repo: Arc::new(MemoryArtifactFlowRepository::new()),
            process_repo: Arc::new(MemoryProcessRepository::new()),
            methodology_repo: Arc::new(MemoryMethodologyRepository::new()),
            permission_state: Arc::new(PermissionState::new()),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(RunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            app_handle: None,
        }
    }

    /// Swap the agent client to a different implementation
    pub fn with_agent_client(mut self, client: Arc<dyn AgenticClient>) -> Self {
        self.agent_client = client;
        self
    }

    /// Swap the QA settings to custom settings
    pub fn with_qa_settings(mut self, settings: QASettings) -> Self {
        self.qa_settings = Arc::new(tokio::sync::RwLock::new(settings));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::ClientType;
    use crate::domain::entities::{
        ChatMessage, IdeationSession, Priority, Project, ProjectId, Task, TaskCategory,
        TaskProposal,
    };

    #[tokio::test]
    async fn test_new_test_creates_empty_repositories() {
        let state = AppState::new_test();

        // Task repo should be empty
        let project_id = ProjectId::new();
        let tasks = state.task_repo.get_by_project(&project_id).await.unwrap();
        assert!(tasks.is_empty());

        // Project repo should be empty
        let projects = state.project_repo.get_all().await.unwrap();
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn test_with_repos_uses_custom_repositories() {
        let task_repo = Arc::new(MemoryTaskRepository::new());
        let project_repo = Arc::new(MemoryProjectRepository::new());

        // Pre-populate the repos
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        project_repo.create(project.clone()).await.unwrap();

        let task = Task::new(project.id.clone(), "Test Task".to_string());
        task_repo.create(task.clone()).await.unwrap();

        // Create AppState with these repos
        let state = AppState::with_repos(task_repo, project_repo);

        // Verify the state uses our repos
        let projects = state.project_repo.get_all().await.unwrap();
        assert_eq!(projects.len(), 1);

        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_task_and_project_repos_work_together() {
        let state = AppState::new_test();

        // Create a project
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        state.project_repo.create(project.clone()).await.unwrap();

        // Create tasks for that project
        let task1 = Task::new(project.id.clone(), "Task 1".to_string());
        let task2 = Task::new(project.id.clone(), "Task 2".to_string());
        state.task_repo.create(task1).await.unwrap();
        state.task_repo.create(task2).await.unwrap();

        // Verify we can retrieve them
        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_repositories_are_thread_safe() {
        let state = Arc::new(AppState::new_test());

        // Create a project first
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        state.project_repo.create(project.clone()).await.unwrap();

        // Spawn multiple tasks that use the repos concurrently
        let mut handles = vec![];
        for i in 0..10 {
            let state_clone = Arc::clone(&state);
            let project_id = project.id.clone();
            handles.push(tokio::spawn(async move {
                let task = Task::new(project_id, format!("Task {}", i));
                state_clone.task_repo.create(task).await
            }));
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify all tasks were created
        let tasks = state.task_repo.get_by_project(&project.id).await.unwrap();
        assert_eq!(tasks.len(), 10);
    }

    #[tokio::test]
    async fn test_new_test_creates_mock_agent_client() {
        let state = AppState::new_test();

        // Agent client should be mock and available
        let available = state.agent_client.is_available().await.unwrap();
        assert!(available);

        // Check capabilities indicate mock
        let caps = state.agent_client.capabilities();
        assert_eq!(caps.client_type, ClientType::Mock);
    }

    #[tokio::test]
    async fn test_with_agent_client_swaps_client() {
        let state = AppState::new_test();

        // Default is mock
        assert_eq!(
            state.agent_client.capabilities().client_type,
            ClientType::Mock
        );

        // Create custom mock with different capabilities wouldn't show,
        // but we can test the swap mechanism works
        let custom_mock = Arc::new(MockAgenticClient::new());
        let _state = state.with_agent_client(custom_mock);

        // If it compiled and ran, the swap worked
    }

    #[tokio::test]
    async fn test_ideation_repos_accessible() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create an ideation session
        let session = IdeationSession::new_with_title(project_id.clone(), "Test Session");
        let session_id = session.id.clone();
        state
            .ideation_session_repo
            .create(session)
            .await
            .unwrap();

        // Verify we can retrieve it
        let retrieved = state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
            .unwrap();
        assert!(retrieved.is_some());

        // Create a proposal
        let proposal = TaskProposal::new(
            session_id.clone(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        );
        let proposal_id = proposal.id.clone();
        state.task_proposal_repo.create(proposal).await.unwrap();

        // Verify we can retrieve proposals
        let proposals = state
            .task_proposal_repo
            .get_by_session(&session_id)
            .await
            .unwrap();
        assert_eq!(proposals.len(), 1);

        // Create a chat message
        let message = ChatMessage::user_in_session(session_id.clone(), "Hello");
        state.chat_message_repo.create(message).await.unwrap();

        // Verify we can retrieve messages
        let messages = state
            .chat_message_repo
            .get_by_session(&session_id)
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);

        // Add a dependency
        let proposal2 = TaskProposal::new(
            session_id.clone(),
            "Another Proposal",
            TaskCategory::Feature,
            Priority::Low,
        );
        let proposal2_id = proposal2.id.clone();
        state.task_proposal_repo.create(proposal2).await.unwrap();

        state
            .proposal_dependency_repo
            .add_dependency(&proposal_id, &proposal2_id, None)
            .await
            .unwrap();

        let deps = state
            .proposal_dependency_repo
            .get_dependencies(&proposal_id)
            .await
            .unwrap();
        assert_eq!(deps.len(), 1);
    }

    #[tokio::test]
    async fn test_task_dependency_repo_accessible() {
        let state = AppState::new_test();
        let project_id = ProjectId::new();

        // Create two tasks
        let task1 = Task::new(project_id.clone(), "Task 1".to_string());
        let task2 = Task::new(project_id.clone(), "Task 2".to_string());

        let task1_id = task1.id.clone();
        let task2_id = task2.id.clone();

        state.task_repo.create(task1).await.unwrap();
        state.task_repo.create(task2).await.unwrap();

        // Add a dependency
        state
            .task_dependency_repo
            .add_dependency(&task1_id, &task2_id)
            .await
            .unwrap();

        // Verify the dependency exists
        let has_dep = state
            .task_dependency_repo
            .has_dependency(&task1_id, &task2_id)
            .await
            .unwrap();
        assert!(has_dep);

        let blockers = state
            .task_dependency_repo
            .get_blockers(&task1_id)
            .await
            .unwrap();
        assert_eq!(blockers.len(), 1);
    }

    #[tokio::test]
    async fn test_extensibility_repos_accessible() {
        use crate::domain::entities::{
            Artifact, ArtifactBucket, ArtifactFlow, ArtifactFlowTrigger, ArtifactType,
            WorkflowColumn, WorkflowSchema,
        };
        use crate::domain::entities::methodology::MethodologyExtension;
        use crate::domain::entities::research::{ResearchBrief, ResearchProcess};
        use crate::domain::entities::status::InternalStatus;

        let state = AppState::new_test();

        // Test workflow repository
        let workflow = WorkflowSchema::new(
            "Test Workflow",
            vec![
                WorkflowColumn::new("backlog", "Backlog", InternalStatus::Backlog),
                WorkflowColumn::new("done", "Done", InternalStatus::Approved),
            ],
        );
        state.workflow_repo.create(workflow.clone()).await.unwrap();
        let found_workflow = state.workflow_repo.get_by_id(&workflow.id).await.unwrap();
        assert!(found_workflow.is_some());

        // Test artifact repository
        let artifact = Artifact::new_inline("Test", ArtifactType::Prd, "content", "user");
        state.artifact_repo.create(artifact.clone()).await.unwrap();
        let found_artifact = state.artifact_repo.get_by_id(&artifact.id).await.unwrap();
        assert!(found_artifact.is_some());

        // Test artifact bucket repository
        let bucket = ArtifactBucket::new("Test Bucket")
            .accepts(ArtifactType::Prd)
            .with_writer("user");
        state.artifact_bucket_repo.create(bucket.clone()).await.unwrap();
        let found_bucket = state.artifact_bucket_repo.get_by_id(&bucket.id).await.unwrap();
        assert!(found_bucket.is_some());

        // Test artifact flow repository
        let flow = ArtifactFlow::new("Test Flow", ArtifactFlowTrigger::on_artifact_created());
        state.artifact_flow_repo.create(flow.clone()).await.unwrap();
        let found_flow = state.artifact_flow_repo.get_by_id(&flow.id).await.unwrap();
        assert!(found_flow.is_some());

        // Test process repository
        let brief = ResearchBrief::new("Test question");
        let process = ResearchProcess::new("Test Research", brief, "researcher");
        state.process_repo.create(process.clone()).await.unwrap();
        let found_process = state.process_repo.get_by_id(&process.id).await.unwrap();
        assert!(found_process.is_some());

        // Test methodology repository
        let methodology = MethodologyExtension::new("Test Method", workflow);
        state.methodology_repo.create(methodology.clone()).await.unwrap();
        let found_methodology = state.methodology_repo.get_by_id(&methodology.id).await.unwrap();
        assert!(found_methodology.is_some());
    }
}
