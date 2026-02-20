// Domain entities - pure Rust types with no external dependencies
// These represent the core business objects of RalphX

pub mod activity_event;
pub mod agent_run;
pub mod app_state;
pub mod artifact;
pub mod artifact_flow;
pub mod chat_attachment;
pub mod chat_conversation;
pub mod ideation;
pub mod memory_archive;
pub mod memory_archive_job;
pub mod memory_entry;
pub mod memory_event;
pub mod memory_rule_binding;
pub mod merge_progress_event;
pub mod methodology;
pub mod plan_branch;
pub mod plan_selection_stats;
pub mod project;
pub mod research;
pub mod review;
pub mod review_issue;
pub mod status;
pub mod task;
pub mod task_context;
pub mod task_metadata;
pub mod task_qa;
pub mod task_step;
pub mod team;
pub mod types;
pub mod workflow;

// Re-export commonly used types for convenience
pub use activity_event::{
    ActivityEvent, ActivityEventId, ActivityEventRole, ActivityEventType,
    ParseActivityEventRoleError, ParseActivityEventTypeError,
};
pub use agent_run::{AgentRun, AgentRunId, AgentRunStatus, InterruptedConversation};
pub use app_state::AppSettings;
pub use artifact::{
    Artifact, ArtifactBucket, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata,
    ArtifactRelation, ArtifactRelationId, ArtifactRelationType, ArtifactType,
    ParseArtifactRelationTypeError, ParseArtifactTypeError, ProcessId, TeamArtifactMetadata,
};
pub use artifact_flow::{
    create_plan_updated_sync_flow, create_research_to_dev_flow, ArtifactFlow, ArtifactFlowContext,
    ArtifactFlowEngine, ArtifactFlowEvaluation, ArtifactFlowEvent, ArtifactFlowFilter,
    ArtifactFlowId, ArtifactFlowStep, ArtifactFlowTrigger, ParseArtifactFlowEventError,
};
pub use chat_attachment::{ChatAttachment, ChatAttachmentId};
pub use chat_conversation::{ChatContextType, ChatConversation, ChatConversationId};
pub use ideation::{
    BusinessValueFactor, ChatMessage, Complexity, ComplexityFactor, CriticalPathFactor,
    DependencyFactor, DependencyGraph, DependencyGraphEdge, DependencyGraphNode, IdeationSession,
    IdeationSessionBuilder, IdeationSessionStatus, MessageRole, ParseComplexityError,
    ParseIdeationSessionStatusError, ParseMessageRoleError, ParsePriorityError,
    ParseProposalCategoryError, ParseProposalStatusError, Priority, PriorityAssessment,
    PriorityAssessmentFactors, PriorityFactors, ProposalCategory, ProposalStatus, SessionLink,
    SessionRelationship, TaskProposal, UserHintFactor,
};
pub use memory_archive::{
    ArchiveJobPayload, ArchiveJobStatus, ArchiveJobType, FullRebuildPayload, MemoryArchiveJob,
    MemoryArchiveJobId, MemorySnapshotPayload, RuleSnapshotPayload,
};
pub use memory_archive_job::{MemoryArchiveJobStatus, MemoryArchiveJobType};
pub use memory_entry::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus};
pub use memory_event::{MemoryActorType, MemoryEvent, MemoryEventId, ParseMemoryActorTypeError};
pub use memory_rule_binding::MemoryRuleBinding;
pub use merge_progress_event::{MergePhase, MergePhaseInfo, MergePhaseStatus, MergeProgressEvent};
pub use methodology::{
    MethodologyExtension, MethodologyId, MethodologyPhase, MethodologyPlanArtifactConfig,
    MethodologyPlanTemplate, MethodologyStatus, MethodologyTemplate, ParseMethodologyStatusError,
};
pub use plan_branch::{ParsePlanBranchStatusError, PlanBranch, PlanBranchId, PlanBranchStatus};
pub use plan_selection_stats::{PlanSelectionStats, SelectionSource};
pub use project::{GitMode, MergeStrategy, MergeValidationMode, Project};
pub use research::{
    CustomDepth, ParseResearchDepthPresetError, ParseResearchProcessStatusError, ResearchBrief,
    ResearchDepth, ResearchDepthPreset, ResearchOutput, ResearchPresets, ResearchProcess,
    ResearchProcessId, ResearchProcessStatus, ResearchProgress, RESEARCH_PRESETS,
};
pub use review::{
    ParseReviewActionTypeError, ParseReviewOutcomeError, ParseReviewStatusError,
    ParseReviewerTypeError, Review, ReviewAction, ReviewActionId, ReviewActionType, ReviewId,
    ReviewIssue, ReviewNote, ReviewNoteId, ReviewOutcome, ReviewStatus, ReviewerType,
};
pub use review_issue::{
    IssueCategory, IssueProgressSummary, IssueSeverity, IssueStatus, ParseIssueCategoryError,
    ParseIssueSeverityError, ParseIssueStatusError, ReviewIssue as ReviewIssueEntity,
    SeverityBreakdown, SeverityCount,
};
pub use status::{InternalStatus, ParseInternalStatusError};
pub use task::{Task, TaskCategory};
pub use task_context::{ArtifactSummary, TaskContext, TaskDependencySummary, TaskProposalSummary};
pub use task_metadata::{
    MergeFailureSource, MergeRecoveryEvent, MergeRecoveryEventKind, MergeRecoveryMetadata,
    MergeRecoveryReasonCode, MergeRecoverySource, MergeRecoveryState,
};
pub use task_qa::TaskQA;
pub use task_step::{StepProgressSummary, TaskStep, TaskStepStatus};
pub use team::{TeamMessageId, TeamMessageRecord, TeamSession, TeamSessionId, TeammateSnapshot};
pub use types::{
    ChatMessageId, IdeationSessionId, ProjectId, ReviewIssueId, SessionLinkId, TaskId,
    TaskProposalId, TaskQAId, TaskStepId,
};
pub use workflow::{
    ColumnBehavior, ConflictResolution, ExternalStatusMapping, ExternalSyncConfig,
    ParseSyncDirectionError, SyncDirection, SyncProvider, SyncSettings, WorkflowColumn,
    WorkflowDefaults, WorkflowId, WorkflowSchema,
};
