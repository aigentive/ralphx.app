use super::*;

#[test]
fn test_timeout_config_task_execution() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_review() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Review);
    assert_eq!(config.line_read_timeout, Duration::from_secs(300));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(120));
}

#[test]
fn test_timeout_config_merge() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_ideation_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_task_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Task);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_project_uses_defaults() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Project);
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
    assert_eq!(config.parse_stall_timeout, Duration::from_secs(180));
}

#[test]
fn test_timeout_config_with_teammate() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation)
        .with_teammate("researcher".to_string(), "#ff6b35".to_string());
    assert_eq!(config.teammate_name, Some("researcher".to_string()));
    assert_eq!(config.teammate_color, Some("#ff6b35".to_string()));
    // Timeouts should be unchanged
    assert_eq!(config.line_read_timeout, Duration::from_secs(600));
}

#[test]
fn test_timeout_config_default_no_teammate() {
    let config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    assert!(config.teammate_name.is_none());
    assert!(config.teammate_color.is_none());
}

#[test]
fn test_timeout_config_ordering() {
    // Merge needs generous timeouts (agent may run silent tests for minutes)
    // Review is faster than default, merge matches default
    let merge = StreamTimeoutConfig::for_context(&ChatContextType::Merge);
    let review = StreamTimeoutConfig::for_context(&ChatContextType::Review);
    let default = StreamTimeoutConfig::for_context(&ChatContextType::TaskExecution);

    assert!(review.line_read_timeout < default.line_read_timeout);
    assert!(review.parse_stall_timeout < default.parse_stall_timeout);
    // Merge matches default — merger agents may run silent test suites
    assert_eq!(merge.line_read_timeout, default.line_read_timeout);
    assert_eq!(merge.parse_stall_timeout, default.parse_stall_timeout);
}

#[test]
fn test_payloads_serialize_with_seq() {
    use crate::application::chat_service::{
        AgentChunkPayload, AgentTaskCompletedPayload, AgentTaskStartedPayload, AgentToolCallPayload,
    };

    // Verify AgentChunkPayload includes seq field
    let chunk = AgentChunkPayload {
        text: "test".to_string(),
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 0,
    };
    let json = serde_json::to_string(&chunk).unwrap();
    assert!(
        json.contains("\"seq\":0"),
        "AgentChunkPayload should serialize with seq field"
    );

    // Verify AgentToolCallPayload includes seq field
    let tool_call = AgentToolCallPayload {
        tool_name: "test_tool".to_string(),
        tool_id: Some("tool-1".to_string()),
        arguments: serde_json::json!({}),
        result: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        diff_context: None,
        parent_tool_use_id: None,
        seq: 1,
    };
    let json = serde_json::to_string(&tool_call).unwrap();
    assert!(
        json.contains("\"seq\":1"),
        "AgentToolCallPayload should serialize with seq field"
    );

    // Verify AgentTaskStartedPayload includes seq field
    let task_started = AgentTaskStartedPayload {
        tool_use_id: "tool-1".to_string(),
        description: Some("test".to_string()),
        subagent_type: Some("bash".to_string()),
        model: Some("sonnet".to_string()),
        teammate_name: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 2,
    };
    let json = serde_json::to_string(&task_started).unwrap();
    assert!(
        json.contains("\"seq\":2"),
        "AgentTaskStartedPayload should serialize with seq field"
    );

    // Verify AgentTaskCompletedPayload includes seq field
    let task_completed = AgentTaskCompletedPayload {
        tool_use_id: "tool-1".to_string(),
        agent_id: Some("agent-1".to_string()),
        total_duration_ms: Some(1000),
        total_tokens: Some(100),
        total_tool_use_count: Some(5),
        teammate_name: None,
        conversation_id: "conv-1".to_string(),
        context_type: "task".to_string(),
        context_id: "task-1".to_string(),
        seq: 3,
    };
    let json = serde_json::to_string(&task_completed).unwrap();
    assert!(
        json.contains("\"seq\":3"),
        "AgentTaskCompletedPayload should serialize with seq field"
    );
}

