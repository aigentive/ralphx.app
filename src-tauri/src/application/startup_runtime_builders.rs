use std::sync::Arc;

use crate::application::runtime_factory::{
    ChatRuntimeFactoryDeps, build_chat_service_with_fallback,
};
use crate::application::{
    AgentClientBundle, ChatResumptionRunner, ChatService, InteractiveProcessRegistry,
    ReconciliationRunner, TaskSchedulerService,
};
use crate::commands::ExecutionState;
use crate::domain::repositories::{
    ActivityEventRepository, AgentLaneSettingsRepository, AgentRunRepository,
    ChatAttachmentRepository, ChatConversationRepository, ChatMessageRepository,
    ExecutionSettingsRepository, IdeationSessionRepository, MemoryEventRepository,
    PlanBranchRepository, ProjectRepository, ReviewRepository, TaskDependencyRepository,
    TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::TaskScheduler;

pub(crate) struct StartupSchedulerDeps {
    pub execution_state: Arc<ExecutionState>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub agent_clients: AgentClientBundle,
    pub plan_branch_repo: Arc<dyn PlanBranchRepository>,
    pub interactive_process_registry: Arc<InteractiveProcessRegistry>,
    pub app_handle: tauri::AppHandle,
}

pub(crate) fn build_startup_task_scheduler(deps: StartupSchedulerDeps) -> Arc<dyn TaskScheduler> {
    let scheduler_concrete = Arc::new(
        TaskSchedulerService::<tauri::Wry>::new(
            Arc::clone(&deps.execution_state),
            deps.project_repo,
            deps.task_repo,
            deps.task_dependency_repo,
            deps.chat_message_repo,
            deps.chat_attachment_repo,
            deps.conversation_repo,
            deps.agent_run_repo,
            deps.ideation_session_repo,
            deps.activity_event_repo,
            deps.message_queue,
            deps.running_agent_registry,
            deps.memory_event_repo,
            Some(deps.app_handle),
        )
        .with_agent_clients(deps.agent_clients)
        .with_plan_branch_repo(deps.plan_branch_repo)
        .with_interactive_process_registry(deps.interactive_process_registry),
    );
    scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn TaskScheduler>);
    scheduler_concrete
}

pub(crate) fn build_startup_recovery_chat_service(
    app_handle: tauri::AppHandle,
    execution_state: Arc<ExecutionState>,
    deps: ChatRuntimeFactoryDeps,
) -> Arc<dyn ChatService> {
    Arc::new(build_chat_service_with_fallback(
        &Some(app_handle),
        Some(execution_state),
        &deps,
    ))
}

pub(crate) struct StartupChatResumptionDeps {
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub execution_state: Arc<ExecutionState>,
    pub chat_runtime_deps: ChatRuntimeFactoryDeps,
    pub execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    pub agent_lane_settings_repo: Arc<dyn AgentLaneSettingsRepository>,
    pub plan_branch_repo: Arc<dyn PlanBranchRepository>,
    pub interactive_process_registry: Arc<InteractiveProcessRegistry>,
    pub app_handle: tauri::AppHandle,
}

pub(crate) fn build_startup_chat_resumption_runner(
    deps: StartupChatResumptionDeps,
) -> ChatResumptionRunner {
    ChatResumptionRunner::<tauri::Wry>::new(
        deps.agent_run_repo,
        deps.task_repo,
        deps.execution_state,
        deps.chat_runtime_deps,
    )
    .with_app_handle(deps.app_handle)
    .with_execution_settings_repo(deps.execution_settings_repo)
    .with_agent_lane_settings_repo(deps.agent_lane_settings_repo)
    .with_plan_branch_repo(deps.plan_branch_repo)
    .with_interactive_process_registry(deps.interactive_process_registry)
}

pub(crate) struct StartupReconciliationDeps {
    pub task_repo: Arc<dyn TaskRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub transition_service: Arc<crate::application::TaskTransitionService>,
    pub execution_state: Arc<ExecutionState>,
    pub execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    pub plan_branch_repo: Arc<dyn PlanBranchRepository>,
    pub interactive_process_registry: Arc<InteractiveProcessRegistry>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub app_handle: tauri::AppHandle,
}

pub(crate) fn build_startup_reconciliation_runner(
    deps: StartupReconciliationDeps,
) -> ReconciliationRunner {
    ReconciliationRunner::new(
        deps.task_repo,
        deps.task_dependency_repo,
        deps.project_repo,
        deps.conversation_repo,
        deps.chat_message_repo,
        deps.chat_attachment_repo,
        deps.ideation_session_repo,
        deps.activity_event_repo,
        deps.message_queue,
        deps.running_agent_registry,
        deps.memory_event_repo,
        deps.agent_run_repo,
        deps.transition_service,
        deps.execution_state,
        Some(deps.app_handle),
    )
    .with_execution_settings_repo(deps.execution_settings_repo)
    .with_plan_branch_repo(deps.plan_branch_repo)
    .with_interactive_process_registry(deps.interactive_process_registry)
    .with_review_repo(deps.review_repo)
}
