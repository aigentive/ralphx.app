use rusqlite::Connection;

use crate::error::AppResult;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_conversation_workspace_publication_events (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            step TEXT NOT NULL,
            status TEXT NOT NULL,
            summary TEXT NOT NULL,
            classification TEXT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(conversation_id) REFERENCES agent_conversation_workspaces(conversation_id) ON DELETE CASCADE
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_workspace_publication_events_conversation
            ON agent_conversation_workspace_publication_events(conversation_id, created_at)",
        [],
    )?;
    Ok(())
}
