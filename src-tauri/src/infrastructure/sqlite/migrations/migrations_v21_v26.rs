// Database migrations v21-v26
// Phase implementations: ideation plans, task CRUD, execution, review, and ideation improvements

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v21: Phase 16 - Ideation plan artifacts and settings
pub(super) fn migrate_v21(conn: &Connection) -> AppResult<()> {
    // ============================================================================
    // Phase 16: Ideation Plan Artifacts
    // Add plan artifact fields to ideation entities and create ideation settings
    // ============================================================================

    // Add plan_artifact_id to ideation_sessions (single plan per session)
    conn.execute(
        "ALTER TABLE ideation_sessions ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add plan fields to task_proposals (with version tracking)
    conn.execute(
        "ALTER TABLE task_proposals ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "ALTER TABLE task_proposals ADD COLUMN plan_version_at_creation INTEGER",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create ideation_settings table with single-row pattern
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ideation_settings (
            id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            plan_mode TEXT NOT NULL DEFAULT 'optional',
            require_plan_approval INTEGER NOT NULL DEFAULT 0,
            suggest_plans_for_complex INTEGER NOT NULL DEFAULT 1,
            auto_link_proposals INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Seed default settings row
    conn.execute(
        "INSERT OR IGNORE INTO ideation_settings (id, updated_at) VALUES (1, datetime('now'))",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add traceability fields to tasks (for worker context access)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN source_proposal_id TEXT REFERENCES task_proposals(id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "ALTER TABLE tasks ADD COLUMN plan_artifact_id TEXT REFERENCES artifacts(id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v22: Phase 18 - Task archive support
pub(super) fn migrate_v22(conn: &Connection) -> AppResult<()> {
    // ============================================================================
    // Phase 18: Task CRUD, Archive & Search
    // Add archived_at field to tasks table for soft delete functionality
    // ============================================================================

    // Add archived_at column
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN archived_at TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index for archived tasks lookup
    conn.execute(
        "CREATE INDEX idx_tasks_archived ON tasks(project_id, archived_at)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v23: Phase 19 - Task steps for deterministic execution
pub(super) fn migrate_v23(conn: &Connection) -> AppResult<()> {
    // ============================================================================
    // Phase 19: Task Execution Experience
    // Add task_steps table for deterministic progress tracking
    // ============================================================================

    // Create task_steps table
    conn.execute(
        "CREATE TABLE task_steps (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            sort_order INTEGER NOT NULL DEFAULT 0,
            depends_on TEXT REFERENCES task_steps(id) ON DELETE SET NULL,
            created_by TEXT NOT NULL DEFAULT 'user',
            completion_note TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index for task lookup
    conn.execute(
        "CREATE INDEX idx_task_steps_task_id ON task_steps(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create composite index for ordered retrieval
    conn.execute(
        "CREATE INDEX idx_task_steps_task_order ON task_steps(task_id, sort_order)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v24: Add content_blocks column to chat_messages
///
/// Content blocks preserve the order of text and tool calls in a message,
/// enabling proper interleaved rendering instead of concatenated content.
pub(super) fn migrate_v24(conn: &Connection) -> AppResult<()> {
    // Add content_blocks column for storing interleaved text and tool call blocks
    conn.execute(
        "ALTER TABLE chat_messages ADD COLUMN content_blocks TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v25: Add review_settings table
///
/// Review settings control the review system behavior including max revision cycles.
/// Single-row table (id=1) following the pattern of ideation_settings.
pub(super) fn migrate_v25(conn: &Connection) -> AppResult<()> {
    // Create review_settings table
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS review_settings (
            id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            ai_review_enabled INTEGER NOT NULL DEFAULT 1,
            ai_review_auto_fix INTEGER NOT NULL DEFAULT 1,
            require_fix_approval INTEGER NOT NULL DEFAULT 0,
            require_human_review INTEGER NOT NULL DEFAULT 0,
            max_fix_attempts INTEGER NOT NULL DEFAULT 3,
            max_revision_cycles INTEGER NOT NULL DEFAULT 5,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Seed default settings row
    conn.execute(
        "INSERT OR IGNORE INTO review_settings (id, updated_at) VALUES (1, datetime('now'))",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v26: Phase 25 - Ideation session seeding
pub(super) fn migrate_v26(conn: &Connection) -> AppResult<()> {
    // ============================================================================
    // Phase 25: Ideation UI Improvements
    // Add seed_task_id to ideation_sessions for seeding sessions from draft tasks
    // ============================================================================

    conn.execute(
        "ALTER TABLE ideation_sessions ADD COLUMN seed_task_id TEXT REFERENCES tasks(id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}
