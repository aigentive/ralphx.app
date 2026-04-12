use super::{
    codex_tool_call_content_block, flush_content_before_error, format_agent_exit_stderr,
    persist_assistant_message_snapshot, process_exit_details, provider_session_ref_for_harness,
    stream_mode_for_harness, upsert_codex_tool_call_snapshot, ProcessExitDetails,
};
use crate::application::chat_service::chat_service_context::create_assistant_message;
use crate::application::AppState;
use crate::domain::agents::{AgentHarnessKind, HarnessStreamMode};
use crate::domain::entities::{ChatContextType, ChatConversationId, ChatMessageId, IdeationSessionId};
use crate::infrastructure::agents::claude::{
    AssistantContent, AssistantMessage, ContentBlockItem, StreamMessage, StreamProcessor, ToolCall,
};
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
        parent_tool_use_id: Some("toolu-parent-1".to_string()),
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
            parent_tool_use_id,
            diff_context,
        } => {
            assert_eq!(id.as_deref(), Some("tool-1"));
            assert_eq!(name, "ralphx::get_task_context");
            assert_eq!(arguments, serde_json::json!({ "task_id": "task-1" }));
            assert_eq!(result, Some(serde_json::json!({ "title": "Task" })));
            assert_eq!(parent_tool_use_id.as_deref(), Some("toolu-parent-1"));
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
        parent_tool_use_id: Some("toolu-parent-1".to_string()),
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
            parent_tool_use_id: Some("toolu-parent-1".to_string()),
            diff_context: None,
            stats: None,
        },
    );

    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id.as_deref(), Some("item_1"));
    assert_eq!(tool_calls[0].result, Some(serde_json::json!({ "plan": null })));
    assert_eq!(tool_calls[0].parent_tool_use_id.as_deref(), Some("toolu-parent-1"));

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
            parent_tool_use_id: None,
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
            parent_tool_use_id: None,
            diff_context: None,
            stats: None,
        },
    );

    assert_eq!(tool_calls.len(), 2);
    assert_eq!(tool_calls[0].id.as_deref(), Some("item_1"));
    assert_eq!(tool_calls[1].id.as_deref(), Some("item_2"));
    assert_eq!(content_blocks.len(), 2);
}

#[tokio::test]
async fn persist_assistant_message_snapshot_keeps_codex_tool_lifecycle_deduped_and_ordered() {
    let state = AppState::new_test();
    let conversation_id = ChatConversationId::new();
    let context_id = IdeationSessionId::new();
    let assistant_message = create_assistant_message(
        ChatContextType::Ideation,
        context_id.as_str(),
        "",
        conversation_id.clone(),
        &[],
        &[],
    );
    let assistant_message_id = assistant_message.id.as_str().to_string();
    state
        .chat_message_repo
        .create(assistant_message)
        .await
        .expect("insert assistant message");

    let repo = Some(state.chat_message_repo.clone());
    let assistant_message_id_opt = Some(assistant_message_id.clone());

    let mut response_text = "First text block".to_string();
    let mut tool_calls = Vec::new();
    let mut content_blocks = vec![ContentBlockItem::Text {
        text: response_text.clone(),
    }];

    persist_assistant_message_snapshot(
        &repo,
        &assistant_message_id_opt,
        &response_text,
        &tool_calls,
        &content_blocks,
    )
    .await;

    upsert_codex_tool_call_snapshot(
        &mut tool_calls,
        &mut content_blocks,
        ToolCall {
            id: Some("item_1".to_string()),
            name: "ralphx::get_task_context".to_string(),
            arguments: serde_json::json!({ "task_id": "task-1" }),
            result: None,
            parent_tool_use_id: Some("toolu-parent-task".to_string()),
            diff_context: None,
            stats: None,
        },
    );

    persist_assistant_message_snapshot(
        &repo,
        &assistant_message_id_opt,
        &response_text,
        &tool_calls,
        &content_blocks,
    )
    .await;

    upsert_codex_tool_call_snapshot(
        &mut tool_calls,
        &mut content_blocks,
        ToolCall {
            id: Some("item_1".to_string()),
            name: "ralphx::get_task_context".to_string(),
            arguments: serde_json::json!({ "task_id": "task-1" }),
            result: Some(serde_json::json!({ "title": "Task" })),
            parent_tool_use_id: Some("toolu-parent-task".to_string()),
            diff_context: None,
            stats: None,
        },
    );

    response_text.push_str("\n\nSecond text block");
    content_blocks.push(ContentBlockItem::Text {
        text: "Second text block".to_string(),
    });

    flush_content_before_error(
        &repo,
        &assistant_message_id_opt,
        &response_text,
        &tool_calls,
        &content_blocks,
    )
    .await;

    let stored = state
        .chat_message_repo
        .get_by_id(&ChatMessageId::from_string(assistant_message_id))
        .await
        .expect("reload message")
        .expect("assistant message should exist");

    assert_eq!(stored.content, "First text block\n\nSecond text block");

    let stored_tool_calls: Vec<ToolCall> = serde_json::from_str(
        stored
            .tool_calls
            .as_deref()
            .expect("tool_calls should be persisted"),
    )
    .expect("tool_calls JSON should parse");
    assert_eq!(stored_tool_calls.len(), 1);
    assert_eq!(stored_tool_calls[0].id.as_deref(), Some("item_1"));
    assert_eq!(
        stored_tool_calls[0].parent_tool_use_id.as_deref(),
        Some("toolu-parent-task")
    );
    assert_eq!(
        stored_tool_calls[0].result,
        Some(serde_json::json!({ "title": "Task" }))
    );

    let stored_blocks: Vec<ContentBlockItem> = serde_json::from_str(
        stored
            .content_blocks
            .as_deref()
            .expect("content_blocks should be persisted"),
    )
    .expect("content_blocks JSON should parse");
    assert_eq!(stored_blocks.len(), 3);
    match &stored_blocks[0] {
        ContentBlockItem::Text { text } => assert_eq!(text, "First text block"),
        other => panic!("expected first block to be text, got {other:?}"),
    }
    match &stored_blocks[1] {
        ContentBlockItem::ToolUse {
            id,
            result,
            parent_tool_use_id,
            ..
        } => {
            assert_eq!(id.as_deref(), Some("item_1"));
            assert_eq!(result, &Some(serde_json::json!({ "title": "Task" })));
            assert_eq!(parent_tool_use_id.as_deref(), Some("toolu-parent-task"));
        }
        other => panic!("expected second block to be tool_use, got {other:?}"),
    }
    match &stored_blocks[2] {
        ContentBlockItem::Text { text } => assert_eq!(text, "Second text block"),
        other => panic!("expected third block to be text, got {other:?}"),
    }
}

