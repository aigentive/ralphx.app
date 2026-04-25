use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|error| AppError::Database(error.to_string()))?;

    conn.execute_batch(
        "DROP TABLE IF EXISTS agent_conversation_workspaces_new;

         CREATE TABLE agent_conversation_workspaces_new (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            mode TEXT NOT NULL CHECK (mode IN ('chat', 'edit', 'ideation')),
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
         );

         INSERT INTO agent_conversation_workspaces_new (
            conversation_id,
            project_id,
            mode,
            base_ref_kind,
            base_ref,
            base_display_name,
            base_commit,
            branch_name,
            worktree_path,
            linked_ideation_session_id,
            linked_plan_branch_id,
            publication_pr_number,
            publication_pr_url,
            publication_pr_status,
            publication_push_status,
            status,
            created_at,
            updated_at
         )
         SELECT
            conversation_id,
            project_id,
            mode,
            base_ref_kind,
            base_ref,
            base_display_name,
            base_commit,
            branch_name,
            worktree_path,
            linked_ideation_session_id,
            linked_plan_branch_id,
            publication_pr_number,
            publication_pr_url,
            publication_pr_status,
            publication_push_status,
            status,
            created_at,
            updated_at
         FROM agent_conversation_workspaces;

         DROP TABLE agent_conversation_workspaces;
         ALTER TABLE agent_conversation_workspaces_new RENAME TO agent_conversation_workspaces;

         CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_project
            ON agent_conversation_workspaces(project_id);
         CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_plan_branch
            ON agent_conversation_workspaces(linked_plan_branch_id);
         CREATE INDEX IF NOT EXISTS idx_agent_conversation_workspaces_ideation_session
            ON agent_conversation_workspaces(linked_ideation_session_id);",
    )
    .map_err(|error| AppError::Database(error.to_string()))?;

    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(|error| AppError::Database(error.to_string()))?;

    Ok(())
}
