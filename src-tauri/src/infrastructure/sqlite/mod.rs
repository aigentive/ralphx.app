// SQLite infrastructure layer
// Database connection management, migrations, and repository implementations

pub mod connection;
pub mod migrations;
pub mod sqlite_agent_profile_repo;
pub mod sqlite_agent_run_repo;
pub mod sqlite_artifact_bucket_repo;
pub mod sqlite_artifact_flow_repo;
pub mod sqlite_artifact_repo;
pub mod sqlite_chat_conversation_repo;
pub mod sqlite_chat_message_repo;
pub mod sqlite_ideation_session_repo;
pub mod sqlite_ideation_settings_repo;
pub mod sqlite_methodology_repo;
pub mod sqlite_process_repo;
pub mod sqlite_project_repo;
pub mod sqlite_proposal_dependency_repo;
pub mod sqlite_task_dependency_repo;
pub mod sqlite_task_proposal_repo;
pub mod sqlite_review_repo;
pub mod sqlite_task_qa_repo;
pub mod sqlite_task_repo;
pub mod sqlite_task_step_repo;
pub mod sqlite_workflow_repo;
pub mod state_machine_repository;

// Re-export commonly used items
pub use connection::{get_default_db_path, open_connection, open_memory_connection};
pub use migrations::{run_migrations, SCHEMA_VERSION};
pub use sqlite_agent_profile_repo::SqliteAgentProfileRepository;
pub use sqlite_agent_run_repo::SqliteAgentRunRepository;
pub use sqlite_artifact_bucket_repo::SqliteArtifactBucketRepository;
pub use sqlite_artifact_flow_repo::SqliteArtifactFlowRepository;
pub use sqlite_artifact_repo::SqliteArtifactRepository;
pub use sqlite_chat_conversation_repo::SqliteChatConversationRepository;
pub use sqlite_chat_message_repo::SqliteChatMessageRepository;
pub use sqlite_ideation_session_repo::SqliteIdeationSessionRepository;
pub use sqlite_ideation_settings_repo::SqliteIdeationSettingsRepository;
pub use sqlite_methodology_repo::SqliteMethodologyRepository;
pub use sqlite_process_repo::SqliteProcessRepository;
pub use sqlite_project_repo::SqliteProjectRepository;
pub use sqlite_proposal_dependency_repo::SqliteProposalDependencyRepository;
pub use sqlite_task_dependency_repo::SqliteTaskDependencyRepository;
pub use sqlite_task_proposal_repo::SqliteTaskProposalRepository;
pub use sqlite_review_repo::SqliteReviewRepository;
pub use sqlite_task_qa_repo::SqliteTaskQARepository;
pub use sqlite_task_repo::SqliteTaskRepository;
pub use sqlite_task_step_repo::SqliteTaskStepRepository;
pub use sqlite_workflow_repo::SqliteWorkflowRepository;
pub use state_machine_repository::TaskStateMachineRepository;