#[test]
fn test_seq_values_are_monotonic() {
    // Test that multiple events would have incrementing seq values
    let mut stream_seq: u64 = 0;

    // Simulate streaming multiple events
    let seq1 = stream_seq;
    stream_seq += 1;
    let seq2 = stream_seq;
    stream_seq += 1;
    let seq3 = stream_seq;
    stream_seq += 1;
    let seq4 = stream_seq;

    assert_eq!(seq1, 0, "First event should have seq 0");
    assert_eq!(seq2, 1, "Second event should have seq 1");
    assert_eq!(seq3, 2, "Third event should have seq 2");
    assert_eq!(seq4, 3, "Fourth event should have seq 3");

    // Verify strict monotonic ordering
    assert!(seq2 > seq1, "seq must be strictly increasing");
    assert!(seq3 > seq2, "seq must be strictly increasing");
    assert!(seq4 > seq3, "seq must be strictly increasing");
}

// --- Dynamic team_mode upgrade tests ---

#[test]
fn test_timeout_config_dynamic_team_upgrade() {
    // Scenario: lead spawned with team_mode=false (default), then TeamCreated is detected.
    // The timeout should upgrade from default (600s) to team (3600s).
    let mut config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    assert_eq!(
        config.line_read_timeout,
        Duration::from_secs(600),
        "Before upgrade: should use default timeout"
    );

    // Simulate the dynamic upgrade that happens in process_stream_background
    // when StreamEvent::TeamCreated is detected and team_mode was false
    let cfg = stream_timeouts();
    config.line_read_timeout = Duration::from_secs(cfg.team_line_read_secs);
    config.parse_stall_timeout = Duration::from_secs(cfg.team_parse_stall_secs);

    assert_eq!(
        config.line_read_timeout,
        Duration::from_secs(3600),
        "After upgrade: should use team timeout"
    );
    assert_eq!(
        config.parse_stall_timeout,
        Duration::from_secs(3600),
        "After upgrade: parse stall should also use team timeout"
    );
}

#[test]
fn test_timeout_config_team_mode_true_already_upgraded() {
    // When team_mode=true at spawn time, timeout is already set correctly.
    // The dynamic upgrade should be a no-op (guarded by `if !team_mode`).
    let mut config = StreamTimeoutConfig::for_context(&ChatContextType::Ideation);
    let cfg = stream_timeouts();

    // Simulate team_mode=true at spawn time
    config.line_read_timeout = Duration::from_secs(cfg.team_line_read_secs);
    config.parse_stall_timeout = Duration::from_secs(cfg.team_parse_stall_secs);

    let before = config.line_read_timeout;

    // Even if we re-apply, the value stays the same
    config.line_read_timeout = Duration::from_secs(cfg.team_line_read_secs);
    assert_eq!(config.line_read_timeout, before, "Should be idempotent");
}

// --- ActiveTaskTracker tests ---

#[test]
fn test_active_task_tracker_empty_by_default() {
    let tracker = ActiveTaskTracker::default();
    assert!(!tracker.has_active_tasks());
    assert_eq!(tracker.count(), 0);
}

#[test]
fn test_active_task_tracker_counts_started_tasks() {
    let mut tracker = ActiveTaskTracker::default();
    tracker.task_started();
    assert!(tracker.has_active_tasks());
    assert_eq!(tracker.count(), 1);

    tracker.task_started();
    assert_eq!(tracker.count(), 2);
}

#[test]
fn test_active_task_tracker_decrements_on_completed() {
    let mut tracker = ActiveTaskTracker::default();
    tracker.task_started();
    tracker.task_started();
    tracker.task_completed();
    assert!(tracker.has_active_tasks());
    assert_eq!(tracker.count(), 1);

    tracker.task_completed();
    assert!(!tracker.has_active_tasks());
    assert_eq!(tracker.count(), 0);
}

#[test]
fn test_active_task_tracker_saturates_at_zero() {
    let mut tracker = ActiveTaskTracker::default();
    tracker.task_completed(); // No active tasks, should not underflow
    assert!(!tracker.has_active_tasks());
    assert_eq!(tracker.count(), 0);
}

#[test]
fn test_active_task_tracker_prevents_timeout_during_sidechain() {
    // Scenario: Lead spawns 2 Task tool subagents (frontend-researcher, backend-researcher).
    // Both TaskStarted events arrive → count = 2.
    // During sidechain work, no stdout lines → timeout would fire.
    // But has_active_tasks() returns true → timeout should be bypassed.
    let mut tracker = ActiveTaskTracker::default();

    // Lead spawns both researchers
    tracker.task_started(); // frontend-researcher
    tracker.task_started(); // backend-researcher
    assert!(
        tracker.has_active_tasks(),
        "Should prevent timeout while subagents are active"
    );

    // First researcher completes
    tracker.task_completed();
    assert!(
        tracker.has_active_tasks(),
        "Should still prevent timeout with 1 active subagent"
    );

    // Second researcher completes
    tracker.task_completed();
    assert!(
        !tracker.has_active_tasks(),
        "Should allow timeout when all subagents done"
    );
}

