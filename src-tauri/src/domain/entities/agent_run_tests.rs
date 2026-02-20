use super::*;

use super::*;
use crate::domain::entities::ChatConversationId;

#[test]
fn test_agent_run_id_creation() {
    let id1 = AgentRunId::new();
    let id2 = AgentRunId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_agent_run_id_from_string() {
    let id = AgentRunId::new();
    let str_id = id.to_string();
    let parsed_id: AgentRunId = str_id.parse().unwrap();
    assert_eq!(id, parsed_id);
}

#[test]
fn test_status_serialization() {
    assert_eq!(AgentRunStatus::Running.to_string(), "running");
    assert_eq!(AgentRunStatus::Completed.to_string(), "completed");
    assert_eq!(AgentRunStatus::Failed.to_string(), "failed");
    assert_eq!(AgentRunStatus::Cancelled.to_string(), "cancelled");
}

#[test]
fn test_status_parsing() {
    assert_eq!(
        "running".parse::<AgentRunStatus>().unwrap(),
        AgentRunStatus::Running
    );
    assert_eq!(
        "completed".parse::<AgentRunStatus>().unwrap(),
        AgentRunStatus::Completed
    );
    assert_eq!(
        "failed".parse::<AgentRunStatus>().unwrap(),
        AgentRunStatus::Failed
    );
    assert_eq!(
        "cancelled".parse::<AgentRunStatus>().unwrap(),
        AgentRunStatus::Cancelled
    );
    assert!("invalid".parse::<AgentRunStatus>().is_err());
}

#[test]
fn test_status_is_terminal() {
    assert!(!AgentRunStatus::Running.is_terminal());
    assert!(AgentRunStatus::Completed.is_terminal());
    assert!(AgentRunStatus::Failed.is_terminal());
    assert!(AgentRunStatus::Cancelled.is_terminal());
}

#[test]
fn test_status_is_active() {
    assert!(AgentRunStatus::Running.is_active());
    assert!(!AgentRunStatus::Completed.is_active());
    assert!(!AgentRunStatus::Failed.is_active());
    assert!(!AgentRunStatus::Cancelled.is_active());
}

#[test]
fn test_new_agent_run() {
    let conversation_id = ChatConversationId::new();
    let run = AgentRun::new(conversation_id);

    assert_eq!(run.conversation_id, conversation_id);
    assert_eq!(run.status, AgentRunStatus::Running);
    assert!(run.is_active());
    assert!(!run.is_terminal());
    assert_eq!(run.completed_at, None);
    assert_eq!(run.error_message, None);
    assert!(run.run_chain_id.is_some());
    assert_eq!(run.parent_run_id, None);
}

#[test]
fn test_new_continuation_run() {
    let conversation_id = ChatConversationId::new();
    let chain_id = "chain-123".to_string();
    let parent_id = "parent-456".to_string();
    let run = AgentRun::new_continuation(conversation_id, chain_id.clone(), parent_id.clone());

    assert_eq!(run.conversation_id, conversation_id);
    assert_eq!(run.status, AgentRunStatus::Running);
    assert_eq!(run.run_chain_id, Some(chain_id));
    assert_eq!(run.parent_run_id, Some(parent_id));
}

#[test]
fn test_complete_agent_run() {
    let conversation_id = ChatConversationId::new();
    let mut run = AgentRun::new(conversation_id);

    run.complete();

    assert_eq!(run.status, AgentRunStatus::Completed);
    assert!(!run.is_active());
    assert!(run.is_terminal());
    assert!(run.completed_at.is_some());
    assert_eq!(run.error_message, None);
}

#[test]
fn test_fail_agent_run() {
    let conversation_id = ChatConversationId::new();
    let mut run = AgentRun::new(conversation_id);

    run.fail("Connection timeout");

    assert_eq!(run.status, AgentRunStatus::Failed);
    assert!(!run.is_active());
    assert!(run.is_terminal());
    assert!(run.completed_at.is_some());
    assert_eq!(run.error_message, Some("Connection timeout".to_string()));
}

#[test]
fn test_cancel_agent_run() {
    let conversation_id = ChatConversationId::new();
    let mut run = AgentRun::new(conversation_id);

    run.cancel();

    assert_eq!(run.status, AgentRunStatus::Cancelled);
    assert!(!run.is_active());
    assert!(run.is_terminal());
    assert!(run.completed_at.is_some());
    assert_eq!(run.error_message, None);
}

#[test]
fn test_duration() {
    let conversation_id = ChatConversationId::new();
    let mut run = AgentRun::new(conversation_id);

    // Duration is None when running
    assert_eq!(run.duration(), None);

    run.complete();

    // Duration is available after completion
    let duration = run.duration().expect("duration should be available");
    assert!(duration.num_milliseconds() >= 0);
}
