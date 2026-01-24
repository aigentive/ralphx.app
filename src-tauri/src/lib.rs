// RalphX - Autonomous AI-driven development system
// Tauri 2.0 backend with clean architecture

// Core modules
pub mod application;
pub mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod testing;

// Re-export common types
pub use application::AppState;
pub use error::{AppError, AppResult};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create application state with production SQLite repositories
    let app_state = AppState::new_production().expect("Failed to initialize AppState");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::health::health_check,
            commands::task_commands::list_tasks,
            commands::task_commands::get_task,
            commands::task_commands::create_task,
            commands::task_commands::update_task,
            commands::task_commands::delete_task,
            commands::project_commands::list_projects,
            commands::project_commands::get_project,
            commands::project_commands::create_project,
            commands::project_commands::update_project,
            commands::project_commands::delete_project,
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
            commands::review_commands::reject_review
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