#[tokio::test]
async fn persist_assistant_message_snapshot_keeps_claude_tool_result_ordered_and_in_place() {
    let state = AppState::new_test();
    let conversation_id = ChatConversationId::new();
    let context_id = IdeationSessionId::new();
    let assistant_message = create_assistant_message(
        ChatContextType::Ideation,
        context_id.as_str(),
        "",
        conversation_id.clone(),
        &[],
        &[],
    );
    let assistant_message_id = assistant_message.id.as_str().to_string();
    state
        .chat_message_repo
        .create(assistant_message)
        .await
        .expect("insert assistant message");

    let repo = Some(state.chat_message_repo.clone());
    let assistant_message_id_opt = Some(assistant_message_id.clone());
    let mut processor = StreamProcessor::new();

    processor.process_message(StreamMessage::Assistant {
        message: AssistantMessage {
            content: vec![AssistantContent::Text {
                text: "First text block".to_string(),
            }],
            stop_reason: None,
            usage: None,
        },
        session_id: None,
    });
    persist_assistant_message_snapshot(
        &repo,
        &assistant_message_id_opt,
        &processor.response_text,
        &processor.tool_calls,
        &processor.content_blocks,
    )
    .await;

    processor.process_message(StreamMessage::Assistant {
        message: AssistantMessage {
            content: vec![AssistantContent::ToolUse {
                id: "toolu_1".to_string(),
                name: "bash".to_string(),
                input: serde_json::json!({ "command": "pwd" }),
            }],
            stop_reason: None,
            usage: None,
        },
        session_id: None,
    });
    persist_assistant_message_snapshot(
        &repo,
        &assistant_message_id_opt,
        &processor.response_text,
        &processor.tool_calls,
        &processor.content_blocks,
    )
    .await;

    let parsed_tool_result = StreamProcessor::parse_line(
        r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_1","type":"tool_result","content":"/Users/test/project","is_error":false}]}}"#,
    )
    .expect("tool_result line should parse");
    processor.process_parsed_line(parsed_tool_result);

    processor.process_message(StreamMessage::Assistant {
        message: AssistantMessage {
            content: vec![AssistantContent::Text {
                text: "Second text block".to_string(),
            }],
            stop_reason: None,
            usage: None,
        },
        session_id: None,
    });

    flush_content_before_error(
        &repo,
        &assistant_message_id_opt,
        &processor.response_text,
        &processor.tool_calls,
        &processor.content_blocks,
    )
    .await;

    let stored = state
        .chat_message_repo
        .get_by_id(&ChatMessageId::from_string(assistant_message_id))
        .await
        .expect("reload message")
        .expect("assistant message should exist");

    assert_eq!(stored.content, "First text blockSecond text block");

    let stored_tool_calls: Vec<ToolCall> = serde_json::from_str(
        stored
            .tool_calls
            .as_deref()
            .expect("tool_calls should be persisted"),
    )
    .expect("tool_calls JSON should parse");
    assert_eq!(stored_tool_calls.len(), 1);
    assert_eq!(stored_tool_calls[0].id.as_deref(), Some("toolu_1"));
    assert_eq!(
        stored_tool_calls[0].result,
        Some(serde_json::json!("/Users/test/project"))
    );

    let stored_blocks: Vec<ContentBlockItem> = serde_json::from_str(
        stored
            .content_blocks
            .as_deref()
            .expect("content_blocks should be persisted"),
    )
    .expect("content_blocks JSON should parse");
    assert_eq!(stored_blocks.len(), 3);
    match &stored_blocks[0] {
        ContentBlockItem::Text { text } => assert_eq!(text, "First text block"),
        other => panic!("expected first block to be text, got {other:?}"),
    }
    match &stored_blocks[1] {
        ContentBlockItem::ToolUse { id, result, .. } => {
            assert_eq!(id.as_deref(), Some("toolu_1"));
            assert_eq!(result, &Some(serde_json::json!("/Users/test/project")));
        }
        other => panic!("expected second block to be tool_use, got {other:?}"),
    }
    match &stored_blocks[2] {
        ContentBlockItem::Text { text } => assert_eq!(text, "Second text block"),
        other => panic!("expected third block to be text, got {other:?}"),
    }
}
