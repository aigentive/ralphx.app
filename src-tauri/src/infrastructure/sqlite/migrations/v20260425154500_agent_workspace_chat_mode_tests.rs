use rusqlite::Connection;

use super::v20260425154500_agent_workspace_chat_mode;

fn setup_legacy_agent_workspace_db() -> Connection {
    let conn = Connection::open_in_memory().expect("create in-memory database");
    conn.execute_batch(
        "CREATE TABLE chat_conversations (
            id TEXT PRIMARY KEY
         );

         CREATE TABLE agent_conversation_workspaces (
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
         );

         CREATE INDEX idx_agent_conversation_workspaces_project
            ON agent_conversation_workspaces(project_id);
         CREATE INDEX idx_agent_conversation_workspaces_plan_branch
            ON agent_conversation_workspaces(linked_plan_branch_id);
         CREATE INDEX idx_agent_conversation_workspaces_ideation_session
            ON agent_conversation_workspaces(linked_ideation_session_id);

         INSERT INTO chat_conversations (id) VALUES ('conversation-edit');
         INSERT INTO agent_conversation_workspaces (
            conversation_id,
            project_id,
            mode,
            base_ref_kind,
            base_ref,
            branch_name,
            worktree_path,
            created_at,
            updated_at
         )
         VALUES (
            'conversation-edit',
            'project-1',
            'edit',
            'project_default',
            'main',
            'ralphx/project/agent-conversation-edit',
            '/tmp/agent-conversation-edit',
            '2026-04-25T12:00:00Z',
            '2026-04-25T12:00:00Z'
         );",
    )
    .expect("create legacy schema");
    conn
}

#[test]
fn migration_allows_chat_workspace_mode_and_preserves_existing_rows() {
    let conn = setup_legacy_agent_workspace_db();

    v20260425154500_agent_workspace_chat_mode::migrate(&conn).unwrap();

    let preserved_mode: String = conn
        .query_row(
            "SELECT mode FROM agent_conversation_workspaces WHERE conversation_id = 'conversation-edit'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(preserved_mode, "edit");

    conn.execute(
        "INSERT INTO chat_conversations (id) VALUES ('conversation-chat')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO agent_conversation_workspaces (
            conversation_id,
            project_id,
            mode,
            base_ref_kind,
            base_ref,
            branch_name,
            worktree_path,
            created_at,
            updated_at
        )
        VALUES (
            'conversation-chat',
            'project-1',
            'chat',
            'current_branch',
            'feature/agent-screen',
            'ralphx/project/agent-conversation-chat',
            '/tmp/agent-conversation-chat',
            '2026-04-25T12:01:00Z',
            '2026-04-25T12:01:00Z'
        )",
        [],
    )
    .unwrap();

    let chat_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_conversation_workspaces WHERE mode = 'chat'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(chat_count, 1);
}
