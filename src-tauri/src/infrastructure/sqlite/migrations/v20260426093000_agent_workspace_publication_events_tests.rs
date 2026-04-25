use rusqlite::Connection;

use super::v20260426093000_agent_workspace_publication_events;

fn setup_agent_workspace_db() -> Connection {
    let conn = Connection::open_in_memory().expect("create in-memory database");
    conn.execute_batch(
        "CREATE TABLE chat_conversations (
            id TEXT PRIMARY KEY
         );

         CREATE TABLE agent_conversation_workspaces (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            mode TEXT NOT NULL CHECK (mode IN ('edit', 'chat', 'ideation')),
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

         INSERT INTO chat_conversations (id) VALUES ('conversation-1');
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
            'conversation-1',
            'project-1',
            'edit',
            'project_default',
            'main',
            'ralphx/project/agent-conversation-1',
            '/tmp/agent-conversation-1',
            '2026-04-26T09:00:00Z',
            '2026-04-26T09:00:00Z'
         );",
    )
    .expect("create agent workspace schema");
    conn
}

#[test]
fn migration_creates_durable_publication_event_table() {
    let conn = setup_agent_workspace_db();

    v20260426093000_agent_workspace_publication_events::migrate(&conn).unwrap();

    conn.execute(
        "INSERT INTO agent_conversation_workspace_publication_events (
            id, conversation_id, step, status, summary, classification, created_at
         ) VALUES (
            'event-1',
            'conversation-1',
            'refreshing',
            'started',
            'Refreshing branch',
            NULL,
            '2026-04-26T09:01:00Z'
         )",
        [],
    )
    .unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_conversation_workspace_publication_events WHERE conversation_id = 'conversation-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
