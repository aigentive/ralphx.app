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
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

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
use tauri::Manager;
use tracing::{info, warn};

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
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all())
                .build()
        )
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
            let http_state = Arc::new(
                AppState::new_production(app_handle).expect("Failed to initialize AppState for HTTP server"),
            );
            tauri::async_runtime::spawn(async move {
                http_server::start_http_server(http_state).await;
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

            // Register app_state with Tauri's state management
            app.manage(app_state);

            Ok(())
        })
        .manage(execution_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::health::health_check,
            commands::task_commands::list_tasks,
            commands::task_commands::get_task,
            commands::task_commands::create_task,
            commands::task_commands::update_task,
            commands::task_commands::delete_task,
            commands::task_commands::answer_user_question,
            commands::task_commands::inject_task,
            commands::task_commands::move_task,
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
            commands::execution_commands::get_execution_status,
            commands::execution_commands::pause_execution,
            commands::execution_commands::resume_execution,
            commands::execution_commands::stop_execution,
            // Ideation session commands
            commands::ideation_commands::create_ideation_session,
            commands::ideation_commands::get_ideation_session,
            commands::ideation_commands::get_ideation_session_with_data,
            commands::ideation_commands::list_ideation_sessions,
            commands::ideation_commands::archive_ideation_session,
            commands::ideation_commands::delete_ideation_session,
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
            // Context-aware chat commands
            commands::context_chat_commands::send_context_message,
            commands::context_chat_commands::list_conversations,
            commands::context_chat_commands::get_conversation,
            commands::context_chat_commands::create_conversation,
            commands::context_chat_commands::get_agent_run_status,
            // Execution chat commands
            commands::execution_chat_commands::get_execution_conversation,
            commands::execution_chat_commands::list_task_executions,
            commands::execution_chat_commands::queue_execution_message,
            commands::execution_chat_commands::get_queued_execution_messages,
            commands::execution_chat_commands::delete_queued_execution_message,
            // Task context commands
            commands::task_context_commands::get_task_context,
            commands::task_context_commands::get_artifact_full,
            commands::task_context_commands::get_artifact_version,
            commands::task_context_commands::get_related_artifacts,
            commands::task_context_commands::search_artifacts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
