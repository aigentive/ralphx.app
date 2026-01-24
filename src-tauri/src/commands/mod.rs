// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod health;
pub mod task_commands;

// Re-export commands for registration
pub use health::health_check;
pub use task_commands::{create_task, delete_task, get_task, list_tasks, update_task};
