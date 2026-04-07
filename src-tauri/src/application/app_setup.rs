use std::sync::Arc;

use crate::AppState;
use crate::application;
use crate::application::runtime_wiring::{create_main_window, register_managed_state};
use crate::application::server_boot::start_server_boot;
use crate::application::setup_settings::initialize_settings_defaults;
use crate::application::startup_cleanup::run_startup_cleanup;
use crate::application::startup_pipeline::StartupPipelineDeps;
use crate::application::TeamStateTracker;
use crate::commands::{ActiveProjectState, ExecutionState};

pub(crate) fn run_app_setup(
    app: &mut tauri::App<tauri::Wry>,
    init_execution_state: Arc<ExecutionState>,
    startup_execution_state: Arc<ExecutionState>,
    startup_active_project_state: Arc<ActiveProjectState>,
    http_execution_state: Arc<ExecutionState>,
    http_team_tracker: TeamStateTracker,
    service_team_tracker: TeamStateTracker,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // Create the main window programmatically to set traffic light position
    create_main_window(app)?;

    // Create application state with production SQLite repositories
    let mut app_state =
        AppState::new_production(app_handle.clone()).expect("Failed to initialize AppState");

    // Construct WebhookPublisher ONCE — Arc-clone into both AppState instances.
    // Follows the question_state/permission_state dual-AppState sharing pattern.
    let webhook_publisher: Arc<dyn crate::domain::state_machine::services::WebhookPublisher> =
        Arc::new(crate::infrastructure::ConcreteWebhookPublisher::new(
            Arc::clone(&app_state.webhook_registration_repo),
            Arc::new(crate::infrastructure::HyperWebhookClient::new()),
        ));
    app_state.webhook_publisher = Some(Arc::clone(&webhook_publisher));
    initialize_settings_defaults(&app_state, init_execution_state);
    run_startup_cleanup(&app_state);
    start_server_boot(
        &app_state,
        app_handle,
        http_execution_state,
        http_team_tracker,
    );

    // Spawn startup job runner to resume tasks in agent-active states
    // Clone references needed for the async task
    let startup_task_repo = Arc::clone(&app_state.task_repo);
    let startup_project_repo = Arc::clone(&app_state.project_repo);
    let startup_task_dependency_repo = Arc::clone(&app_state.task_dependency_repo);
    let startup_plan_branch_repo = Arc::clone(&app_state.plan_branch_repo);
    let startup_step_repo = Arc::clone(&app_state.task_step_repo);
    let startup_chat_message_repo = Arc::clone(&app_state.chat_message_repo);
    let startup_chat_attachment_repo = Arc::clone(&app_state.chat_attachment_repo);
    let startup_artifact_repo = Arc::clone(&app_state.artifact_repo);
    let startup_conversation_repo = Arc::clone(&app_state.chat_conversation_repo);
    let startup_agent_run_repo = Arc::clone(&app_state.agent_run_repo);
    let startup_ideation_session_repo = Arc::clone(&app_state.ideation_session_repo);
    let startup_activity_event_repo = Arc::clone(&app_state.activity_event_repo);
    let startup_message_queue = Arc::clone(&app_state.message_queue);
    let startup_running_agent_registry = Arc::clone(&app_state.running_agent_registry);
    let startup_memory_event_repo = Arc::clone(&app_state.memory_event_repo);
    let startup_app_state_repo = Arc::clone(&app_state.app_state_repo);
    let startup_memory_archive_repo = Arc::clone(&app_state.memory_archive_repo);
    let startup_memory_entry_repo = Arc::clone(&app_state.memory_entry_repo);
    let startup_execution_settings_repo = Arc::clone(&app_state.execution_settings_repo);
    let startup_agent_lane_settings_repo = Arc::clone(&app_state.agent_lane_settings_repo);
    let startup_ideation_effort_settings_repo =
        Arc::clone(&app_state.ideation_effort_settings_repo);
    let startup_ideation_model_settings_repo = Arc::clone(&app_state.ideation_model_settings_repo);
    let startup_interactive_process_registry = Arc::clone(&app_state.interactive_process_registry);
    let startup_review_repo = Arc::clone(&app_state.review_repo);
    let startup_external_events_repo = Arc::clone(&app_state.external_events_repo);
    let startup_pr_poller_registry = Arc::clone(&app_state.pr_poller_registry);
    let startup_agent_client = Arc::clone(&app_state.agent_client);
    let startup_webhook_publisher = app_state.webhook_publisher.clone();
    let startup_session_merge_locks = Arc::clone(&app_state.session_merge_locks);
    // Clone app handle to enable event emission in startup tasks
    let startup_app_handle = app.handle().clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            application::startup_pipeline::run_startup_pipeline(StartupPipelineDeps {
                execution_state: Arc::clone(&startup_execution_state),
                active_project_state: Arc::clone(&startup_active_project_state),
                task_repo: startup_task_repo,
                project_repo: startup_project_repo,
                task_dependency_repo: startup_task_dependency_repo,
                plan_branch_repo: startup_plan_branch_repo,
                step_repo: startup_step_repo,
                chat_message_repo: startup_chat_message_repo,
                chat_attachment_repo: startup_chat_attachment_repo,
                artifact_repo: startup_artifact_repo,
                conversation_repo: startup_conversation_repo,
                agent_run_repo: startup_agent_run_repo,
                ideation_session_repo: startup_ideation_session_repo,
                activity_event_repo: startup_activity_event_repo,
                message_queue: startup_message_queue,
                running_agent_registry: startup_running_agent_registry,
                memory_event_repo: startup_memory_event_repo,
                app_state_repo: startup_app_state_repo,
                memory_archive_repo: startup_memory_archive_repo,
                memory_entry_repo: startup_memory_entry_repo,
                execution_settings_repo: startup_execution_settings_repo,
                agent_lane_settings_repo: startup_agent_lane_settings_repo,
                ideation_effort_settings_repo: startup_ideation_effort_settings_repo,
                ideation_model_settings_repo: startup_ideation_model_settings_repo,
                interactive_process_registry: startup_interactive_process_registry,
                review_repo: startup_review_repo,
                external_events_repo: startup_external_events_repo,
                pr_poller_registry: startup_pr_poller_registry,
                agent_client: startup_agent_client,
                webhook_publisher: startup_webhook_publisher,
                session_merge_locks: startup_session_merge_locks,
                app_handle: startup_app_handle,
            })
            .await
        {
            tracing::error!(error = %error, "Startup recovery pipeline failed");
        }
    });

    register_managed_state(app, app_state, service_team_tracker);

    Ok(())
}