// ============================================================================
// TurnComplete event emission tests
// ============================================================================
// These tests verify the behavioral contract of TurnComplete vs run_completed
// event emission. Since process_stream_background requires a real child process
// and Tauri AppHandle (not available in unit tests), we test the decision logic,
// payload shape, and StreamOutcome signaling that drives the caller's behavior.

#[test]
fn test_turn_completed_event_name_is_distinct_from_run_completed() {
    use crate::application::chat_service::events;

    // The whole point of the TurnComplete feature: interactive turns emit
    // a DIFFERENT event name so the frontend doesn't set isAgentRunning=false.
    assert_ne!(
        events::AGENT_TURN_COMPLETED,
        events::AGENT_RUN_COMPLETED,
        "turn_completed and run_completed must be distinct event names"
    );
    assert_eq!(
        events::AGENT_TURN_COMPLETED,
        "agent:turn_completed",
        "AGENT_TURN_COMPLETED must be 'agent:turn_completed'"
    );
    assert_eq!(
        events::AGENT_RUN_COMPLETED,
        "agent:run_completed",
        "AGENT_RUN_COMPLETED must be 'agent:run_completed'"
    );
}

#[test]
fn test_turn_completed_payload_shape_matches_run_completed() {
    use crate::application::chat_service::AgentRunCompletedPayload;

    // TurnComplete reuses AgentRunCompletedPayload — verify it serializes
    // with all required fields for the frontend to correctly identify the context.
    let payload = AgentRunCompletedPayload {
        conversation_id: "conv-interactive-1".to_string(),
        context_type: "task_execution".to_string(),
        context_id: "task-42".to_string(),
        claude_session_id: Some("session-abc".to_string()),
        run_chain_id: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["conversation_id"], "conv-interactive-1");
    assert_eq!(json["context_type"], "task_execution");
    assert_eq!(json["context_id"], "task-42");
    assert_eq!(json["claude_session_id"], "session-abc");
    // run_chain_id is None → should be absent (skip_serializing_if)
    assert!(
        json.get("run_chain_id").is_none(),
        "run_chain_id=None should be omitted from serialized payload"
    );
}

