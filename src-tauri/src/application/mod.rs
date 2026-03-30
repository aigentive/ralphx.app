// Application layer - dependency injection and service orchestration
// This layer bridges the domain and infrastructure layers

pub mod app_state;
pub mod apply_service;
pub mod chat_attachment_service;
pub mod chat_resumption;
pub mod chat_service;
pub mod dependency_service;
pub mod event_cleanup_service;
pub mod execution_settings_bootstrap;
pub mod ideation_effort_bootstrap;
pub mod diff_service;
pub mod git_service;
pub mod ideation_service;
pub mod interactive_process_registry;
pub mod memory_archive_service;
pub mod memory_orchestration;
pub mod pending_session_drain;
pub mod permission_state;
pub mod plan_ranking;
pub mod priority_service;
pub mod pr_startup_recovery;
pub mod prune_engine;
pub mod qa_service;
pub mod question_state;
pub mod reconciliation;
pub mod recovery_queue;
pub mod resume_validator;
pub mod services;
pub mod review_issue_service;
pub mod review_service;
pub mod session_export_service;
pub mod session_reopen_service;
pub mod startup_jobs;
pub mod supervisor_service;
pub mod task_cleanup_service;
pub mod task_context_service;
pub mod task_scheduler_service;
pub mod task_transition_service;
pub mod throttled_emitter;
pub mod team_events;
pub mod team_service;
pub mod team_state_tracker;
pub mod team_stream_processor;
pub mod webhook_service;

// Re-export commonly used items
pub use app_state::AppState;
pub use apply_service::{
    ApplyProposalsOptions, ApplyProposalsResult, ApplyService, SelectionValidation, TargetColumn,
};
pub use chat_attachment_service::ChatAttachmentService;
pub use chat_resumption::ChatResumptionRunner;
pub use dependency_service::{DependencyAnalysis, DependencyService, ValidationResult};
pub use event_cleanup_service::EventCleanupService;
pub use execution_settings_bootstrap::{
    load_or_seed_execution_settings_defaults, ExecutionSettingsBootstrapResult,
};
pub use diff_service::{ConflictDiff, DiffService, FileChange, FileChangeStatus, FileDiff};
pub use git_service::{
    checkout_free::CheckoutFreeMergeResult, CommitInfo, DiffStats, GitService, MergeAttemptResult,
    MergeResult, RebaseResult,
};
pub use interactive_process_registry::{InteractiveProcessKey, InteractiveProcessRegistry};
pub use ideation_service::{
    CreateProposalOptions, IdeationService, SessionStats, SessionWithData, UpdateProposalOptions,
    UpdateSource,
};
pub use memory_archive_service::MemoryArchiveService;
pub use permission_state::{PendingPermissionInfo, PermissionDecision, PermissionState};
pub use plan_ranking::{
    compute_activity_score, compute_final_score, compute_final_score_with_breakdown,
    compute_interaction_score, compute_recency_score, ScoreBreakdown,
};
pub use priority_service::PriorityService;
pub use recovery_queue::{ProcessSummary, RecoveryItem, RecoveryPriority, RecoveryQueue};
pub use prune_engine::PruneEngine;
pub use qa_service::{QAPrepStatus, QAService, TaskQAState};
pub use question_state::{PendingQuestionInfo, QuestionAnswer, QuestionOption, QuestionState};
pub use reconciliation::ReconciliationRunner;
pub use services::PrPollerRegistry;
pub use resume_validator::{ResumeValidationResult, ResumeValidator};
pub use review_issue_service::{CreateIssueInput, ReviewIssueService};
pub use review_service::ReviewService;
pub use session_export_service::{
    DependencyData, ImportedSession, PlanVersionData, PriorityFactorsData, ProposalData,
    SessionData, SessionExport, SessionExportService, SourceInstance,
};
pub use session_reopen_service::SessionReopenService;
pub use startup_jobs::StartupJobRunner;
pub use supervisor_service::{SupervisorConfig, SupervisorService, TaskMonitorState};
pub use task_cleanup_service::{
    CleanupReport, StopMode, TaskCleanupService, TaskGroup, TaskStopper,
};
pub use task_context_service::TaskContextService;
pub use task_scheduler_service::{ReadyWatchdog, TaskSchedulerService};
pub use task_transition_service::TaskTransitionService;
pub use throttled_emitter::ThrottledEmitter;
pub use team_service::TeamService;
pub use team_state_tracker::TeamStateTracker;
pub use webhook_service::WebhookService;

#[cfg(test)]
mod app_state_shared_state_tests;
#[cfg(test)]
mod recovery_queue_tests;
#[cfg(test)]
mod webhook_service_tests;
#[cfg(test)]
mod prune_engine_tests;
#[cfg(test)]
mod session_export_service_tests;
#[cfg(test)]
mod throttled_emitter_tests;

// Unified chat service (handles all chat contexts: ideation, task, project, task_execution)
pub use chat_service::{
    AgentChunkPayload, AgentErrorPayload, AgentMessageCreatedPayload, AgentMessageQueuedPayload,
    AgentQueueSentPayload, AgentRunCompletedPayload, AgentRunStartedPayload, AgentToolCallPayload,
    ChatConversationWithMessages, ChatService, ChatServiceError, ClaudeChatService,
    MockChatResponse, MockChatService, SendResult, TeamCostUpdatePayload, TeamCreatedPayload,
    TeamDisbandedPayload, TeamMessagePayload, TeamTeammateIdlePayload, TeamTeammateShutdownPayload,
    TeamTeammateSpawnedPayload, AGENT_MESSAGE_QUEUED,
};
