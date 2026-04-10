use super::{
    codex_tool_call_content_block, format_agent_exit_stderr, process_exit_details,
    provider_session_ref_for_harness, stream_mode_for_harness, upsert_codex_tool_call_snapshot,
    ProcessExitDetails,
};
use crate::domain::agents::{AgentHarnessKind, HarnessStreamMode};
use crate::infrastructure::agents::claude::{ContentBlockItem, ToolCall};
use std::os::unix::process::ExitStatusExt;

#[test]
fn process_exit_details_reports_non_zero_code() {
    let status = ExitStatusExt::from_raw(1 << 8);
    let details = process_exit_details(&status);

    assert_eq!(
        details,
        ProcessExitDetails {
            exit_code: Some(1),
            exit_signal: None,
            success: false,
        }
    );
}

#[test]
fn format_agent_exit_stderr_prefers_stderr_content() {
    let details = ProcessExitDetails {
        exit_code: Some(1),
        exit_signal: None,
        success: false,
    };

    assert_eq!(
        format_agent_exit_stderr(details, "provider exploded"),
        "provider exploded"
    );
}

#[test]
fn format_agent_exit_stderr_uses_signal_name_when_available() {
    let details = ProcessExitDetails {
        exit_code: None,
        exit_signal: Some(9),
        success: false,
    };

    assert_eq!(
        format_agent_exit_stderr(details, ""),
        "Agent process exited with signal 9 (SIGKILL)"
    );
}

#[test]
fn stream_mode_for_harness_routes_known_harnesses() {
    assert_eq!(
        stream_mode_for_harness(AgentHarnessKind::Claude),
        HarnessStreamMode::ClaudeEvents
    );
    assert_eq!(
        stream_mode_for_harness(AgentHarnessKind::Codex),
        HarnessStreamMode::CodexJsonl
    );
}

#[test]
fn provider_session_ref_for_harness_keeps_harness_and_id() {
    let session_ref = provider_session_ref_for_harness(AgentHarnessKind::Codex, "thread-123");

    assert_eq!(session_ref.harness, AgentHarnessKind::Codex);
    assert_eq!(session_ref.provider_session_id, "thread-123");
}

#[test]
fn codex_tool_call_content_block_preserves_orderable_tool_payload() {
    let tool_call = ToolCall {
        id: Some("tool-1".to_string()),
        name: "ralphx::get_task_context".to_string(),
        arguments: serde_json::json!({ "task_id": "task-1" }),
        result: Some(serde_json::json!({ "title": "Task" })),
        diff_context: None,
        stats: None,
    };

    let block = codex_tool_call_content_block(&tool_call);

    match block {
        ContentBlockItem::ToolUse {
            id,
            name,
            arguments,
            result,
            diff_context,
        } => {
            assert_eq!(id.as_deref(), Some("tool-1"));
            assert_eq!(name, "ralphx::get_task_context");
            assert_eq!(arguments, serde_json::json!({ "task_id": "task-1" }));
            assert_eq!(result, Some(serde_json::json!({ "title": "Task" })));
            assert!(diff_context.is_none());
        }
        other => panic!("expected tool_use block, got {other:?}"),
    }
}

#[test]
fn upsert_codex_tool_call_snapshot_updates_existing_tool_call_in_place() {
    let mut tool_calls = vec![ToolCall {
        id: Some("item_1".to_string()),
        name: "ralphx::get_session_plan".to_string(),
        arguments: serde_json::json!({ "session_id": "s1" }),
        result: None,
        diff_context: None,
        stats: None,
    }];
    let mut content_blocks = vec![codex_tool_call_content_block(&tool_calls[0])];

    upsert_codex_tool_call_snapshot(
        &mut tool_calls,
        &mut content_blocks,
        ToolCall {
            id: Some("item_1".to_string()),
            name: "ralphx::get_session_plan".to_string(),
            arguments: serde_json::json!({ "session_id": "s1" }),
            result: Some(serde_json::json!({ "plan": null })),
            diff_context: None,
            stats: None,
        },
    );

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id.as_deref(), Some("item_1"));
    assert_eq!(tool_calls[0].result, Some(serde_json::json!({ "plan": null })));

    assert_eq!(content_blocks.len(), 1);
    match &content_blocks[0] {
        ContentBlockItem::ToolUse { id, result, .. } => {
            assert_eq!(id.as_deref(), Some("item_1"));
            assert_eq!(result, &Some(serde_json::json!({ "plan": null })));
        }
        other => panic!("expected tool_use block, got {other:?}"),
    }
}

#[test]
fn upsert_codex_tool_call_snapshot_appends_new_tool_ids_in_order() {
    let mut tool_calls = Vec::new();
    let mut content_blocks = Vec::new();

    upsert_codex_tool_call_snapshot(
        &mut tool_calls,
        &mut content_blocks,
        ToolCall {
            id: Some("item_1".to_string()),
            name: "ralphx::get_session_plan".to_string(),
            arguments: serde_json::json!({ "session_id": "s1" }),
            result: None,
            diff_context: None,
            stats: None,
        },
    );
    upsert_codex_tool_call_snapshot(
        &mut tool_calls,
        &mut content_blocks,
        ToolCall {
            id: Some("item_2".to_string()),
            name: "ralphx::list_session_proposals".to_string(),
            arguments: serde_json::json!({ "session_id": "s1" }),
            result: None,
            diff_context: None,
            stats: None,
        },
    );

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0].id.as_deref(), Some("item_1"));
    assert_eq!(tool_calls[1].id.as_deref(), Some("item_2"));
    assert_eq!(content_blocks.len(), 2);
}
