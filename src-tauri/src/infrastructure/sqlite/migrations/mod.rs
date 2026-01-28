// Database migrations for SQLite
// Creates and updates schema as needed

// Allow items after test module - migrations are defined after tests for readability
#![allow(clippy::items_after_test_module)]

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Current schema version
pub const SCHEMA_VERSION: i32 = 25;

/// Run all pending migrations on the database
pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    // Create migrations table if it doesn't exist
    create_migrations_table(conn)?;

    // Get current version
    let current_version = get_schema_version(conn)?;

    // Run migrations sequentially
    if current_version < 1 {
        migrate_v1(conn)?;
        set_schema_version(conn, 1)?;
    }

    if current_version < 2 {
        migrate_v2(conn)?;
        set_schema_version(conn, 2)?;
    }

    if current_version < 3 {
        migrate_v3(conn)?;
        set_schema_version(conn, 3)?;
    }

    if current_version < 4 {
        migrate_v4(conn)?;
        set_schema_version(conn, 4)?;
    }

    if current_version < 5 {
        migrate_v5(conn)?;
        set_schema_version(conn, 5)?;
    }

    if current_version < 6 {
        migrate_v6(conn)?;
        set_schema_version(conn, 6)?;
    }

    if current_version < 7 {
        migrate_v7(conn)?;
        set_schema_version(conn, 7)?;
    }

    if current_version < 8 {
        migrate_v8(conn)?;
        set_schema_version(conn, 8)?;
    }

    if current_version < 9 {
        migrate_v9(conn)?;
        set_schema_version(conn, 9)?;
    }

    if current_version < 10 {
        migrate_v10(conn)?;
        set_schema_version(conn, 10)?;
    }

    if current_version < 11 {
        migrate_v11(conn)?;
        set_schema_version(conn, 11)?;
    }

    if current_version < 12 {
        migrate_v12(conn)?;
        set_schema_version(conn, 12)?;
    }

    if current_version < 13 {
        migrate_v13(conn)?;
        set_schema_version(conn, 13)?;
    }

    if current_version < 14 {
        migrate_v14(conn)?;
        set_schema_version(conn, 14)?;
    }

    if current_version < 15 {
        migrate_v15(conn)?;
        set_schema_version(conn, 15)?;
    }

    if current_version < 16 {
        migrate_v16(conn)?;
        set_schema_version(conn, 16)?;
    }

    if current_version < 17 {
        migrate_v17(conn)?;
        set_schema_version(conn, 17)?;
    }

    if current_version < 18 {
        migrate_v18(conn)?;
        set_schema_version(conn, 18)?;
    }

    if current_version < 19 {
        migrate_v19(conn)?;
        set_schema_version(conn, 19)?;
    }

    if current_version < 20 {
        migrate_v20(conn)?;
        set_schema_version(conn, 20)?;
    }

    if current_version < 21 {
        migrate_v21(conn)?;
        set_schema_version(conn, 21)?;
    }

    if current_version < 22 {
        migrate_v22(conn)?;
        set_schema_version(conn, 22)?;
    }

    if current_version < 23 {
        migrate_v23(conn)?;
        set_schema_version(conn, 23)?;
    }

    if current_version < 24 {
        migrate_v24(conn)?;
        set_schema_version(conn, 24)?;
    }

    if current_version < 25 {
        migrate_v25(conn)?;
        set_schema_version(conn, 25)?;
    }

    Ok(())
}

/// Create the migrations tracking table
fn create_migrations_table(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Get the current schema version
fn get_schema_version(conn: &Connection) -> AppResult<i32> {
    let result: Result<i32, _> = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    );

    result.map_err(|e| AppError::Database(e.to_string()))
}

