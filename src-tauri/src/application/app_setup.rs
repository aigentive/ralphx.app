use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::AppState;
use crate::application::runtime_wiring::{create_main_window, register_managed_state};
use crate::application::server_boot::start_server_boot;
use crate::application::setup_settings::initialize_settings_defaults;
use crate::application::startup_cleanup::run_startup_cleanup;
use crate::application::startup_pipeline_launch::launch_startup_pipeline;
use crate::application::TeamStateTracker;
use crate::commands::{ActiveProjectState, ExecutionState};
use tauri::Manager;
use tracing::{info, warn};

const PLUGIN_DIR_ENV: &str = "RALPHX_PLUGIN_DIR";
const GENERATED_PLUGIN_DIR_ENV: &str = "RALPHX_GENERATED_PLUGIN_DIR";
const BUNDLED_PLUGIN_DIR_REL: &str = "plugins/app";
const BUNDLED_AGENTS_DIR_REL: &str = "agents";
const GENERATED_CLAUDE_PLUGIN_DIR_REL: &str = "generated/claude-plugin";

#[derive(Debug, Clone, PartialEq, Eq)]
struct BundledRuntimePaths {
    plugin_dir: PathBuf,
    generated_plugin_dir: PathBuf,
}

fn resolve_bundled_runtime_paths(
    resource_dir: &Path,
    app_data_dir: &Path,
) -> Option<BundledRuntimePaths> {
    let plugin_dir = resource_dir.join(BUNDLED_PLUGIN_DIR_REL);
    let agents_dir = resource_dir.join(BUNDLED_AGENTS_DIR_REL);

    if !plugin_dir.is_dir() || !agents_dir.is_dir() {
        return None;
    }

    Some(BundledRuntimePaths {
        plugin_dir,
        generated_plugin_dir: app_data_dir.join(GENERATED_CLAUDE_PLUGIN_DIR_REL),
    })
}

fn configure_bundled_runtime_env(app: &tauri::App<tauri::Wry>) {
    let resource_dir = match app.path().resource_dir() {
        Ok(path) => path,
        Err(error) => {
            warn!(%error, "Failed to resolve app resource directory for bundled runtime discovery");
            return;
        }
    };

    let app_data_dir = match app.path().app_data_dir() {
        Ok(path) => path,
        Err(error) => {
            warn!(%error, "Failed to resolve app data directory for bundled runtime discovery");
            return;
        }
    };

    let Some(paths) = resolve_bundled_runtime_paths(&resource_dir, &app_data_dir) else {
        return;
    };

    if std::env::var_os(PLUGIN_DIR_ENV).is_none() {
        info!(
            plugin_dir = %paths.plugin_dir.display(),
            "Configuring bundled plugin runtime directory"
        );
        std::env::set_var(PLUGIN_DIR_ENV, &paths.plugin_dir);
    }

    if std::env::var_os(GENERATED_PLUGIN_DIR_ENV).is_none() {
        info!(
            generated_plugin_dir = %paths.generated_plugin_dir.display(),
            "Configuring writable generated plugin runtime directory"
        );
        std::env::set_var(GENERATED_PLUGIN_DIR_ENV, &paths.generated_plugin_dir);
    }
}

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
    configure_bundled_runtime_env(app);

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

#[cfg(test)]
mod tests {
    use super::resolve_bundled_runtime_paths;
    use tempfile::tempdir;

    #[test]
    fn bundled_runtime_paths_require_plugin_and_agents_directories() {
        let temp = tempdir().expect("tempdir");
        let resource_dir = temp.path().join("Resources");
        let app_data_dir = temp.path().join("AppData");

        std::fs::create_dir_all(resource_dir.join("plugins/app")).expect("plugin dir");
        assert!(
            resolve_bundled_runtime_paths(&resource_dir, &app_data_dir).is_none(),
            "bundled runtime should not resolve without canonical agents"
        );

        std::fs::create_dir_all(resource_dir.join("agents")).expect("agents dir");
        let paths = resolve_bundled_runtime_paths(&resource_dir, &app_data_dir)
            .expect("bundled runtime paths");

        assert_eq!(paths.plugin_dir, resource_dir.join("plugins/app"));
        assert_eq!(
            paths.generated_plugin_dir,
            app_data_dir.join("generated/claude-plugin")
        );
    }
}
