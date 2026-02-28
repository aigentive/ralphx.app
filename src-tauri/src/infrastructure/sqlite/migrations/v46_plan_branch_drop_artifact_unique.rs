// Migration v46: Drop UNIQUE constraint on plan_branches.plan_artifact_id
//
// Multiple ideation sessions can share the same plan_artifact_id (parent/child sessions).
// Each session needs its own feature branch, so plan_artifact_id must not be UNIQUE.
// The session_id UNIQUE constraint (from v16) is preserved — one branch per session.
//
// Uses the 4-step SQLite table recreation pattern since ALTER TABLE cannot drop constraints.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute_batch(
        "-- Safety: drop partial leftover from interrupted previous run
        DROP TABLE IF EXISTS plan_branches_new;

        -- Recreate plan_branches without UNIQUE on plan_artifact_id
        CREATE TABLE plan_branches_new (
            id TEXT PRIMARY KEY,
            plan_artifact_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            project_id TEXT NOT NULL,
            branch_name TEXT NOT NULL,
            source_branch TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            merge_task_id TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            merged_at TEXT
        );

        -- Copy existing data
        INSERT INTO plan_branches_new
        SELECT * FROM plan_branches;

        -- Drop old table (also drops inline UNIQUE constraint)
        DROP TABLE plan_branches;

        -- Rename new table
        ALTER TABLE plan_branches_new RENAME TO plan_branches;

        -- Recreate UNIQUE index on session_id (from v16)
        CREATE UNIQUE INDEX idx_plan_branches_session_id
            ON plan_branches(session_id);

        -- Non-unique index on plan_artifact_id for lookup performance
        CREATE INDEX idx_plan_branches_plan_artifact_id
            ON plan_branches(plan_artifact_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
