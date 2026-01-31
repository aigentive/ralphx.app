// Tauri commands for Task CRUD operations
// Modular structure: types, helpers, query (read), mutation (write), tests

pub mod types;
pub mod helpers;
pub mod query;
pub mod mutation;

#[cfg(test)]
mod tests;

// Re-export types
pub use types::{
    CreateTaskInput,
    UpdateTaskInput,
    AnswerUserQuestionInput,
    AnswerUserQuestionResponse,
    InjectTaskInput,
    InjectTaskResponse,
    TaskResponse,
    TaskListResponse,
    StatusTransition,
};

// Re-export helpers (for use by other command modules)
pub use helpers::{
    default_target,
    emit_queue_changed,
    emit_task_lifecycle_event,
    status_to_label,
};

// Re-export query commands
pub use query::{
    list_tasks,
    get_task,
    get_archived_count,
    search_tasks,
    get_valid_transitions,
    get_tasks_awaiting_review,
};

// Re-export mutation commands
pub use mutation::{
    create_task,
    update_task,
    delete_task,
    move_task,
    inject_task,
    answer_user_question,
    archive_task,
    restore_task,
    permanently_delete_task,
};
