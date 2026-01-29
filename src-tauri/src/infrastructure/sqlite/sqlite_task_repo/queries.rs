// SQL query constants for task repository operations
// Some constants are defined for future use to maintain consistency

/// Standard task SELECT columns
pub(super) const TASK_COLUMNS: &str =
    "id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at";

/// Get task by ID
pub(super) const GET_BY_ID: &str =
    "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
     FROM tasks WHERE id = ?1";

/// Get tasks by project (sorted by priority and creation date)
pub(super) const GET_BY_PROJECT: &str =
    "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
     FROM tasks WHERE project_id = ?1
     ORDER BY priority DESC, created_at ASC";

/// Update task query (for future use)
#[allow(dead_code)]
pub(super) const UPDATE_TASK: &str =
    "UPDATE tasks SET
         project_id = ?1,
         category = ?2,
         title = ?3,
         description = ?4,
         priority = ?5,
         internal_status = ?6,
         needs_review_point = ?7,
         source_proposal_id = ?8,
         plan_artifact_id = ?9,
         updated_at = ?10,
         started_at = ?11,
         completed_at = ?12,
         archived_at = ?13
     WHERE id = ?14";

/// Delete task query
pub(super) const DELETE_TASK: &str = "DELETE FROM tasks WHERE id = ?1";

/// Insert status history (for future use)
#[allow(dead_code)]
pub(super) const INSERT_STATUS_HISTORY: &str =
    "INSERT INTO task_status_history (id, task_id, from_status, to_status, changed_at)
     VALUES (?1, ?2, ?3, ?4, ?5)";

/// Get status history for a task (for future use)
#[allow(dead_code)]
pub(super) const GET_STATUS_HISTORY: &str =
    "SELECT id, task_id, from_status, to_status, changed_at
     FROM task_status_history
     WHERE task_id = ?1
     ORDER BY changed_at ASC";
