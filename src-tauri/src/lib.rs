// RalphX - Autonomous AI-driven development system
// Tauri 2.0 backend with clean architecture

// Core modules
pub mod application;
pub mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;

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
        .invoke_handler(tauri::generate_handler![greet, commands::health::health_check])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
