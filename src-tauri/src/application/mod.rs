// Application layer - dependency injection and service orchestration
// This layer bridges the domain and infrastructure layers

pub mod app_state;
pub mod apply_service;
pub mod dependency_service;
pub mod execution_chat_service;
pub mod ideation_service;
pub mod orchestrator_service;
pub mod permission_state;
pub mod priority_service;
pub mod qa_service;
pub mod review_service;
pub mod supervisor_service;
pub mod task_context_service;

// Re-export commonly used items
pub use app_state::AppState;
pub use apply_service::{
    ApplyProposalsOptions, ApplyProposalsResult, ApplyService, SelectionValidation, TargetColumn,
};
pub use dependency_service::{DependencyAnalysis, DependencyService, ValidationResult};
pub use ideation_service::{
    CreateProposalOptions, IdeationService, SessionStats, SessionWithData, UpdateProposalOptions,
};
pub use priority_service::PriorityService;
pub use qa_service::{QAPrepStatus, QAService, TaskQAState};
pub use review_service::ReviewService;
pub use supervisor_service::{SupervisorConfig, SupervisorService, TaskMonitorState};
pub use orchestrator_service::{
    ChatChunkPayload, ChatMessageCreatedPayload, ChatRunCompletedPayload, ChatToolCallPayload,
    ClaudeOrchestratorService, MockOrchestratorService, MockResponse, OrchestratorError,
    OrchestratorEvent, OrchestratorResult, OrchestratorService, ToolCall, ToolCallResult,
};
pub use execution_chat_service::{
    ClaudeExecutionChatService, ExecutionChatError, ExecutionChatService, ExecutionEvent,
    ExecutionResult, MockExecutionChatService, MockExecutionResponse, SpawnResult,
};
pub use permission_state::{PendingPermissionInfo, PermissionDecision, PermissionState};
pub use task_context_service::TaskContextService;
