// RalphX - Autonomous AI-driven development system
// Tauri 2.0 backend with clean architecture

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
        .setup(|app| {
            let app_handle = app.handle().clone();

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
            commands::permission_commands::get_pending_permissions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
