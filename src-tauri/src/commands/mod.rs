// Tauri commands - thin layer bridging frontend to backend
// Commands should be minimal - delegate to domain/infrastructure

pub mod agent_profile_commands;
pub mod execution_commands;
pub mod health;
pub mod project_commands;
pub mod qa_commands;
pub mod review_commands;
pub mod task_commands;

// Re-export commands for registration
pub use agent_profile_commands::{
    get_agent_profile, get_agent_profiles_by_role, get_builtin_agent_profiles,
    get_custom_agent_profiles, list_agent_profiles, seed_builtin_profiles,
};
pub use execution_commands::{
    get_execution_status, pause_execution, resume_execution, stop_execution, ExecutionState,
};
pub use health::health_check;
pub use project_commands::{
    create_project, delete_project, get_project, list_projects, update_project,
};
pub use qa_commands::{
    get_qa_results, get_qa_settings, get_task_qa, retry_qa, skip_qa, update_qa_settings,
};
pub use review_commands::{
    approve_fix_task, approve_review, get_fix_task_attempts, get_pending_reviews,
    get_review_by_id, get_reviews_by_task_id, get_task_state_history, reject_fix_task,
    reject_review, request_changes,
};
pub use task_commands::{answer_user_question, create_task, delete_task, get_task, inject_task, list_tasks, update_task};
