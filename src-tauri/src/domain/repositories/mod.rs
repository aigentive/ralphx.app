// Repository traits - domain layer abstractions for data persistence
// These traits define the contract; implementations live in infrastructure layer

pub mod status_transition;
pub mod task_repository;
pub mod project_repository;

// Re-exports for convenience
pub use status_transition::StatusTransition;
