// Application state container for dependency injection
// Holds repository trait objects that can be swapped for testing

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::application::PermissionState;
use crate::application::QuestionState;
use crate::application::ResumeValidator;
use crate::domain::agents::AgenticClient;
use crate::domain::entities::IdeationSessionId;
use crate::domain::qa::QASettings;
use crate::domain::repositories::{
    ActivePlanRepository, ActivityEventRepository, AgentProfileRepository, AgentRunRepository,
    AppStateRepository, ArtifactBucketRepository, ArtifactFlowRepository, ArtifactRepository,
    ChatAttachmentRepository, ChatConversationRepository, ChatMessageRepository,
    ExecutionPlanRepository, ExecutionSettingsRepository, GlobalExecutionSettingsRepository,
    IdeationSessionRepository, IdeationSettingsRepository, MemoryArchiveRepository,
    MemoryEntryRepository, MemoryEventRepository, MethodologyRepository, PlanBranchRepository,
    PlanSelectionStatsRepository, ProcessRepository, ProjectRepository,
    ProposalDependencyRepository, ReviewRepository, ReviewSettingsRepository,
    SessionLinkRepository, TaskDependencyRepository, TaskProposalRepository, TaskQARepository,
    TaskRepository, TaskStepRepository, TeamMessageRepository, TeamSessionRepository,
    WorkflowRepository,
};
use crate::domain::services::{MemoryRunningAgentRegistry, MessageQueue, RunningAgentRegistry};
use crate::error::AppResult;
use crate::infrastructure::memory::{
    InMemoryMemoryEntryRepository, InMemoryMemoryEventRepository, MemoryActivePlanRepository,
    MemoryActivityEventRepository, MemoryAgentProfileRepository, MemoryAgentRunRepository,
    MemoryAppStateRepository, MemoryArtifactBucketRepository, MemoryArtifactFlowRepository,
    MemoryArtifactRepository, MemoryChatAttachmentRepository, MemoryChatConversationRepository,
    MemoryChatMessageRepository, MemoryExecutionPlanRepository, MemoryExecutionSettingsRepository,
    MemoryGlobalExecutionSettingsRepository, MemoryIdeationSessionRepository,
    MemoryIdeationSettingsRepository, MemoryMethodologyRepository, MemoryPermissionRepository,
    MemoryPlanBranchRepository, MemoryPlanSelectionStatsRepository, MemoryProcessRepository,
    MemoryProjectRepository, MemoryProposalDependencyRepository, MemoryQuestionRepository,
    MemoryReviewIssueRepository, MemoryReviewRepository, MemoryReviewSettingsRepository,
    MemorySessionLinkRepository, MemoryTaskDependencyRepository, MemoryTaskProposalRepository,
    MemoryTaskQARepository, MemoryTaskRepository, MemoryTaskStepRepository,
    MemoryTeamMessageRepository, MemoryTeamSessionRepository, MemoryWorkflowRepository,
};
use crate::infrastructure::sqlite::ReviewIssueRepository;
use crate::infrastructure::sqlite::{
    get_app_data_db_path, get_default_db_path, open_connection, run_migrations,
    SqliteActivePlanRepository, SqliteActivityEventRepository, SqliteAgentProfileRepository,
    SqliteAgentRunRepository, SqliteAppStateRepository, SqliteArtifactBucketRepository,
    SqliteArtifactFlowRepository, SqliteArtifactRepository, SqliteChatAttachmentRepository,
    SqliteChatConversationRepository, SqliteChatMessageRepository,
    SqliteExecutionPlanRepository, SqliteExecutionSettingsRepository,
    SqliteGlobalExecutionSettingsRepository, SqliteIdeationSessionRepository,
    SqliteIdeationSettingsRepository, SqliteMemoryArchiveRepository, SqliteMemoryEntryRepository,
    SqliteMemoryEventRepository, SqliteMethodologyRepository, SqlitePermissionRepository,
    SqlitePlanBranchRepository, SqlitePlanSelectionStatsRepository, SqliteProcessRepository,
    SqliteProjectRepository, SqliteProposalDependencyRepository, SqliteQuestionRepository,
    SqliteReviewIssueRepository, SqliteReviewRepository, SqliteReviewSettingsRepository,
    SqliteRunningAgentRegistry, SqliteSessionLinkRepository, SqliteTaskDependencyRepository,
    SqliteTaskProposalRepository, SqliteTaskQARepository, SqliteTaskRepository,
    SqliteTaskStepRepository, SqliteTeamMessageRepository, SqliteTeamSessionRepository,
    SqliteWorkflowRepository,
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
    /// Session link repository for managing parent-child session relationships
    pub session_link_repo: Arc<dyn SessionLinkRepository>,
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
    /// Question state for inline AskUserQuestion from agents
    pub question_state: Arc<QuestionState>,
    /// Unified message queue for all chat contexts
    pub message_queue: Arc<MessageQueue>,
    /// Registry for tracking running agent processes
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    /// Sessions currently undergoing dependency analysis (for status reporting in MCP tools).
    ///
    /// **Volatility note:** This is a pure in-memory set — it is NOT persisted to SQLite.
    /// On app restart, this set starts empty. Any session whose analysis was in progress at
    /// the time of the crash/restart will NOT appear here. This is correct behaviour:
    /// - The backend auto-clears any stuck entries with a 60-second safety timeout.
    /// - The frontend `isAnalyzingDependencies` state resets to `false` on component mount
    ///   (React `useState` default), so there is no stuck loading UI after a restart.
    /// - If the user restarts mid-analysis, they can manually trigger re-analysis from the
    ///   UI. No manual cleanup is needed.
    pub analyzing_dependencies: Arc<tokio::sync::RwLock<HashSet<IdeationSessionId>>>,
    /// Plan branch repository for feature branch tracking
    pub plan_branch_repo: Arc<dyn PlanBranchRepository>,
    /// Plan selection stats repository for tracking plan selection interactions
    pub plan_selection_stats_repo: Arc<dyn PlanSelectionStatsRepository>,
    /// App state repository for persisting active_project_id across restarts
    pub app_state_repo: Arc<dyn AppStateRepository>,
    /// Active plan repository for persisting active plan per project
    pub active_plan_repo: Arc<dyn ActivePlanRepository>,
    // Memory framework repositories
    /// Memory entry repository for storing ingested memories
    pub memory_entry_repo: Arc<dyn MemoryEntryRepository>,
    /// Memory event repository for audit trail
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    /// Memory archive repository for snapshot generation job queue
    pub memory_archive_repo: Arc<dyn MemoryArchiveRepository>,
    /// Team session repository for agent team history
    pub team_session_repo: Arc<dyn TeamSessionRepository>,
    /// Team message repository for agent team messages
    pub team_message_repo: Arc<dyn TeamMessageRepository>,
    /// Execution plan repository for tracking plan implementation attempts
    pub execution_plan_repo: Arc<dyn ExecutionPlanRepository>,
    /// Chat attachment repository for file uploads in chat
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    /// Storage path for chat attachments
    pub attachment_storage_path: PathBuf,
    /// Streaming state cache for hydrating frontend on navigation to active conversations
    pub streaming_state_cache: crate::application::chat_service::StreamingStateCache,
    /// Interactive process registry for stdin-based multi-turn messaging
    pub interactive_process_registry: Arc<crate::application::InteractiveProcessRegistry>,
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
        let task_proposal_repo: Arc<dyn TaskProposalRepository> = Arc::new(
            SqliteTaskProposalRepository::from_shared(Arc::clone(&shared_conn)),
        );
        let artifact_repo: Arc<dyn ArtifactRepository> = Arc::new(
            SqliteArtifactRepository::from_shared(Arc::clone(&shared_conn)),
        );

        // Chat attachment repository
        let chat_attachment_repo: Arc<dyn ChatAttachmentRepository> = Arc::new(
            SqliteChatAttachmentRepository::from_shared(Arc::clone(&shared_conn)),
        );
        let attachment_storage_path = if cfg!(debug_assertions) {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        } else {
            app_handle
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
        };

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
            task_qa_repo: Arc::new(SqliteTaskQARepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            review_repo: Arc::new(SqliteReviewRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
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
                SqliteGlobalExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn)),
            ),
            ideation_session_repo: Arc::new(SqliteIdeationSessionRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            ideation_settings_repo: Arc::new(SqliteIdeationSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            session_link_repo: Arc::new(SqliteSessionLinkRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
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
            methodology_repo: Arc::new(SqliteMethodologyRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            plan_branch_repo: Arc::new(SqlitePlanBranchRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            plan_selection_stats_repo: Arc::new(SqlitePlanSelectionStatsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            app_state_repo: Arc::new(SqliteAppStateRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            active_plan_repo: Arc::new(SqliteActivePlanRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_entry_repo: Arc::new(SqliteMemoryEntryRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_event_repo: Arc::new(SqliteMemoryEventRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_archive_repo: Arc::new(SqliteMemoryArchiveRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            team_session_repo: Arc::new(SqliteTeamSessionRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            team_message_repo: Arc::new(SqliteTeamMessageRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            execution_plan_repo: Arc::new(SqliteExecutionPlanRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            chat_attachment_repo,
            attachment_storage_path,
            permission_state: Arc::new(PermissionState::with_repo(Arc::new(
                SqlitePermissionRepository::from_shared(Arc::clone(&shared_conn)),
            ))),
            question_state: Arc::new(QuestionState::with_repo(Arc::new(
                SqliteQuestionRepository::from_shared(Arc::clone(&shared_conn)),
            ))),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(SqliteRunningAgentRegistry::new(shared_conn)),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            streaming_state_cache: crate::application::chat_service::StreamingStateCache::new(),
            interactive_process_registry: Arc::new(crate::application::InteractiveProcessRegistry::new()),
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
        let task_proposal_repo: Arc<dyn TaskProposalRepository> = Arc::new(
            SqliteTaskProposalRepository::from_shared(Arc::clone(&shared_conn)),
        );
        let artifact_repo: Arc<dyn ArtifactRepository> = Arc::new(
            SqliteArtifactRepository::from_shared(Arc::clone(&shared_conn)),
        );

        // Chat attachment repository
        let chat_attachment_repo: Arc<dyn ChatAttachmentRepository> = Arc::new(
            SqliteChatAttachmentRepository::from_shared(Arc::clone(&shared_conn)),
        );
        let attachment_storage_path = app_handle
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| PathBuf::from("."));

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
            task_qa_repo: Arc::new(SqliteTaskQARepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            review_repo: Arc::new(SqliteReviewRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
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
                SqliteGlobalExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn)),
            ),
            ideation_session_repo: Arc::new(SqliteIdeationSessionRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            ideation_settings_repo: Arc::new(SqliteIdeationSettingsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            session_link_repo: Arc::new(SqliteSessionLinkRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
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
            methodology_repo: Arc::new(SqliteMethodologyRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            plan_branch_repo: Arc::new(SqlitePlanBranchRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            plan_selection_stats_repo: Arc::new(SqlitePlanSelectionStatsRepository::from_shared(
                Arc::clone(&shared_conn),
            )),
            app_state_repo: Arc::new(SqliteAppStateRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            active_plan_repo: Arc::new(SqliteActivePlanRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_entry_repo: Arc::new(SqliteMemoryEntryRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_event_repo: Arc::new(SqliteMemoryEventRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            memory_archive_repo: Arc::new(SqliteMemoryArchiveRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            team_session_repo: Arc::new(SqliteTeamSessionRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            team_message_repo: Arc::new(SqliteTeamMessageRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            execution_plan_repo: Arc::new(SqliteExecutionPlanRepository::from_shared(Arc::clone(
                &shared_conn,
            ))),
            chat_attachment_repo,
            attachment_storage_path,
            permission_state: Arc::new(PermissionState::with_repo(Arc::new(
                SqlitePermissionRepository::from_shared(Arc::clone(&shared_conn)),
            ))),
            question_state: Arc::new(QuestionState::with_repo(Arc::new(
                SqliteQuestionRepository::from_shared(Arc::clone(&shared_conn)),
            ))),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(SqliteRunningAgentRegistry::new(shared_conn)),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            streaming_state_cache: crate::application::chat_service::StreamingStateCache::new(),
            interactive_process_registry: Arc::new(crate::application::InteractiveProcessRegistry::new()),
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

        // Chat attachment repository for tests
        let chat_attachment_repo: Arc<dyn ChatAttachmentRepository> =
            Arc::new(MemoryChatAttachmentRepository::new());
        let attachment_storage_path = std::env::temp_dir();

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
            session_link_repo: Arc::new(MemorySessionLinkRepository::new()),
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
            plan_branch_repo: Arc::new(MemoryPlanBranchRepository::new()),
            plan_selection_stats_repo: Arc::new(MemoryPlanSelectionStatsRepository::new()),
            app_state_repo: Arc::new(MemoryAppStateRepository::new()),
            active_plan_repo: Arc::new(MemoryActivePlanRepository::new()),
            memory_entry_repo: Arc::new(InMemoryMemoryEntryRepository::new()),
            memory_event_repo: Arc::new(InMemoryMemoryEventRepository::new()),
            memory_archive_repo: Arc::new(SqliteMemoryArchiveRepository::new(
                open_connection(&PathBuf::from(":memory:"))
                    .expect("Failed to create in-memory connection"),
            )),
            team_session_repo: Arc::new(MemoryTeamSessionRepository::new()),
            team_message_repo: Arc::new(MemoryTeamMessageRepository::new()),
            execution_plan_repo: Arc::new(MemoryExecutionPlanRepository::new()),
            chat_attachment_repo,
            attachment_storage_path,
            permission_state: Arc::new(PermissionState::with_repo(Arc::new(
                MemoryPermissionRepository::new(),
            ))),
            question_state: Arc::new(QuestionState::with_repo(Arc::new(
                MemoryQuestionRepository::new(),
            ))),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(MemoryRunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            streaming_state_cache: crate::application::chat_service::StreamingStateCache::new(),
            interactive_process_registry: Arc::new(crate::application::InteractiveProcessRegistry::new()),
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

        // Chat attachment repository for tests
        let chat_attachment_repo: Arc<dyn ChatAttachmentRepository> =
            Arc::new(MemoryChatAttachmentRepository::new());
        let attachment_storage_path = std::env::temp_dir();

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
            session_link_repo: Arc::new(MemorySessionLinkRepository::new()),
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
            plan_branch_repo: Arc::new(MemoryPlanBranchRepository::new()),
            plan_selection_stats_repo: Arc::new(MemoryPlanSelectionStatsRepository::new()),
            app_state_repo: Arc::new(MemoryAppStateRepository::new()),
            active_plan_repo: Arc::new(MemoryActivePlanRepository::new()),
            memory_entry_repo: Arc::new(InMemoryMemoryEntryRepository::new()),
            memory_event_repo: Arc::new(InMemoryMemoryEventRepository::new()),
            memory_archive_repo: Arc::new(SqliteMemoryArchiveRepository::new(
                open_connection(&PathBuf::from(":memory:"))
                    .expect("Failed to create in-memory connection"),
            )),
            team_session_repo: Arc::new(MemoryTeamSessionRepository::new()),
            team_message_repo: Arc::new(MemoryTeamMessageRepository::new()),
            execution_plan_repo: Arc::new(MemoryExecutionPlanRepository::new()),
            chat_attachment_repo,
            attachment_storage_path,
            permission_state: Arc::new(PermissionState::with_repo(Arc::new(
                MemoryPermissionRepository::new(),
            ))),
            question_state: Arc::new(QuestionState::with_repo(Arc::new(
                MemoryQuestionRepository::new(),
            ))),
            message_queue: Arc::new(MessageQueue::new()),
            running_agent_registry: Arc::new(MemoryRunningAgentRegistry::new()),
            analyzing_dependencies: Arc::new(tokio::sync::RwLock::new(HashSet::new())),
            streaming_state_cache: crate::application::chat_service::StreamingStateCache::new(),
            interactive_process_registry: Arc::new(crate::application::InteractiveProcessRegistry::new()),
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

    /// Create a ResumeValidator for task resume validation
    pub fn resume_validator(&self) -> ResumeValidator {
        ResumeValidator::new(Arc::clone(&self.running_agent_registry))
            .with_interactive_process_registry(Arc::clone(&self.interactive_process_registry))
    }
}

#[cfg(test)]
#[path = "app_state_tests.rs"]
mod tests;
