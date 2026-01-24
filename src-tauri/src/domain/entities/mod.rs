// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod project;
pub mod review;
pub mod status;
pub mod task;
pub mod task_qa;
pub mod types;

// Re-export commonly used types for convenience
pub use project::{GitMode, Project};
pub use review::{
    ParseReviewActionTypeError, ParseReviewStatusError, ParseReviewerTypeError, Review,
    ReviewAction, ReviewActionId, ReviewActionType, ReviewId, ReviewStatus, ReviewerType,
};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use task::Task;
pub use task_qa::TaskQA;
pub use types::{ProjectId, TaskId, TaskQAId};
