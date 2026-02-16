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

// Re-export common types
pub use application::AppState;
pub use error::{AppError, AppResult};

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use application::{
    ChatResumptionRunner, ReconciliationRunner, StartupJobRunner, TaskSchedulerService,
    TaskTransitionService,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing subscriber so tracing macros produce visible output.
    // Respects RUST_LOG env var; defaults to ralphx=info plus warn for everything else.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("ralphx=info,warn")),
        )
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
            let app_state =
                AppState::new_production(app_handle.clone()).expect("Failed to initialize AppState");

            // Load execution settings from database and apply to ExecutionState
            // This must happen before HTTP server starts to ensure consistent configuration
            let init_settings_repo = Arc::clone(&app_state.execution_settings_repo);
            let init_global_settings_repo = Arc::clone(&app_state.global_execution_settings_repo);
            tauri::async_runtime::block_on(async move {
                // Load per-project default settings (project_id = None)
                match init_settings_repo.get_settings(None).await {
                    Ok(settings) => {
                        init_execution_state.set_max_concurrent(settings.max_concurrent_tasks);
                        info!(
                            "Initialized execution settings from database: max_concurrent={}",
                            settings.max_concurrent_tasks
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load execution settings from database, using defaults: {}",
                            e
                        );
                    }
                }

                // Phase 82: Load global execution settings (global_max_concurrent cap)
                match init_global_settings_repo.get_settings().await {
                    Ok(global_settings) => {
                        init_execution_state.set_global_max_concurrent(global_settings.global_max_concurrent);
                        info!(
                            "Initialized global execution settings from database: global_max_concurrent={}",
                            global_settings.global_max_concurrent
                        );
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load global execution settings from database, using defaults: {}",
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

            // Start HTTP server for MCP proxy on port 3847
            // Create a second AppState for HTTP server with its own DB connection,
            // but share in-memory state (question_state, permission_state, message_queue)
            // so MCP handlers and Tauri commands operate on the same data.
            let shared_question_state = Arc::clone(&app_state.question_state);
            let shared_permission_state = Arc::clone(&app_state.permission_state);
            let shared_message_queue = Arc::clone(&app_state.message_queue);
            let mut http_app_state_inner =
                AppState::new_production(app_handle).expect("Failed to initialize AppState for HTTP server");
            http_app_state_inner.question_state = shared_question_state;
            http_app_state_inner.permission_state = shared_permission_state;
            http_app_state_inner.message_queue = shared_message_queue;
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
            let startup_spawn_orchestrator_job_repo = Arc::clone(&app_state.spawn_orchestrator_job_repo);
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
                ));
                scheduler_concrete.set_self_ref(Arc::clone(&scheduler_concrete) as Arc<dyn domain::state_machine::services::TaskScheduler>);
                let task_scheduler: Arc<dyn domain::state_machine::services::TaskScheduler> = scheduler_concrete;

                // Clone repos for ChatResumptionRunner before they're consumed by TaskTransitionService/StartupJobRunner
                let chat_resumption_agent_run_repo = Arc::clone(&startup_agent_run_repo);
                let chat_resumption_task_repo = Arc::clone(&startup_task_repo);
                let chat_resumption_task_dependency_repo = Arc::clone(&startup_task_dependency_repo);
                let chat_resumption_project_repo = Arc::clone(&startup_project_repo);
                let chat_resumption_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let chat_resumption_chat_attachment_repo = Arc::clone(&startup_chat_attachment_repo);
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
                let reconcile_app_handle = startup_app_handle.clone();

                // Clone task_dependency_repo for StartupJobRunner (before TaskTransitionService consumes it)
                let startup_runner_task_dep_repo = Arc::clone(&startup_task_dependency_repo);
                let startup_runner_app_handle = startup_app_handle.clone();

                // Create TaskTransitionService for startup resumption
                let transition_service = Arc::new(TaskTransitionService::new(
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
                .with_step_repo(Arc::clone(&startup_step_repo)));

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
                    startup_execution_settings_repo,
                )
                .with_task_scheduler(Arc::clone(&task_scheduler))
                .with_app_handle(startup_runner_app_handle);

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
                    chat_resumption_project_repo,
                    chat_resumption_ideation_session_repo,
                    chat_resumption_activity_event_repo,
                    chat_resumption_message_queue,
                    chat_resumption_running_agent_registry,
                    chat_resumption_memory_event_repo,
                    Arc::clone(&startup_execution_state),
                )
                .with_app_handle(chat_resumption_app_handle)
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo));

                chat_resumption.run().await;

                let reconcile_transition_service = Arc::new(TaskTransitionService::new(
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
                    .with_step_repo(Arc::clone(&startup_step_repo)));

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
                .with_plan_branch_repo(Arc::clone(&startup_plan_branch_repo));

                reconcile_runner.reconcile_stuck_tasks().await;

                tauri::async_runtime::spawn(async move {
                    let interval = Duration::from_secs(30);
                    loop {
                        tokio::time::sleep(interval).await;
                        reconcile_runner.reconcile_stuck_tasks().await;
                    }
                });

                // Spawn memory archive job background processing loop
                // Clone required repositories for the archive job processor
                let archive_job_memory_archive_repo = Arc::clone(&startup_memory_archive_repo);
                let archive_job_memory_entry_repo = Arc::clone(&startup_memory_entry_repo);
                let archive_job_project_repo = Arc::clone(&startup_project_repo);

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

                // Spawn orchestrator job background processing loop
                // Processes jobs that spawn orchestrator-ideation agents for child sessions
                let spawn_job_repo = Arc::clone(&startup_spawn_orchestrator_job_repo);

                // Find plugin directory and project root for orchestrator spawning
                let spawn_plugin_dir = infrastructure::agents::claude::find_plugin_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("ralphx-plugin"));
                let spawn_project_root = std::path::PathBuf::from(".");

                tauri::async_runtime::spawn(async move {
                    let claude_client = Arc::new(infrastructure::ClaudeCodeClient::new());
                    let worker = application::SpawnOrchestratorWorker::new(
                        spawn_job_repo,
                        claude_client,
                        spawn_plugin_dir,
                        spawn_project_root,
                    );

                    info!("Starting spawn orchestrator worker loop...");
                    application::run_worker_loop(
                        Arc::new(worker),
                        Duration::from_secs(5),
                    ).await;
                });
            });

            // Clone team repos before app_state is moved into Tauri state
            let team_session_repo = Arc::clone(&app_state.team_session_repo);
            let team_message_repo = Arc::clone(&app_state.team_message_repo);

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
            commands::task_commands::mutation::delete_task,
            commands::task_commands::mutation::answer_user_question,
            commands::task_commands::mutation::inject_task,
            commands::task_commands::mutation::move_task,
            commands::task_commands::mutation::archive_task,
            commands::task_commands::mutation::restore_task,
            commands::task_commands::mutation::permanently_delete_task,
            commands::task_commands::mutation::block_task,
            commands::task_commands::mutation::unblock_task,
            commands::task_commands::mutation::cleanup_task,
            commands::task_commands::mutation::cleanup_tasks_in_group,
            commands::task_commands::mutation::cancel_tasks_in_group,
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
            commands::task_step_commands::delete_task_step,
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
            commands::project_commands::delete_project,
            commands::project_commands::get_git_branches,
            commands::project_commands::get_git_default_branch,
            commands::project_commands::reanalyze_project,
            commands::project_commands::update_custom_analysis,
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
            // Ideation session commands
            commands::ideation_commands::create_ideation_session,
            commands::ideation_commands::get_ideation_session,
            commands::ideation_commands::get_ideation_session_with_data,
            commands::ideation_commands::list_ideation_sessions,
            commands::ideation_commands::archive_ideation_session,
            commands::ideation_commands::delete_ideation_session,
            commands::ideation_commands::reopen_ideation_session,
            commands::ideation_commands::update_ideation_session_title,
            commands::ideation_commands::spawn_session_namer,
            commands::ideation_commands::spawn_dependency_suggester,
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
            commands::ideation_commands::add_proposal_dependency,
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
            // Workflow commands
            commands::workflow_commands::get_workflows,
            commands::workflow_commands::get_workflow,
            commands::workflow_commands::create_workflow,
            commands::workflow_commands::update_workflow,
            commands::workflow_commands::delete_workflow,
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
            commands::artifact_commands::delete_artifact,
            commands::artifact_commands::get_artifacts_by_bucket,
            commands::artifact_commands::get_artifacts_by_task,
            commands::artifact_commands::get_team_artifacts_by_session,
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
            // Git commands (Phase 66 - Per-task branch isolation)
            commands::git_commands::get_task_commits,
            commands::git_commands::get_task_diff_stats,
            commands::git_commands::resolve_merge_conflict,
            commands::git_commands::retry_merge,
            commands::git_commands::cleanup_task_branch,
            commands::git_commands::change_project_git_mode,
            // Plan branch commands (Phase 85 - Feature branch for plan groups)
            commands::plan_branch_commands::get_plan_branch,
            commands::plan_branch_commands::get_project_plan_branches,
            commands::plan_branch_commands::enable_feature_branch,
            commands::plan_branch_commands::disable_feature_branch,
            commands::plan_branch_commands::update_project_feature_branch_setting,
            // Plan commands (Active plan management)
            commands::plan_commands::get_active_plan,
            commands::plan_commands::set_active_plan,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
