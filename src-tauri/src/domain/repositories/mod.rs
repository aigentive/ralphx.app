// Repository traits - domain layer abstractions for data persistence
// These traits define the contract; implementations live in infrastructure layer

pub mod agent_profile_repository;
pub mod artifact_bucket_repository;
pub mod artifact_repository;
pub mod chat_message_repository;
pub mod ideation_session_repository;
pub mod project_repository;
pub mod proposal_dependency_repository;
pub mod review_repository;
pub mod status_transition;
pub mod task_dependency_repository;
pub mod task_proposal_repository;
pub mod task_qa_repository;
pub mod task_repository;
pub mod workflow_repository;

// Re-exports for convenience
pub use agent_profile_repository::{AgentProfileId, AgentProfileRepository};
pub use artifact_bucket_repository::ArtifactBucketRepository;
pub use artifact_repository::ArtifactRepository;
pub use chat_message_repository::ChatMessageRepository;
pub use ideation_session_repository::IdeationSessionRepository;
pub use project_repository::ProjectRepository;
pub use proposal_dependency_repository::ProposalDependencyRepository;
pub use review_repository::ReviewRepository;
pub use status_transition::StatusTransition;
pub use task_dependency_repository::TaskDependencyRepository;
pub use task_proposal_repository::TaskProposalRepository;
pub use task_qa_repository::TaskQARepository;
pub use task_repository::TaskRepository;
pub use workflow_repository::WorkflowRepository;
