// Migration v32: Fix FK constraint violations on task deletion
//
// This migration adds ON DELETE SET NULL to two foreign keys:
// 1. task_proposals.created_task_id → tasks(id)
// 2. artifacts.task_id → tasks(id)
//
// Both columns are nullable, so SET NULL is correct: deleting a task should
// orphan the linked proposal/artifact, not cascade-delete it.
//
// The tables are recreated using the 4-step SQLite pattern:
// 1. Disable foreign keys
// 2. Create temp table with correct schema
// 3. Copy data from old table
// 4. Drop old table + rename temp + recreate indexes
// 5. Re-enable foreign keys
//
// proposal_dependencies has FKs to task_proposals, so FKs are disabled
// during the recreation to avoid constraint violations.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v32: Fix FK constraints on task_proposals and artifacts
pub fn migrate(conn: &Connection) -> AppResult<()> {
    // Disable foreign keys during the recreation to avoid constraint violations
    // with proposal_dependencies → task_proposals
    conn.execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    // Recreate task_proposals table with ON DELETE SET NULL on created_task_id
    recreate_task_proposals(conn)?;

    // Recreate artifacts table with ON DELETE SET NULL on task_id
    recreate_artifacts(conn)?;

    // Re-enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Recreate task_proposals table with corrected FK constraints
fn recreate_task_proposals(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "-- Create temp table with correct schema (ON DELETE SET NULL on created_task_id)
        CREATE TABLE task_proposals_new (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT,
            category TEXT NOT NULL,
            steps TEXT,
            acceptance_criteria TEXT,
            suggested_priority TEXT NOT NULL,
            priority_score INTEGER NOT NULL DEFAULT 50,
            priority_reason TEXT,
            priority_factors TEXT,
            estimated_complexity TEXT DEFAULT 'moderate',
            user_priority TEXT,
            user_modified INTEGER DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending',
            selected INTEGER DEFAULT 0,
            created_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')),
            plan_artifact_id TEXT,
            plan_version_at_creation INTEGER
        );

        -- Copy data from old table
        INSERT INTO task_proposals_new
        SELECT * FROM task_proposals;

        -- Drop old table
        DROP TABLE task_proposals;

        -- Rename temp table
        ALTER TABLE task_proposals_new RENAME TO task_proposals;

        -- Recreate all indexes
        CREATE INDEX IF NOT EXISTS idx_task_proposals_session_id
            ON task_proposals(session_id);
        CREATE INDEX IF NOT EXISTS idx_task_proposals_sort_order
            ON task_proposals(session_id, sort_order);
        CREATE INDEX IF NOT EXISTS idx_task_proposals_created_task_id
            ON task_proposals(created_task_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Recreate artifacts table with corrected FK constraints
fn recreate_artifacts(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "-- Create temp table with correct schema (ON DELETE SET NULL on task_id)
        CREATE TABLE artifacts_new (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            name TEXT NOT NULL,
            content_type TEXT NOT NULL,
            content_text TEXT,
            content_path TEXT,
            bucket_id TEXT REFERENCES artifact_buckets(id),
            task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
            process_id TEXT,
            created_by TEXT NOT NULL,
            version INTEGER DEFAULT 1,
            previous_version_id TEXT REFERENCES artifacts(id),
            metadata_json TEXT,
            created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
        );

        -- Copy data from old table
        INSERT INTO artifacts_new
        SELECT * FROM artifacts;

        -- Drop old table
        DROP TABLE artifacts;

        -- Rename temp table
        ALTER TABLE artifacts_new RENAME TO artifacts;

        -- Recreate all indexes
        CREATE INDEX IF NOT EXISTS idx_artifacts_bucket
            ON artifacts(bucket_id);
        CREATE INDEX IF NOT EXISTS idx_artifacts_type
            ON artifacts(type);
        CREATE INDEX IF NOT EXISTS idx_artifacts_task
            ON artifacts(task_id);",
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
