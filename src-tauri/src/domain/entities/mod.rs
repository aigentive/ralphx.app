// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod project;
pub mod status;
pub mod task;
pub mod types;

// Re-export commonly used types for convenience
pub use project::{GitMode, Project};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use task::Task;
pub use types::{ProjectId, TaskId};
