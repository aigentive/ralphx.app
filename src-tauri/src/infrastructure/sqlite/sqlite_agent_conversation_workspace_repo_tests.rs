use crate::domain::entities::{
    AgentConversationWorkspace, AgentConversationWorkspaceMode,
    AgentConversationWorkspacePublicationEvent, ChatConversationId, IdeationAnalysisBaseRefKind,
    ProjectId,
};
use crate::domain::repositories::AgentConversationWorkspaceRepository;
use crate::testing::SqliteTestDb;

use super::SqliteAgentConversationWorkspaceRepository;

fn setup_repo() -> (SqliteTestDb, SqliteAgentConversationWorkspaceRepository, ChatConversationId) {
    let db = SqliteTestDb::new("sqlite_agent_conversation_workspace_repo_tests");
    let conversation_id =
        ChatConversationId::from_string("11111111-1111-1111-1111-111111111111");
    db.with_connection(|conn| {
        conn.execute(
            "INSERT INTO chat_conversations (
                id, context_type, context_id, title, message_count, created_at, updated_at
             ) VALUES (
                ?1, 'project', 'project-1', 'Workspace chat', 0,
                '2026-04-26T09:00:00Z', '2026-04-26T09:00:00Z'
             )",
            rusqlite::params![conversation_id.as_str()],
        )
        .unwrap();
    });
    let repo = SqliteAgentConversationWorkspaceRepository::from_shared(db.shared_conn());
    (db, repo, conversation_id)
}

fn make_workspace(conversation_id: ChatConversationId) -> AgentConversationWorkspace {
    AgentConversationWorkspace::new(
        conversation_id,
        ProjectId::from_string("project-1".to_string()),
        AgentConversationWorkspaceMode::Edit,
        IdeationAnalysisBaseRefKind::ProjectDefault,
        "main".to_string(),
        Some("Project default (main)".to_string()),
        Some("base-sha".to_string()),
        "ralphx/project/agent-11111111".to_string(),
        "/tmp/ralphx/agent-11111111".to_string(),
    )
}

#[tokio::test]
async fn publication_events_round_trip_in_created_order() {
    let (_db, repo, conversation_id) = setup_repo();
    repo.create_or_update(make_workspace(conversation_id))
        .await
        .unwrap();

    repo.append_publication_event(AgentConversationWorkspacePublicationEvent::new(
        conversation_id,
        "checking",
        "started",
        "Checking workspace",
        None,
    ))
    .await
    .unwrap();
    repo.append_publication_event(AgentConversationWorkspacePublicationEvent::new(
        conversation_id,
        "needs_agent",
        "failed",
        "Pre-commit hook failed",
        Some("agent_fixable".to_string()),
    ))
    .await
    .unwrap();

    let events = repo
        .list_publication_events(&conversation_id)
        .await
        .unwrap();

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].step, "checking");
    assert_eq!(events[0].summary, "Checking workspace");
    assert_eq!(events[1].classification.as_deref(), Some("agent_fixable"));
}
