// Migration v52: Clean up stale execution plans and orphaned duplicate tasks
//
// After the accept-plan performance fix (Commits 1-3), we need to:
//
// 1. **Supersede stale execution_plans**: 172 extra plans were created due to the
//    duplicate-apply bug. Only plans referenced by project_active_plan are current.
//    The rest should be superseded.
//
// 2. **Archive orphaned duplicate tasks**: 5 tasks created by the duplicate apply
//    that have no legitimate execution plan. Archived rather than deleted to preserve
//    activity history.
//
// 3. **Clean up dependencies** pointing to/from the archived tasks to prevent
//    orphaned blocker references.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Step 1: Supersede stale execution plans.
    // Keep only plans that are referenced by project_active_plan as 'active'.
    // This handles databases where rapid double-clicks on Accept Plan created
    // multiple execution_plan records for the same session.
    conn.execute(
        "UPDATE execution_plans SET status = 'superseded'
         WHERE status = 'active'
           AND id NOT IN (
               SELECT execution_plan_id FROM project_active_plan
               WHERE execution_plan_id IS NOT NULL
           )",
        [],
    )
    .map_err(|e| {
        AppError::Database(format!(
            "v52: failed to supersede stale execution plans: {}",
            e
        ))
    })?;

    // Step 2: Archive the known orphaned duplicate tasks created by the duplicate-apply bug.
    // These tasks were created in a second apply invocation and are not linked to any
    // active execution plan. We archive rather than delete to preserve history.
    conn.execute(
        "UPDATE tasks SET archived_at = CURRENT_TIMESTAMP
         WHERE id IN ('06acf5a9', '4cf85f47', '2c248946', 'f97cd185', '3285e763')
           AND archived_at IS NULL",
        [],
    )
    .map_err(|e| {
        AppError::Database(format!(
            "v52: failed to archive orphaned duplicate tasks: {}",
            e
        ))
    })?;

    // Step 3: Remove dependencies pointing to or from the archived tasks.
    // This prevents them from acting as phantom blockers in the dependency graph.
    conn.execute(
        "DELETE FROM task_dependencies
         WHERE task_id IN ('06acf5a9', '4cf85f47', '2c248946', 'f97cd185', '3285e763')
            OR depends_on_task_id IN ('06acf5a9', '4cf85f47', '2c248946', 'f97cd185', '3285e763')",
        [],
    )
    .map_err(|e| {
        AppError::Database(format!(
            "v52: failed to clean up orphaned task dependencies: {}",
            e
        ))
    })?;

    tracing::info!("v52: cleaned up stale execution plans and orphaned duplicate tasks");

    Ok(())
}
