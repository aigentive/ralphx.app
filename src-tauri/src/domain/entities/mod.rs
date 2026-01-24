// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod ideation;
pub mod project;
pub mod review;
pub mod status;
pub mod task;
pub mod task_qa;
pub mod types;

// Re-export commonly used types for convenience
pub use ideation::{
    BusinessValueFactor, Complexity, ComplexityFactor, CriticalPathFactor, DependencyFactor,
    IdeationSession, IdeationSessionBuilder, IdeationSessionStatus, ParseComplexityError,
    ParseIdeationSessionStatusError, ParsePriorityError, ParseProposalStatusError,
    ParseTaskCategoryError, Priority, PriorityAssessment, PriorityAssessmentFactors,
    PriorityFactors, ProposalStatus, TaskCategory, TaskProposal, UserHintFactor,
};
pub use project::{GitMode, Project};
pub use review::{
    ParseReviewActionTypeError, ParseReviewOutcomeError, ParseReviewStatusError,
    ParseReviewerTypeError, Review, ReviewAction, ReviewActionId, ReviewActionType, ReviewId,
    ReviewNote, ReviewNoteId, ReviewOutcome, ReviewStatus, ReviewerType,
};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use task::Task;
pub use task_qa::TaskQA;
pub use types::{IdeationSessionId, ProjectId, TaskId, TaskProposalId, TaskQAId};
