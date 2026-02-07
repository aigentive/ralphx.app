// Repository traits - domain layer abstractions for data persistence
// These traits define the contract; implementations live in infrastructure layer

pub mod activity_event_repository;
pub mod agent_profile_repository;
pub mod app_state_repository;
pub mod agent_run_repository;
pub mod artifact_bucket_repository;
pub mod artifact_flow_repository;
pub mod artifact_repository;
pub mod chat_conversation_repository;
pub mod chat_message_repository;
pub mod execution_settings_repository;
pub mod ideation_session_repository;
pub mod ideation_settings_repository;
pub mod methodology_repo;
pub mod plan_branch_repository;
pub mod process_repo;
pub mod project_repository;
pub mod proposal_dependency_repository;
pub mod review_repository;
pub mod review_settings_repository;
pub mod status_transition;
pub mod task_dependency_repository;
pub mod task_proposal_repository;
pub mod task_qa_repository;
pub mod task_repository;
pub mod task_step_repository;
pub mod workflow_repository;

// Re-exports for convenience
pub use app_state_repository::AppStateRepository;
pub use activity_event_repository::{
    ActivityEventFilter, ActivityEventPage, ActivityEventRepository,
};
pub use agent_profile_repository::{AgentProfileId, AgentProfileRepository};
pub use agent_run_repository::AgentRunRepository;
pub use artifact_bucket_repository::ArtifactBucketRepository;
pub use artifact_flow_repository::ArtifactFlowRepository;
pub use artifact_repository::{ArtifactRepository, ArtifactVersionSummary};
pub use chat_conversation_repository::ChatConversationRepository;
pub use chat_message_repository::ChatMessageRepository;
pub use execution_settings_repository::{
    ExecutionSettingsRepository, GlobalExecutionSettingsRepository,
};
pub use ideation_session_repository::IdeationSessionRepository;
pub use ideation_settings_repository::IdeationSettingsRepository;
pub use methodology_repo::MethodologyRepository;
pub use plan_branch_repository::PlanBranchRepository;
pub use process_repo::ProcessRepository;
pub use project_repository::ProjectRepository;
pub use proposal_dependency_repository::ProposalDependencyRepository;
pub use review_repository::ReviewRepository;
pub use review_settings_repository::ReviewSettingsRepository;
pub use status_transition::StatusTransition;
pub use task_dependency_repository::TaskDependencyRepository;
pub use task_proposal_repository::TaskProposalRepository;
pub use task_qa_repository::TaskQARepository;
pub use task_repository::{StateHistoryMetadata, TaskRepository};
pub use task_step_repository::TaskStepRepository;
pub use workflow_repository::WorkflowRepository;
