use std::sync::Arc;
use std::time::Duration;

use tracing::info;

use crate::application::runtime_factory::{ChatRuntimeFactoryDeps, RuntimeFactoryDeps};
use crate::application::startup_runtime_builders::{
    build_startup_chat_resumption_runner, build_startup_reconciliation_runner,
    build_startup_recovery_chat_service, build_startup_task_scheduler, StartupChatResumptionDeps,
    StartupReconciliationDeps, StartupSchedulerDeps,
};
use crate::application::startup_transition_factory::StartupTransitionFactory;
use crate::application::{
    startup_background, startup_jobs, AgentClientBundle, InteractiveProcessRegistry,
    StartupJobRunner,
};
use crate::commands::{ActiveProjectState, ExecutionState};
use crate::domain::repositories::{
    ActivityEventRepository, AgentLaneSettingsRepository, AgentRunRepository, AppStateRepository,
    ArtifactRepository, ChatAttachmentRepository, ChatConversationRepository,
    ChatMessageRepository, ExecutionSettingsRepository, ExternalEventsRepository,
    IdeationEffortSettingsRepository, IdeationModelSettingsRepository, IdeationSessionRepository,
    MemoryArchiveRepository, MemoryEntryRepository, MemoryEventRepository,
    PlanBranchRepository, ProjectRepository, ReviewRepository, TaskDependencyRepository,
    TaskRepository, ExecutionPlanRepository,
    TaskStepRepository,
};
use crate::domain::services::{MessageQueue, RunningAgentRegistry};
use crate::domain::state_machine::services::WebhookPublisher;
use crate::error::AppResult;

pub(crate) struct StartupPipelineDeps {
    pub execution_state: Arc<ExecutionState>,
    pub active_project_state: Arc<ActiveProjectState>,
    pub task_repo: Arc<dyn TaskRepository>,
    pub project_repo: Arc<dyn ProjectRepository>,
    pub task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    pub execution_plan_repo: Arc<dyn ExecutionPlanRepository>,
    pub plan_branch_repo: Arc<dyn PlanBranchRepository>,
    pub step_repo: Arc<dyn TaskStepRepository>,
    pub chat_message_repo: Arc<dyn ChatMessageRepository>,
    pub chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    pub artifact_repo: Arc<dyn ArtifactRepository>,
    pub conversation_repo: Arc<dyn ChatConversationRepository>,
    pub agent_run_repo: Arc<dyn AgentRunRepository>,
    pub ideation_session_repo: Arc<dyn IdeationSessionRepository>,
    pub activity_event_repo: Arc<dyn ActivityEventRepository>,
    pub message_queue: Arc<MessageQueue>,
    pub running_agent_registry: Arc<dyn RunningAgentRegistry>,
    pub memory_event_repo: Arc<dyn MemoryEventRepository>,
    pub app_state_repo: Arc<dyn AppStateRepository>,
    pub memory_archive_repo: Arc<dyn MemoryArchiveRepository>,
    pub memory_entry_repo: Arc<dyn MemoryEntryRepository>,
    pub execution_settings_repo: Arc<dyn ExecutionSettingsRepository>,
    pub agent_lane_settings_repo: Arc<dyn AgentLaneSettingsRepository>,
    pub ideation_effort_settings_repo: Arc<dyn IdeationEffortSettingsRepository>,
    pub ideation_model_settings_repo: Arc<dyn IdeationModelSettingsRepository>,
    pub interactive_process_registry: Arc<InteractiveProcessRegistry>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub external_events_repo: Arc<dyn ExternalEventsRepository>,
    pub pr_poller_registry: Arc<crate::application::PrPollerRegistry>,
    pub agent_clients: AgentClientBundle,
    pub webhook_publisher: Option<Arc<dyn WebhookPublisher>>,
    pub session_merge_locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Mutex<()>>>>,
    pub app_handle: tauri::AppHandle,
}

