use super::{conversation_spawn_harness_override, interactive_run_started_provider_session};
use crate::application::interactive_process_registry::InteractiveProcessMetadata;
use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{ChatConversation, IdeationSessionId, TaskId};

#[test]
fn interactive_run_started_provider_session_prefers_process_metadata_harness() {
    let conversation =
        ChatConversation::new_ideation(IdeationSessionId::from_string("session-1".to_string()));

    let (harness, provider_session_id) = interactive_run_started_provider_session(
        &conversation,
        Some(&InteractiveProcessMetadata {
            harness: Some(AgentHarnessKind::Codex),
            provider_session_id: None,
        }),
    );

    assert_eq!(harness, AgentHarnessKind::Codex);
    assert_eq!(provider_session_id, None);
}

#[test]
fn interactive_run_started_provider_session_falls_back_to_conversation_session_ref() {
    let mut conversation =
        ChatConversation::new_task_execution(TaskId::from_string("task-1".to_string()));
    conversation.set_provider_session_ref(ProviderSessionRef {
        harness: AgentHarnessKind::Claude,
        provider_session_id: "claude-session-123".to_string(),
    });

    let (harness, provider_session_id) =
        interactive_run_started_provider_session(&conversation, None);

    assert_eq!(harness, AgentHarnessKind::Claude);
    assert_eq!(provider_session_id.as_deref(), Some("claude-session-123"));
}

#[test]
fn conversation_spawn_harness_override_falls_back_to_parent_conversation() {
    let task_id = TaskId::from_string("task-parent-1".to_string());
    let child = ChatConversation::new_task_execution(task_id.clone());
    let mut parent = ChatConversation::new_task_execution(task_id);
    parent.set_provider_session_ref(ProviderSessionRef {
        harness: AgentHarnessKind::Codex,
        provider_session_id: "codex-parent-session".to_string(),
    });

    let harness = conversation_spawn_harness_override(&child, Some(&parent));

    assert_eq!(harness, Some(AgentHarnessKind::Codex));
}
