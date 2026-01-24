// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod health;
pub mod project_commands;
pub mod task_commands;

// Re-export commands for registration
pub use health::health_check;
pub use project_commands::{create_project, delete_project, get_project, list_projects, update_project};
pub use task_commands::{create_task, delete_task, get_task, list_tasks, update_task};