pub(crate) async fn run_startup_pipeline(deps: StartupPipelineDeps) -> AppResult<()> {
    tokio::time::sleep(Duration::from_millis(500)).await;

    if startup_jobs::is_startup_recovery_disabled() {
        info!(
            env_var = startup_jobs::RALPHX_DISABLE_STARTUP_RECOVERY_ENV,
            "Startup recovery disabled via environment; skipping startup recovery pipeline"
        );
        return Ok(());
    }

    info!("Starting startup job runner...");

    let StartupPipelineDeps {
        execution_state,
        active_project_state,
        task_repo,
        project_repo,
        task_dependency_repo,
        execution_plan_repo,
        plan_branch_repo,
        step_repo,
        chat_message_repo,
        chat_attachment_repo,
        artifact_repo,
        conversation_repo,
        agent_run_repo,
        ideation_session_repo,
        activity_event_repo,
        message_queue,
        running_agent_registry,
        memory_event_repo,
        app_state_repo,
        memory_archive_repo,
        memory_entry_repo,
        execution_settings_repo,
        agent_lane_settings_repo,
        ideation_effort_settings_repo,
        ideation_model_settings_repo,
        interactive_process_registry,
        review_repo,
        external_events_repo,
        pr_poller_registry,
        agent_clients,
        webhook_publisher,
        session_merge_locks,
        app_handle,
    } = deps;

    let task_scheduler = build_startup_task_scheduler(StartupSchedulerDeps {
        execution_state: Arc::clone(&execution_state),
        project_repo: Arc::clone(&project_repo),
        task_repo: Arc::clone(&task_repo),
        task_dependency_repo: Arc::clone(&task_dependency_repo),
        execution_plan_repo: Arc::clone(&execution_plan_repo),
        chat_message_repo: Arc::clone(&chat_message_repo),
        chat_attachment_repo: Arc::clone(&chat_attachment_repo),
        conversation_repo: Arc::clone(&conversation_repo),
        agent_run_repo: Arc::clone(&agent_run_repo),
        ideation_session_repo: Arc::clone(&ideation_session_repo),
        activity_event_repo: Arc::clone(&activity_event_repo),
        message_queue: Arc::clone(&message_queue),
        running_agent_registry: Arc::clone(&running_agent_registry),
        memory_event_repo: Arc::clone(&memory_event_repo),
        agent_clients: agent_clients.clone(),
        plan_branch_repo: Arc::clone(&plan_branch_repo),
        interactive_process_registry: Arc::clone(&interactive_process_registry),
        app_handle: app_handle.clone(),
    });

    let startup_transition_factory = StartupTransitionFactory {
        execution_state: Arc::clone(&execution_state),
        execution_settings_repo: Arc::clone(&execution_settings_repo),
        agent_lane_settings_repo: Arc::clone(&agent_lane_settings_repo),
        plan_branch_repo: Arc::clone(&plan_branch_repo),
        interactive_process_registry: Arc::clone(&interactive_process_registry),
        agent_clients: agent_clients.clone(),
        task_scheduler: Arc::clone(&task_scheduler),
        step_repo: Arc::clone(&step_repo),
        external_events_repo: Arc::clone(&external_events_repo),
        webhook_publisher: webhook_publisher.clone(),
        session_merge_locks: Arc::clone(&session_merge_locks),
    };

    let core_runtime_deps = RuntimeFactoryDeps::from_core(
        Arc::clone(&task_repo),
        Arc::clone(&task_dependency_repo),
        Arc::clone(&project_repo),
        Arc::clone(&chat_message_repo),
        Arc::clone(&chat_attachment_repo),
        Arc::clone(&conversation_repo),
        Arc::clone(&agent_run_repo),
        Arc::clone(&ideation_session_repo),
        Arc::clone(&activity_event_repo),
        Arc::clone(&message_queue),
        Arc::clone(&running_agent_registry),
        Arc::clone(&memory_event_repo),
    );

    let transition_service = Arc::new(startup_transition_factory.build(
        core_runtime_deps.clone(),
        app_handle.clone(),
    ));

    tracing::info!("Running PR startup recovery...");
    crate::application::pr_startup_recovery::recover_pr_pollers(
        Arc::clone(&task_repo),
        Arc::clone(&plan_branch_repo),
        Arc::clone(&pr_poller_registry),
        Arc::clone(&project_repo),
        Arc::clone(&transition_service),
    )
    .await;

    let recovery_chat_service_deps = ChatRuntimeFactoryDeps::from_core(
        Arc::clone(&chat_message_repo),
        Arc::clone(&chat_attachment_repo),
        Arc::clone(&artifact_repo),
        Arc::clone(&conversation_repo),
        Arc::clone(&agent_run_repo),
        Arc::clone(&project_repo),
        Arc::clone(&task_repo),
        Arc::clone(&task_dependency_repo),
        Arc::clone(&ideation_session_repo),
        Arc::clone(&activity_event_repo),
        Arc::clone(&message_queue),
        Arc::clone(&running_agent_registry),
        Arc::clone(&memory_event_repo),
    )
    .with_runtime_support(
        Some(Arc::clone(&execution_settings_repo)),
        Some(Arc::clone(&agent_lane_settings_repo)),
        None,
        Some(Arc::clone(&interactive_process_registry)),
    )
    .with_ideation_runtime_support(
        Some(Arc::clone(&ideation_effort_settings_repo)),
        Some(Arc::clone(&ideation_model_settings_repo)),
    );
    let recovery_chat_service = build_startup_recovery_chat_service(
        app_handle.clone(),
        Arc::clone(&execution_state),
        recovery_chat_service_deps.clone(),
    );

    let runner = StartupJobRunner::new(
        Arc::clone(&task_repo),
        Arc::clone(&task_dependency_repo),
        Arc::clone(&project_repo),
        Arc::clone(&conversation_repo),
        Arc::clone(&chat_message_repo),
        Arc::clone(&chat_attachment_repo),
        Arc::clone(&ideation_session_repo),
        Arc::clone(&activity_event_repo),
        Arc::clone(&message_queue),
        Arc::clone(&running_agent_registry),
        Arc::clone(&memory_event_repo),
        Arc::clone(&agent_run_repo),
        Arc::clone(&transition_service),
        Arc::clone(&execution_state),
        Arc::clone(&active_project_state),
        Arc::clone(&app_state_repo),
        Arc::clone(&execution_settings_repo),
        Some(Arc::clone(&plan_branch_repo)),
    )
    .with_task_scheduler(Arc::clone(&task_scheduler))
    .with_app_handle(app_handle.clone())
    .with_review_repo(Arc::clone(&review_repo))
    .with_chat_service(recovery_chat_service);

    let startup_ideation_recovery_claims = runner.run().await;

    startup_background::recover_memory_archive_jobs_on_startup(
        Arc::clone(&memory_archive_repo),
        Arc::clone(&memory_entry_repo),
        Arc::clone(&project_repo),
    )
    .await;

    info!("Starting chat resumption runner...");
    let chat_resumption = build_startup_chat_resumption_runner(StartupChatResumptionDeps {
        agent_run_repo: Arc::clone(&agent_run_repo),
        task_repo: Arc::clone(&task_repo),
        execution_state: Arc::clone(&execution_state),
        chat_runtime_deps: recovery_chat_service_deps.clone(),
        execution_settings_repo: Arc::clone(&execution_settings_repo),
        agent_lane_settings_repo: Arc::clone(&agent_lane_settings_repo),
        plan_branch_repo: Arc::clone(&plan_branch_repo),
        interactive_process_registry: Arc::clone(&interactive_process_registry),
        app_handle: app_handle.clone(),
    });
    chat_resumption.run().await;

    let reconcile_transition_service = Arc::new(startup_transition_factory.build(
        core_runtime_deps,
        app_handle.clone(),
    ));

    let reconcile_runner = build_startup_reconciliation_runner(StartupReconciliationDeps {
        task_repo: Arc::clone(&task_repo),
        task_dependency_repo: Arc::clone(&task_dependency_repo),
        project_repo: Arc::clone(&project_repo),
        conversation_repo: Arc::clone(&conversation_repo),
        chat_message_repo: Arc::clone(&chat_message_repo),
        chat_attachment_repo: Arc::clone(&chat_attachment_repo),
        ideation_session_repo: Arc::clone(&ideation_session_repo),
        activity_event_repo: Arc::clone(&activity_event_repo),
        message_queue: Arc::clone(&message_queue),
        running_agent_registry: Arc::clone(&running_agent_registry),
        memory_event_repo: Arc::clone(&memory_event_repo),
        agent_run_repo: Arc::clone(&agent_run_repo),
        transition_service: reconcile_transition_service,
        execution_state: Arc::clone(&execution_state),
        execution_settings_repo: Arc::clone(&execution_settings_repo),
        plan_branch_repo: Arc::clone(&plan_branch_repo),
        interactive_process_registry: Arc::clone(&interactive_process_registry),
        review_repo: Arc::clone(&review_repo),
        app_handle: app_handle.clone(),
    });

    reconcile_runner.recover_timeout_failures().await;
    reconcile_runner.reconcile_stuck_tasks().await;

    tauri::async_runtime::spawn(async move {
        let interval = Duration::from_secs(30);
        loop {
            tokio::time::sleep(interval).await;
            reconcile_runner.reconcile_stuck_tasks().await;
        }
    });

    startup_background::spawn_watchdog(
        Arc::clone(&task_scheduler),
        Arc::clone(&task_repo),
        Arc::clone(&project_repo),
    );

    {
        use crate::application::harness_runtime_registry::default_verification_reconciliation_config;
        use crate::application::reconciliation::recovery_queue::{
            create_recovery_queue, RecoveryQueueConfig,
        };
        use crate::application::reconciliation::verification_reconciliation::{
            VerificationReconciliationService,
        };

        let recovery_config = RecoveryQueueConfig::default();
        let recovery_queue_chat_deps = recovery_chat_service_deps.clone();
        let recovery_chat_service = build_startup_recovery_chat_service(
            app_handle.clone(),
            Arc::clone(&execution_state),
            recovery_queue_chat_deps,
        );
        let (recovery_queue, recovery_processor) = create_recovery_queue(
            Arc::clone(&running_agent_registry),
            Arc::clone(&interactive_process_registry),
            Arc::clone(&ideation_session_repo),
            recovery_chat_service,
            Some(app_handle.clone()),
            recovery_config,
        );
        let recovery_queue = Arc::new(recovery_queue);
        startup_background::spawn_recovery_queue_processor(recovery_processor);

        let verification_config = default_verification_reconciliation_config();
        let svc = Arc::new(
            VerificationReconciliationService::new(
                Arc::clone(&ideation_session_repo),
                verification_config,
            )
            .with_app_handle(app_handle.clone())
            .with_recovery_queue(Arc::clone(&recovery_queue))
            .with_running_agent_registry(Arc::clone(&running_agent_registry)),
        );
        startup_background::startup_scan_verification_reconciliation(
            svc,
            &startup_ideation_recovery_claims,
        )
        .await;
    }

    startup_background::spawn_cleanup_loops(
        Arc::clone(&external_events_repo),
        Arc::clone(&memory_archive_repo),
        Arc::clone(&memory_entry_repo),
        Arc::clone(&project_repo),
    );

    startup_background::maybe_start_external_mcp(app_handle, |port, timeout| {
        Box::pin(crate::wait_for_backend_ready(port, timeout))
    })
    .await;

    Ok(())
}
