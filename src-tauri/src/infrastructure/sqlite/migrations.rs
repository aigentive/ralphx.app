// Database migrations for SQLite
// Creates and updates schema as needed

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Current schema version
pub const SCHEMA_VERSION: i32 = 19;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::connection::open_memory_connection;

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(SCHEMA_VERSION, 19);
    }

    #[test]
    fn test_run_migrations_creates_migrations_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify migrations table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_migrations_creates_projects_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify projects table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_migrations_creates_tasks_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify tasks table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_tasks_table_has_needs_review_point_column() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a task and verify needs_review_point column works
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('test-proj', 'Test', '/tmp')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status, needs_review_point)
             VALUES ('task-1', 'test-proj', 'feature', 'Test Task', 'backlog', 1)",
            [],
        )
        .unwrap();

        // Query the needs_review_point value
        let needs_review_point: i32 = conn
            .query_row(
                "SELECT needs_review_point FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(needs_review_point, 1);

        // Insert task without specifying needs_review_point (should default to 0)
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status)
             VALUES ('task-2', 'test-proj', 'feature', 'Test Task 2', 'backlog')",
            [],
        )
        .unwrap();

        let needs_review_point: i32 = conn
            .query_row(
                "SELECT needs_review_point FROM tasks WHERE id = 'task-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(needs_review_point, 0);
    }

    #[test]
    fn test_run_migrations_creates_task_state_history_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_state_history table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_state_history'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_run_migrations_sets_schema_version() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 19);
    }

    #[test]
    fn test_run_migrations_is_idempotent() {
        let conn = open_memory_connection().unwrap();

        // Run migrations twice
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        // Should still work and have correct version
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 19);
    }

    #[test]
    fn test_projects_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Try inserting a complete project record
        let result = conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, worktree_path, worktree_branch, base_branch)
             VALUES ('test-id', 'Test Project', '/path/to/project', 'local', NULL, NULL, NULL)",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project first (foreign key reference)
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Try inserting a complete task record
        let result = conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, description, priority, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'Description', 5, 'backlog')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_state_history_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test')",
            [],
        )
        .unwrap();

        // Try inserting a history record
        let result = conn.execute(
            "INSERT INTO task_state_history (id, task_id, from_status, to_status, changed_by, reason, metadata)
             VALUES ('hist-1', 'task-1', 'backlog', 'ready', 'system', 'Started', '{}')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_index_on_project_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_tasks_index_on_internal_status_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_internal_status'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_history_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_state_history_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_projects_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert minimal project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Check default values
        let git_mode: String = conn
            .query_row(
                "SELECT git_mode FROM projects WHERE id = 'proj-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(git_mode, "local");
    }

    #[test]
    fn test_tasks_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Insert minimal task
        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test')",
            [],
        )
        .unwrap();

        // Check default values
        let (priority, status): (i32, String) = conn
            .query_row(
                "SELECT priority, internal_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(priority, 0);
        assert_eq!(status, "backlog");
    }

    #[test]
    fn test_run_migrations_creates_task_blockers_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_blockers table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_blockers'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Try inserting a blocker relationship
        let result = conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_blockers_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_blockers_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_index_on_blocker_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_blockers_blocker_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_blockers_primary_key_prevents_duplicates() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // First insert should succeed
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Duplicate should fail
        let result = conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_blockers_cascade_delete_on_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Add blocker relationship
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Delete the blocked task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // Blocker relationship should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_blockers_cascade_delete_on_blocker() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Add blocker relationship (task-1 is blocked by task-2)
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        // Delete the blocker task
        conn.execute("DELETE FROM tasks WHERE id = 'task-2'", []).unwrap();

        // Blocker relationship should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE blocker_id = 'task-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_blockers_multiple_blockers_per_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and tasks
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-3', 'proj-1', 'feature', 'Task 3')",
            [],
        )
        .unwrap();

        // Task 1 is blocked by both task 2 and task 3
        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_blockers (task_id, blocker_id) VALUES ('task-1', 'task-3')",
            [],
        )
        .unwrap();

        // Count blockers for task-1
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_blockers WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    // ==================
    // V3 Migration Tests: task_state_data table
    // ==================

    #[test]
    fn test_run_migrations_creates_task_state_data_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_state_data table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_state_data'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_data_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, internal_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1', 'qa_failed')",
            [],
        )
        .unwrap();

        // Try inserting a state data record
        let result = conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data)
             VALUES ('task-1', 'qa_failed', '{\"failures\": [], \"retry_count\": 0}')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_state_data_index_on_state_type_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_state_data_state_type'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_state_data_primary_key_prevents_duplicates() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // First insert should succeed
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // Duplicate should fail (primary key violation)
        let result = conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'failed', '{}')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_state_data_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert state data
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // State data should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_state_data_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert JSON data
        let json_data = r#"{"failures":[{"test_name":"test_foo","error":"assertion failed"}],"retry_count":2}"#;
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT data FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_task_state_data_can_update() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert initial data
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{\"retry_count\":0}')",
            [],
        )
        .unwrap();

        // Update the data using REPLACE
        conn.execute(
            "INSERT OR REPLACE INTO task_state_data (task_id, state_type, data)
             VALUES ('task-1', 'qa_failed', '{\"retry_count\":1}')",
            [],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT data FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(retrieved.contains("\"retry_count\":1"));
    }

    #[test]
    fn test_task_state_data_updated_at_has_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert without specifying updated_at
        conn.execute(
            "INSERT INTO task_state_data (task_id, state_type, data) VALUES ('task-1', 'qa_failed', '{}')",
            [],
        )
        .unwrap();

        // updated_at should not be null
        let updated_at: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM task_state_data WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(updated_at.is_some());
    }

    // ==================
    // V4 Migration Tests: agent_profiles table
    // ==================

    #[test]
    fn test_run_migrations_creates_agent_profiles_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify agent_profiles table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='agent_profiles'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Try inserting a complete agent profile record
        let profile_json = r#"{"claude_code":{"agent_definition":"agents/worker.md"}}"#;
        let result = conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin)
             VALUES ('prof-1', 'Worker', 'worker', ?1, 1)",
            [profile_json],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_profiles_name_unique_constraint() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let profile_json = r#"{}"#;

        // First insert should succeed
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', ?1)",
            [profile_json],
        )
        .unwrap();

        // Duplicate name should fail
        let result = conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-2', 'Worker', 'worker', ?1)",
            [profile_json],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_agent_profiles_index_on_role_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_agent_profiles_role'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_index_on_is_builtin_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_agent_profiles_is_builtin'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_agent_profiles_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert minimal profile
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Check default values
        let is_builtin: i32 = conn
            .query_row(
                "SELECT is_builtin FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(is_builtin, 0);
    }

    #[test]
    fn test_agent_profiles_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert JSON profile
        let json_data = r#"{"name":"worker","execution":{"model":"sonnet","max_iterations":30},"behavior":{"autonomy_level":"semi_autonomous"}}"#;
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT profile_json FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_agent_profiles_filter_by_role() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert profiles with different roles
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-2', 'Reviewer', 'reviewer', '{}')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-3', 'Another Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Query by role
        let worker_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE role = 'worker'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(worker_count, 2);
    }

    #[test]
    fn test_agent_profiles_filter_by_is_builtin() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert builtin and custom profiles
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin) VALUES ('prof-1', 'Builtin Worker', 'worker', '{}', 1)",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin) VALUES ('prof-2', 'Custom Worker', 'worker', '{}', 0)",
            [],
        )
        .unwrap();

        // Query builtin profiles
        let builtin_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE is_builtin = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(builtin_count, 1);

        // Query custom profiles
        let custom_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE is_builtin = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(custom_count, 1);
    }

    #[test]
    fn test_agent_profiles_timestamps() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert profile
        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json) VALUES ('prof-1', 'Worker', 'worker', '{}')",
            [],
        )
        .unwrap();

        // Check timestamps exist
        let (created_at, updated_at): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT created_at, updated_at FROM agent_profiles WHERE id = 'prof-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert!(created_at.is_some());
        assert!(updated_at.is_some());
    }

    // ==================
    // V5 Migration Tests: task_qa table
    // ==================

    #[test]
    fn test_run_migrations_creates_task_qa_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify task_qa table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_qa'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_qa_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Try inserting a complete task_qa record
        let acceptance_criteria = r#"[{"id":"AC1","description":"Test","testable":true,"type":"visual"}]"#;
        let qa_test_steps = r#"[{"id":"QA1","criteria_id":"AC1","description":"Test","commands":[],"expected":"Pass"}]"#;
        let test_results = r#"{"task_id":"task-1","overall_status":"passed","steps":[]}"#;
        let screenshots = r#"["screenshots/test.png"]"#;

        let result = conn.execute(
            "INSERT INTO task_qa (
                id, task_id,
                acceptance_criteria, qa_test_steps, prep_agent_id, prep_started_at, prep_completed_at,
                actual_implementation, refined_test_steps, refinement_agent_id, refinement_completed_at,
                test_results, screenshots, test_agent_id, test_completed_at
            ) VALUES (
                'qa-1', 'task-1',
                ?1, ?2, 'agent-prep-1', '2026-01-24 10:00:00', '2026-01-24 10:05:00',
                'Implemented feature X', ?2, 'agent-refine-1', '2026-01-24 10:10:00',
                ?3, ?4, 'agent-test-1', '2026-01-24 10:15:00'
            )",
            [acceptance_criteria, qa_test_steps, test_results, screenshots],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_qa_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_qa_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_qa_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert task_qa record
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // task_qa should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_qa WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_task_qa_stores_json() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert JSON data
        let json_data = r#"[{"id":"AC1","description":"User can see task board","testable":true,"type":"visual"}]"#;
        conn.execute(
            "INSERT INTO task_qa (id, task_id, acceptance_criteria) VALUES ('qa-1', 'task-1', ?1)",
            [json_data],
        )
        .unwrap();

        // Read it back
        let retrieved: String = conn
            .query_row(
                "SELECT acceptance_criteria FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(retrieved, json_data);
    }

    #[test]
    fn test_task_qa_allows_null_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert minimal task_qa (only required columns)
        let result = conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        );

        assert!(result.is_ok());

        // Verify nulls are stored
        let acceptance: Option<String> = conn
            .query_row(
                "SELECT acceptance_criteria FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(acceptance.is_none());
    }

    #[test]
    fn test_task_qa_created_at_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert without created_at
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // created_at should not be null
        let created_at: Option<String> = conn
            .query_row(
                "SELECT created_at FROM task_qa WHERE id = 'qa-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(created_at.is_some());
    }

    #[test]
    fn test_task_qa_multiple_per_task_prevented() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert first QA record (unique ID)
        conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-1', 'task-1')",
            [],
        )
        .unwrap();

        // Second QA record for same task but different ID should work
        // (no unique constraint on task_id, just foreign key)
        let result = conn.execute(
            "INSERT INTO task_qa (id, task_id) VALUES ('qa-2', 'task-1')",
            [],
        );

        assert!(result.is_ok());
    }

    // ==================
    // V6 Migration Tests: QA columns on tasks table
    // ==================

    #[test]
    fn test_tasks_has_needs_qa_column() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task with needs_qa
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        let result = conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, needs_qa)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1', 1)",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_tasks_needs_qa_can_be_null() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task without specifying needs_qa
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Verify needs_qa is NULL by default
        let needs_qa: Option<bool> = conn
            .query_row(
                "SELECT needs_qa FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(needs_qa.is_none());
    }

    #[test]
    fn test_tasks_has_qa_prep_status_column() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, qa_prep_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1', 'running')",
            [],
        )
        .unwrap();

        // Verify the value
        let status: String = conn
            .query_row(
                "SELECT qa_prep_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "running");
    }

    #[test]
    fn test_tasks_qa_prep_status_defaults_to_pending() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task without specifying qa_prep_status
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Verify default
        let status: String = conn
            .query_row(
                "SELECT qa_prep_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "pending");
    }

    #[test]
    fn test_tasks_has_qa_test_status_column() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, qa_test_status)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1', 'passed')",
            [],
        )
        .unwrap();

        // Verify the value
        let status: String = conn
            .query_row(
                "SELECT qa_test_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "passed");
    }

    #[test]
    fn test_tasks_qa_test_status_defaults_to_pending() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task without specifying qa_test_status
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Verify default
        let status: String = conn
            .query_row(
                "SELECT qa_test_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "pending");
    }

    #[test]
    fn test_tasks_qa_columns_can_be_updated() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title)
             VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Update QA columns
        conn.execute(
            "UPDATE tasks SET needs_qa = 1, qa_prep_status = 'completed', qa_test_status = 'running'
             WHERE id = 'task-1'",
            [],
        )
        .unwrap();

        // Verify updates
        let (needs_qa, prep_status, test_status): (bool, String, String) = conn
            .query_row(
                "SELECT needs_qa, qa_prep_status, qa_test_status FROM tasks WHERE id = 'task-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();

        assert!(needs_qa);
        assert_eq!(prep_status, "completed");
        assert_eq!(test_status, "running");
    }

    #[test]
    fn test_tasks_qa_columns_all_statuses() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Test all valid prep statuses
        let prep_statuses = ["pending", "running", "completed", "failed"];
        for (i, status) in prep_statuses.iter().enumerate() {
            let task_id = format!("task-prep-{}", i);
            conn.execute(
                &format!(
                    "INSERT INTO tasks (id, project_id, category, title, qa_prep_status)
                     VALUES ('{}', 'proj-1', 'feature', 'Task', '{}')",
                    task_id, status
                ),
                [],
            )
            .unwrap();

            let stored: String = conn
                .query_row(
                    &format!(
                        "SELECT qa_prep_status FROM tasks WHERE id = '{}'",
                        task_id
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(&stored, *status);
        }

        // Test all valid test statuses
        let test_statuses = ["pending", "waiting_for_prep", "running", "passed", "failed"];
        for (i, status) in test_statuses.iter().enumerate() {
            let task_id = format!("task-test-{}", i);
            conn.execute(
                &format!(
                    "INSERT INTO tasks (id, project_id, category, title, qa_test_status)
                     VALUES ('{}', 'proj-1', 'feature', 'Task', '{}')",
                    task_id, status
                ),
                [],
            )
            .unwrap();

            let stored: String = conn
                .query_row(
                    &format!(
                        "SELECT qa_test_status FROM tasks WHERE id = '{}'",
                        task_id
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(&stored, *status);
        }
    }

    // ==================
    // V7 Migration Tests: reviews table
    // ==================

    #[test]
    fn test_run_migrations_creates_reviews_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify reviews table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reviews'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_reviews_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Try inserting a complete review record
        let result = conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status, notes, completed_at)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai', 'approved', 'Looks good', '2026-01-24 10:00:00')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_reviews_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_reviews_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_reviews_index_on_project_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_reviews_project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_reviews_index_on_status_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_reviews_status'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_reviews_status_default_is_pending() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert review without specifying status
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Verify default status
        let status: String = conn
            .query_row(
                "SELECT status FROM reviews WHERE id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(status, "pending");
    }

    #[test]
    fn test_reviews_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // Review should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_reviews_all_reviewer_types() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Test AI reviewer
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-ai', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Test human reviewer
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-human', 'proj-1', 'task-1', 'human')",
            [],
        )
        .unwrap();

        // Verify both exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_reviews_all_statuses() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Test all valid statuses
        let statuses = ["pending", "approved", "changes_requested", "rejected"];
        for (i, status) in statuses.iter().enumerate() {
            let review_id = format!("rev-{}", i);
            conn.execute(
                &format!(
                    "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
                     VALUES ('{}', 'proj-1', 'task-1', 'ai', '{}')",
                    review_id, status
                ),
                [],
            )
            .unwrap();

            let stored: String = conn
                .query_row(
                    &format!("SELECT status FROM reviews WHERE id = '{}'", review_id),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(&stored, *status);
        }
    }

    #[test]
    fn test_reviews_created_at_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert review without created_at
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // created_at should not be null
        let created_at: Option<String> = conn
            .query_row(
                "SELECT created_at FROM reviews WHERE id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(created_at.is_some());
    }

    #[test]
    fn test_reviews_notes_can_be_null() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert review without notes
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Verify notes is NULL
        let notes: Option<String> = conn
            .query_row(
                "SELECT notes FROM reviews WHERE id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(notes.is_none());
    }

    #[test]
    fn test_reviews_completed_at_can_be_null() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert review without completed_at
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Verify completed_at is NULL
        let completed_at: Option<String> = conn
            .query_row(
                "SELECT completed_at FROM reviews WHERE id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(completed_at.is_none());
    }

    #[test]
    fn test_reviews_multiple_per_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert multiple reviews for same task
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai', 'changes_requested')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-2', 'proj-1', 'task-1', 'ai', 'approved')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-3', 'proj-1', 'task-1', 'human', 'approved')",
            [],
        )
        .unwrap();

        // All three should exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_reviews_filter_by_status() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert reviews with different statuses
        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai', 'pending')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-2', 'proj-1', 'task-1', 'ai', 'pending')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type, status)
             VALUES ('rev-3', 'proj-1', 'task-1', 'human', 'approved')",
            [],
        )
        .unwrap();

        // Query pending reviews
        let pending_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE status = 'pending'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(pending_count, 2);

        // Query approved reviews
        let approved_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM reviews WHERE status = 'approved'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(approved_count, 1);
    }

    // ==================
    // V8 Migration Tests: review_actions table
    // ==================

    #[test]
    fn test_run_migrations_creates_review_actions_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify review_actions table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='review_actions'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_review_actions_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Fix Task')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Try inserting a complete review action record
        let result = conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type, target_task_id)
             VALUES ('action-1', 'rev-1', 'created_fix_task', 'task-2')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_review_actions_index_on_review_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_review_actions_review_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_review_actions_index_on_target_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_review_actions_target_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_review_actions_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, review, and action
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type)
             VALUES ('action-1', 'rev-1', 'approved')",
            [],
        )
        .unwrap();

        // Delete the review
        conn.execute("DELETE FROM reviews WHERE id = 'rev-1'", []).unwrap();

        // Review action should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM review_actions WHERE review_id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_review_actions_all_action_types() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Test all action types
        let action_types = ["created_fix_task", "moved_to_backlog", "approved"];
        for (i, action_type) in action_types.iter().enumerate() {
            let action_id = format!("action-{}", i);
            conn.execute(
                &format!(
                    "INSERT INTO review_actions (id, review_id, action_type)
                     VALUES ('{}', 'rev-1', '{}')",
                    action_id, action_type
                ),
                [],
            )
            .unwrap();

            let stored: String = conn
                .query_row(
                    &format!("SELECT action_type FROM review_actions WHERE id = '{}'", action_id),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(&stored, *action_type);
        }
    }

    #[test]
    fn test_review_actions_target_task_id_can_be_null() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Insert action without target_task_id
        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type)
             VALUES ('action-1', 'rev-1', 'approved')",
            [],
        )
        .unwrap();

        // Verify target_task_id is NULL
        let target_task_id: Option<String> = conn
            .query_row(
                "SELECT target_task_id FROM review_actions WHERE id = 'action-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(target_task_id.is_none());
    }

    #[test]
    fn test_review_actions_created_at_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Insert action without created_at
        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type)
             VALUES ('action-1', 'rev-1', 'approved')",
            [],
        )
        .unwrap();

        // created_at should not be null
        let created_at: Option<String> = conn
            .query_row(
                "SELECT created_at FROM review_actions WHERE id = 'action-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(created_at.is_some());
    }

    #[test]
    fn test_review_actions_multiple_per_review() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-fix', 'proj-1', 'feature', 'Fix Task')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Insert multiple actions for same review
        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type, target_task_id)
             VALUES ('action-1', 'rev-1', 'created_fix_task', 'task-fix')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type)
             VALUES ('action-2', 'rev-1', 'approved')",
            [],
        )
        .unwrap();

        // Both should exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM review_actions WHERE review_id = 'rev-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_review_actions_lookup_by_target_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, tasks, and review
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-fix', 'proj-1', 'feature', 'Fix Task')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO reviews (id, project_id, task_id, reviewer_type)
             VALUES ('rev-1', 'proj-1', 'task-1', 'ai')",
            [],
        )
        .unwrap();

        // Insert action with target task
        conn.execute(
            "INSERT INTO review_actions (id, review_id, action_type, target_task_id)
             VALUES ('action-1', 'rev-1', 'created_fix_task', 'task-fix')",
            [],
        )
        .unwrap();

        // Look up by target task
        let review_id: String = conn
            .query_row(
                "SELECT review_id FROM review_actions WHERE target_task_id = 'task-fix'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(review_id, "rev-1");
    }

    // ==================
    // V9 Migration Tests: review_notes table
    // ==================

    #[test]
    fn test_run_migrations_creates_review_notes_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify review_notes table exists
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='review_notes'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_review_notes_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Try inserting a complete review note record
        let result = conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, notes)
             VALUES ('note-1', 'task-1', 'ai', 'approved', 'Looks good')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_review_notes_index_on_task_id_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_review_notes_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_review_notes_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project, task, and note
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome)
             VALUES ('note-1', 'task-1', 'ai', 'approved')",
            [],
        )
        .unwrap();

        // Delete the task
        conn.execute("DELETE FROM tasks WHERE id = 'task-1'", []).unwrap();

        // Review note should be gone
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM review_notes WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn test_review_notes_all_reviewer_types() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Test AI reviewer
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome)
             VALUES ('note-ai', 'task-1', 'ai', 'approved')",
            [],
        )
        .unwrap();

        // Test human reviewer
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome)
             VALUES ('note-human', 'task-1', 'human', 'changes_requested')",
            [],
        )
        .unwrap();

        // Verify both exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM review_notes WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_review_notes_all_outcomes() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Test all outcomes
        let outcomes = ["approved", "changes_requested", "rejected"];
        for (i, outcome) in outcomes.iter().enumerate() {
            let note_id = format!("note-{}", i);
            conn.execute(
                &format!(
                    "INSERT INTO review_notes (id, task_id, reviewer, outcome)
                     VALUES ('{}', 'task-1', 'ai', '{}')",
                    note_id, outcome
                ),
                [],
            )
            .unwrap();

            let stored: String = conn
                .query_row(
                    &format!("SELECT outcome FROM review_notes WHERE id = '{}'", note_id),
                    [],
                    |row| row.get(0),
                )
                .unwrap();

            assert_eq!(&stored, *outcome);
        }
    }

    #[test]
    fn test_review_notes_notes_can_be_null() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert note without notes text
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome)
             VALUES ('note-1', 'task-1', 'ai', 'approved')",
            [],
        )
        .unwrap();

        // Verify notes is NULL
        let notes: Option<String> = conn
            .query_row(
                "SELECT notes FROM review_notes WHERE id = 'note-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(notes.is_none());
    }

    #[test]
    fn test_review_notes_created_at_default() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert note without created_at
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome)
             VALUES ('note-1', 'task-1', 'ai', 'approved')",
            [],
        )
        .unwrap();

        // created_at should not be null
        let created_at: Option<String> = conn
            .query_row(
                "SELECT created_at FROM review_notes WHERE id = 'note-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(created_at.is_some());
    }

    #[test]
    fn test_review_notes_multiple_per_task() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert multiple notes for same task (review history)
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, notes)
             VALUES ('note-1', 'task-1', 'ai', 'changes_requested', 'Tests missing')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, notes)
             VALUES ('note-2', 'task-1', 'ai', 'changes_requested', 'Still missing integration test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, notes)
             VALUES ('note-3', 'task-1', 'human', 'approved', 'Looks good now')",
            [],
        )
        .unwrap();

        // All three should exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM review_notes WHERE task_id = 'task-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_review_notes_ordered_by_created_at() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and task
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Insert notes with explicit timestamps
        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, created_at)
             VALUES ('note-1', 'task-1', 'ai', 'changes_requested', '2026-01-24 10:00:00')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO review_notes (id, task_id, reviewer, outcome, created_at)
             VALUES ('note-2', 'task-1', 'ai', 'approved', '2026-01-24 11:00:00')",
            [],
        )
        .unwrap();

        // Query ordered by created_at
        let mut stmt = conn
            .prepare(
                "SELECT outcome FROM review_notes WHERE task_id = 'task-1' ORDER BY created_at ASC",
            )
            .unwrap();

        let outcomes: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(outcomes, vec!["changes_requested", "approved"]);
    }

    // ==========================================
    // Migration v11 Tests: Ideation System
    // ==========================================

    #[test]
    fn test_run_migrations_creates_ideation_sessions_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ideation_sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_ideation_sessions_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a project first
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Try inserting a complete session record
        let result = conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, archived_at, converted_at)
             VALUES ('session-1', 'proj-1', 'Auth Feature', 'active', NULL, NULL)",
            [],
        );

        assert!(result.is_ok());

        // Verify default values
        let status: String = conn
            .query_row(
                "SELECT status FROM ideation_sessions WHERE id = 'session-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");
    }

    #[test]
    fn test_ideation_sessions_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let project_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_ideation_sessions_project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(project_idx, 1);

        let status_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_ideation_sessions_status'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status_idx, 1);
    }

    #[test]
    fn test_ideation_sessions_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert project and session
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        // Delete project
        conn.execute("DELETE FROM projects WHERE id = 'proj-1'", [])
            .unwrap();

        // Session should be gone due to CASCADE
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM ideation_sessions WHERE id = 'session-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_run_migrations_creates_task_proposals_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_proposals'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_proposals_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        // Try inserting a complete proposal record
        let result = conn.execute(
            "INSERT INTO task_proposals (
                id, session_id, title, description, category, steps, acceptance_criteria,
                suggested_priority, priority_score, priority_reason, priority_factors,
                estimated_complexity, user_priority, user_modified, status, selected, sort_order
            ) VALUES (
                'prop-1', 'session-1', 'Setup Database', 'Create DB schema', 'setup',
                '[\"Create tables\", \"Add indexes\"]', '[\"Tables exist\", \"Indexes work\"]',
                'high', 75, 'Blocks other tasks', '{\"dependency\": 25}',
                'moderate', NULL, 0, 'pending', 1, 0
            )",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_proposals_default_values() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        // Insert minimal proposal
        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-1', 'session-1', 'Test', 'feature', 'medium')",
            [],
        )
        .unwrap();

        // Check default values
        let (score, complexity, modified, status, selected): (i32, String, i32, String, i32) = conn
            .query_row(
                "SELECT priority_score, estimated_complexity, user_modified, status, selected
                 FROM task_proposals WHERE id = 'prop-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();

        assert_eq!(score, 50);
        assert_eq!(complexity, "moderate");
        assert_eq!(modified, 0);
        assert_eq!(status, "pending");
        assert_eq!(selected, 1);
    }

    #[test]
    fn test_task_proposals_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let session_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_proposals_session_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(session_idx, 1);

        let sort_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_proposals_sort_order'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(sort_idx, 1);
    }

    #[test]
    fn test_task_proposals_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-1', 'session-1', 'Test', 'feature', 'medium')",
            [],
        )
        .unwrap();

        // Delete session
        conn.execute("DELETE FROM ideation_sessions WHERE id = 'session-1'", [])
            .unwrap();

        // Proposal should be gone due to CASCADE
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_proposals WHERE id = 'prop-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_run_migrations_creates_proposal_dependencies_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='proposal_dependencies'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_proposal_dependencies_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-1', 'session-1', 'Task 1', 'feature', 'high')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-2', 'session-1', 'Task 2', 'feature', 'medium')",
            [],
        )
        .unwrap();

        // Insert dependency
        let result = conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', 'prop-2', 'prop-1')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_proposal_dependencies_unique_constraint() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-1', 'session-1', 'Task 1', 'feature', 'high')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-2', 'session-1', 'Task 2', 'feature', 'medium')",
            [],
        )
        .unwrap();

        // First dependency insert succeeds
        conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', 'prop-2', 'prop-1')",
            [],
        )
        .unwrap();

        // Duplicate should fail
        let result = conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-2', 'prop-2', 'prop-1')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_proposal_dependencies_self_reference_check() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_proposals (id, session_id, title, category, suggested_priority)
             VALUES ('prop-1', 'session-1', 'Task 1', 'feature', 'high')",
            [],
        )
        .unwrap();

        // Self-reference should fail due to CHECK constraint
        let result = conn.execute(
            "INSERT INTO proposal_dependencies (id, proposal_id, depends_on_proposal_id)
             VALUES ('dep-1', 'prop-1', 'prop-1')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_proposal_dependencies_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let prop_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_proposal_dependencies_proposal_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(prop_idx, 1);

        let dep_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_proposal_dependencies_depends_on'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dep_idx, 1);
    }

    #[test]
    fn test_run_migrations_creates_chat_messages_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='chat_messages'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_chat_messages_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        // Insert a chat message
        let result = conn.execute(
            "INSERT INTO chat_messages (id, session_id, project_id, role, content, metadata)
             VALUES ('msg-1', 'session-1', 'proj-1', 'user', 'Hello', '{\"key\": \"value\"}')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_chat_messages_with_task_context() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Test')",
            [],
        )
        .unwrap();

        // Insert a chat message about a task
        let result = conn.execute(
            "INSERT INTO chat_messages (id, project_id, task_id, role, content)
             VALUES ('msg-1', 'proj-1', 'task-1', 'orchestrator', 'Task analysis')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_chat_messages_parent_reference() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title) VALUES ('session-1', 'proj-1', 'Test')",
            [],
        )
        .unwrap();

        // Insert parent message
        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content)
             VALUES ('msg-1', 'session-1', 'user', 'First message')",
            [],
        )
        .unwrap();

        // Insert child message with parent reference
        let result = conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content, parent_message_id)
             VALUES ('msg-2', 'session-1', 'orchestrator', 'Reply', 'msg-1')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_chat_messages_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let session_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_chat_messages_session_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(session_idx, 1);

        let project_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_chat_messages_project_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(project_idx, 1);

        let task_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_chat_messages_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task_idx, 1);
    }

    #[test]
    fn test_run_migrations_creates_task_dependencies_table() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_dependencies'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_task_dependencies_table_has_correct_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // Insert task dependency
        let result = conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', 'task-2', 'task-1')",
            [],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_task_dependencies_unique_constraint() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        // First insert succeeds
        conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', 'task-2', 'task-1')",
            [],
        )
        .unwrap();

        // Duplicate should fail
        let result = conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-2', 'task-2', 'task-1')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_dependencies_self_reference_check() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        // Self-reference should fail due to CHECK constraint
        let result = conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', 'task-1', 'task-1')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_task_dependencies_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let task_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_dependencies_task_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task_idx, 1);

        let dep_idx: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_task_dependencies_depends_on'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dep_idx, 1);
    }

    #[test]
    fn test_task_dependencies_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert prerequisite data
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-1', 'proj-1', 'feature', 'Task 1')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (id, project_id, category, title) VALUES ('task-2', 'proj-1', 'feature', 'Task 2')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO task_dependencies (id, task_id, depends_on_task_id)
             VALUES ('dep-1', 'task-2', 'task-1')",
            [],
        )
        .unwrap();

        // Delete task-2 (the dependent task)
        conn.execute("DELETE FROM tasks WHERE id = 'task-2'", [])
            .unwrap();

        // Dependency should be gone due to CASCADE
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM task_dependencies WHERE task_id = 'task-2'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    // ============== Extensibility Migrations Tests (v12-v19) ==============

    #[test]
    fn test_workflows_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='workflows'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_workflows_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Insert a workflow with all columns
        let result = conn.execute(
            "INSERT INTO workflows (id, name, description, schema_json, is_default)
             VALUES ('wf-1', 'Test Workflow', 'A test workflow', '{\"columns\":[]}', 1)",
            [],
        );
        assert!(result.is_ok());

        // Verify is_default column
        let is_default: i32 = conn
            .query_row(
                "SELECT is_default FROM workflows WHERE id = 'wf-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(is_default, 1);
    }

    #[test]
    fn test_workflows_index_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_workflows_is_default'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_artifact_buckets_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifact_buckets'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_artifact_buckets_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let result = conn.execute(
            "INSERT INTO artifact_buckets (id, name, config_json, is_system)
             VALUES ('bucket-1', 'Research Outputs', '{\"acceptedTypes\":[\"research_document\"]}', 1)",
            [],
        );
        assert!(result.is_ok());

        let is_system: i32 = conn
            .query_row(
                "SELECT is_system FROM artifact_buckets WHERE id = 'bucket-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(is_system, 1);
    }

    #[test]
    fn test_artifacts_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifacts'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_artifacts_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create a bucket first
        conn.execute(
            "INSERT INTO artifact_buckets (id, name, config_json)
             VALUES ('bucket-1', 'Test Bucket', '{}')",
            [],
        )
        .unwrap();

        // Insert an artifact with inline content
        let result = conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, bucket_id, created_by, version, metadata_json)
             VALUES ('art-1', 'prd', 'Test PRD', 'inline', 'PRD content here', 'bucket-1', 'user', 1, '{}')",
            [],
        );
        assert!(result.is_ok());

        // Insert an artifact with file content
        let result = conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_path, bucket_id, created_by, version)
             VALUES ('art-2', 'code_change', 'Feature Code', 'file', '/path/to/file.rs', 'bucket-1', 'worker-agent', 1)",
            [],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_artifacts_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        for index_name in &["idx_artifacts_bucket", "idx_artifacts_type", "idx_artifacts_task"] {
            let count: i32 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{}'",
                        index_name
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Index {} should exist", index_name);
        }
    }

    #[test]
    fn test_artifact_relations_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifact_relations'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_artifact_relations_constraints() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create bucket and artifacts
        conn.execute(
            "INSERT INTO artifact_buckets (id, name, config_json) VALUES ('b-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, bucket_id, created_by)
             VALUES ('art-1', 'prd', 'PRD 1', 'inline', 'Content', 'b-1', 'user')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, bucket_id, created_by)
             VALUES ('art-2', 'specification', 'Spec from PRD', 'inline', 'Content', 'b-1', 'agent')",
            [],
        )
        .unwrap();

        // Create a relation
        conn.execute(
            "INSERT INTO artifact_relations (id, from_artifact_id, to_artifact_id, relation_type)
             VALUES ('rel-1', 'art-2', 'art-1', 'derived_from')",
            [],
        )
        .unwrap();

        // Duplicate with same type should fail
        let result = conn.execute(
            "INSERT INTO artifact_relations (id, from_artifact_id, to_artifact_id, relation_type)
             VALUES ('rel-2', 'art-2', 'art-1', 'derived_from')",
            [],
        );
        assert!(result.is_err());

        // Different relation type should succeed
        let result = conn.execute(
            "INSERT INTO artifact_relations (id, from_artifact_id, to_artifact_id, relation_type)
             VALUES ('rel-3', 'art-2', 'art-1', 'related_to')",
            [],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_artifact_relations_cascade_delete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create bucket and artifacts
        conn.execute(
            "INSERT INTO artifact_buckets (id, name, config_json) VALUES ('b-1', 'Test', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, bucket_id, created_by)
             VALUES ('art-1', 'prd', 'PRD', 'inline', 'Content', 'b-1', 'user')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifacts (id, type, name, content_type, content_text, bucket_id, created_by)
             VALUES ('art-2', 'spec', 'Spec', 'inline', 'Content', 'b-1', 'user')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO artifact_relations (id, from_artifact_id, to_artifact_id, relation_type)
             VALUES ('rel-1', 'art-2', 'art-1', 'derived_from')",
            [],
        )
        .unwrap();

        // Delete artifact should cascade to relations
        conn.execute("DELETE FROM artifacts WHERE id = 'art-1'", [])
            .unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM artifact_relations WHERE to_artifact_id = 'art-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_artifact_flows_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='artifact_flows'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_artifact_flows_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let result = conn.execute(
            "INSERT INTO artifact_flows (id, name, trigger_json, steps_json, is_active)
             VALUES ('flow-1', 'Research to Dev', '{\"event\":\"artifact_created\"}', '[{\"type\":\"copy\",\"toBucket\":\"prd-library\"}]', 1)",
            [],
        );
        assert!(result.is_ok());

        let is_active: i32 = conn
            .query_row(
                "SELECT is_active FROM artifact_flows WHERE id = 'flow-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(is_active, 1);
    }

    #[test]
    fn test_processes_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='processes'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_processes_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let result = conn.execute(
            "INSERT INTO processes (id, type, name, config_json, status, current_iteration)
             VALUES ('proc-1', 'research', 'Deep Research', '{\"maxIterations\":200}', 'running', 42)",
            [],
        );
        assert!(result.is_ok());

        let (status, iteration): (String, i32) = conn
            .query_row(
                "SELECT status, current_iteration FROM processes WHERE id = 'proc-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "running");
        assert_eq!(iteration, 42);
    }

    #[test]
    fn test_processes_indexes_exist() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        for index_name in &["idx_processes_status", "idx_processes_type"] {
            let count: i32 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='{}'",
                        index_name
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Index {} should exist", index_name);
        }
    }

    #[test]
    fn test_tasks_extensibility_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create project
        conn.execute(
            "INSERT INTO projects (id, name, working_directory) VALUES ('proj-1', 'Test', '/path')",
            [],
        )
        .unwrap();

        // Create task with all extensibility columns
        let result = conn.execute(
            "INSERT INTO tasks (id, project_id, category, title, external_status, wave, checkpoint_type, phase_id, plan_id, must_haves_json)
             VALUES ('task-1', 'proj-1', 'feature', 'Test Task', 'in_qa', 2, 'human-verify', '01-setup', '01-02', '{\"truths\":[\"tests pass\"]}')",
            [],
        );
        assert!(result.is_ok());

        // Verify columns
        let (external_status, wave, checkpoint_type, phase_id, plan_id): (
            Option<String>,
            Option<i32>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT external_status, wave, checkpoint_type, phase_id, plan_id FROM tasks WHERE id = 'task-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
        assert_eq!(external_status, Some("in_qa".to_string()));
        assert_eq!(wave, Some(2));
        assert_eq!(checkpoint_type, Some("human-verify".to_string()));
        assert_eq!(phase_id, Some("01-setup".to_string()));
        assert_eq!(plan_id, Some("01-02".to_string()));
    }

    #[test]
    fn test_tasks_wave_index_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_wave'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_tasks_external_status_index_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_tasks_external_status'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_methodology_extensions_table_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='methodology_extensions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_methodology_extensions_table_columns() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let result = conn.execute(
            "INSERT INTO methodology_extensions (id, name, description, config_json, is_active)
             VALUES ('bmad', 'BMAD Method', 'Breakthrough Method for Agile AI-Driven Development', '{\"workflow\":{},\"agents\":[]}', 1)",
            [],
        );
        assert!(result.is_ok());

        let (name, is_active): (String, i32) = conn
            .query_row(
                "SELECT name, is_active FROM methodology_extensions WHERE id = 'bmad'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, "BMAD Method");
        assert_eq!(is_active, 1);
    }

    #[test]
    fn test_methodology_extensions_index_exists() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_methodology_extensions_active'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_extensibility_migrations_complete() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Verify all extensibility tables exist
        let tables = vec![
            "workflows",
            "artifact_buckets",
            "artifacts",
            "artifact_relations",
            "artifact_flows",
            "processes",
            "methodology_extensions",
        ];

        for table in tables {
            let count: i32 = conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Table {} should exist", table);
        }

        // Verify schema version
        let version = get_schema_version(&conn).unwrap();
        assert_eq!(version, 19);
    }
}
