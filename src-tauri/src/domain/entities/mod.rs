// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod activity_event;
pub mod agent_run;
pub mod artifact;
pub mod artifact_flow;
pub mod chat_conversation;
pub mod ideation;
pub mod methodology;
pub mod plan_branch;
pub mod research;
pub mod project;
pub mod review;
pub mod review_issue;
pub mod status;
pub mod task;
pub mod task_context;
pub mod task_qa;
pub mod task_step;
pub mod types;
pub mod workflow;

// Re-export commonly used types for convenience
pub use ideation::{
    BusinessValueFactor, ChatMessage, Complexity, ComplexityFactor, CriticalPathFactor,
    DependencyFactor, DependencyGraph, DependencyGraphEdge, DependencyGraphNode, IdeationSession,
    IdeationSessionBuilder, IdeationSessionStatus, MessageRole, ParseComplexityError,
    ParseIdeationSessionStatusError, ParseMessageRoleError, ParsePriorityError,
    ParseProposalStatusError, ParseTaskCategoryError, Priority, PriorityAssessment,
    PriorityAssessmentFactors, PriorityFactors, ProposalStatus, TaskCategory, TaskProposal,
    UserHintFactor,
};
pub use plan_branch::{ParsePlanBranchStatusError, PlanBranch, PlanBranchId, PlanBranchStatus};
pub use project::{GitMode, Project};
pub use review::{
    ParseReviewActionTypeError, ParseReviewOutcomeError, ParseReviewStatusError,
    ParseReviewerTypeError, Review, ReviewAction, ReviewActionId, ReviewActionType, ReviewId,
    ReviewIssue, ReviewNote, ReviewNoteId, ReviewOutcome, ReviewStatus, ReviewerType,
};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use task::Task;
pub use task_qa::TaskQA;
pub use types::{ChatMessageId, IdeationSessionId, ProjectId, ReviewIssueId, TaskId, TaskProposalId, TaskQAId, TaskStepId};
pub use workflow::{
    ColumnBehavior, ConflictResolution, ExternalStatusMapping, ExternalSyncConfig,
    ParseSyncDirectionError, SyncDirection, SyncProvider, SyncSettings, WorkflowColumn,
    WorkflowDefaults, WorkflowId, WorkflowSchema,
};
pub use artifact::{
    Artifact, ArtifactBucket, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata,
    ArtifactRelation, ArtifactRelationId, ArtifactRelationType, ArtifactType,
    ParseArtifactRelationTypeError, ParseArtifactTypeError, ProcessId,
};
pub use artifact_flow::{
    ArtifactFlow, ArtifactFlowContext, ArtifactFlowEngine, ArtifactFlowEvaluation,
    ArtifactFlowEvent, ArtifactFlowFilter, ArtifactFlowId, ArtifactFlowStep, ArtifactFlowTrigger,
    ParseArtifactFlowEventError, create_plan_updated_sync_flow, create_research_to_dev_flow,
};
pub use research::{
    CustomDepth, ParseResearchDepthPresetError, ParseResearchProcessStatusError, ResearchBrief,
    ResearchDepth, ResearchDepthPreset, ResearchOutput, ResearchPresets, ResearchProcess,
    ResearchProcessId, ResearchProcessStatus, ResearchProgress, RESEARCH_PRESETS,
};
pub use methodology::{
    MethodologyExtension, MethodologyId, MethodologyPhase, MethodologyPlanArtifactConfig,
    MethodologyPlanTemplate, MethodologyStatus, MethodologyTemplate, ParseMethodologyStatusError,
};
pub use chat_conversation::{ChatContextType, ChatConversation, ChatConversationId};
pub use agent_run::{AgentRun, AgentRunId, AgentRunStatus, InterruptedConversation};
pub use task_context::{ArtifactSummary, TaskContext, TaskDependencySummary, TaskProposalSummary};
pub use task_step::{StepProgressSummary, TaskStep, TaskStepStatus};
pub use activity_event::{
    ActivityEvent, ActivityEventId, ActivityEventRole, ActivityEventType,
    ParseActivityEventRoleError, ParseActivityEventTypeError,
};
pub use review_issue::{
    IssueCategory, IssueProgressSummary, IssueSeverity, IssueStatus, ParseIssueCategoryError,
    ParseIssueSeverityError, ParseIssueStatusError, ReviewIssue as ReviewIssueEntity,
    SeverityBreakdown, SeverityCount,
};
