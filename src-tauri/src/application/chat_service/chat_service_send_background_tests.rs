use super::session_changed_after_resume;

#[test]
fn session_changed_returns_true_when_ids_differ() {
    assert!(session_changed_after_resume(
        Some("session-old-abc"),
        Some("session-new-xyz"),
    ));
}

#[test]
fn session_changed_returns_false_when_ids_match() {
    assert!(!session_changed_after_resume(
        Some("session-abc"),
        Some("session-abc"),
    ));
}

#[test]
fn session_changed_returns_false_when_no_stored_id() {
    // --resume was not used; no comparison possible
    assert!(!session_changed_after_resume(None, Some("session-new")));
}

#[test]
fn session_changed_returns_false_when_no_new_id() {
    // Stream returned no session ID; cannot detect change
    assert!(!session_changed_after_resume(Some("session-old"), None));
}

#[test]
fn session_changed_returns_false_when_both_none() {
    assert!(!session_changed_after_resume(None, None));
}

/// Verifies that session swap recovery enqueues rehydration at front of queue,
/// preserving ordering: recovery context → pending user messages.
#[test]
fn session_swap_recovery_enqueues_rehydration_before_user_messages() {
    use crate::domain::entities::ChatContextType;
    use crate::domain::services::MessageQueue;

    let queue = MessageQueue::new();

    // Simulate: user queued messages while agent was running
    queue.queue(ChatContextType::Ideation, "ctx-1", "User follow-up 1".to_string());
    queue.queue(ChatContextType::Ideation, "ctx-1", "User follow-up 2".to_string());

    // Session swap detected → recovery enqueues rehydration at front
    let rehydration_content = "<instructions>Your session was recovered</instructions>".to_string();
    queue.queue_front(ChatContextType::Ideation, "ctx-1", rehydration_content.clone());

    // Verify queue order: rehydration first, then user messages
    let queued = queue.get_queued(ChatContextType::Ideation, "ctx-1");
    assert_eq!(queued.len(), 3);
    assert_eq!(queued[0].content, rehydration_content);
    assert_eq!(queued[1].content, "User follow-up 1");
    assert_eq!(queued[2].content, "User follow-up 2");

    // Pop order should match: rehydration processed first via --resume
    let first = queue.pop(ChatContextType::Ideation, "ctx-1").unwrap();
    assert!(first.content.contains("session was recovered"));
}
