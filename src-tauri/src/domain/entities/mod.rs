// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod project;
pub mod status;
pub mod types;

// Placeholder modules - will be implemented in subsequent tasks
// pub mod task;

// Re-export commonly used types for convenience
pub use project::{GitMode, Project};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use types::{ProjectId, TaskId};
