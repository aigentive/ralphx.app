// RalphX - Autonomous AI-driven development system
// Tauri 2.0 backend with clean architecture

// Allow clippy lints for patterns used throughout the codebase
#![allow(clippy::derivable_impls)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::single_match)]
#![allow(clippy::type_complexity)]
#![allow(clippy::identity_op)]
#![allow(clippy::comparison_to_empty)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::unnecessary_literal_unwrap)]
#![allow(clippy::bool_comparison)]
#![allow(clippy::while_let_loop)]
#![allow(clippy::const_is_empty)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::useless_vec)]
#![allow(clippy::let_and_return)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::unnecessary_map_or)]

// Core modules
pub mod application;
pub mod commands;
pub mod domain;
pub mod error;
pub mod http_server;
pub mod infrastructure;
pub mod testing;
pub mod utils;

// Re-export common types
pub use application::AppState;
pub use error::{AppError, AppResult};

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use crate::utils::redacting_writer::RedactingMakeWriter;

use application::ideation_effort_bootstrap::seed_ideation_effort_defaults;
use application::ideation_model_bootstrap::seed_ideation_model_settings;
use application::runtime_factory::{
    ChatRuntimeFactoryDeps, RuntimeFactoryDeps, build_chat_service_with_fallback,
    build_transition_service_with_fallback,
};
use application::runtime_wiring::{
    build_http_app_state, create_main_window, register_managed_state,
};
use application::{
    load_or_seed_agent_lane_settings_defaults, load_or_seed_execution_settings_defaults,
    ChatResumptionRunner, ReconciliationRunner,
    StartupJobRunner, TaskSchedulerService,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Wait for the local HTTP backend at `port` to respond with HTTP 200 on `/health`.
/// Retries every 200ms until `timeout` elapses (2s per-request timeout).
async fn wait_for_backend_ready(port: u16, timeout: Duration) -> Result<(), String> {
    wait_for_backend_ready_with_probe(port, timeout, probe_backend_health).await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendReadyProbeResult {
    Ready,
    HttpStatus(u16),
    Unreachable,
}

async fn wait_for_backend_ready_with_probe<F, Fut>(
    port: u16,
    timeout: Duration,
    mut probe: F,
) -> Result<(), String>
where
    F: FnMut(u16) -> Fut,
    Fut: std::future::Future<Output = BackendReadyProbeResult>,
{
    let start = Instant::now();
    let mut logged_non_200 = false;
    let mut logged_conn_refused = false;
    loop {
        if start.elapsed() > timeout {
            return Err(format!("Backend :{port} not ready after {timeout:?}"));
        }

        match probe(port).await {
            BackendReadyProbeResult::Ready => return Ok(()),
            BackendReadyProbeResult::HttpStatus(status) => {
                logged_conn_refused = false;
                if !logged_non_200 {
                    tracing::debug!(
                        "Backend :{port} /health returned status {status} (expected 200), retrying"
                    );
                    logged_non_200 = true;
                }
            }
            BackendReadyProbeResult::Unreachable => {
                if !logged_conn_refused {
                    tracing::debug!("Backend :{port} not yet accepting connections, retrying");
                    logged_conn_refused = true;
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn probe_backend_health(port: u16) -> BackendReadyProbeResult {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let addr = format!("127.0.0.1:{port}");
    let conn = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(&addr),
    )
    .await;
    match conn {
        Ok(Ok(mut stream)) => {
            let req = "GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
            if tokio::time::timeout(Duration::from_secs(2), stream.write_all(req.as_bytes()))
                .await
                .is_ok()
            {
                let mut buf = [0u8; 1024];
                if let Ok(Ok(n)) =
                    tokio::time::timeout(Duration::from_secs(2), stream.read(&mut buf)).await
                {
                    let response = std::str::from_utf8(&buf[..n]).unwrap_or("");
                    let status = response
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(0);
                    if status == 200 {
                        return BackendReadyProbeResult::Ready;
                    }
                    return BackendReadyProbeResult::HttpStatus(status);
                }
            }

            BackendReadyProbeResult::Unreachable
        }
        _ => BackendReadyProbeResult::Unreachable,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Startup hardening: increase per-thread minimum stack when not explicitly set.
    // Tokio runtime workers execute deeply nested async recovery paths at startup,
    // and the platform default stack can be too small for worst-case futures.
    if std::env::var_os("RUST_MIN_STACK").is_none() {
        std::env::set_var("RUST_MIN_STACK", "8388608");
    }

    // Initialize layered tracing subscriber: console + optional per-launch log file.
    // Respects RUST_LOG env var; defaults to ralphx=info plus warn for everything else.
    // File logging controlled by RALPHX_FILE_LOGGING env / ralphx.yaml `file_logging` (default: true).
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("ralphx=info,warn"));

    let file_logging_enabled =
        infrastructure::agents::claude::resolve_file_logging_early();

    let (_log_guard, file_layer) = if file_logging_enabled {
        // Determine log directory: dev → {repo_root}/.artifacts/logs/, prod → ~/Library/Application Support/com.ralphx.app/logs/
        let log_dir = if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/logs")
        } else {
            let home = std::env::var("HOME").expect("HOME environment variable not set");
            PathBuf::from(home).join("Library/Application Support/com.ralphx.app/logs")
        };
        std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_filename = format!("ralphx_{timestamp}.log");
        let log_file = std::fs::File::create(log_dir.join(&log_filename))
            .expect("Failed to create log file");

        let (non_blocking_writer, guard) = tracing_appender::non_blocking(log_file);
        let layer = fmt::layer()
            .with_writer(RedactingMakeWriter::new(non_blocking_writer))
            .with_ansi(false);

        // Log path is printed after subscriber init below
        eprintln!("File logging: {}", log_dir.join(&log_filename).display());

        (Some(guard), Some(layer))
    } else {
        (None, None)
    };

    let console_layer = fmt::layer().with_writer(RedactingMakeWriter::new(std::io::stdout));

    Registry::default()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    // Load local runtime overrides from project-root/.env and src-tauri/.env when present.
    // These can drive claude settings profile env mappings (RALPHX_* -> settings.env.*).
    let dotenv_paths = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.env"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env"),
    ];
    for dotenv_path in dotenv_paths {
        match dotenvy::from_path(&dotenv_path) {
            Ok(_) => info!(path = %dotenv_path.display(), "Loaded local environment overrides"),
            Err(dotenvy::Error::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => warn!(
                path = %dotenv_path.display(),
                error = %err,
                "Failed to load local environment overrides"
            ),
        }
    }

    // Create execution state for global execution control
    let execution_state = Arc::new(commands::ExecutionState::new());
    // Create active project state for per-project execution scoping (Phase 82)
    let active_project_state = Arc::new(commands::ActiveProjectState::new());
    // Create team state tracker for agent teams (must be managed early for HTTP server)
    let team_tracker = application::TeamStateTracker::new();

    // Clone for usage inside setup closure before closure borrows them
    let init_execution_state = Arc::clone(&execution_state);
    let startup_execution_state = Arc::clone(&execution_state);
    let startup_active_project_state = Arc::clone(&active_project_state);
    let http_execution_state = Arc::clone(&execution_state);
    let http_team_tracker = team_tracker.clone();
    let service_team_tracker = team_tracker.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all())
                .build()
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
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

            // Load execution settings from database and apply to ExecutionState
            // This must happen before HTTP server starts to ensure consistent configuration
            let init_settings_repo = Arc::clone(&app_state.execution_settings_repo);
            let init_global_settings_repo = Arc::clone(&app_state.global_execution_settings_repo);
            let init_agent_lane_settings_repo = Arc::clone(&app_state.agent_lane_settings_repo);
            let execution_defaults =
                infrastructure::agents::claude::execution_defaults_config().clone();
            let agent_harness_defaults =
                infrastructure::agents::claude::agent_harness_defaults_config().clone();
            tauri::async_runtime::block_on(async move {
                match load_or_seed_execution_settings_defaults(
                    init_settings_repo,
                    init_global_settings_repo,
                    &execution_defaults.project,
                    &execution_defaults.global,
                )
                .await
                {
                    Ok(result) => {
                        init_execution_state
                            .set_max_concurrent(result.project_defaults.max_concurrent_tasks);
                        init_execution_state
                            .set_global_max_concurrent(result.global_defaults.global_max_concurrent);
                        init_execution_state
                            .set_global_ideation_max(result.global_defaults.global_ideation_max);
                        init_execution_state.set_allow_ideation_borrow_idle_execution(
                            result.global_defaults.allow_ideation_borrow_idle_execution,
                        );
                        info!(
                            seeded_project_defaults = result.seeded_project_defaults,
                            seeded_global_defaults = result.seeded_global_defaults,
                            max_concurrent = result.project_defaults.max_concurrent_tasks,
                            project_ideation_max = result.project_defaults.project_ideation_max,
                            global_max_concurrent = result.global_defaults.global_max_concurrent,
                            global_ideation_max = result.global_defaults.global_ideation_max,
                            allow_ideation_borrow_idle_execution =
                                result.global_defaults.allow_ideation_borrow_idle_execution,
                            "Initialized execution settings from DB/YAML defaults"
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load/seed execution settings from database, using defaults: {}",
                            e
                        );
                    }
                }

                match load_or_seed_agent_lane_settings_defaults(
                    init_agent_lane_settings_repo,
                    &agent_harness_defaults,
                )
                .await
                {
                    Ok(result) => {
                        info!(
                            seeded_global_lane_count = result.seeded_global_lanes.len(),
                            seeded_global_lanes = ?result
                                .seeded_global_lanes
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>(),
                            configured_global_lane_count = result.global_defaults.len(),
                            "Initialized agent harness defaults from DB/YAML defaults"
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load/seed agent harness defaults from database, using runtime fallbacks: {}",
                            e
                        );
                    }
                }
            });

            // Seed ideation effort defaults (idempotent — only seeds when no global row exists)
            let init_effort_repo = Arc::clone(&app_state.ideation_effort_settings_repo);
            tauri::async_runtime::block_on(async move {
                match seed_ideation_effort_defaults(init_effort_repo).await {
                    Ok(result) => {
                        if result.seeded_global {
                            tracing::info!("Seeded global ideation effort defaults (inherit/inherit)");
                        }
                    }
                    Err(e) => tracing::warn!("Failed to seed ideation effort defaults: {}", e),
                }
            });

            // Seed ideation model defaults (idempotent — only seeds when no global row exists)
            let init_model_repo = Arc::clone(&app_state.ideation_model_settings_repo);
            tauri::async_runtime::block_on(async move {
                match seed_ideation_model_settings(init_model_repo).await {
                    Ok(_) => {
                        tracing::debug!("Ideation model settings seeded (or already existed)");
                    }
                    Err(e) => tracing::warn!("Failed to seed ideation model settings: {}", e),
                }
            });

            // Expire stale pending questions/permissions from previous runs.
            // Must happen before the HTTP server starts accepting agent requests.
            {
                let qs = Arc::clone(&app_state.question_state);
                let ps = Arc::clone(&app_state.permission_state);
                tauri::async_runtime::block_on(async move {
                    qs.expire_stale_on_startup().await;
                    ps.expire_stale_on_startup().await;
                });
            }

            // Periodic sweep for orphaned in-memory pending questions.
            // Cleans up questions from agents that died without resolving them
            // (complement to expire_stale_on_startup which only runs once at boot).
            {
                let qs = Arc::clone(&app_state.question_state);
                tauri::async_runtime::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                        qs.sweep_stale(std::time::Duration::from_secs(900)).await;
                    }
                });
            }

            // Cleanup stale process state from previous session.
            // All spawned agent team processes are children of the Tauri app, so any
            // restart (including crash) leaves their DB rows in an active state.
            {
                let team_repo = Arc::clone(&app_state.team_session_repo);
                tauri::async_runtime::block_on(async move {
                    match team_repo.disband_all_active("app_restart").await {
                        Ok(n) => info!(count = n, "Disbanded stale team sessions on startup"),
                        Err(e) => warn!(error = %e, "Failed to disband stale team sessions"),
                    }
                });
            }

            // All spawned processes are Tauri children — app restart means they are dead.
            {
                let process_repo = Arc::clone(&app_state.process_repo);
                tauri::async_runtime::block_on(async move {
                    match process_repo.fail_all_active("app_restart").await {
                        Ok(n) => info!(count = n, "Marked stale research processes failed on startup"),
                        Err(e) => warn!(error = %e, "Failed to mark stale research processes failed on startup"),
                    }
                });
            }

            // Start HTTP server for MCP proxy on port 3847
            // Create a second AppState sharing the Tauri AppState's DB connection,
            // plus shared in-memory state (question_state, permission_state, message_queue)
            // so MCP handlers and Tauri commands operate on the same data.
            let http_app_state = build_http_app_state(&app_state, app_handle)
                .expect("Failed to initialize AppState for HTTP server");
            // Spawn HTTP server with pre-cloned state
            tauri::async_runtime::spawn(async move {
                if let Err(e) = http_server::start_http_server(http_app_state, http_execution_state, http_team_tracker).await {
                    tracing::error!("HTTP server failed: {}", e);
                }
            });

            // Register configured MCP server with Claude Code CLI
            // This ensures the MCP tools are available regardless of user's working directory
            if let (Some(cli_path), Some(plugin_dir)) = (
                infrastructure::agents::claude::find_claude_cli(),
                infrastructure::agents::claude::find_plugin_dir(),
            ) {
                info!("Registering configured MCP server...");
                tauri::async_runtime::spawn(async move {
                    match infrastructure::agents::claude::register_mcp_server(&cli_path, &plugin_dir).await {
                        Ok(()) => info!("Configured MCP server registered successfully"),
                        Err(e) => warn!("Failed to register configured MCP server: {}", e),
                    }
                });
            } else {
                warn!("Could not find Claude CLI or plugin directory - MCP server not registered");
            }

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
            let startup_ideation_effort_settings_repo = Arc::clone(&app_state.ideation_effort_settings_repo);
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
                // Wait for HTTP server to be ready
                tokio::time::sleep(Duration::from_millis(500)).await;

                if application::startup_jobs::is_startup_recovery_disabled() {
                    info!(
                        env_var = application::startup_jobs::RALPHX_DISABLE_STARTUP_RECOVERY_ENV,
                        "Startup recovery disabled via environment; skipping startup recovery pipeline"
                    );
                    return;
                }

                info!("Starting startup job runner...");

                // Create TaskSchedulerService for auto-scheduling Ready tasks
                let scheduler_concrete = Arc::new(TaskSchedulerService::<tauri::Wry>::new(
                    Arc::clone(&startup_execution_state),
                    startup_project_repo.clone(),
                    startup_task_repo.clone(),
                    startup_task_dependency_repo.clone(),
                    startup_chat_message_repo.clone(),
                    startup_chat_attachment_repo.clone(),
                    startup_conversation_repo.clone(),
                    startup_agent_run_repo.clone(),
                    startup_ideation_session_repo.clone(),
                    startup_activity_event_repo.clone(),
                    startup_message_queue.clone(),
                    startup_running_agent_registry.clone(),
                    startup_memory_event_repo.clone(),
                    Some(startup_app_handle.clone()),
                )
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry)));
                scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn domain::state_machine::services::TaskScheduler>);
                let task_scheduler: Arc<dyn domain::state_machine::services::TaskScheduler> = scheduler_concrete;

                // Clone repos for ChatResumptionRunner before they're consumed by TaskTransitionService/StartupJobRunner
                let chat_resumption_agent_run_repo = Arc::clone(&startup_agent_run_repo);
                let chat_resumption_task_repo = Arc::clone(&startup_task_repo);
                let chat_resumption_task_dependency_repo = Arc::clone(&startup_task_dependency_repo);
                let chat_resumption_project_repo = Arc::clone(&startup_project_repo);
                let chat_resumption_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let chat_resumption_chat_attachment_repo = Arc::clone(&startup_chat_attachment_repo);
                let chat_resumption_artifact_repo = Arc::clone(&startup_artifact_repo);
                let chat_resumption_conversation_repo = Arc::clone(&startup_conversation_repo);
                let chat_resumption_ideation_session_repo = Arc::clone(&startup_ideation_session_repo);
                let chat_resumption_activity_event_repo = Arc::clone(&startup_activity_event_repo);
                let chat_resumption_message_queue = Arc::clone(&startup_message_queue);
                let chat_resumption_running_agent_registry = Arc::clone(&startup_running_agent_registry);
                let chat_resumption_memory_event_repo = Arc::clone(&startup_memory_event_repo);
                let chat_resumption_execution_settings_repo =
                    Arc::clone(&startup_execution_settings_repo);
                let chat_resumption_agent_lane_settings_repo =
                    Arc::clone(&startup_agent_lane_settings_repo);
                let chat_resumption_app_handle = startup_app_handle.clone();

                let startup_runner_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let startup_runner_ideation_session_repo = Arc::clone(&startup_ideation_session_repo);
                let startup_runner_activity_event_repo = Arc::clone(&startup_activity_event_repo);
                let startup_runner_message_queue = Arc::clone(&startup_message_queue);
                let startup_runner_running_agent_registry =
                    Arc::clone(&startup_running_agent_registry);

                // Clone repos for periodic reconciliation runner
                let reconcile_task_repo = Arc::clone(&startup_task_repo);
                let reconcile_task_dependency_repo = Arc::clone(&startup_task_dependency_repo);
                let reconcile_project_repo = Arc::clone(&startup_project_repo);
                let reconcile_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let reconcile_chat_attachment_repo = Arc::clone(&startup_chat_attachment_repo);
                let reconcile_conversation_repo = Arc::clone(&startup_conversation_repo);
                let reconcile_agent_run_repo = Arc::clone(&startup_agent_run_repo);
                let reconcile_ideation_session_repo = Arc::clone(&startup_ideation_session_repo);
                let reconcile_activity_event_repo = Arc::clone(&startup_activity_event_repo);
                let reconcile_message_queue = Arc::clone(&startup_message_queue);
                let reconcile_running_agent_registry = Arc::clone(&startup_running_agent_registry);
                let reconcile_memory_event_repo = Arc::clone(&startup_memory_event_repo);
                let reconcile_review_repo = Arc::clone(&startup_review_repo);
                let reconcile_app_handle = startup_app_handle.clone();
                let verification_recon_app_handle = startup_app_handle.clone();
                // Clone for external MCP startup (after HTTP server ready, at end of startup_jobs)
                let external_mcp_app_handle = startup_app_handle.clone();
                // Clone for recovery queue processor's chat service and event emission
                let recovery_cs_app_handle = startup_app_handle.clone();
                // Pre-clone repos for recovery queue processor's ClaudeChatService
                // (must happen before StartupJobRunner moves some of these originals)
                let recovery_cs_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let recovery_cs_chat_attachment_repo = Arc::clone(&startup_chat_attachment_repo);
                let recovery_cs_artifact_repo = Arc::clone(&startup_artifact_repo);
                let recovery_cs_conversation_repo = Arc::clone(&startup_conversation_repo);
                let recovery_cs_agent_run_repo = Arc::clone(&startup_agent_run_repo);
                let recovery_cs_project_repo = Arc::clone(&startup_project_repo);
                let recovery_cs_task_repo = Arc::clone(&startup_task_repo);
                let recovery_cs_task_dep_repo = Arc::clone(&startup_task_dependency_repo);
                let recovery_cs_ideation_repo = Arc::clone(&startup_ideation_session_repo);
                let recovery_cs_activity_repo = Arc::clone(&startup_activity_event_repo);
                let recovery_cs_message_queue = Arc::clone(&startup_message_queue);
                let recovery_cs_running_reg = Arc::clone(&startup_running_agent_registry);
                let recovery_cs_memory_event_repo = Arc::clone(&startup_memory_event_repo);
                let recovery_cs_ipr = Arc::clone(&startup_interactive_process_registry);
                let recovery_cs_execution_settings_repo =
                    Arc::clone(&startup_execution_settings_repo);
                let recovery_cs_agent_lane_repo = Arc::clone(&startup_agent_lane_settings_repo);
                let recovery_cs_ideation_effort_repo = Arc::clone(&startup_ideation_effort_settings_repo);
                let recovery_cs_ideation_model_repo = Arc::clone(&startup_ideation_model_settings_repo);

                // Clone task_dependency_repo for StartupJobRunner (before TaskTransitionService consumes it)
                let startup_runner_task_dep_repo = Arc::clone(&startup_task_dependency_repo);
                let startup_runner_app_handle = startup_app_handle.clone();
                // Clone task_repo for watchdog (before StartupJobRunner moves it)
                let watchdog_task_repo = Arc::clone(&startup_task_repo);
                let watchdog_project_repo = Arc::clone(&startup_project_repo);

                let build_transition_service_builder =
                    |task_repo,
                     task_dependency_repo,
                     project_repo,
                     chat_message_repo,
                     chat_attachment_repo,
                     conversation_repo,
                     agent_run_repo,
                     ideation_session_repo,
                     activity_event_repo,
                     message_queue,
                     running_agent_registry,
                     memory_event_repo,
                     app_handle: tauri::AppHandle| {
                        let deps = RuntimeFactoryDeps {
                            task_repo,
                            task_dependency_repo,
                            project_repo,
                            chat_message_repo,
                            chat_attachment_repo,
                            conversation_repo,
                            agent_run_repo,
                            ideation_session_repo,
                            activity_event_repo,
                            message_queue,
                            running_agent_registry,
                            memory_event_repo,
                            execution_settings_repo: Some(Arc::clone(
                                &startup_execution_settings_repo,
                            )),
                            agent_lane_settings_repo: Some(Arc::clone(
                                &startup_agent_lane_settings_repo,
                            )),
                            plan_branch_repo: Some(Arc::clone(&startup_plan_branch_repo)),
                            interactive_process_registry: Some(Arc::clone(
                                &startup_interactive_process_registry,
                            )),
                        };
                        let service = build_transition_service_with_fallback(
                            &Some(app_handle.clone()),
                            Arc::clone(&startup_execution_state),
                            &deps,
                        )
                        .with_agentic_client(Arc::clone(&startup_agent_client));

                        service
                            .with_task_scheduler(Arc::clone(&task_scheduler))
                            .with_step_repo(Arc::clone(&startup_step_repo))
                    };

                // Create TaskTransitionService for startup resumption
                let mut transition_service_builder = build_transition_service_builder(
                    Arc::clone(&startup_task_repo),
                    Arc::clone(&startup_task_dependency_repo),
                    Arc::clone(&startup_project_repo),
                    Arc::clone(&startup_chat_message_repo),
                    Arc::clone(&startup_chat_attachment_repo),
                    Arc::clone(&startup_conversation_repo),
                    Arc::clone(&startup_agent_run_repo),
                    Arc::clone(&startup_ideation_session_repo),
                    Arc::clone(&startup_activity_event_repo),
                    Arc::clone(&startup_message_queue),
                    Arc::clone(&startup_running_agent_registry),
                    Arc::clone(&startup_memory_event_repo),
                    startup_app_handle,
                );

                if let Some(ref pub_) = startup_webhook_publisher {
                    transition_service_builder = transition_service_builder.with_webhook_publisher_for_emitter(Arc::clone(pub_));
                }

                let transition_service = Arc::new(
                    transition_service_builder
                        .with_external_events_repo(Arc::clone(&startup_external_events_repo))
                        .with_session_merge_locks(Arc::clone(&startup_session_merge_locks))
                );

                // PR startup recovery: restart pollers for tasks that were polling when app shut down.
                // Must run BEFORE StartupJobRunner to prevent reconciler re-entering on_enter(Merging)
                // for PR-mode tasks before their pollers exist.
                tracing::info!("Running PR startup recovery...");
                crate::application::pr_startup_recovery::recover_pr_pollers(
                    Arc::clone(&startup_task_repo),
                    Arc::clone(&startup_plan_branch_repo),
                    Arc::clone(&startup_pr_poller_registry),
                    Arc::clone(&startup_project_repo),
                    Arc::clone(&transition_service),
                ).await;

                // Create chat service for Phase N+1 ideation recovery.
                // Must be constructed BEFORE StartupJobRunner::new() consumes the repos.
                let recovery_chat_service_app_handle = startup_runner_app_handle.clone();
                let recovery_chat_service_deps = ChatRuntimeFactoryDeps {
                    chat_message_repo: Arc::clone(&startup_chat_message_repo),
                    chat_attachment_repo: Arc::clone(&startup_chat_attachment_repo),
                    artifact_repo: Arc::clone(&startup_artifact_repo),
                    conversation_repo: Arc::clone(&startup_conversation_repo),
                    agent_run_repo: Arc::clone(&startup_agent_run_repo),
                    project_repo: Arc::clone(&startup_project_repo),
                    task_repo: Arc::clone(&startup_task_repo),
                    task_dependency_repo: Arc::clone(&startup_task_dependency_repo),
                    ideation_session_repo: Arc::clone(&startup_ideation_session_repo),
                    activity_event_repo: Arc::clone(&startup_activity_event_repo),
                    message_queue: Arc::clone(&startup_message_queue),
                    running_agent_registry: Arc::clone(&startup_running_agent_registry),
                    memory_event_repo: Arc::clone(&startup_memory_event_repo),
                    execution_settings_repo: Some(Arc::clone(&startup_execution_settings_repo)),
                    agent_lane_settings_repo: Some(Arc::clone(&startup_agent_lane_settings_repo)),
                    ideation_effort_settings_repo: Some(Arc::clone(
                        &startup_ideation_effort_settings_repo,
                    )),
                    ideation_model_settings_repo: Some(Arc::clone(
                        &startup_ideation_model_settings_repo,
                    )),
                    plan_branch_repo: None,
                    task_proposal_repo: None,
                    task_step_repo: None,
                    review_repo: None,
                    interactive_process_registry: Some(Arc::clone(
                        &startup_interactive_process_registry,
                    )),
                    streaming_state_cache: None,
                };
                let recovery_chat_service: Arc<dyn application::ChatService> = Arc::new(
                    build_chat_service_with_fallback(
                        &Some(recovery_chat_service_app_handle.clone()),
                        Some(Arc::clone(&startup_execution_state)),
                        &recovery_chat_service_deps,
                    ),
                );

                let runner = StartupJobRunner::new(
                    startup_task_repo,
                    startup_runner_task_dep_repo,
                    Arc::clone(&startup_project_repo),
                    startup_conversation_repo.clone(),
                    startup_runner_chat_message_repo,
                    Arc::clone(&startup_chat_attachment_repo),
                    startup_runner_ideation_session_repo,
                    startup_runner_activity_event_repo,
                    startup_runner_message_queue,
                    startup_runner_running_agent_registry,
                    Arc::clone(&startup_memory_event_repo),
                    startup_agent_run_repo,
                    Arc::clone(&transition_service),
                    Arc::clone(&startup_execution_state),
                    Arc::clone(&startup_active_project_state),
                    startup_app_state_repo,
                    Arc::clone(&startup_execution_settings_repo),
                    Some(Arc::clone(&startup_plan_branch_repo)),
                )
                .with_task_scheduler(Arc::clone(&task_scheduler))
                .with_app_handle(startup_runner_app_handle)
                .with_review_repo(Arc::clone(&startup_review_repo))
                .with_chat_service(recovery_chat_service);

                let startup_ideation_recovery_claims = runner.run().await;

                application::startup_background::recover_memory_archive_jobs_on_startup(
                    Arc::clone(&startup_memory_archive_repo),
                    Arc::clone(&startup_memory_entry_repo),
                    Arc::clone(&startup_project_repo),
                )
                .await;

                // Resume interrupted chat conversations (Ideation, Task, Project, TaskExecution, Review)
                // This runs after StartupJobRunner to avoid duplicate resumption of task-based chats
                info!("Starting chat resumption runner...");
                let chat_resumption = ChatResumptionRunner::<tauri::Wry>::new(
                    chat_resumption_agent_run_repo,
                    chat_resumption_conversation_repo,
                    chat_resumption_task_repo,
                    chat_resumption_task_dependency_repo,
                    chat_resumption_chat_message_repo,
                    chat_resumption_chat_attachment_repo,
                    chat_resumption_artifact_repo,
                    chat_resumption_project_repo,
                    chat_resumption_ideation_session_repo,
                    chat_resumption_activity_event_repo,
                    chat_resumption_message_queue,
                    chat_resumption_running_agent_registry,
                    chat_resumption_memory_event_repo,
                    Arc::clone(&startup_execution_state),
                )
                .with_app_handle(chat_resumption_app_handle)
                .with_execution_settings_repo(chat_resumption_execution_settings_repo)
                .with_agent_lane_settings_repo(chat_resumption_agent_lane_settings_repo)
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry));

                chat_resumption.run().await;

                let reconcile_webhook_publisher = startup_webhook_publisher.clone();

                let mut reconcile_transition_service_builder =
                    build_transition_service_builder(
                        Arc::clone(&reconcile_task_repo),
                        Arc::clone(&reconcile_task_dependency_repo),
                        Arc::clone(&reconcile_project_repo),
                        Arc::clone(&reconcile_chat_message_repo),
                        Arc::clone(&reconcile_chat_attachment_repo),
                        Arc::clone(&reconcile_conversation_repo),
                        Arc::clone(&reconcile_agent_run_repo),
                        Arc::clone(&reconcile_ideation_session_repo),
                        Arc::clone(&reconcile_activity_event_repo),
                        Arc::clone(&reconcile_message_queue),
                        Arc::clone(&reconcile_running_agent_registry),
                        Arc::clone(&reconcile_memory_event_repo),
                        reconcile_app_handle.clone(),
                    );

                if let Some(ref pub_) = reconcile_webhook_publisher {
                    reconcile_transition_service_builder = reconcile_transition_service_builder.with_webhook_publisher_for_emitter(Arc::clone(pub_));
                }

                let reconcile_transition_service = Arc::new(
                    reconcile_transition_service_builder
                        .with_external_events_repo(Arc::clone(&startup_external_events_repo))
                        .with_session_merge_locks(Arc::clone(&startup_session_merge_locks))
                );

                let reconcile_runner = ReconciliationRunner::new(
                    reconcile_task_repo,
                    reconcile_task_dependency_repo,
                    reconcile_project_repo,
                    reconcile_conversation_repo,
                    reconcile_chat_message_repo,
                    reconcile_chat_attachment_repo,
                    reconcile_ideation_session_repo,
                    reconcile_activity_event_repo,
                    reconcile_message_queue,
                    reconcile_running_agent_registry,
                    reconcile_memory_event_repo,
                    reconcile_agent_run_repo,
                    reconcile_transition_service,
                    Arc::clone(&startup_execution_state),
                    Some(reconcile_app_handle),
                )
                .with_execution_settings_repo(Arc::clone(&startup_execution_settings_repo))
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry))
                .with_review_repo(reconcile_review_repo);

                // One-shot startup recovery: re-queue timeout-failed tasks (attempt_count < 3).
                // Must run before reconcile_stuck_tasks so recovered tasks are visible immediately.
                reconcile_runner.recover_timeout_failures().await;

                reconcile_runner.reconcile_stuck_tasks().await;

                tauri::async_runtime::spawn(async move {
                    let interval = Duration::from_secs(30);
                    loop {
                        tokio::time::sleep(interval).await;
                        reconcile_runner.reconcile_stuck_tasks().await;
                    }
                });

                // Spawn scheduler watchdog: periodic safety net for tasks stuck in Ready
                // and review-parked PendingReview after freshness backoff expiry.
                application::startup_background::spawn_watchdog(
                    Arc::clone(&task_scheduler),
                    watchdog_task_repo,
                    watchdog_project_repo,
                );

                // Spawn verification reconciliation service: resets stuck in_progress sessions.
                // Startup scan runs immediately; periodic scan runs every reconciliation_interval_secs.
                {
                    use application::reconciliation::recovery_queue::{
                        create_recovery_queue, RecoveryQueueConfig,
                    };
                    use application::reconciliation::verification_reconciliation::{
                        VerificationReconciliationConfig, VerificationReconciliationService,
                    };

                    // PDM-172 Phase 1: Construct shared RecoveryQueue infrastructure.
                    // Spawn ordering (NON-NEGOTIABLE — Constraint 9):
                    //   1. Construct queue + processor
                    //   2. Spawn processor task (receiver must be ready before items are submitted)
                    //   3. Call startup_scan() which may submit recovery items
                    let recovery_config = RecoveryQueueConfig::default();
                    // Construct a ClaudeChatService for the recovery queue processor.
                    // Uses pre-cloned repos (cloned before StartupJobRunner consumed some originals).
                    let recovery_queue_chat_deps = ChatRuntimeFactoryDeps {
                        chat_message_repo: recovery_cs_chat_message_repo,
                        chat_attachment_repo: recovery_cs_chat_attachment_repo,
                        artifact_repo: recovery_cs_artifact_repo,
                        conversation_repo: recovery_cs_conversation_repo,
                        agent_run_repo: recovery_cs_agent_run_repo,
                        project_repo: recovery_cs_project_repo,
                        task_repo: recovery_cs_task_repo,
                        task_dependency_repo: recovery_cs_task_dep_repo,
                        ideation_session_repo: recovery_cs_ideation_repo,
                        activity_event_repo: recovery_cs_activity_repo,
                        message_queue: recovery_cs_message_queue,
                        running_agent_registry: recovery_cs_running_reg,
                        memory_event_repo: recovery_cs_memory_event_repo,
                        execution_settings_repo: Some(recovery_cs_execution_settings_repo),
                        agent_lane_settings_repo: Some(recovery_cs_agent_lane_repo),
                        ideation_effort_settings_repo: Some(recovery_cs_ideation_effort_repo),
                        ideation_model_settings_repo: Some(recovery_cs_ideation_model_repo),
                        plan_branch_repo: None,
                        task_proposal_repo: None,
                        task_step_repo: None,
                        review_repo: None,
                        interactive_process_registry: Some(recovery_cs_ipr),
                        streaming_state_cache: None,
                    };
                    let recovery_chat_service: std::sync::Arc<dyn application::chat_service::ChatService> =
                        std::sync::Arc::new(
                            build_chat_service_with_fallback(
                                &Some(recovery_cs_app_handle.clone()),
                                Some(Arc::clone(&startup_execution_state)),
                                &recovery_queue_chat_deps,
                            ),
                        );
                    let (recovery_queue, recovery_processor) = create_recovery_queue(
                        Arc::clone(&startup_running_agent_registry),
                        Arc::clone(&startup_interactive_process_registry),
                        Arc::clone(&startup_ideation_session_repo),
                        recovery_chat_service,
                        Some(recovery_cs_app_handle),
                        recovery_config,
                    );
                    let recovery_queue = Arc::new(recovery_queue);
                    application::startup_background::spawn_recovery_queue_processor(
                        recovery_processor,
                    );

                    let vcfg = infrastructure::agents::claude::verification_config();
                    let ext_cfg = infrastructure::agents::claude::external_mcp_config();
                    let verification_config = VerificationReconciliationConfig {
                        stale_after_secs: vcfg.reconciliation_stale_after_secs,
                        auto_verify_stale_secs: vcfg.auto_verify_stale_secs,
                        interval_secs: vcfg.reconciliation_interval_secs,
                        external_session_stale_secs: ext_cfg.external_session_stale_secs,
                        external_session_startup_grace_secs: ext_cfg.external_session_startup_grace_secs,
                    };
                    let verification_session_repo = Arc::clone(&startup_ideation_session_repo);
                    let svc = Arc::new(
                        VerificationReconciliationService::new(
                            verification_session_repo,
                            verification_config,
                        )
                        .with_app_handle(verification_recon_app_handle)
                        .with_recovery_queue(Arc::clone(&recovery_queue))
                        .with_running_agent_registry(Arc::clone(&startup_running_agent_registry)),
                    );
                    application::startup_background::startup_scan_verification_reconciliation(
                        svc,
                        &startup_ideation_recovery_claims,
                    )
                    .await;
                }

                application::startup_background::spawn_cleanup_loops(
                    Arc::clone(&startup_external_events_repo),
                    Arc::clone(&startup_memory_archive_repo),
                    Arc::clone(&startup_memory_entry_repo),
                    Arc::clone(&startup_project_repo),
                );

                application::startup_background::maybe_start_external_mcp(
                    external_mcp_app_handle,
                    |port, timeout| Box::pin(wait_for_backend_ready(port, timeout)),
                )
                .await;

            });

            register_managed_state(app, app_state, service_team_tracker);

            Ok(())
        })
        .manage(execution_state)
        .manage(active_project_state)
        .manage(team_tracker)
        .invoke_handler(crate::register_tauri_commands!())
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            application::shutdown::handle_run_event(app_handle, &event);
        });
}
