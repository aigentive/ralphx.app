use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_conversation_workspaces (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            mode TEXT NOT NULL CHECK (mode IN ('edit', 'ideation')),
            base_ref_kind TEXT NOT NULL,
            base_ref TEXT NOT NULL,
            base_display_name TEXT NULL,
            base_commit TEXT NULL,
            branch_name TEXT NOT NULL,
            worktree_path TEXT NOT NULL,
            linked_ideation_session_id TEXT NULL,
            linked_plan_branch_id TEXT NULL,
            publication_pr_number INTEGER NULL,
            publication_pr_url TEXT NULL,
            publication_pr_status TEXT NULL,
            publication_push_status TEXT NULL,
            status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived', 'missing')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(conversation_id) REFERENCES chat_conversations(id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_project
            ON agent_conversation_workspaces(project_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_plan_branch
            ON agent_conversation_workspaces(linked_plan_branch_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_ideation_session
            ON agent_conversation_workspaces(linked_ideation_session_id)",
        [],
    )?;
    Ok(())
}
