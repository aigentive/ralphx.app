// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod agent_profile_commands;
pub mod health;
pub mod project_commands;
pub mod qa_commands;
pub mod task_commands;

// Re-export commands for registration
pub use agent_profile_commands::{
    get_agent_profile, get_agent_profiles_by_role, get_builtin_agent_profiles,
    get_custom_agent_profiles, list_agent_profiles, seed_builtin_profiles,
};
pub use health::health_check;
pub use project_commands::{
    create_project, delete_project, get_project, list_projects, update_project,
};
pub use qa_commands::{
    get_qa_results, get_qa_settings, get_task_qa, retry_qa, skip_qa, update_qa_settings,
};
pub use task_commands::{create_task, delete_task, get_task, list_tasks, update_task};