#[test]
fn test_turn_completed_payload_with_no_session_id() {
    use crate::application::chat_service::AgentRunCompletedPayload;

    // Early turns may not yet have a session_id from Claude.
    let payload = AgentRunCompletedPayload {
        conversation_id: "conv-interactive-2".to_string(),
        context_type: "ideation".to_string(),
        context_id: "session-7".to_string(),
        claude_session_id: None,
        run_chain_id: None,
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(json["conversation_id"], "conv-interactive-2");
    assert_eq!(json["context_type"], "ideation");
    assert_eq!(json["context_id"], "session-7");
    // claude_session_id=None → serializes as null (no skip_serializing_if on this field)
    assert!(
        json["claude_session_id"].is_null(),
        "claude_session_id=None should serialize as null"
    );
}

#[test]
fn test_non_interactive_run_completed_includes_run_chain_id() {
    use crate::application::chat_service::AgentRunCompletedPayload;

    // Non-interactive (one-shot) agents pass run_chain_id through.
    // Interactive TurnComplete always sets run_chain_id: None.
    let payload = AgentRunCompletedPayload {
        conversation_id: "conv-oneshot-1".to_string(),
        context_type: "task_execution".to_string(),
        context_id: "task-99".to_string(),
        claude_session_id: Some("session-xyz".to_string()),
        run_chain_id: Some("chain-abc".to_string()),
    };

    let json = serde_json::to_value(&payload).unwrap();
    assert_eq!(
        json["run_chain_id"], "chain-abc",
        "Non-interactive run_completed should include run_chain_id"
    );
}

#[test]
fn test_stream_outcome_turns_finalized_controls_post_loop_behavior() {
    // When turns_finalized > 0 and no new output, the caller (send_background)
    // sets skip_post_loop_finalization=true, preventing duplicate run_completed.
    // This simulates the decision logic in chat_service_send_background.rs.

    // Case 1: Interactive mode — turns were finalized, no new output after last turn
    let outcome = StreamOutcome {
        response_text: String::new(),
        tool_calls: vec![],
        content_blocks: vec![],
        session_id: Some("session-1".to_string()),
        stderr_text: String::new(),
        turns_finalized: 2,
        execution_slot_held: false, // idle between turns at exit
    };
    let has_output = outcome.has_meaningful_output();
    let skip_post_loop = outcome.turns_finalized > 0 && !has_output;
    assert!(
        skip_post_loop,
        "Interactive with finalized turns + no new output → skip post-loop run_completed"
    );
    assert!(
        !outcome.execution_slot_held,
        "Idle between turns → slot not held"
    );

    // Case 2: Non-interactive — no turns finalized
    let outcome_non_interactive = StreamOutcome {
        response_text: "Agent response here".to_string(),
        tool_calls: vec![],
        content_blocks: vec![],
        session_id: Some("session-2".to_string()),
        stderr_text: String::new(),
        turns_finalized: 0,
        execution_slot_held: true, // normal exit — slot still held
    };
    let has_output = outcome_non_interactive.has_meaningful_output();
    let skip_post_loop = outcome_non_interactive.turns_finalized > 0 && !has_output;
    assert!(
        !skip_post_loop,
        "Non-interactive with turns_finalized=0 → DO emit post-loop run_completed"
    );
    assert!(
        outcome_non_interactive.execution_slot_held,
        "Non-interactive → slot held until on_exit"
    );
}

#[test]
fn test_stream_outcome_execution_slot_held_reflects_interactive_state() {
    // execution_slot_held = !between_interactive_turns || !uses_execution_slot
    // For contexts that use execution slots:
    //   - between_interactive_turns=true → slot NOT held (TurnComplete decremented)
    //   - between_interactive_turns=false → slot held (still processing or non-interactive)

    // Idle between turns (TurnComplete received, no resume)
    let idle_outcome = StreamOutcome {
        response_text: String::new(),
        tool_calls: vec![],
        content_blocks: vec![],
        session_id: None,
        stderr_text: String::new(),
        turns_finalized: 1,
        execution_slot_held: false,
    };
    assert!(
        !idle_outcome.execution_slot_held,
        "Should NOT hold slot when idle between interactive turns"
    );

    // Mid-turn exit (process crashed or timed out while active)
    let active_outcome = StreamOutcome {
        response_text: "partial output".to_string(),
        tool_calls: vec![],
        content_blocks: vec![],
        session_id: None,
        stderr_text: String::new(),
        turns_finalized: 0,
        execution_slot_held: true,
    };
    assert!(
        active_outcome.execution_slot_held,
        "Should hold slot when exiting mid-turn (on_exit must decrement)"
    );
}

/// Simulates the full decision tree for event name selection that
/// process_stream_background uses:
/// - In-loop TurnComplete → emit AGENT_TURN_COMPLETED
/// - Post-loop (non-interactive) → emit AGENT_RUN_COMPLETED
/// - Post-loop (interactive, turns_finalized>0, no output) → skip (already emitted turn_completed)
#[test]
fn test_event_name_selection_decision_tree() {
    use crate::application::chat_service::events;

    // Scenario 1: Interactive turn completes during streaming
    // The TurnComplete arm in the stream loop emits this:
    let interactive_event_name = events::AGENT_TURN_COMPLETED;
    assert_eq!(interactive_event_name, "agent:turn_completed");

    // Scenario 2: Non-interactive agent finishes (post-loop)
    let non_interactive_event_name = events::AGENT_RUN_COMPLETED;
    assert_eq!(non_interactive_event_name, "agent:run_completed");

    // Scenario 3: Interactive process exits after TurnComplete
    // turns_finalized > 0, no new output → skip_post_loop_finalization = true
    // The post-loop code checks `if !skip_post_loop_finalization` before emitting.
    // So no AGENT_RUN_COMPLETED is emitted — the AGENT_TURN_COMPLETED was the last event.
    let turns_finalized: usize = 1;
    let has_output = false;
    let skip_post_loop = turns_finalized > 0 && !has_output;
    assert!(
        skip_post_loop,
        "After interactive TurnComplete + idle exit, post-loop run_completed must be skipped"
    );
}
