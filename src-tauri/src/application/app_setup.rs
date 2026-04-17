use std::sync::Arc;

use crate::AppState;
use crate::application::runtime_wiring::{create_main_window, register_managed_state};
use crate::application::server_boot::start_server_boot;
use crate::application::setup_settings::initialize_settings_defaults;
use crate::application::startup_cleanup::run_startup_cleanup;
use crate::application::startup_pipeline_launch::launch_startup_pipeline;
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
    launch_startup_pipeline(
        app,
        &app_state,
        startup_execution_state,
        startup_active_project_state,
    );

    register_managed_state(app, app_state, service_team_tracker);

    Ok(())
}
