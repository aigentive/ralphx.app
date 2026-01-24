// In-memory repository implementations for testing
// These implementations use HashMap/RwLock for thread-safe in-memory storage

pub mod memory_agent_profile_repo;
pub mod memory_chat_message_repo;
pub mod memory_ideation_session_repo;
pub mod memory_project_repo;
pub mod memory_proposal_dependency_repo;
pub mod memory_review_repo;
pub mod memory_task_dependency_repo;
pub mod memory_task_proposal_repo;
pub mod memory_task_qa_repo;
pub mod memory_task_repo;
pub mod memory_workflow_repo;

// Re-exports for convenience
pub use memory_agent_profile_repo::MemoryAgentProfileRepository;
pub use memory_chat_message_repo::MemoryChatMessageRepository;
pub use memory_ideation_session_repo::MemoryIdeationSessionRepository;
pub use memory_project_repo::MemoryProjectRepository;
pub use memory_proposal_dependency_repo::MemoryProposalDependencyRepository;
pub use memory_review_repo::MemoryReviewRepository;
pub use memory_task_dependency_repo::MemoryTaskDependencyRepository;
pub use memory_task_proposal_repo::MemoryTaskProposalRepository;
pub use memory_task_qa_repo::MemoryTaskQARepository;
pub use memory_task_repo::MemoryTaskRepository;
pub use memory_workflow_repo::MemoryWorkflowRepository;
