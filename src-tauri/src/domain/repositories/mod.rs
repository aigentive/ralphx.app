// Repository traits - domain layer abstractions for data persistence
// These traits define the contract; implementations live in infrastructure layer

pub mod agent_profile_repository;
pub mod project_repository;
pub mod review_repository;
pub mod status_transition;
pub mod task_qa_repository;
pub mod task_repository;

// Re-exports for convenience
pub use agent_profile_repository::{AgentProfileId, AgentProfileRepository};
pub use project_repository::ProjectRepository;
pub use review_repository::ReviewRepository;
pub use status_transition::StatusTransition;
pub use task_qa_repository::TaskQARepository;
pub use task_repository::TaskRepository;
