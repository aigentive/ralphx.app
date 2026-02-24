use super::*;

#[test]
fn test_processor_text_accumulation() {
    let mut processor = StreamProcessor::new();

    // Simulate text delta messages
    let msg1 = StreamMessage::ContentBlockDelta {
        index: Some(0),
        delta: ContentDelta {
            delta_type: "text_delta".to_string(),
            text: Some("Hello ".to_string()),
            partial_json: None,
        },
    };
    let msg2 = StreamMessage::ContentBlockDelta {
        index: Some(0),
        delta: ContentDelta {
            delta_type: "text_delta".to_string(),
            text: Some("world!".to_string()),
            partial_json: None,
        },
    };

    let events1 = processor.process_message(msg1);
    let events2 = processor.process_message(msg2);

    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 1);

    let result = processor.finish();
    assert_eq!(result.response_text, "Hello world!");
}

#[test]
fn test_processor_assistant_message() {
    let mut processor = StreamProcessor::new();

    let msg = StreamMessage::Assistant {
        message: AssistantMessage {
            content: vec![
                AssistantContent::Text {
                    text: "Here's my response".to_string(),
                },
                AssistantContent::ToolUse {
                    id: "toolu_456".to_string(),
                    name: "search".to_string(),
                    input: serde_json::json!({"query": "test"}),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
        },
        session_id: Some("session-abc".to_string()),
    };

    let events = processor.process_message(msg);

    // Should emit: TextChunk, ToolCallCompleted, SessionId
    assert_eq!(events.len(), 3);
    assert!(matches!(&events[0], StreamEvent::TextChunk(t) if t == "Here's my response"));
    assert!(
        matches!(&events[1], StreamEvent::ToolCallCompleted { ref tool_call, .. } if tool_call.name == "search")
    );
    assert!(matches!(&events[2], StreamEvent::SessionId(id) if id == "session-abc"));

    let result = processor.finish();
    assert_eq!(result.response_text, "Here's my response");
    assert_eq!(result.tool_calls.len(), 1);
    assert_eq!(result.session_id, Some("session-abc".to_string()));
}

#[test]
fn test_processor_session_id_from_result() {
    let mut processor = StreamProcessor::new();

    let msg = StreamMessage::Result {
        result: Some("Done".to_string()),
        session_id: Some("result-session".to_string()),
        is_error: false,
        errors: Vec::new(),
        subtype: None,
        cost_usd: 0.01,
    };

    let events = processor.process_message(msg);

    // Result message emits both SessionId and TurnComplete for the lead
    assert_eq!(events.len(), 2);
    assert!(matches!(&events[0], StreamEvent::SessionId(id) if id == "result-session"));
    assert!(matches!(
        &events[1],
        StreamEvent::TurnComplete { session_id } if *session_id == Some("result-session".to_string())
    ));

    let result = processor.finish();
    assert_eq!(result.session_id, Some("result-session".to_string()));
}

#[test]
fn test_processor_thinking_block_streaming() {
    let mut processor = StreamProcessor::new();

    // Thinking block start
    let start = StreamMessage::ContentBlockStart {
        index: Some(0),
        content_block: ContentBlock {
            block_type: "thinking".to_string(),
            id: None,
            name: None,
            text: None,
            input: None,
        },
    };

    // Thinking content delta
    let delta1 = StreamMessage::ContentBlockDelta {
        index: Some(0),
        delta: ContentDelta {
            delta_type: "thinking_delta".to_string(),
            text: Some("Let me analyze ".to_string()),
            partial_json: None,
        },
    };

    let delta2 = StreamMessage::ContentBlockDelta {
        index: Some(0),
        delta: ContentDelta {
            delta_type: "thinking_delta".to_string(),
            text: Some("this problem.".to_string()),
            partial_json: None,
        },
    };

    // Thinking block stop
    let stop = StreamMessage::ContentBlockStop { index: Some(0) };

    let events1 = processor.process_message(start);
    assert!(events1.is_empty()); // start doesn't emit event

    let events2 = processor.process_message(delta1);
    assert_eq!(events2.len(), 1);
    assert!(matches!(&events2[0], StreamEvent::Thinking(t) if t == "Let me analyze "));

    let events3 = processor.process_message(delta2);
    assert_eq!(events3.len(), 1);
    assert!(matches!(&events3[0], StreamEvent::Thinking(t) if t == "this problem."));

    let events4 = processor.process_message(stop);
    assert!(events4.is_empty()); // stop doesn't emit event for thinking
}

#[test]
fn test_processor_thinking_block_verbose() {
    let mut processor = StreamProcessor::new();

    let msg = StreamMessage::Assistant {
        message: AssistantMessage {
            content: vec![
                AssistantContent::Thinking {
                    thinking: "Deep analysis of the problem...".to_string(),
                },
                AssistantContent::Text {
                    text: "Here's my answer.".to_string(),
                },
            ],
            stop_reason: Some("end_turn".to_string()),
        },
        session_id: Some("sess-456".to_string()),
    };

    let events = processor.process_message(msg);

    // Should emit: Thinking, TextChunk, SessionId
    assert_eq!(events.len(), 3);
    assert!(
        matches!(&events[0], StreamEvent::Thinking(t) if t == "Deep analysis of the problem...")
    );
    assert!(matches!(&events[1], StreamEvent::TextChunk(t) if t == "Here's my answer."));
    assert!(matches!(&events[2], StreamEvent::SessionId(id) if id == "sess-456"));
}

#[test]
fn test_system_without_subtype_still_works() {
    let mut processor = StreamProcessor::new();

    // Regular system message (no subtype) should still emit SessionId
    let msg = StreamMessage::System {
        message: Some("Init".to_string()),
        session_id: Some("sess-regular".to_string()),
        subtype: None,
        hook_id: None,
        hook_name: None,
        hook_event: None,
        output: None,
        exit_code: None,
        outcome: None,
    };

    let events = processor.process_message(msg);
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], StreamEvent::SessionId(id) if id == "sess-regular"));
}
