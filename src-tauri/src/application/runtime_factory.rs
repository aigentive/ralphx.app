use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime};

use crate::application::chat_service::{ClaudeChatService, StreamingStateCache};
use crate::application::{
    AgentClientBundle, AppState, InteractiveProcessRegistry, TaskSchedulerService,
    TaskTransitionService,
};
use crate::commands::ExecutionState;
use crate::domain::repositories::{
    ActivityEventRepository, AgentLaneSettingsRepository, AgentRunRepository, ArtifactRepository,
    ChatAttachmentRepository, ChatConversationRepository, ChatMessageRepository,
    ExecutionSettingsRepository, IdeationEffortSettingsRepository, IdeationModelSettingsRepository,
    IdeationSessionRepository, MemoryEventRepository, PlanBranchRepository, ProjectRepository,
    ReviewRepository, TaskDependencyRepository, TaskProposalRepository, TaskRepository,
    TaskStepRepository,
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
    pub agent_clients: Option<AgentClientBundle>,
    pub execution_settings_repo: Option<Arc<dyn ExecutionSettingsRepository>>,
    pub agent_lane_settings_repo: Option<Arc<dyn AgentLaneSettingsRepository>>,
    pub plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    pub interactive_process_registry: Option<Arc<InteractiveProcessRegistry>>,
}

#[derive(Clone)]
pub(crate) struct ChatRuntimeFactoryDeps {
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub artifact_repo: Arc<dyn ArtifactRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub execution_settings_repo: Option<Arc<dyn ExecutionSettingsRepository>>,
    pub agent_lane_settings_repo: Option<Arc<dyn AgentLaneSettingsRepository>>,
    pub ideation_effort_settings_repo: Option<Arc<dyn IdeationEffortSettingsRepository>>,
    pub ideation_model_settings_repo: Option<Arc<dyn IdeationModelSettingsRepository>>,
    pub plan_branch_repo: Option<Arc<dyn PlanBranchRepository>>,
    pub task_proposal_repo: Option<Arc<dyn TaskProposalRepository>>,
    pub task_step_repo: Option<Arc<dyn TaskStepRepository>>,
    pub review_repo: Option<Arc<dyn ReviewRepository>>,
    pub interactive_process_registry: Option<Arc<InteractiveProcessRegistry>>,
    pub streaming_state_cache: Option<StreamingStateCache>,
}

impl ChatRuntimeFactoryDeps {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_core(
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
        artifact_repo: Arc<dyn ArtifactRepository>,
        conversation_repo: Arc<dyn ChatConversationRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        project_repo: Arc<dyn ProjectRepository>,
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        ideation_session_repo: Arc<dyn IdeationSessionRepository>,
        activity_event_repo: Arc<dyn ActivityEventRepository>,
        message_queue: Arc<MessageQueue>,
        running_agent_registry: Arc<dyn RunningAgentRegistry>,
        memory_event_repo: Arc<dyn MemoryEventRepository>,
    ) -> Self {
        Self {
            chat_message_repo,
            chat_attachment_repo,
            artifact_repo,
            conversation_repo,
            agent_run_repo,
            project_repo,
            task_repo,
            task_dependency_repo,
            ideation_session_repo,
            activity_event_repo,
            message_queue,
            running_agent_registry,
            memory_event_repo,
            execution_settings_repo: None,
            agent_lane_settings_repo: None,
            ideation_effort_settings_repo: None,
            ideation_model_settings_repo: None,
            plan_branch_repo: None,
            task_proposal_repo: None,
            task_step_repo: None,
            review_repo: None,
            interactive_process_registry: None,
            streaming_state_cache: None,
        }
    }

    pub(crate) fn with_execution_settings_repo(
        mut self,
        repo: Arc<dyn ExecutionSettingsRepository>,
    ) -> Self {
        self.execution_settings_repo = Some(repo);
        self
    }

    pub(crate) fn with_agent_lane_settings_repo(
        mut self,
        repo: Arc<dyn AgentLaneSettingsRepository>,
    ) -> Self {
        self.agent_lane_settings_repo = Some(repo);
        self
    }

    pub(crate) fn with_ideation_effort_settings_repo(
        mut self,
        repo: Arc<dyn IdeationEffortSettingsRepository>,
    ) -> Self {
        self.ideation_effort_settings_repo = Some(repo);
        self
    }

    pub(crate) fn with_ideation_model_settings_repo(
        mut self,
        repo: Arc<dyn IdeationModelSettingsRepository>,
    ) -> Self {
        self.ideation_model_settings_repo = Some(repo);
        self
    }

    pub(crate) fn with_plan_branch_repo(mut self, repo: Arc<dyn PlanBranchRepository>) -> Self {
        self.plan_branch_repo = Some(repo);
        self
    }

