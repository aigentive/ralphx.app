// SQL query constants for task repository operations

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

/// Delete task query
pub(super) const DELETE_TASK: &str = "DELETE FROM tasks WHERE id = ?1";

/// Get the oldest Ready task across all projects (Phase 26 - Auto-Scheduler)
/// Returns the task with earliest created_at that is in Ready status and not archived
pub(super) const GET_OLDEST_READY_TASK: &str =
    "SELECT id, project_id, category, title, description, priority, internal_status, needs_review_point, source_proposal_id, plan_artifact_id, created_at, updated_at, started_at, completed_at, archived_at
     FROM tasks
     WHERE internal_status = 'ready'
       AND archived_at IS NULL
     ORDER BY created_at ASC
     LIMIT 1";
