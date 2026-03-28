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
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use crate::utils::redacting_writer::RedactingMakeWriter;

use crate::infrastructure::{ExternalMcpHandle, ExternalMcpSupervisor};

use application::{
    load_or_seed_execution_settings_defaults, ChatResumptionRunner, ClaudeChatService,
    EventCleanupService, ReconciliationRunner, StartupJobRunner, TaskSchedulerService,
    TaskTransitionService,
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
        // Determine log directory: dev → {repo_root}/logs/, prod → ~/Library/Application Support/com.ralphx.app/logs/
        let log_dir = if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../logs")
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
            {
                use tauri::{WebviewUrl, WebviewWindowBuilder, TitleBarStyle, LogicalPosition, Position};

                let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                    .title("")
                    .inner_size(1200.0, 800.0)
                    .decorations(true)
                    .hidden_title(true)
                    .visible(false); // Start hidden to avoid resize flash

                #[cfg(target_os = "macos")]
                {
                    builder = builder
                        .title_bar_style(TitleBarStyle::Overlay)
                        .traffic_light_position(Position::Logical(LogicalPosition { x: 20.0, y: 30.0 }));
                }

                let webview_window = builder.build()?;

                // Plugin with with_state_flags auto-restores, just show the window
                let _ = webview_window.show();
            }

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
            let execution_defaults =
                infrastructure::agents::claude::execution_defaults_config().clone();
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
            let shared_db_conn = Arc::clone(app_state.db.inner());
            let shared_question_state = Arc::clone(&app_state.question_state);
            let shared_permission_state = Arc::clone(&app_state.permission_state);
            let shared_message_queue = Arc::clone(&app_state.message_queue);
            let shared_interactive_process_registry = Arc::clone(&app_state.interactive_process_registry);
            let shared_github_service = app_state.github_service.clone();
            let shared_pr_poller_registry = Arc::clone(&app_state.pr_poller_registry);
            let mut http_app_state_inner =
                AppState::new_production_shared(app_handle, shared_db_conn).expect("Failed to initialize AppState for HTTP server");
            http_app_state_inner.question_state = shared_question_state;
            http_app_state_inner.permission_state = shared_permission_state;
            http_app_state_inner.message_queue = shared_message_queue;
            http_app_state_inner.interactive_process_registry = shared_interactive_process_registry;
            http_app_state_inner.github_service = shared_github_service;
            http_app_state_inner.pr_poller_registry = shared_pr_poller_registry;
            // INVARIANT: streaming_state_cache uses Arc internally — .clone() shares the same data.
            // Do NOT change StreamingStateCache to deep-clone without updating this sharing.
            http_app_state_inner.streaming_state_cache = app_state.streaming_state_cache.clone();
            http_app_state_inner.webhook_publisher = app_state.webhook_publisher.clone();
            let http_app_state = Arc::new(http_app_state_inner);
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
            let startup_interactive_process_registry = Arc::clone(&app_state.interactive_process_registry);
            let startup_review_repo = Arc::clone(&app_state.review_repo);
            let startup_external_events_repo = Arc::clone(&app_state.external_events_repo);
            let startup_pr_poller_registry = Arc::clone(&app_state.pr_poller_registry);
            let startup_webhook_publisher = app_state.webhook_publisher.clone();
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

                // Clone task_dependency_repo for StartupJobRunner (before TaskTransitionService consumes it)
                let startup_runner_task_dep_repo = Arc::clone(&startup_task_dependency_repo);
                let startup_runner_app_handle = startup_app_handle.clone();
                // Clone task_repo for watchdog (before StartupJobRunner moves it)
                let watchdog_task_repo = Arc::clone(&startup_task_repo);

                // Create TaskTransitionService for startup resumption
                let mut transition_service_builder = TaskTransitionService::new(
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
                    Arc::clone(&startup_execution_state),
                    Some(startup_app_handle),
                    Arc::clone(&startup_memory_event_repo),
                )
                .with_task_scheduler(Arc::clone(&task_scheduler))
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                .with_step_repo(Arc::clone(&startup_step_repo))
                .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry));

                if let Some(ref pub_) = startup_webhook_publisher {
                    transition_service_builder = transition_service_builder.with_webhook_publisher_for_emitter(Arc::clone(pub_));
                }

                let transition_service = Arc::new(
                    transition_service_builder.with_external_events_repo(Arc::clone(&startup_external_events_repo))
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
                let recovery_chat_service: Arc<dyn application::ChatService> = Arc::new(
                    ClaudeChatService::new(
                        Arc::clone(&startup_chat_message_repo),
                        Arc::clone(&startup_chat_attachment_repo),
                        Arc::clone(&startup_artifact_repo),
                        Arc::clone(&startup_conversation_repo),
                        Arc::clone(&startup_agent_run_repo),
                        Arc::clone(&startup_project_repo),
                        Arc::clone(&startup_task_repo),
                        Arc::clone(&startup_task_dependency_repo),
                        Arc::clone(&startup_ideation_session_repo),
                        Arc::clone(&startup_activity_event_repo),
                        Arc::clone(&startup_message_queue),
                        Arc::clone(&startup_running_agent_registry),
                        Arc::clone(&startup_memory_event_repo),
                    )
                    .with_execution_state(Arc::clone(&startup_execution_state))
                    .with_execution_settings_repo(Arc::clone(&startup_execution_settings_repo))
                    .with_app_handle(recovery_chat_service_app_handle)
                    .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry)),
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

                runner.run().await;

                // Recover pending/failed memory archive jobs from previous session
                // Process any jobs that were interrupted by app shutdown
                info!("Recovering pending memory archive jobs...");
                let archive_service = Arc::new(application::MemoryArchiveService::new(
                    Arc::clone(&startup_memory_archive_repo),
                    Arc::clone(&startup_memory_entry_repo),
                    Arc::clone(&startup_project_repo),
                    std::path::PathBuf::from("."), // Use current working directory as project root
                ));

                // Process all pending/failed jobs on startup
                let recovered_count = match startup_memory_archive_repo.count_claimable().await {
                    Ok(count) => {
                        info!(pending_jobs = count, "Found memory archive jobs to recover");
                        let mut processed = 0;
                        while processed < count {
                            match archive_service.process_next_job().await {
                                Ok(true) => processed += 1,
                                Ok(false) => break, // No more jobs
                                Err(e) => {
                                    tracing::warn!(error = %e, "Failed to process archive job during recovery");
                                    break;
                                }
                            }
                        }
                        processed
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to count claimable archive jobs");
                        0
                    }
                };
                if recovered_count > 0 {
                    info!(recovered = recovered_count, "Completed memory archive job recovery");
                }

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
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry));

                chat_resumption.run().await;

                let reconcile_webhook_publisher = startup_webhook_publisher.clone();

                let mut reconcile_transition_service_builder = TaskTransitionService::new(
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
                        Arc::clone(&startup_execution_state),
                        Some(reconcile_app_handle.clone()),
                        Arc::clone(&reconcile_memory_event_repo),
                    )
                    .with_task_scheduler(Arc::clone(&task_scheduler))
                    .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo))
                    .with_step_repo(Arc::clone(&startup_step_repo))
                    .with_interactive_process_registry(Arc::clone(&startup_interactive_process_registry));

                if let Some(ref pub_) = reconcile_webhook_publisher {
                    reconcile_transition_service_builder = reconcile_transition_service_builder.with_webhook_publisher_for_emitter(Arc::clone(pub_));
                }

                let reconcile_transition_service = Arc::new(
                    reconcile_transition_service_builder.with_external_events_repo(Arc::clone(&startup_external_events_repo))
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

                // Spawn Ready-task watchdog: periodic safety net for tasks stuck in Ready state.
                // Reschedules stale Ready tasks every 60s (safety net for S5, S6, S7, S8).
                let watchdog_scheduler = Arc::clone(&task_scheduler);
                tauri::async_runtime::spawn(async move {
                    application::ReadyWatchdog::new(watchdog_scheduler, watchdog_task_repo)
                        .run_loop()
                        .await;
                });

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
                    let recovery_chat_service: std::sync::Arc<dyn application::chat_service::ChatService> =
                        std::sync::Arc::new(
                            application::chat_service::ClaudeChatService::<tauri::Wry>::new(
                                recovery_cs_chat_message_repo,
                                recovery_cs_chat_attachment_repo,
                                recovery_cs_artifact_repo,
                                recovery_cs_conversation_repo,
                                recovery_cs_agent_run_repo,
                                recovery_cs_project_repo,
                                recovery_cs_task_repo,
                                recovery_cs_task_dep_repo,
                                recovery_cs_ideation_repo,
                                recovery_cs_activity_repo,
                                recovery_cs_message_queue,
                                recovery_cs_running_reg,
                                recovery_cs_memory_event_repo,
                            )
                            .with_execution_state(Arc::clone(&startup_execution_state))
                            .with_app_handle(recovery_cs_app_handle.clone())
                            .with_interactive_process_registry(recovery_cs_ipr),
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
                    tauri::async_runtime::spawn(async move {
                        recovery_processor.run().await;
                    });

                    let vcfg = infrastructure::agents::claude::verification_config();
                    let ext_cfg = infrastructure::agents::claude::external_mcp_config();
                    let verification_config = VerificationReconciliationConfig {
                        stale_after_secs: vcfg.reconciliation_stale_after_secs,
                        auto_verify_stale_secs: vcfg.auto_verify_stale_secs,
                        interval_secs: vcfg.reconciliation_interval_secs,
                        external_session_stale_secs: ext_cfg.external_session_stale_secs,
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
                    svc.startup_scan().await;
                    tauri::async_runtime::spawn(async move { svc.run_periodic().await });
                }

                // Spawn memory archive job background processing loop
                // Clone required repositories for the archive job processor
                let archive_job_memory_archive_repo = Arc::clone(&startup_memory_archive_repo);
                let archive_job_memory_entry_repo = Arc::clone(&startup_memory_entry_repo);
                let archive_job_project_repo = Arc::clone(&startup_project_repo);

                // Spawn external_events cleanup job (hourly pruning of old rows)
                let cleanup_external_events_repo = Arc::clone(&startup_external_events_repo);
                tauri::async_runtime::spawn(async move {
                    EventCleanupService::new(cleanup_external_events_repo)
                        .run_loop()
                        .await;
                });

                tauri::async_runtime::spawn(async move {
                    let archive_service = Arc::new(application::MemoryArchiveService::new(
                        archive_job_memory_archive_repo,
                        archive_job_memory_entry_repo,
                        archive_job_project_repo,
                        std::path::PathBuf::from("."),
                    ));

                    let mut backoff_duration = Duration::from_secs(0);

                    loop {
                        if !backoff_duration.is_zero() {
                            tracing::debug!(
                                backoff_secs = backoff_duration.as_secs(),
                                "Memory archive job processor backing off after error"
                            );
                            tokio::time::sleep(backoff_duration).await;
                            backoff_duration = Duration::from_secs(0);
                        }

                        match archive_service.process_next_job().await {
                            Ok(true) => {
                                // Job processed successfully, immediately check for more without sleeping
                                tracing::debug!("Memory archive job processed, checking for more");
                                backoff_duration = Duration::from_secs(0);
                                // Loop back immediately without sleep
                            }
                            Ok(false) => {
                                // No jobs available, sleep 30s before next poll
                                tracing::debug!("No memory archive jobs available, sleeping");
                                tokio::time::sleep(Duration::from_secs(30)).await;
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "Failed to process memory archive job");
                                // Back off for 60s on error
                                backoff_duration = Duration::from_secs(60);
                                tokio::time::sleep(backoff_duration).await;
                            }
                        }
                    }
                });

                // ── External MCP auto-start ──────────────────────────────────────────────
                // Starts the external MCP server (:3848) after :3847 is confirmed ready.
                // Gated on: config.enabled + TLS validation + entry_path.exists()
                {
                    let config = infrastructure::agents::claude::external_mcp_config().clone();
                    if config.enabled {
                        // Ensure :3847 is ready before spawning :3848
                        match wait_for_backend_ready(3847, Duration::from_secs(30)).await {
                            Err(e) => {
                                warn!("Backend not ready, skipping external MCP start: {}", e);
                            }
                            Ok(()) => {
                                info!("Backend :3847 ready, starting external MCP server");
                                // Validate config (TLS enforcement, port/host checks)
                                match infrastructure::agents::claude::validate_external_mcp_config(
                                    &config,
                                ) {
                                    Err(e) => {
                                        warn!("External MCP config invalid, skipping start: {}", e);
                                    }
                                    Ok(()) => {
                                        // Resolve entry path: plugin_dir/ralphx-external-mcp/build/index.js
                                        let entry_path = infrastructure::agents::claude::find_plugin_dir()
                                            .map(|p| p.join("ralphx-external-mcp/build/index.js"));
                                        match entry_path {
                                            None => {
                                                warn!("Plugin dir not found, cannot start external MCP");
                                            }
                                            Some(ep) if !ep.exists() => {
                                                warn!(
                                                    path = %ep.display(),
                                                    "External MCP entry not found — run `npm run build` in ralphx-plugin/ralphx-external-mcp"
                                                );
                                            }
                                            Some(ep) => {
                                                let node_path = infrastructure::agents::claude::node_utils::find_node_binary();
                                                let app_data_dir = external_mcp_app_handle
                                                    .path()
                                                    .app_data_dir()
                                                    .unwrap_or_else(|_| PathBuf::from("."));
                                                let supervisor = Arc::new(ExternalMcpSupervisor::new(
                                                    config,
                                                    external_mcp_app_handle.clone(),
                                                    app_data_dir,
                                                ));
                                                match Arc::clone(&supervisor).start(node_path, ep).await {
                                                    Ok(()) => {
                                                        let handle = external_mcp_app_handle
                                                            .state::<ExternalMcpHandle>();
                                                        if handle.set(supervisor).is_err() {
                                                            warn!("ExternalMcpHandle already initialized");
                                                        } else {
                                                            info!("External MCP supervisor started and registered");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        warn!("Failed to start external MCP: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

            });

            // Clone team repos before app_state is moved into Tauri state
            let team_session_repo = Arc::clone(&app_state.team_session_repo);
            let team_message_repo = Arc::clone(&app_state.team_message_repo);

            // Register ThrottledEmitter for batching rapid event emissions.
            // Must be registered before app_state since services read it via try_state().
            let throttled_emitter = application::ThrottledEmitter::new(app.handle().clone());
            app.manage(throttled_emitter);

            // Register app_state with Tauri's state management
            app.manage(app_state);

            // Register team service (wraps tracker with event emission + persistence)
            let team_service = std::sync::Arc::new(application::TeamService::new_with_repos(
                std::sync::Arc::new(service_team_tracker),
                app.handle().clone(),
                team_session_repo,
                team_message_repo,
            ));
            app.manage(team_service);

            // Pre-register ExternalMcpHandle (OnceLock) — populated later in startup_jobs.
            // Must be registered here (before build()) so app.state::<ExternalMcpHandle>() works.
            app.manage(ExternalMcpHandle::new());

            Ok(())
        })
        .manage(execution_state)
        .manage(active_project_state)
        .manage(team_tracker)
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::health::health_check,
            commands::task_commands::query::list_tasks,
            commands::task_commands::query::get_task,
            commands::task_commands::mutation::create_task,
            commands::task_commands::mutation::update_task,
            commands::task_commands::mutation::answer_user_question,
            commands::task_commands::mutation::inject_task,
            commands::task_commands::mutation::move_task,
            commands::task_commands::mutation::archive_task,
            commands::task_commands::mutation::restore_task,
            commands::task_commands::mutation::block_task,
            commands::task_commands::mutation::unblock_task,
            commands::task_commands::mutation::cleanup_task,
            commands::task_commands::mutation::cleanup_tasks_in_group,
            commands::task_commands::mutation::cancel_tasks_in_group,
            commands::task_commands::mutation::pause_tasks_in_group,
            commands::task_commands::mutation::resume_tasks_in_group,
            commands::task_commands::mutation::archive_tasks_in_group,
            commands::task_commands::mutation::pause_task,
            commands::task_commands::mutation::resume_task,
            commands::task_commands::mutation::stop_task,
            commands::task_commands::query::get_archived_count,
            commands::task_commands::query::search_tasks,
            commands::task_commands::query::get_valid_transitions,
            commands::task_commands::query::get_tasks_awaiting_review,
            commands::task_commands::query::get_task_state_transitions,
            commands::task_commands::query::get_task_dependency_graph,
            commands::task_commands::query::get_task_timeline_events,
            // Task step commands
            commands::task_step_commands::create_task_step,
            commands::task_step_commands::get_task_steps,
            commands::task_step_commands::update_task_step,
            commands::task_step_commands::reorder_task_steps,
            commands::task_step_commands::get_step_progress,
            commands::task_step_commands::start_step,
            commands::task_step_commands::complete_step,
            commands::task_step_commands::skip_step,
            commands::task_step_commands::fail_step,
            commands::project_commands::list_projects,
            commands::project_commands::get_project,
            commands::project_commands::create_project,
            commands::project_commands::update_project,
            commands::project_commands::archive_project,
            commands::project_commands::get_git_branches,
            commands::project_commands::get_git_default_branch,
            commands::project_commands::reanalyze_project,
            commands::project_commands::update_custom_analysis,
            commands::project_commands::get_git_remote_url,
            commands::project_commands::check_gh_auth,
            commands::project_commands::update_github_pr_enabled,
            commands::agent_profile_commands::list_agent_profiles,
            commands::agent_profile_commands::get_agent_profile,
            commands::agent_profile_commands::get_agent_profiles_by_role,
            commands::agent_profile_commands::get_builtin_agent_profiles,
            commands::agent_profile_commands::get_custom_agent_profiles,
            commands::agent_profile_commands::seed_builtin_profiles,
            commands::qa_commands::get_qa_settings,
            commands::qa_commands::update_qa_settings,
            commands::qa_commands::get_task_qa,
            commands::qa_commands::get_qa_results,
            commands::qa_commands::retry_qa,
            commands::qa_commands::skip_qa,
            commands::review_commands::get_pending_reviews,
            commands::review_commands::get_review_by_id,
            commands::review_commands::get_reviews_by_task_id,
            commands::review_commands::get_task_state_history,
            commands::review_commands::approve_review,
            commands::review_commands::request_changes,
            commands::review_commands::reject_review,
            commands::review_commands::approve_fix_task,
            commands::review_commands::reject_fix_task,
            commands::review_commands::get_fix_task_attempts,
            commands::review_commands::approve_task_for_review,
            commands::review_commands::request_task_changes_for_review,
            commands::review_commands::re_review_task_from_escalated,
            // Review issue commands
            commands::review_commands::get_task_issues,
            commands::review_commands::get_issue_progress,
            commands::review_commands::verify_issue,
            commands::review_commands::reopen_issue,
            commands::review_commands::mark_issue_in_progress,
            commands::review_commands::mark_issue_addressed,
            commands::execution_commands::get_execution_status,
            commands::execution_commands::pause_execution,
            commands::execution_commands::resume_execution,
            commands::execution_commands::stop_execution,
            commands::execution_commands::restart_task,
            commands::execution_commands::recover_task_execution,
            commands::execution_commands::resolve_recovery_prompt,
            commands::execution_commands::set_max_concurrent,
            commands::execution_commands::get_execution_settings,
            commands::execution_commands::update_execution_settings,
            commands::execution_commands::set_active_project,
            commands::execution_commands::get_active_project,
            commands::execution_commands::get_global_execution_settings,
            commands::execution_commands::update_global_execution_settings,
            commands::execution_commands::get_running_processes,
            // Merge pipeline commands
            commands::merge_pipeline_commands::get_merge_pipeline,
            commands::merge_pipeline_commands::get_merge_progress,
            commands::merge_pipeline_commands::get_merge_phase_list,
            // Metrics commands
            commands::metrics_commands::get_project_stats,
            commands::metrics_commands::get_project_trends,
            commands::metrics_commands::get_metrics_config,
            commands::metrics_commands::save_metrics_config,
            commands::metrics_commands::get_column_metrics,
            commands::metrics_commands::get_task_metrics,
            // Ideation session commands
            commands::ideation_commands::create_ideation_session,
            commands::ideation_commands::create_cross_project_session,
            commands::ideation_commands::migrate_proposals,
            commands::ideation_commands::get_ideation_session,
            commands::ideation_commands::get_ideation_session_with_data,
            commands::ideation_commands::list_ideation_sessions,
            commands::ideation_commands::get_session_group_counts,
            commands::ideation_commands::list_sessions_by_group,
            commands::ideation_commands::archive_ideation_session,
            commands::ideation_commands::reopen_ideation_session,
            commands::ideation_commands::update_ideation_session_title,
            commands::ideation_commands::spawn_session_namer,
            commands::ideation_commands::get_child_sessions,
            // Task proposal commands
            commands::ideation_commands::create_task_proposal,
            commands::ideation_commands::get_task_proposal,
            commands::ideation_commands::list_session_proposals,
            commands::ideation_commands::update_task_proposal,
            commands::ideation_commands::delete_task_proposal,
            commands::ideation_commands::toggle_proposal_selection,
            commands::ideation_commands::set_proposal_selection,
            commands::ideation_commands::reorder_proposals,
            commands::ideation_commands::assess_proposal_priority,
            commands::ideation_commands::assess_all_priorities,
            // Dependency and apply commands
            commands::ideation_commands::remove_proposal_dependency,
            commands::ideation_commands::get_proposal_dependencies,
            commands::ideation_commands::get_proposal_dependents,
            commands::ideation_commands::analyze_dependencies,
            commands::ideation_commands::apply_proposals_to_kanban,
            commands::ideation_commands::get_task_blockers,
            commands::ideation_commands::get_blocked_tasks,
            // Chat message commands
            commands::ideation_commands::send_chat_message,
            commands::ideation_commands::get_session_messages,
            commands::ideation_commands::get_recent_session_messages,
            commands::ideation_commands::get_project_messages,
            commands::ideation_commands::get_task_messages,
            commands::ideation_commands::delete_chat_message,
            commands::ideation_commands::delete_session_messages,
            commands::ideation_commands::count_session_messages,
            // Orchestrator commands
            commands::ideation_commands::send_orchestrator_message,
            commands::ideation_commands::is_orchestrator_available,
            // Ideation settings commands
            commands::ideation_commands::get_ideation_settings,
            commands::ideation_commands::update_ideation_settings,
            // Ideation export/import commands
            commands::ideation_commands::export_ideation_session,
            commands::ideation_commands::import_ideation_session,
            // Workflow commands
            commands::workflow_commands::get_workflows,
            commands::workflow_commands::get_workflow,
            commands::workflow_commands::create_workflow,
            commands::workflow_commands::update_workflow,
            commands::workflow_commands::set_default_workflow,
            commands::workflow_commands::get_active_workflow_columns,
            commands::workflow_commands::get_builtin_workflows,
            commands::workflow_commands::seed_builtin_workflows,
            // Artifact commands
            commands::artifact_commands::get_artifacts,
            commands::artifact_commands::get_artifact,
            commands::artifact_commands::get_artifact_at_version,
            commands::artifact_commands::create_artifact,
            commands::artifact_commands::update_artifact,
            commands::artifact_commands::archive_artifact,
            commands::artifact_commands::get_artifacts_by_bucket,
            commands::artifact_commands::get_artifacts_by_task,
            commands::artifact_commands::get_team_artifacts_by_session,
            commands::artifact_commands::get_artifact_version_history,
            // Bucket commands
            commands::artifact_commands::get_buckets,
            commands::artifact_commands::create_bucket,
            commands::artifact_commands::get_system_buckets,
            // Artifact relation commands
            commands::artifact_commands::add_artifact_relation,
            commands::artifact_commands::get_artifact_relations,
            // Research commands
            commands::research_commands::start_research,
            commands::research_commands::pause_research,
            commands::research_commands::resume_research,
            commands::research_commands::stop_research,
            commands::research_commands::get_research_processes,
            commands::research_commands::get_research_process,
            commands::research_commands::get_research_presets,
            // Methodology commands
            commands::methodology_commands::get_methodologies,
            commands::methodology_commands::get_active_methodology,
            commands::methodology_commands::activate_methodology,
            commands::methodology_commands::deactivate_methodology,
            // Test data commands (for visual audits)
            commands::test_data_commands::seed_test_data,
            commands::test_data_commands::seed_visual_audit_data,
            commands::test_data_commands::clear_test_data,
            // Permission commands
            commands::permission_commands::resolve_permission_request,
            commands::permission_commands::get_pending_permissions,
            // Question commands (AskUserQuestion)
            commands::question_commands::resolve_user_question,
            commands::question_commands::get_pending_questions,
            // Task context commands
            commands::task_context_commands::get_task_context,
            commands::task_context_commands::get_artifact_full,
            commands::task_context_commands::get_artifact_version,
            commands::task_context_commands::get_related_artifacts,
            commands::task_context_commands::search_artifacts,
            // Unified chat commands (new API - consolidates context_chat + execution_chat)
            commands::unified_chat_commands::send_agent_message,
            commands::unified_chat_commands::queue_agent_message,
            commands::unified_chat_commands::get_queued_agent_messages,
            commands::unified_chat_commands::delete_queued_agent_message,
            commands::unified_chat_commands::list_agent_conversations,
            commands::unified_chat_commands::get_agent_conversation,
            commands::unified_chat_commands::create_agent_conversation,
            commands::unified_chat_commands::get_agent_run_status_unified,
            commands::unified_chat_commands::is_chat_service_available,
            commands::unified_chat_commands::stop_agent,
            commands::unified_chat_commands::is_agent_running,
            // Chat attachment commands
            commands::chat_attachment_commands::upload_chat_attachment,
            commands::chat_attachment_commands::link_attachments_to_message,
            commands::chat_attachment_commands::list_conversation_attachments,
            commands::chat_attachment_commands::list_message_attachments,
            commands::chat_attachment_commands::delete_chat_attachment,
            // Activity event commands (pagination, filtering)
            commands::activity_commands::list_task_activity_events,
            commands::activity_commands::list_session_activity_events,
            commands::activity_commands::list_all_activity_events,
            commands::activity_commands::count_task_activity_events,
            commands::activity_commands::count_session_activity_events,
            // Diff commands
            commands::diff_commands::get_task_file_changes,
            commands::diff_commands::get_file_diff,
            commands::diff_commands::get_commit_file_changes,
            commands::diff_commands::get_commit_file_diff,
            commands::diff_commands::detect_merge_conflicts,
            commands::diff_commands::get_conflict_file_diff,
            // Git commands (Phase 66 - Per-task branch isolation)
            commands::git_commands::get_task_commits,
            commands::git_commands::get_task_diff_stats,
            commands::git_commands::resolve_merge_conflict,
            commands::git_commands::retry_merge,
            commands::git_commands::cleanup_task_branch,
            commands::git_commands::change_project_git_mode,
            // Plan branch commands (Phase 85 - Feature branch for plan groups)
            commands::plan_branch_commands::get_plan_branch,
            commands::plan_branch_commands::get_plan_branch_by_task_id,
            commands::plan_branch_commands::get_project_plan_branches,
            commands::plan_branch_commands::enable_feature_branch,
            // Plan commands (Active plan management)
            commands::plan_commands::get_active_plan,
            commands::plan_commands::set_active_plan,
            commands::plan_commands::get_active_execution_plan,
            commands::plan_commands::clear_active_plan,
            commands::plan_commands::list_plan_selector_candidates,
            // Team commands (agent teams collaboration)
            commands::team_commands::create_team,
            commands::team_commands::disband_team,
            commands::team_commands::get_team_status,
            commands::team_commands::send_team_message,
            commands::team_commands::send_teammate_message,
            commands::team_commands::stop_teammate,
            commands::team_commands::stop_team,
            commands::team_commands::get_team_messages,
            commands::team_commands::get_teammate_cost,
            commands::team_commands::get_team_history,
            // API key management commands (Tauri IPC — replaces HTTP fetch in settings UI)
            commands::api_key_commands::list_api_keys,
            commands::api_key_commands::create_api_key,
            commands::api_key_commands::revoke_api_key,
            commands::api_key_commands::rotate_api_key,
            commands::api_key_commands::update_api_key_projects,
            commands::api_key_commands::update_api_key_permissions,
            commands::api_key_commands::get_api_key_audit_log,
            commands::diagnostic_commands::get_agent_health,
            // UI feature flag commands
            commands::ui_commands::get_ui_feature_flags,
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                let app_state = app_handle.state::<AppState>();

                // Set shutdown flag FIRST — before killing agents — so that any stream handlers
                // still in flight can detect the shutdown and skip escalation. SeqCst ensures
                // immediate visibility across all threads.
                let exec_state = app_handle.state::<Arc<commands::ExecutionState>>();
                exec_state.is_shutting_down.store(true, Ordering::SeqCst);

                let registry = Arc::clone(&app_state.running_agent_registry);
                let interactive = Arc::clone(&app_state.interactive_process_registry);
                let db = app_state.db.clone();

                // Step 1: Agent shutdown — capped at 2.5s so MCP + WAL fit within macOS 5s window.
                // If agents are slow, they are abandoned and the OS will clean up.
                tauri::async_runtime::block_on(async {
                    let _ = tokio::time::timeout(Duration::from_millis(2500), async move {
                        let ipr_dump = interactive.dump_state().await;
                        tracing::info!(
                            count = ipr_dump.len(),
                            "[IPR_EXIT_DUMP] IPR entries at shutdown: {:?}",
                            ipr_dump
                        );
                        interactive.clear().await;
                        let stopped = registry.stop_all().await;
                        crate::infrastructure::agents::claude::kill_all_tracked_processes().await;
                        if !stopped.is_empty() {
                            tracing::info!(
                                count = stopped.len(),
                                "Killed running agents on app exit"
                            );
                        }
                    })
                    .await;
                });

                // Step 2: External MCP shutdown — separate OS thread to avoid nested block_on deadlock.
                // Supervisor.shutdown() is internally capped at 2s (SIGTERM → SIGKILL).
                if let Some(supervisor) = app_handle.state::<ExternalMcpHandle>().get() {
                    let supervisor = supervisor.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                            .unwrap();
                        rt.block_on(supervisor.shutdown());
                    })
                    .join()
                    .ok();
                }

                // Step 3: WAL checkpoint — runs last, ~100ms. Total budget: 2.5 + 2 + 0.1 = 4.6s < 5s.
                tauri::async_runtime::block_on(async {
                    let checkpoint_result = db
                        .run(|conn| {
                            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
                                .map_err(|e| {
                                    crate::error::AppError::Database(format!(
                                        "WAL checkpoint failed: {e}"
                                    ))
                                })
                        })
                        .await;
                    if let Err(e) = checkpoint_result {
                        tracing::warn!(error = %e, "WAL checkpoint on exit failed");
                    }
                });
            }
        });
}
