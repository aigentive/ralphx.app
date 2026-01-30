// Database migrations v11-v20
// Ideation, workflows, artifacts, processes, and methodology support

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

/// Migration v11: Create ideation tables (sessions, proposals, chat)
pub(super) fn migrate_v11(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v12(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v13(conn: &Connection) -> AppResult<()> {
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
/// Note: Drops the old v5 artifacts table and creates a new schema with expanded fields
pub(super) fn migrate_v14(conn: &Connection) -> AppResult<()> {
    // Drop the old artifacts table from v5 (it had a simpler schema)
    conn.execute("DROP TABLE IF EXISTS artifact_flows", [])
        .map_err(|e| AppError::Database(e.to_string()))?;
    conn.execute("DROP TABLE IF EXISTS artifacts", [])
        .map_err(|e| AppError::Database(e.to_string()))?;

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
pub(super) fn migrate_v15(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v16(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v17(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v18(conn: &Connection) -> AppResult<()> {
    // Add category for task categorization (feature, bug, etc.)
    conn.execute(
        "ALTER TABLE tasks ADD COLUMN category TEXT NOT NULL DEFAULT 'feature'",
        [],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;

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
pub(super) fn migrate_v19(conn: &Connection) -> AppResult<()> {
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
pub(super) fn migrate_v20(conn: &Connection) -> AppResult<()> {
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
