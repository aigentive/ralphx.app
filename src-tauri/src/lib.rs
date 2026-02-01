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

use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tracing::{info, warn};

use application::{ChatResumptionRunner, StartupJobRunner, TaskSchedulerService, TaskTransitionService};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create execution state for global execution control
    let execution_state = Arc::new(commands::ExecutionState::new());

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

            // Start HTTP server for MCP proxy on port 3847
            // Create a second AppState for HTTP server (repos are Arc'd so this is efficient)
            let http_app_state = Arc::new(
                AppState::new_production(app_handle).expect("Failed to initialize AppState for HTTP server"),
            );
            // Clone execution_state from Tauri state for HTTP server
            let http_execution_state = app.state::<Arc<commands::ExecutionState>>().inner().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = http_server::start_http_server(http_app_state, http_execution_state).await {
                    tracing::error!("HTTP server failed: {}", e);
                }
            });

            // Register RalphX MCP server with Claude Code CLI
            // This ensures the MCP tools are available regardless of user's working directory
            if let (Some(cli_path), Some(plugin_dir)) = (
                infrastructure::agents::claude::find_claude_cli(),
                infrastructure::agents::claude::find_plugin_dir(),
            ) {
                info!("Registering RalphX MCP server...");
                tauri::async_runtime::spawn(async move {
                    match infrastructure::agents::claude::register_mcp_server(&cli_path, &plugin_dir).await {
                        Ok(()) => info!("RalphX MCP server registered successfully"),
                        Err(e) => warn!("Failed to register RalphX MCP server: {}", e),
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
            let startup_chat_message_repo = Arc::clone(&app_state.chat_message_repo);
            let startup_conversation_repo = Arc::clone(&app_state.chat_conversation_repo);
            let startup_agent_run_repo = Arc::clone(&app_state.agent_run_repo);
            let startup_ideation_session_repo = Arc::clone(&app_state.ideation_session_repo);
            let startup_activity_event_repo = Arc::clone(&app_state.activity_event_repo);
            let startup_message_queue = Arc::clone(&app_state.message_queue);
            let startup_running_agent_registry = Arc::clone(&app_state.running_agent_registry);
            let startup_execution_state = app.state::<Arc<commands::ExecutionState>>().inner().clone();
            // Clone app handle to enable event emission in startup tasks
            let startup_app_handle = app.handle().clone();

            tauri::async_runtime::spawn(async move {
                // Wait for HTTP server to be ready
                tokio::time::sleep(Duration::from_millis(500)).await;

                info!("Starting startup job runner...");

                // Create TaskSchedulerService for auto-scheduling Ready tasks
                let task_scheduler: Arc<dyn domain::state_machine::services::TaskScheduler> =
                    Arc::new(TaskSchedulerService::<tauri::Wry>::new(
                        Arc::clone(&startup_execution_state),
                        startup_project_repo.clone(),
                        startup_task_repo.clone(),
                        startup_task_dependency_repo.clone(),
                        startup_chat_message_repo.clone(),
                        startup_conversation_repo.clone(),
                        startup_agent_run_repo.clone(),
                        startup_ideation_session_repo.clone(),
                        startup_activity_event_repo.clone(),
                        startup_message_queue.clone(),
                        startup_running_agent_registry.clone(),
                        Some(startup_app_handle.clone()),
                    ));

                // Clone repos for ChatResumptionRunner before they're consumed by TaskTransitionService/StartupJobRunner
                let chat_resumption_agent_run_repo = Arc::clone(&startup_agent_run_repo);
                let chat_resumption_task_repo = Arc::clone(&startup_task_repo);
                let chat_resumption_task_dependency_repo = Arc::clone(&startup_task_dependency_repo);
                let chat_resumption_project_repo = Arc::clone(&startup_project_repo);
                let chat_resumption_chat_message_repo = Arc::clone(&startup_chat_message_repo);
                let chat_resumption_conversation_repo = Arc::clone(&startup_conversation_repo);
                let chat_resumption_ideation_session_repo = Arc::clone(&startup_ideation_session_repo);
                let chat_resumption_activity_event_repo = Arc::clone(&startup_activity_event_repo);
                let chat_resumption_message_queue = Arc::clone(&startup_message_queue);
                let chat_resumption_running_agent_registry = Arc::clone(&startup_running_agent_registry);
                let chat_resumption_app_handle = startup_app_handle.clone();

                // Create TaskTransitionService for startup resumption
                let transition_service: TaskTransitionService<tauri::Wry> = TaskTransitionService::new(
                    startup_task_repo.clone(),
                    startup_task_dependency_repo,
                    startup_project_repo.clone(),
                    startup_chat_message_repo,
                    startup_conversation_repo,
                    startup_agent_run_repo.clone(),
                    startup_ideation_session_repo,
                    startup_activity_event_repo,
                    startup_message_queue,
                    startup_running_agent_registry,
                    Arc::clone(&startup_execution_state),
                    Some(startup_app_handle),
                )
                .with_task_scheduler(Arc::clone(&task_scheduler));

                let runner = StartupJobRunner::new(
                    startup_task_repo,
                    startup_project_repo,
                    startup_agent_run_repo,
                    transition_service,
                    Arc::clone(&startup_execution_state),
                )
                .with_task_scheduler(task_scheduler);

                runner.run().await;

                // Resume interrupted chat conversations (Ideation, Task, Project, TaskExecution, Review)
                // This runs after StartupJobRunner to avoid duplicate resumption of task-based chats
                info!("Starting chat resumption runner...");
                let chat_resumption = ChatResumptionRunner::<tauri::Wry>::new(
                    chat_resumption_agent_run_repo,
                    chat_resumption_conversation_repo,
                    chat_resumption_task_repo,
                    chat_resumption_task_dependency_repo,
                    chat_resumption_chat_message_repo,
                    chat_resumption_project_repo,
                    chat_resumption_ideation_session_repo,
                    chat_resumption_activity_event_repo,
                    chat_resumption_message_queue,
                    chat_resumption_running_agent_registry,
                    Arc::clone(&startup_execution_state),
                )
                .with_app_handle(chat_resumption_app_handle);

                chat_resumption.run().await;
            });

            // Register app_state with Tauri's state management
            app.manage(app_state);

            Ok(())
        })
        .manage(execution_state)
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
            commands::task_commands::query::get_archived_count,
            commands::task_commands::query::search_tasks,
            commands::task_commands::query::get_valid_transitions,
            commands::task_commands::query::get_tasks_awaiting_review,
            commands::task_commands::query::get_task_state_transitions,
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
            commands::execution_commands::set_max_concurrent,
            // Ideation session commands
            commands::ideation_commands::create_ideation_session,
            commands::ideation_commands::get_ideation_session,
            commands::ideation_commands::get_ideation_session_with_data,
            commands::ideation_commands::list_ideation_sessions,
            commands::ideation_commands::archive_ideation_session,
            commands::ideation_commands::delete_ideation_session,
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
            // Activity event commands (pagination, filtering)
            commands::activity_commands::list_task_activity_events,
            commands::activity_commands::list_session_activity_events,
            commands::activity_commands::list_all_activity_events,
            commands::activity_commands::count_task_activity_events,
            commands::activity_commands::count_session_activity_events,
            // Diff commands
            commands::diff_commands::get_task_file_changes,
            commands::diff_commands::get_file_diff
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