    pub(crate) fn with_task_proposal_repo(
        mut self,
        repo: Arc<dyn TaskProposalRepository>,
    ) -> Self {
        self.task_proposal_repo = Some(repo);
        self
    }

    pub(crate) fn with_task_step_repo(mut self, repo: Arc<dyn TaskStepRepository>) -> Self {
        self.task_step_repo = Some(repo);
        self
    }

    pub(crate) fn with_review_repo(mut self, repo: Arc<dyn ReviewRepository>) -> Self {
        self.review_repo = Some(repo);
        self
    }

    pub(crate) fn with_interactive_process_registry(
        mut self,
        registry: Arc<InteractiveProcessRegistry>,
    ) -> Self {
        self.interactive_process_registry = Some(registry);
        self
    }

    pub(crate) fn with_streaming_state_cache(mut self, cache: StreamingStateCache) -> Self {
        self.streaming_state_cache = Some(cache);
        self
    }
}

pub(crate) fn build_chat_service_from_deps<R: Runtime>(
    app_handle: Option<AppHandle<R>>,
    execution_state: Option<Arc<ExecutionState>>,
    deps: &ChatRuntimeFactoryDeps,
) -> ClaudeChatService<R> {
    let mut service = ClaudeChatService::new(
        Arc::clone(&deps.chat_message_repo),
        Arc::clone(&deps.chat_attachment_repo),
        Arc::clone(&deps.artifact_repo),
        Arc::clone(&deps.conversation_repo),
        Arc::clone(&deps.agent_run_repo),
        Arc::clone(&deps.project_repo),
        Arc::clone(&deps.task_repo),
        Arc::clone(&deps.task_dependency_repo),
        Arc::clone(&deps.ideation_session_repo),
        Arc::clone(&deps.activity_event_repo),
        Arc::clone(&deps.message_queue),
        Arc::clone(&deps.running_agent_registry),
        Arc::clone(&deps.memory_event_repo),
    );

    if let Some(state) = execution_state {
        service = service.with_execution_state(state);
    }
    if let Some(handle) = app_handle {
        service = service.with_app_handle(handle);
    }
    if let Some(repo) = deps.execution_settings_repo.as_ref() {
        service = service.with_execution_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.agent_lane_settings_repo.as_ref() {
        service = service.with_agent_lane_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.ideation_effort_settings_repo.as_ref() {
        service = service.with_ideation_effort_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.ideation_model_settings_repo.as_ref() {
        service = service.with_ideation_model_settings_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.plan_branch_repo.as_ref() {
        service = service.with_plan_branch_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.task_proposal_repo.as_ref() {
        service = service.with_task_proposal_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.task_step_repo.as_ref() {
        service = service.with_task_step_repo(Arc::clone(repo));
    }
    if let Some(repo) = deps.review_repo.as_ref() {
        service = service.with_review_repo(Arc::clone(repo));
    }
    if let Some(ipr) = deps.interactive_process_registry.as_ref() {
        service = service.with_interactive_process_registry(Arc::clone(ipr));
    }
    if let Some(cache) = deps.streaming_state_cache.as_ref() {
        service = service.with_streaming_state_cache(cache.clone());
    }

    service
}

pub(crate) fn build_chat_service_with_fallback<R: Runtime>(
    app_handle: &Option<AppHandle<R>>,
    execution_state: Option<Arc<ExecutionState>>,
    deps: &ChatRuntimeFactoryDeps,
) -> ClaudeChatService<R> {
    if let Some(handle) = app_handle {
        if let Some(app_state) = handle.try_state::<AppState>() {
            return app_state.build_chat_service_for_runtime(execution_state, app_handle.clone());
        }
    }

    build_chat_service_from_deps(app_handle.clone(), execution_state, deps)
}

pub(crate) fn build_transition_service_with_fallback<R: Runtime>(
    app_handle: &Option<AppHandle<R>>,
    execution_state: Arc<ExecutionState>,
    deps: &RuntimeFactoryDeps,
) -> TaskTransitionService<R> {
    if let Some(handle) = app_handle {
        if let Some(app_state) = handle.try_state::<AppState>() {
            return app_state
                .build_transition_service_for_runtime(execution_state, app_handle.clone());
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
    if let Some(agent_clients) = deps.agent_clients.as_ref() {
        service = service.with_agent_clients(agent_clients.clone());
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
            return app_state.build_task_scheduler_for_runtime(execution_state, app_handle.clone());
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
    if let Some(agent_clients) = deps.agent_clients.as_ref() {
        scheduler = scheduler.with_agent_clients(agent_clients.clone());
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