/// Set the schema version after a migration
fn set_schema_version(conn: &Connection, version: i32) -> AppResult<()> {
    conn.execute(
        "INSERT INTO schema_migrations (version) VALUES (?1)",
        [version],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}

/// Migration v1: Create core tables (projects, tasks, task_state_history)
fn migrate_v1(conn: &Connection) -> AppResult<()> {
    // Projects table
    conn.execute(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            working_directory TEXT NOT NULL,
            git_mode TEXT NOT NULL DEFAULT 'local',
            worktree_path TEXT,
            worktree_branch TEXT,
            base_branch TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Tasks table
    conn.execute(
        "CREATE TABLE tasks (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id),
            category TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            priority INTEGER DEFAULT 0,
            internal_status TEXT NOT NULL DEFAULT 'backlog',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            started_at DATETIME,
            completed_at DATETIME
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on project_id for faster lookups
    conn.execute(
        "CREATE INDEX idx_tasks_project_id ON tasks(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on internal_status for filtering
    conn.execute(
        "CREATE INDEX idx_tasks_internal_status ON tasks(internal_status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Task state history table (audit log)
    conn.execute(
        "CREATE TABLE task_state_history (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            from_status TEXT,
            to_status TEXT NOT NULL,
            changed_by TEXT NOT NULL,
            reason TEXT,
            metadata JSON,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Create index on task_id for history lookups
    conn.execute(
        "CREATE INDEX idx_task_state_history_task_id ON task_state_history(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v2: Create task_blockers table for dependency tracking
fn migrate_v2(conn: &Connection) -> AppResult<()> {
    // Task blockers table (many-to-many relationship)
    // task_id is blocked BY blocker_id
    conn.execute(
        "CREATE TABLE task_blockers (
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            blocker_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (task_id, blocker_id)
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for "what blocks this task?" queries
    conn.execute(
        "CREATE INDEX idx_task_blockers_task_id ON task_blockers(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on blocker_id for "what does this task block?" queries
    conn.execute(
        "CREATE INDEX idx_task_blockers_blocker_id ON task_blockers(blocker_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v3: Create task_state_data table for state-local data persistence
///
/// States like QaFailed and Failed have associated data that needs to persist
/// across application restarts. This table stores that data as JSON.
fn migrate_v3(conn: &Connection) -> AppResult<()> {
    // Task state data table
    // Stores state-local data for states like qa_failed and failed
    conn.execute(
        "CREATE TABLE task_state_data (
            task_id TEXT PRIMARY KEY REFERENCES tasks(id) ON DELETE CASCADE,
            state_type TEXT NOT NULL,
            data TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on state_type for querying all tasks in a specific state with data
    conn.execute(
        "CREATE INDEX idx_task_state_data_state_type ON task_state_data(state_type)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v4: Create agent_profiles table for storing agent configurations
///
/// Agent profiles define how agents behave - their model, execution settings,
/// skills, and behavioral flags. Built-in profiles are seeded on first run.
fn migrate_v4(conn: &Connection) -> AppResult<()> {
    // Agent profiles table
    conn.execute(
        "CREATE TABLE agent_profiles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL,
            profile_json TEXT NOT NULL,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on role for filtering profiles by role
    conn.execute(
        "CREATE INDEX idx_agent_profiles_role ON agent_profiles(role)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_builtin for filtering built-in vs custom profiles
    conn.execute(
        "CREATE INDEX idx_agent_profiles_is_builtin ON agent_profiles(is_builtin)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v5: Create task_qa table for QA artifacts
///
/// The task_qa table stores QA-related data for each task:
/// - Acceptance criteria generated by QA Prep agent
/// - Test steps (initial and refined)
/// - Test results and screenshots from QA Executor
fn migrate_v5(conn: &Connection) -> AppResult<()> {
    // Task QA table
    conn.execute(
        "CREATE TABLE task_qa (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,

            -- Phase 1: QA Prep (runs in parallel with execution)
            acceptance_criteria TEXT,
            qa_test_steps TEXT,
            prep_agent_id TEXT,
            prep_started_at DATETIME,
            prep_completed_at DATETIME,

            -- Phase 2: QA Refinement (after execution completes)
            actual_implementation TEXT,
            refined_test_steps TEXT,
            refinement_agent_id TEXT,
            refinement_completed_at DATETIME,

            -- Phase 3: Test Execution (browser tests)
            test_results TEXT,
            screenshots TEXT,
            test_agent_id TEXT,
            test_completed_at DATETIME,

            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for efficient lookup
    conn.execute(
        "CREATE INDEX idx_task_qa_task_id ON task_qa(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v6: Add QA columns to tasks table
///
/// Adds per-task QA configuration columns:
/// - needs_qa: Boolean override for QA requirement (NULL = inherit from global)
/// - qa_prep_status: Status of QA preparation phase
/// - qa_test_status: Status of QA testing phase
fn migrate_v6(conn: &Connection) -> AppResult<()> {
    // Add needs_qa column (nullable boolean for override)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN needs_qa BOOLEAN DEFAULT NULL",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add qa_prep_status column (pending, running, completed, failed)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN qa_prep_status TEXT DEFAULT 'pending'",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add qa_test_status column (pending, waiting_for_prep, running, passed, failed)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN qa_test_status TEXT DEFAULT 'pending'",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v7: Create reviews table for AI and human code review
///
/// The reviews table tracks code reviews (AI or human) for tasks:
/// - Reviewer type (AI or human)
/// - Review status (pending, approved, changes_requested, rejected)
/// - Notes and feedback
/// - Timestamps for created/completed
fn migrate_v7(conn: &Connection) -> AppResult<()> {
    // Reviews table
    conn.execute(
        "CREATE TABLE reviews (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id),
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            reviewer_type TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            completed_at DATETIME
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for efficient lookup
    conn.execute(
        "CREATE INDEX idx_reviews_task_id ON reviews(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on project_id for project-wide queries
    conn.execute(
        "CREATE INDEX idx_reviews_project_id ON reviews(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on status for filtering pending reviews
    conn.execute(
        "CREATE INDEX idx_reviews_status ON reviews(status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v8: Create review_actions table for tracking review actions
///
/// The review_actions table tracks actions taken on reviews:
/// - When fix tasks are created
/// - When tasks are moved to backlog
/// - When reviews are approved
fn migrate_v8(conn: &Connection) -> AppResult<()> {
    // Review actions table
    conn.execute(
        "CREATE TABLE review_actions (
            id TEXT PRIMARY KEY,
            review_id TEXT NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
            action_type TEXT NOT NULL,
            target_task_id TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on review_id for efficient lookup
    conn.execute(
        "CREATE INDEX idx_review_actions_review_id ON review_actions(review_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on target_task_id for finding actions related to a task
    conn.execute(
        "CREATE INDEX idx_review_actions_target_task_id ON review_actions(target_task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v9: Create review_notes table for storing review notes
///
/// The review_notes table stores notes from reviewers:
/// - Who reviewed (AI or human)
/// - The outcome of the review
/// - Notes/feedback from the reviewer
fn migrate_v9(conn: &Connection) -> AppResult<()> {
    // Review notes table
    conn.execute(
        "CREATE TABLE review_notes (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            reviewer TEXT NOT NULL,
            outcome TEXT NOT NULL,
            notes TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for efficient lookup of history
    conn.execute(
        "CREATE INDEX idx_review_notes_task_id ON review_notes(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v10: Add needs_review_point column to tasks table
/// This column indicates whether a task requires a human-in-the-loop checkpoint
/// before execution (e.g., for destructive operations or complex tasks)
fn migrate_v10(conn: &Connection) -> AppResult<()> {
    // Add needs_review_point column to tasks table with default value false (0)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN needs_review_point INTEGER DEFAULT 0",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v11: Create ideation system tables
/// This includes ideation_sessions, task_proposals, proposal_dependencies,
/// chat_messages, and task_dependencies tables for the ideation workflow.
fn migrate_v11(conn: &Connection) -> AppResult<()> {
    // Ideation Sessions table
    conn.execute(
        "CREATE TABLE ideation_sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            title TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            archived_at DATETIME,
            converted_at DATETIME
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on project_id for efficient session lookups
    conn.execute(
        "CREATE INDEX idx_ideation_sessions_project_id ON ideation_sessions(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on status for filtering active sessions
    conn.execute(
        "CREATE INDEX idx_ideation_sessions_status ON ideation_sessions(status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Task Proposals table
    conn.execute(
        "CREATE TABLE task_proposals (
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
            selected INTEGER DEFAULT 1,
            created_task_id TEXT REFERENCES tasks(id),
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on session_id for efficient proposal lookups
    conn.execute(
        "CREATE INDEX idx_task_proposals_session_id ON task_proposals(session_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on sort_order for ordered retrieval
    conn.execute(
        "CREATE INDEX idx_task_proposals_sort_order ON task_proposals(session_id, sort_order)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Proposal Dependencies table
    conn.execute(
        "CREATE TABLE proposal_dependencies (
            id TEXT PRIMARY KEY,
            proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
            depends_on_proposal_id TEXT NOT NULL REFERENCES task_proposals(id) ON DELETE CASCADE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(proposal_id, depends_on_proposal_id),
            CHECK(proposal_id != depends_on_proposal_id)
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on proposal_id for efficient dependency lookups
    conn.execute(
        "CREATE INDEX idx_proposal_dependencies_proposal_id ON proposal_dependencies(proposal_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on depends_on_proposal_id for reverse dependency lookups
    conn.execute(
        "CREATE INDEX idx_proposal_dependencies_depends_on ON proposal_dependencies(depends_on_proposal_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Chat Messages table
    conn.execute(
        "CREATE TABLE chat_messages (
            id TEXT PRIMARY KEY,
            session_id TEXT REFERENCES ideation_sessions(id) ON DELETE CASCADE,
            project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
            task_id TEXT REFERENCES tasks(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT,
            parent_message_id TEXT REFERENCES chat_messages(id),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on session_id for chat history
    conn.execute(
        "CREATE INDEX idx_chat_messages_session_id ON chat_messages(session_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on project_id for project-level chat
    conn.execute(
        "CREATE INDEX idx_chat_messages_project_id ON chat_messages(project_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for task-specific chat
    conn.execute(
        "CREATE INDEX idx_chat_messages_task_id ON chat_messages(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Task Dependencies table (for applied tasks)
    // Only create if it doesn't exist (may have been created elsewhere)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS task_dependencies (
            id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            depends_on_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(task_id, depends_on_task_id),
            CHECK(task_id != depends_on_task_id)
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for dependency lookups (only if table was just created)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on depends_on_task_id for reverse dependency lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v12: Create workflows table for custom workflow schemas
fn migrate_v12(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE workflows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            schema_json TEXT NOT NULL,
            is_default INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_default for quick default workflow lookup
    conn.execute(
        "CREATE INDEX idx_workflows_is_default ON workflows(is_default)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v13: Create artifact_buckets table for artifact storage organization
fn migrate_v13(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE artifact_buckets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            config_json TEXT NOT NULL,
            is_system INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_system for system bucket queries
    conn.execute(
        "CREATE INDEX idx_artifact_buckets_is_system ON artifact_buckets(is_system)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v14: Create artifacts table for typed documents
fn migrate_v14(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE artifacts (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            name TEXT NOT NULL,
            content_type TEXT NOT NULL,
            content_text TEXT,
            content_path TEXT,
            bucket_id TEXT REFERENCES artifact_buckets(id),
            task_id TEXT REFERENCES tasks(id),
            process_id TEXT,
            created_by TEXT NOT NULL,
            version INTEGER DEFAULT 1,
            previous_version_id TEXT REFERENCES artifacts(id),
            metadata_json TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on bucket_id for bucket queries
    conn.execute(
        "CREATE INDEX idx_artifacts_bucket ON artifacts(bucket_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on type for type filtering
    conn.execute(
        "CREATE INDEX idx_artifacts_type ON artifacts(type)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on task_id for task-related artifact queries
    conn.execute(
        "CREATE INDEX idx_artifacts_task ON artifacts(task_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v15: Create artifact_relations table for artifact derivation/relations
fn migrate_v15(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE artifact_relations (
            id TEXT PRIMARY KEY,
            from_artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
            to_artifact_id TEXT NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
            relation_type TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(from_artifact_id, to_artifact_id, relation_type)
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on from_artifact_id for forward relation lookups
    conn.execute(
        "CREATE INDEX idx_artifact_relations_from ON artifact_relations(from_artifact_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on to_artifact_id for reverse relation lookups
    conn.execute(
        "CREATE INDEX idx_artifact_relations_to ON artifact_relations(to_artifact_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v16: Create artifact_flows table for automated artifact routing
fn migrate_v16(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE artifact_flows (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            trigger_json TEXT NOT NULL,
            steps_json TEXT NOT NULL,
            is_active INTEGER DEFAULT 1,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_active for active flow queries
    conn.execute(
        "CREATE INDEX idx_artifact_flows_active ON artifact_flows(is_active)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v17: Create processes table for research and other long-running processes
fn migrate_v17(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE processes (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            name TEXT NOT NULL,
            config_json TEXT NOT NULL,
            status TEXT NOT NULL,
            current_iteration INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            started_at DATETIME,
            completed_at DATETIME
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on status for process status queries
    conn.execute(
        "CREATE INDEX idx_processes_status ON processes(status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on type for process type queries
    conn.execute(
        "CREATE INDEX idx_processes_type ON processes(type)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v18: Add extensibility columns to tasks table for methodology support
fn migrate_v18(conn: &Connection) -> AppResult<()> {
    // Add external_status for custom workflow column mapping
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN external_status TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add wave for parallel execution grouping (GSD method)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN wave INTEGER",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add checkpoint_type for human-in-loop checkpoints
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN checkpoint_type TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add phase_id for methodology phase tracking
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN phase_id TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add plan_id for methodology plan tracking
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN plan_id TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Add must_haves_json for goal-backward verification (GSD method)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN must_haves_json TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on wave for wave-based queries
    conn.execute(
        "CREATE INDEX idx_tasks_wave ON tasks(wave)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on external_status for external status queries
    conn.execute(
        "CREATE INDEX idx_tasks_external_status ON tasks(external_status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v19: Create methodology_extensions table for methodology support
fn migrate_v19(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE methodology_extensions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            config_json TEXT NOT NULL,
            is_active INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index on is_active for active methodology lookup
    conn.execute(
        "CREATE INDEX idx_methodology_extensions_active ON methodology_extensions(is_active)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

/// Migration v20: Chat conversations, agent runs, and tool calls
fn migrate_v20(conn: &Connection) -> AppResult<()> {
    // ============================================================================
    // Chat Conversations Table
    // Links a context (ideation session, task, project) to Claude sessions
    // ============================================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_conversations (
            id TEXT PRIMARY KEY,
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL,
            claude_session_id TEXT,
            title TEXT,
            message_count INTEGER NOT NULL DEFAULT 0,
            last_message_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Multiple conversations per context allowed
    conn.execute(
        "CREATE INDEX idx_chat_conversations_context ON chat_conversations(context_type, context_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_chat_conversations_claude_session ON chat_conversations(claude_session_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // ============================================================================
    // Agent Runs Table
    // Tracks active/completed agent runs for streaming persistence
    // ============================================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_runs (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL REFERENCES chat_conversations(id) ON DELETE CASCADE,
            status TEXT NOT NULL DEFAULT 'running',
            started_at TEXT NOT NULL,
            completed_at TEXT,
            error_message TEXT
        )",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_agent_runs_conversation ON agent_runs(conversation_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "CREATE INDEX idx_agent_runs_status ON agent_runs(status)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // ============================================================================
    // Modify chat_messages Table
    // Add conversation reference and tool_calls
    // ============================================================================
    conn.execute(
        "ALTER TABLE chat_messages ADD COLUMN conversation_id TEXT REFERENCES chat_conversations(id) ON DELETE CASCADE",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    conn.execute(
        "ALTER TABLE chat_messages ADD COLUMN tool_calls TEXT",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // Index for conversation lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_conversation ON chat_messages(conversation_id)",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    // ============================================================================
    // Update message_count trigger
    // ============================================================================
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_conversation_message_count
        AFTER INSERT ON chat_messages
        FOR EACH ROW
        WHEN NEW.conversation_id IS NOT NULL
        BEGIN
            UPDATE chat_conversations
            SET message_count = message_count + 1,
                last_message_at = NEW.created_at,
                updated_at = datetime('now')
            WHERE id = NEW.conversation_id;
        END",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

    Ok(())
}

fn migrate_v21(conn: &Connection) -> AppResult<()> {
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

fn migrate_v22(conn: &Connection) -> AppResult<()> {
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

fn migrate_v23(conn: &Connection) -> AppResult<()> {
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
fn migrate_v24(conn: &Connection) -> AppResult<()> {
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
fn migrate_v25(conn: &Connection) -> AppResult<()> {
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


#[cfg(test)]
mod tests;
