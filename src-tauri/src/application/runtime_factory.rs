use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime};

use crate::application::{AppState, InteractiveProcessRegistry, TaskSchedulerService, TaskTransitionService};
use crate::commands::ExecutionState;
use crate::domain::repositories::{
    ActivityEventRepository, AgentLaneSettingsRepository, AgentRunRepository,
    ChatAttachmentRepository, ChatConversationRepository, ChatMessageRepository,
    ExecutionSettingsRepository, IdeationSessionRepository, MemoryEventRepository,
    PlanBranchRepository, ProjectRepository, TaskDependencyRepository, TaskRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};

#[derive(Clone)]
pub(crate) struct RuntimeFactoryDeps {
    pub task_repo: Arc<dyn TaskRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub execution_settings_repo: Option<Arc<dyn ExecutionSettingsRepository>>,
    pub agent_lane_settings_repo: Option<Arc<dyn AgentLaneSettingsRepository>>,
    pub plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    pub interactive_process_registry: Option<Arc<InteractiveProcessRegistry>>,
}

pub(crate) fn build_transition_service_with_fallback<R: Runtime>(
    app_handle: &Option<AppHandle<R>>,
    execution_state: Arc<ExecutionState>,
    deps: &RuntimeFactoryDeps,
) -> TaskTransitionService<R> {
    if let Some(handle) = app_handle {
        if let Some(app_state) = handle.try_state::<AppState>() {
            return app_state.build_transition_service_for_runtime(
                execution_state,
                app_handle.clone(),
            );
        }
    }

    let mut service = TaskTransitionService::new(
        Arc::clone(&deps.task_repo),
        Arc::clone(&deps.task_dependency_repo),
        Arc::clone(&deps.project_repo),
        Arc::clone(&deps.chat_message_repo),
        Arc::clone(&deps.chat_attachment_repo),
        Arc::clone(&deps.conversation_repo),
        Arc::clone(&deps.agent_run_repo),
        Arc::clone(&deps.ideation_session_repo),
        Arc::clone(&deps.activity_event_repo),
        Arc::clone(&deps.message_queue),
        Arc::clone(&deps.running_agent_registry),
        execution_state,
        app_handle.clone(),
        Arc::clone(&deps.memory_event_repo),
    );
    if let Some(repo) = deps.execution_settings_repo.as_ref() {
        service = service.with_execution_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.agent_lane_settings_repo.as_ref() {
        service = service.with_agent_lane_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.plan_branch_repo.as_ref() {
        service = service.with_plan_branch_repo(Arc::clone(repo));
    }
    if let Some(ipr) = deps.interactive_process_registry.as_ref() {
        service = service.with_interactive_process_registry(Arc::clone(ipr));
    }
    service
}

pub(crate) fn build_task_scheduler_with_fallback<R: Runtime>(
    app_handle: &Option<AppHandle<R>>,
    execution_state: Arc<ExecutionState>,
    deps: &RuntimeFactoryDeps,
) -> TaskSchedulerService<R> {
    if let Some(handle) = app_handle {
        if let Some(app_state) = handle.try_state::<AppState>() {
            return app_state.build_task_scheduler_for_runtime(
                execution_state,
                app_handle.clone(),
            );
        }
    }

    let mut scheduler = TaskSchedulerService::new(
        execution_state,
        Arc::clone(&deps.project_repo),
        Arc::clone(&deps.task_repo),
        Arc::clone(&deps.task_dependency_repo),
        Arc::clone(&deps.chat_message_repo),
        Arc::clone(&deps.chat_attachment_repo),
        Arc::clone(&deps.conversation_repo),
        Arc::clone(&deps.agent_run_repo),
        Arc::clone(&deps.ideation_session_repo),
        Arc::clone(&deps.activity_event_repo),
        Arc::clone(&deps.message_queue),
        Arc::clone(&deps.running_agent_registry),
        Arc::clone(&deps.memory_event_repo),
        app_handle.clone(),
    );
    if let Some(repo) = deps.execution_settings_repo.as_ref() {
        scheduler = scheduler.with_execution_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.agent_lane_settings_repo.as_ref() {
        scheduler = scheduler.with_agent_lane_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.plan_branch_repo.as_ref() {
        scheduler = scheduler.with_plan_branch_repo(Arc::clone(repo));
    }
    if let Some(ipr) = deps.interactive_process_registry.as_ref() {
        scheduler = scheduler.with_interactive_process_registry(Arc::clone(ipr));
    }
    scheduler
}
