// Type definitions for task_commands module

use serde::{Deserialize, Serialize};
use crate::domain::entities::Task;

/// Input for creating a new task
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInput {
    pub project_id: String,
    pub title: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub steps: Option<Vec<String>>,
}

/// Input for updating a task
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i32>,
    pub internal_status: Option<String>,
}

/// Input for answering an agent's question
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerUserQuestionInput {
    pub task_id: String,
    pub selected_options: Vec<String>,
    #[serde(default)]
    pub custom_response: Option<String>,
}

/// Response for the answer_user_question command
#[derive(Debug, Serialize)]
pub struct AnswerUserQuestionResponse {
    pub task_id: String,
    pub resumed_status: String,
    pub answer_recorded: bool,
}

/// Input for injecting a task mid-loop
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectTaskInput {
    /// The project ID to inject the task into
    pub project_id: String,
    /// Title of the task
    pub title: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Category (defaults to "feature")
    #[serde(default)]
    pub category: Option<String>,
    /// Where to inject: "backlog" (deferred) or "planned" (immediate queue)
    #[serde(default = "super::helpers::default_target")]
    pub target: String,
    /// If true and target is "planned", make this task the highest priority
    #[serde(default)]
    pub make_next: bool,
}

/// Response for the inject_task command
#[derive(Debug, Serialize)]
pub struct InjectTaskResponse {
    pub task: TaskResponse,
    pub target: String,
    pub priority: i32,
    pub make_next_applied: bool,
}

/// Response wrapper for task operations
#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub project_id: String,
    pub category: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: i32,
    pub internal_status: String,
    pub needs_review_point: bool,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub archived_at: Option<String>,
    pub blocked_reason: Option<String>,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id.as_str().to_string(),
            project_id: task.project_id.as_str().to_string(),
            category: task.category,
            title: task.title,
            description: task.description,
            priority: task.priority,
            internal_status: task.internal_status.as_str().to_string(),
            needs_review_point: task.needs_review_point,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            started_at: task.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: task.completed_at.map(|dt| dt.to_rfc3339()),
            archived_at: task.archived_at.map(|dt| dt.to_rfc3339()),
            blocked_reason: task.blocked_reason,
        }
    }
}

/// Response for paginated task list
#[derive(Debug, Serialize)]
pub struct TaskListResponse {
    pub tasks: Vec<TaskResponse>,
    pub total: u32,
    pub has_more: bool,
    pub offset: u32,
}

/// Response for status transition options
#[derive(Debug, Serialize)]
pub struct StatusTransition {
    /// The internal status string (e.g., "ready", "cancelled")
    pub status: String,
    /// User-friendly label for the UI (e.g., "Ready for Work", "Cancel")
    pub label: String,
}
