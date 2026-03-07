use super::*;

#[test]
fn test_should_skip_error_messages() {
    assert!(ReplayBuilder::should_skip_message(
        &MessageRole::System,
        &format!(
            "{} timeout]",
            crate::application::chat_service::AGENT_ERROR_PREFIX
        )
    ));
    assert!(!ReplayBuilder::should_skip_message(
        &MessageRole::User,
        "Hello"
    ));
}

#[test]
fn test_estimate_tokens() {
    let turn = Turn {
        role: MessageRole::User,
        content: "Hello world".to_string(), // 11 chars ~ 2-3 tokens
        tool_calls: vec![],
        tool_results: vec![],
    };
    let tokens = ReplayBuilder::estimate_tokens(&turn);
    assert_eq!(tokens, 2); // 11 / 4 = 2
}

#[test]
fn test_estimate_tokens_with_tool_calls() {
    let turn = Turn {
        role: MessageRole::Orchestrator,
        content: "Processing".to_string(),
        tool_calls: vec![serde_json::json!({"name": "test"})],
        tool_results: vec![],
    };
    let tokens = ReplayBuilder::estimate_tokens(&turn);
    assert_eq!(tokens, 202); // 10/4 + 200 = 2 + 200
}

#[test]
fn test_build_rehydration_prompt() {
    let replay = ConversationReplay {
        turns: vec![
            Turn {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_calls: vec![],
                tool_results: vec![],
            },
            Turn {
                role: MessageRole::Orchestrator,
                content: "Hi!".to_string(),
                tool_calls: vec![],
                tool_results: vec![],
            },
        ],
        total_tokens: 100,
        is_truncated: false,
    };

    let prompt = build_rehydration_prompt(
        &replay,
        ChatContextType::Ideation,
        "session-123",
        "Continue conversation",
        None,
    );

    assert!(prompt.contains("Hello"));
    assert!(prompt.contains("Hi!"));
    assert!(prompt.contains("<current_message>Continue conversation</current_message>"));
    assert!(prompt.contains("ideation"));
    assert!(prompt.contains("session-123"));
    assert!(prompt.contains("<turn role"));
    assert!(prompt.contains("</turn>"));
    // No ideation_state block when metadata is None
    assert!(!prompt.contains("<ideation_state>"));
}

#[test]
fn test_build_rehydration_prompt_with_ideation_metadata() {
    let replay = ConversationReplay {
        turns: vec![Turn {
            role: MessageRole::User,
            content: "Hello".to_string(),
            tool_calls: vec![],
            tool_results: vec![],
        }],
        total_tokens: 10,
        is_truncated: false,
    };

    let metadata = IdeationRecoveryMetadata {
        session_status: "active".to_string(),
        plan_artifact_id: Some("artifact-abc-123".to_string()),
        proposal_count: 5,
        parent_session_id: Some("parent-xyz".to_string()),
        team_mode: Some("solo".to_string()),
        session_title: Some("Feature Plan".to_string()),
        verification_status: "unverified".to_string(),
        verification_in_progress: false,
        current_round: 0,
    };

    let prompt = build_rehydration_prompt(
        &replay,
        ChatContextType::Ideation,
        "session-123",
        "Continue",
        Some(&metadata),
    );

    // Verify ideation_state block is present with all fields
    assert!(prompt.contains("<ideation_state>"));
    assert!(prompt.contains("<session_status>active</session_status>"));
    assert!(prompt.contains("<plan_artifact_id>artifact-abc-123</plan_artifact_id>"));
    assert!(prompt.contains("<proposal_count>5</proposal_count>"));
    assert!(prompt.contains("<parent_session_id>parent-xyz</parent_session_id>"));
    assert!(prompt.contains("<team_mode>solo</team_mode>"));
    assert!(prompt.contains("<session_title>Feature Plan</session_title>"));
    assert!(prompt.contains("</ideation_state>"));

    // Verify recovery_note is present
    assert!(prompt.contains("<recovery_note>"));
    assert!(prompt.contains("Session recovered from local storage"));
}

#[test]
fn test_build_rehydration_prompt_with_minimal_ideation_metadata() {
    let replay = ConversationReplay {
        turns: vec![],
        total_tokens: 0,
        is_truncated: false,
    };

    // Minimal metadata with only required fields
    let metadata = IdeationRecoveryMetadata {
        session_status: "active".to_string(),
        plan_artifact_id: None,
        proposal_count: 0,
        parent_session_id: None,
        team_mode: None,
        session_title: None,
        verification_status: "unverified".to_string(),
        verification_in_progress: false,
        current_round: 0,
    };

    let prompt = build_rehydration_prompt(
        &replay,
        ChatContextType::Ideation,
        "session-minimal",
        "Start",
        Some(&metadata),
    );

    assert!(prompt.contains("<ideation_state>"));
    assert!(prompt.contains("<session_status>active</session_status>"));
    assert!(prompt.contains("<proposal_count>0</proposal_count>"));
    // Optional fields should NOT appear when None
    assert!(!prompt.contains("<plan_artifact_id>"));
    assert!(!prompt.contains("<parent_session_id>"));
    assert!(!prompt.contains("<team_mode>"));
    assert!(!prompt.contains("<session_title>"));
}

#[test]
fn test_ideation_state_xml_placed_after_instructions() {
    let replay = ConversationReplay {
        turns: vec![],
        total_tokens: 0,
        is_truncated: false,
    };

    let metadata = IdeationRecoveryMetadata {
        session_status: "active".to_string(),
        plan_artifact_id: None,
        proposal_count: 1,
        parent_session_id: None,
        team_mode: None,
        session_title: None,
        verification_status: "unverified".to_string(),
        verification_in_progress: false,
        current_round: 0,
    };

    let prompt = build_rehydration_prompt(
        &replay,
        ChatContextType::Ideation,
        "session-123",
        "Test",
        Some(&metadata),
    );

    // Verify ordering: </instructions> then <ideation_state> then <conversation_history>
    let instructions_end = prompt
        .find("</instructions>")
        .expect("instructions end tag");
    let ideation_start = prompt.find("<ideation_state>").expect("ideation_state tag");
    let history_start = prompt
        .find("<conversation_history>")
        .expect("conversation_history tag");

    assert!(
        instructions_end < ideation_start,
        "ideation_state should come after </instructions>"
    );
    assert!(
        ideation_start < history_start,
        "conversation_history should come after ideation_state"
    );
}
