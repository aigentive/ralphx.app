// Types for TaskStep commands
// Extracted from task_step_commands.rs to reduce file size

use serde::{Deserialize, Serialize};

use crate::domain::entities::TaskStep;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a new task step
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskStepInput {
    pub title: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

/// Input for updating a task step
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStepInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

// ============================================================================
// Response Types
// ============================================================================

/// Response wrapper for task step operations
#[derive(Debug, Serialize)]
pub struct TaskStepResponse {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub depends_on: Option<String>,
    pub created_by: String,
    pub completion_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

impl From<TaskStep> for TaskStepResponse {
    fn from(step: TaskStep) -> Self {
        Self {
            id: step.id.as_str().to_string(),
            task_id: step.task_id.as_str().to_string(),
            title: step.title,
            description: step.description,
            status: step.status.to_db_string().to_string(),
            sort_order: step.sort_order,
            depends_on: step.depends_on.map(|id| id.as_str().to_string()),
            created_by: step.created_by,
            completion_note: step.completion_note,
            created_at: step.created_at.to_rfc3339(),
            updated_at: step.updated_at.to_rfc3339(),
            started_at: step.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: step.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}
