    use super::*;

    #[test]
    fn test_parse_text_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(parsed.parent_tool_use_id.is_none());
        assert!(
            matches!(parsed.message, StreamMessage::ContentBlockDelta { .. }),
            "Expected ContentBlockDelta, got different variant"
        );
        let StreamMessage::ContentBlockDelta { delta, .. } = parsed.message else {
            unreachable!()
        };
        assert_eq!(delta.delta_type, "text_delta");
        assert_eq!(delta.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_parse_line_with_data_prefix() {
        let line =
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(matches!(
            parsed.message,
            StreamMessage::ContentBlockDelta { .. }
        ));
    }

    #[test]
    fn test_parse_tool_use_start() {
        let line = r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"toolu_123","name":"create_task_proposal"}}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(
            matches!(parsed.message, StreamMessage::ContentBlockStart { .. }),
            "Expected ContentBlockStart, got different variant"
        );
        let StreamMessage::ContentBlockStart { content_block, .. } = parsed.message else {
            unreachable!()
        };
        assert_eq!(content_block.block_type, "tool_use");
        assert_eq!(content_block.name, Some("create_task_proposal".to_string()));
        assert_eq!(content_block.id, Some("toolu_123".to_string()));
    }

    #[test]
    fn test_parse_result() {
        let line = r#"{"type":"result","session_id":"550e8400-e29b-41d4-a716-446655440000","result":"Done","is_error":false,"cost_usd":0.05}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(
            matches!(parsed.message, StreamMessage::Result { .. }),
            "Expected Result, got different variant"
        );
        let StreamMessage::Result { session_id, .. } = parsed.message else {
            unreachable!()
        };
        assert_eq!(
            session_id,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}],"stop_reason":"end_turn"},"session_id":"sess-123"}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(
            matches!(parsed.message, StreamMessage::Assistant { .. }),
            "Expected Assistant message, got different variant"
        );
        let StreamMessage::Assistant {
            message,
            session_id,
        } = parsed.message
        else {
            unreachable!()
        };
        assert_eq!(session_id, Some("sess-123".to_string()));
        assert_eq!(message.content.len(), 1);
        assert!(
            matches!(&message.content[0], AssistantContent::Text { .. }),
            "Expected Text content, got different variant"
        );
        let AssistantContent::Text { text } = &message.content[0] else {
            unreachable!()
        };
        assert_eq!(text, "Hello world");
    }

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
    fn test_processor_tool_call() {
        let mut processor = StreamProcessor::new();

        // Tool call start
        let start = StreamMessage::ContentBlockStart {
            index: Some(0),
            content_block: ContentBlock {
                block_type: "tool_use".to_string(),
                id: Some("toolu_123".to_string()),
                name: Some("create_task".to_string()),
                text: None,
                input: None,
            },
        };

        // Tool call input delta
        let delta = StreamMessage::ContentBlockDelta {
            index: Some(0),
            delta: ContentDelta {
                delta_type: "input_json_delta".to_string(),
                text: None,
                partial_json: Some(r#"{"title":"Test"}"#.to_string()),
            },
        };

        // Tool call stop
        let stop = StreamMessage::ContentBlockStop { index: Some(0) };

        let events1 = processor.process_message(start);
        let events2 = processor.process_message(delta);
        let events3 = processor.process_message(stop);

        assert!(matches!(events1[0], StreamEvent::ToolCallStarted { .. }));
        assert!(events2.is_empty()); // input_json_delta doesn't emit events
        assert!(matches!(events3[0], StreamEvent::ToolCallCompleted { .. }));

        let result = processor.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "create_task");
        assert_eq!(result.tool_calls[0].id, Some("toolu_123".to_string()));
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

        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::SessionId(id) if id == "result-session"));

        let result = processor.finish();
        assert_eq!(result.session_id, Some("result-session".to_string()));
    }

    #[test]
    fn test_processor_tool_result() {
        let mut processor = StreamProcessor::new();

        // First, send an assistant message with a tool use
        let assistant_msg = StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_789".to_string(),
                    name: "bash".to_string(),
                    input: serde_json::json!({"command": "pwd"}),
                }],
                stop_reason: None,
            },
            session_id: None,
        };

        let events1 = processor.process_message(assistant_msg);
        assert_eq!(events1.len(), 1);
        assert!(
            matches!(&events1[0], StreamEvent::ToolCallCompleted { ref tool_call, .. } if tool_call.name == "bash")
        );

        // Verify tool call is stored with no result
        assert_eq!(processor.tool_calls.len(), 1);
        assert!(processor.tool_calls[0].result.is_none());

        // Now send a user message with tool result
        let user_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_789".to_string(),
                    content: serde_json::json!("/Users/test/project"),
                    is_error: false,
                }],
            },
        };

        let events2 = processor.process_message(user_msg);
        assert_eq!(events2.len(), 1);
        assert!(matches!(
            &events2[0],
            StreamEvent::ToolResultReceived { tool_use_id, .. } if tool_use_id == "toolu_789"
        ));

        // Verify tool call now has result
        let result = processor.finish();
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "bash");
        assert!(result.tool_calls[0].result.is_some());
        assert_eq!(
            result.tool_calls[0].result,
            Some(serde_json::json!("/Users/test/project"))
        );
    }

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: Some("toolu_01ABC".to_string()),
            name: "create_task_proposal".to_string(),
            arguments: serde_json::json!({"title": "Test task"}),
            result: None,
            diff_context: None,
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("toolu_01ABC"));
        assert!(json.contains("create_task_proposal"));
        // diff_context: None should be skipped via skip_serializing_if
        assert!(!json.contains("diff_context"));

        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "create_task_proposal");
        assert!(parsed.diff_context.is_none());
    }

    #[test]
    fn test_tool_call_with_diff_context_serialization() {
        let tool_call = ToolCall {
            id: Some("toolu_02DEF".to_string()),
            name: "Edit".to_string(),
            arguments: serde_json::json!({"file_path": "src/main.rs", "old_string": "old", "new_string": "new"}),
            result: None,
            diff_context: Some(DiffContext {
                old_content: Some("fn main() {\n    old\n}\n".to_string()),
                file_path: "/project/src/main.rs".to_string(),
            }),
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("diff_context"));
        assert!(json.contains("old_content"));
        assert!(json.contains("/project/src/main.rs"));

        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert!(parsed.diff_context.is_some());
        let ctx = parsed.diff_context.unwrap();
        assert_eq!(ctx.file_path, "/project/src/main.rs");
        assert!(ctx.old_content.is_some());
    }

    #[test]
    fn test_tool_call_diff_context_new_file() {
        let tool_call = ToolCall {
            id: Some("toolu_03GHI".to_string()),
            name: "Write".to_string(),
            arguments: serde_json::json!({"file_path": "src/new.rs", "content": "fn new() {}"}),
            result: None,
            diff_context: Some(DiffContext {
                old_content: None,
                file_path: "/project/src/new.rs".to_string(),
            }),
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        let ctx = parsed.diff_context.unwrap();
        assert!(ctx.old_content.is_none());
        assert_eq!(ctx.file_path, "/project/src/new.rs");
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
    fn test_parse_thinking_content() {
        // Test parsing thinking content from assistant message JSON
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me think..."}],"stop_reason":"end_turn"},"session_id":"sess-789"}"#;
        let parsed = StreamProcessor::parse_line(line);

        let parsed = parsed.expect("Expected Some(ParsedLine)");
        assert!(
            matches!(parsed.message, StreamMessage::Assistant { .. }),
            "Expected Assistant message, got different variant"
        );
        let StreamMessage::Assistant { message, .. } = parsed.message else {
            unreachable!()
        };
        assert_eq!(message.content.len(), 1);
        assert!(
            matches!(&message.content[0], AssistantContent::Thinking { .. }),
            "Expected Thinking content, got different variant"
        );
        let AssistantContent::Thinking { thinking } = &message.content[0] else {
            unreachable!()
        };
        assert_eq!(thinking, "Let me think...");
    }

    // ====================================================================
    // parent_tool_use_id and Task subagent tests
    // ====================================================================

    /// Regression test: teammate Result events must NOT overwrite the lead's session_id.
    /// In team mode, Claude CLI embeds each teammate's result (with a non-None
    /// parent_tool_use_id) into the lead's stdout. The last result event to arrive must
    /// not win — only the lead's own top-level result (parent_tool_use_id = None) sets
    /// the stored session_id.
    #[test]
    fn test_result_session_id_ignored_when_parent_tool_use_id_set() {
        let mut processor = StreamProcessor::new();

        // Teammate result event: has a parent_tool_use_id → session_id must be ignored
        let teammate_result = ParsedLine {
            message: StreamMessage::Result {
                result: Some("teammate done".to_string()),
                session_id: Some("teammate-session-id".to_string()),
                is_error: false,
                errors: Vec::new(),
                subtype: None,
                cost_usd: 0.001,
            },
            parent_tool_use_id: Some("toolu_task_abc".to_string()),
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(teammate_result);

        // No SessionId event should be emitted for a teammate result
        assert!(
            !events.iter().any(|e| matches!(e, StreamEvent::SessionId(_))),
            "Expected no SessionId event from teammate result, but got one"
        );
        assert!(
            processor.session_id.is_none(),
            "session_id must not be set from teammate result event"
        );

        // Lead result event: no parent_tool_use_id → session_id must be captured
        let lead_result = ParsedLine {
            message: StreamMessage::Result {
                result: Some("lead done".to_string()),
                session_id: Some("lead-session-id".to_string()),
                is_error: false,
                errors: Vec::new(),
                subtype: None,
                cost_usd: 0.05,
            },
            parent_tool_use_id: None,
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(lead_result);

        // SessionId event must be emitted for the lead result
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamEvent::SessionId(id) if id == "lead-session-id")),
            "Expected SessionId(\"lead-session-id\") event from lead result"
        );
        assert_eq!(
            processor.session_id,
            Some("lead-session-id".to_string()),
            "session_id must be set from lead result event"
        );
    }

    /// Regression test: teammate Assistant messages must NOT overwrite the lead's session_id.
    #[test]
    fn test_assistant_session_id_ignored_when_parent_tool_use_id_set() {
        let mut processor = StreamProcessor::new();

        // Teammate assistant message: has a parent_tool_use_id → session_id must be ignored
        let teammate_msg = ParsedLine {
            message: StreamMessage::Assistant {
                message: AssistantMessage {
                    content: vec![AssistantContent::Text {
                        text: "teammate response".to_string(),
                    }],
                    stop_reason: Some("end_turn".to_string()),
                },
                session_id: Some("teammate-assistant-session".to_string()),
            },
            parent_tool_use_id: Some("toolu_task_xyz".to_string()),
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(teammate_msg);

        assert!(
            !events.iter().any(|e| matches!(e, StreamEvent::SessionId(_))),
            "Expected no SessionId event from teammate assistant message, but got one"
        );
        assert!(
            processor.session_id.is_none(),
            "session_id must not be set from teammate assistant message"
        );

        // Lead assistant message: no parent_tool_use_id → session_id must be captured
        let lead_msg = ParsedLine {
            message: StreamMessage::Assistant {
                message: AssistantMessage {
                    content: vec![AssistantContent::Text {
                        text: "lead response".to_string(),
                    }],
                    stop_reason: Some("end_turn".to_string()),
                },
                session_id: Some("lead-assistant-session".to_string()),
            },
            parent_tool_use_id: None,
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(lead_msg);

        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamEvent::SessionId(id) if id == "lead-assistant-session")),
            "Expected SessionId(\"lead-assistant-session\") event from lead assistant message"
        );
        assert_eq!(
            processor.session_id,
            Some("lead-assistant-session".to_string()),
            "session_id must be set from lead assistant message"
        );
    }

    #[test]
    fn test_parse_line_extracts_parent_tool_use_id() {
        let line = r#"{"type":"assistant","parent_tool_use_id":"toolu_01CdYLhs","message":{"content":[{"type":"text","text":"subagent text"}],"stop_reason":"end_turn"}}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");

        assert_eq!(
            parsed.parent_tool_use_id,
            Some("toolu_01CdYLhs".to_string())
        );
        assert!(matches!(parsed.message, StreamMessage::Assistant { .. }));
    }

    #[test]
    fn test_parse_line_null_parent_tool_use_id() {
        let line = r#"{"type":"assistant","parent_tool_use_id":null,"message":{"content":[{"type":"text","text":"parent text"}],"stop_reason":"end_turn"}}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");

        assert!(parsed.parent_tool_use_id.is_none());
    }

    #[test]
    fn test_parent_tool_use_id_propagates_to_tool_call_started() {
        let mut processor = StreamProcessor::new();

        let parsed = ParsedLine {
            message: StreamMessage::ContentBlockStart {
                index: Some(0),
                content_block: ContentBlock {
                    block_type: "tool_use".to_string(),
                    id: Some("toolu_sub1".to_string()),
                    name: Some("Grep".to_string()),
                    text: None,
                    input: None,
                },
            },
            parent_tool_use_id: Some("toolu_parent".to_string()),
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(parsed);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ToolCallStarted {
                name,
                id,
                parent_tool_use_id,
            } => {
                assert_eq!(name, "Grep");
                assert_eq!(id, &Some("toolu_sub1".to_string()));
                assert_eq!(parent_tool_use_id, &Some("toolu_parent".to_string()));
            }
            other => panic!("Expected ToolCallStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_parent_tool_use_id_propagates_to_tool_call_completed() {
        let mut processor = StreamProcessor::new();

        // Start tool call
        processor.process_message(StreamMessage::ContentBlockStart {
            index: Some(0),
            content_block: ContentBlock {
                block_type: "tool_use".to_string(),
                id: Some("toolu_sub2".to_string()),
                name: Some("Read".to_string()),
                text: None,
                input: None,
            },
        });

        // Delta
        processor.process_message(StreamMessage::ContentBlockDelta {
            index: Some(0),
            delta: ContentDelta {
                delta_type: "input_json_delta".to_string(),
                text: None,
                partial_json: Some(r#"{"file":"test.rs"}"#.to_string()),
            },
        });

        // Stop with parent_tool_use_id
        let parsed = ParsedLine {
            message: StreamMessage::ContentBlockStop { index: Some(0) },
            parent_tool_use_id: Some("toolu_parent".to_string()),
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(parsed);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ToolCallCompleted {
                tool_call,
                parent_tool_use_id,
            } => {
                assert_eq!(tool_call.name, "Read");
                assert_eq!(parent_tool_use_id, &Some("toolu_parent".to_string()));
            }
            other => panic!("Expected ToolCallCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_task_started_emitted_verbose_mode() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_task1".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "description": "Search codebase",
                        "subagent_type": "Explore",
                        "model": "sonnet",
                        "prompt": "Find all files"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        };

        let events = processor.process_message(msg);

        // Should emit: TaskStarted, ToolCallCompleted
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskStarted {
                tool_use_id,
                description,
                subagent_type,
                model,
                teammate_name,
                team_name,
            } => {
                assert_eq!(tool_use_id, "toolu_task1");
                assert_eq!(description, &Some("Search codebase".to_string()));
                assert_eq!(subagent_type, &Some("Explore".to_string()));
                assert_eq!(model, &Some("sonnet".to_string()));
                assert!(teammate_name.is_none());
                assert!(team_name.is_none());
            }
            other => panic!("Expected TaskStarted, got {:?}", other),
        }
        assert!(matches!(&events[1], StreamEvent::ToolCallCompleted { .. }));
    }

    #[test]
    fn test_task_started_emitted_streaming_mode() {
        let mut processor = StreamProcessor::new();

        // Start Task tool call
        processor.process_message(StreamMessage::ContentBlockStart {
            index: Some(0),
            content_block: ContentBlock {
                block_type: "tool_use".to_string(),
                id: Some("toolu_task2".to_string()),
                name: Some("Task".to_string()),
                text: None,
                input: None,
            },
        });

        // Input delta
        processor.process_message(StreamMessage::ContentBlockDelta {
            index: Some(0),
            delta: ContentDelta {
                delta_type: "input_json_delta".to_string(),
                text: None,
                partial_json: Some(
                    r#"{"description":"Run tests","subagent_type":"Bash","model":"haiku"}"#
                        .to_string(),
                ),
            },
        });

        // Stop
        let events = processor.process_message(StreamMessage::ContentBlockStop { index: Some(0) });

        // Should emit: TaskStarted, ToolCallCompleted
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskStarted {
                tool_use_id,
                description,
                subagent_type,
                model,
                teammate_name,
                team_name,
            } => {
                assert_eq!(tool_use_id, "toolu_task2");
                assert_eq!(description, &Some("Run tests".to_string()));
                assert_eq!(subagent_type, &Some("Bash".to_string()));
                assert_eq!(model, &Some("haiku".to_string()));
                assert!(teammate_name.is_none());
                assert!(team_name.is_none());
            }
            other => panic!("Expected TaskStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_task_started_with_team_args_streaming_mode() {
        let mut processor = StreamProcessor::new();

        // Simulate streaming: tool_use start for Task with team args
        processor.process_message(StreamMessage::ContentBlockStart {
            index: Some(0),
            content_block: ContentBlock {
                block_type: "tool_use".to_string(),
                id: Some("toolu_team1".to_string()),
                name: Some("Task".to_string()),
                text: None,
                input: None,
            },
        });

        // Stream the input JSON with team_name and name args
        processor.process_message(StreamMessage::ContentBlockDelta {
            index: Some(0),
            delta: ContentDelta {
                delta_type: "input_json_delta".to_string(),
                text: None,
                partial_json: Some(
                    r#"{"prompt":"do stuff","subagent_type":"general-purpose","team_name":"my-team","name":"researcher","model":"sonnet"}"#
                        .to_string(),
                ),
            },
        });

        // Stop
        let events = processor.process_message(StreamMessage::ContentBlockStop { index: Some(0) });

        // Should emit: TaskStarted, ToolCallCompleted
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskStarted {
                tool_use_id,
                description: _,
                subagent_type,
                model,
                teammate_name,
                team_name,
            } => {
                assert_eq!(tool_use_id, "toolu_team1");
                assert_eq!(subagent_type, &Some("general-purpose".to_string()));
                assert_eq!(model, &Some("sonnet".to_string()));
                assert_eq!(teammate_name, &Some("researcher".to_string()));
                assert_eq!(team_name, &Some("my-team".to_string()));
            }
            other => panic!("Expected TaskStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_task_started_without_team_args_has_none() {
        let mut processor = StreamProcessor::new();

        // Verbose mode: Task tool without team args
        let events = processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_notm".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "prompt": "search files",
                        "subagent_type": "Explore",
                        "description": "Find config"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Should emit TaskStarted in verbose mode
        assert!(!events.is_empty());
        match &events[0] {
            StreamEvent::TaskStarted {
                teammate_name,
                team_name,
                ..
            } => {
                assert!(teammate_name.is_none());
                assert!(team_name.is_none());
            }
            other => panic!("Expected TaskStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_task_completed_emitted_on_tool_result() {
        let mut processor = StreamProcessor::new();

        // First, register a Task tool_use
        let task_msg = StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_task3".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "description": "Search files",
                        "subagent_type": "Explore"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        };
        processor.process_message(task_msg);

        // Now send the tool_result with metadata
        let result_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_task3".to_string(),
                    content: serde_json::json!({
                        "tool_use_result": {
                            "agentId": "agent-abc-123",
                            "totalDurationMs": 12500,
                            "totalTokens": 4500,
                            "totalToolUseCount": 8
                        }
                    }),
                    is_error: false,
                }],
            },
        };

        let events = processor.process_message(result_msg);

        // Should emit: TaskCompleted, ToolResultReceived
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskCompleted {
                tool_use_id,
                agent_id,
                total_duration_ms,
                total_tokens,
                total_tool_use_count,
            } => {
                assert_eq!(tool_use_id, "toolu_task3");
                assert_eq!(agent_id, &Some("agent-abc-123".to_string()));
                assert_eq!(total_duration_ms, &Some(12500));
                assert_eq!(total_tokens, &Some(4500));
                assert_eq!(total_tool_use_count, &Some(8));
            }
            other => panic!("Expected TaskCompleted, got {:?}", other),
        }
        assert!(matches!(&events[1], StreamEvent::ToolResultReceived { .. }));
    }

    #[test]
    fn test_non_task_tool_use_does_not_emit_task_started() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_other".to_string(),
                    name: "Grep".to_string(),
                    input: serde_json::json!({"pattern": "test"}),
                }],
                stop_reason: None,
            },
            session_id: None,
        };

        let events = processor.process_message(msg);
        // Should only emit ToolCallCompleted, NOT TaskStarted
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], StreamEvent::ToolCallCompleted { .. }));
    }

    #[test]
    fn test_parent_tool_use_id_propagates_to_tool_result() {
        let mut processor = StreamProcessor::new();

        // Register a tool call
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_sub_result".to_string(),
                    name: "Grep".to_string(),
                    input: serde_json::json!({"pattern": "foo"}),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Send tool result with parent_tool_use_id
        let parsed = ParsedLine {
            message: StreamMessage::User {
                message: UserMessage {
                    content: vec![UserContent::ToolResult {
                        tool_use_id: "toolu_sub_result".to_string(),
                        content: serde_json::json!("found 3 matches"),
                        is_error: false,
                    }],
                },
            },
            parent_tool_use_id: Some("toolu_parent_task".to_string()),
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(parsed);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::ToolResultReceived {
                tool_use_id,
                parent_tool_use_id,
                ..
            } => {
                assert_eq!(tool_use_id, "toolu_sub_result");
                assert_eq!(parent_tool_use_id, &Some("toolu_parent_task".to_string()));
            }
            other => panic!("Expected ToolResultReceived, got {:?}", other),
        }
    }

    // ====================================================================
    // <usage> text format parsing tests
    // ====================================================================

    #[test]
    fn test_parse_usage_text_basic() {
        let text = "Some output\nagentId: a7db0f4 (for resuming...)\n<usage>total_tokens: 12345\ntool_uses: 8\nduration_ms: 45000</usage>";
        let (agent_id, duration, tokens, tools) = parse_usage_text(text);

        assert_eq!(agent_id, Some("a7db0f4".to_string()));
        assert_eq!(duration, Some(45000));
        assert_eq!(tokens, Some(12345));
        assert_eq!(tools, Some(8));
    }

    #[test]
    fn test_parse_usage_text_no_usage_block() {
        let text = "Just some plain text output\nagentId: abc123";
        let (agent_id, duration, tokens, tools) = parse_usage_text(text);

        assert_eq!(agent_id, Some("abc123".to_string()));
        assert_eq!(duration, None);
        assert_eq!(tokens, None);
        assert_eq!(tools, None);
    }

    #[test]
    fn test_parse_usage_text_no_agent_id() {
        let text = "<usage>total_tokens: 500\ntool_uses: 2\nduration_ms: 3000</usage>";
        let (agent_id, duration, tokens, tools) = parse_usage_text(text);

        assert_eq!(agent_id, None);
        assert_eq!(duration, Some(3000));
        assert_eq!(tokens, Some(500));
        assert_eq!(tools, Some(2));
    }

    #[test]
    fn test_value_to_text_string() {
        let val = serde_json::json!("plain text result");
        assert_eq!(value_to_text(&val), "plain text result");
    }

    #[test]
    fn test_value_to_text_content_blocks() {
        let val = serde_json::json!([
            {"type": "text", "text": "output line 1"},
            {"type": "tool_use", "id": "t1", "name": "Read"},
            {"type": "text", "text": "agentId: abc\n<usage>total_tokens: 100\ntool_uses: 1\nduration_ms: 2000</usage>"}
        ]);
        let text = value_to_text(&val);
        assert!(text.contains("output line 1"));
        assert!(text.contains("<usage>"));
        assert!(text.contains("agentId: abc"));
    }

    #[test]
    fn test_task_completed_parses_usage_text_format() {
        let mut processor = StreamProcessor::new();

        // Register a Task tool_use
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_task_text".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "description": "Search codebase",
                        "subagent_type": "Explore"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Send tool_result as plain text with <usage> block (actual Claude CLI format)
        let result_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_task_text".to_string(),
                    content: serde_json::json!(
                        "Found 3 matching files in src/components/\nagentId: a7db0f4 (for resuming to continue this agent's work if needed)\n<usage>total_tokens: 44969\ntool_uses: 12\nduration_ms: 7900</usage>"
                    ),
                    is_error: false,
                }],
            },
        };

        let events = processor.process_message(result_msg);

        // Should emit: TaskCompleted, ToolResultReceived
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskCompleted {
                tool_use_id,
                agent_id,
                total_duration_ms,
                total_tokens,
                total_tool_use_count,
            } => {
                assert_eq!(tool_use_id, "toolu_task_text");
                assert_eq!(agent_id, &Some("a7db0f4".to_string()));
                assert_eq!(total_duration_ms, &Some(7900));
                assert_eq!(total_tokens, &Some(44969));
                assert_eq!(total_tool_use_count, &Some(12));
            }
            other => panic!("Expected TaskCompleted, got {:?}", other),
        }
        assert!(matches!(&events[1], StreamEvent::ToolResultReceived { .. }));
    }

    #[test]
    fn test_task_completed_parses_content_blocks_format() {
        let mut processor = StreamProcessor::new();

        // Register a Task tool_use
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_task_blocks".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "description": "Run tests",
                        "subagent_type": "Bash"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Send tool_result as content block array (text blocks with usage info)
        let result_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_task_blocks".to_string(),
                    content: serde_json::json!([
                        {"type": "text", "text": "All tests passed.\n"},
                        {"type": "text", "text": "agentId: ff0011 (for resuming...)\n<usage>total_tokens: 8000\ntool_uses: 3\nduration_ms: 15000</usage>"}
                    ]),
                    is_error: false,
                }],
            },
        };

        let events = processor.process_message(result_msg);

        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskCompleted {
                tool_use_id,
                agent_id,
                total_duration_ms,
                total_tokens,
                total_tool_use_count,
            } => {
                assert_eq!(tool_use_id, "toolu_task_blocks");
                assert_eq!(agent_id, &Some("ff0011".to_string()));
                assert_eq!(total_duration_ms, &Some(15000));
                assert_eq!(total_tokens, &Some(8000));
                assert_eq!(total_tool_use_count, &Some(3));
            }
            other => panic!("Expected TaskCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_task_completed_no_stats_still_emits_event() {
        let mut processor = StreamProcessor::new();

        // Register a Task tool_use
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_task_nostats".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "description": "Simple task",
                        "subagent_type": "Bash"
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Send tool_result with no stats at all
        let result_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_task_nostats".to_string(),
                    content: serde_json::json!("Just some plain output with no stats"),
                    is_error: false,
                }],
            },
        };

        let events = processor.process_message(result_msg);

        // TaskCompleted should still be emitted, just with None stats
        assert_eq!(events.len(), 2);
        match &events[0] {
            StreamEvent::TaskCompleted {
                tool_use_id,
                agent_id,
                total_duration_ms,
                total_tokens,
                total_tool_use_count,
            } => {
                assert_eq!(tool_use_id, "toolu_task_nostats");
                assert_eq!(agent_id, &None);
                assert_eq!(total_duration_ms, &None);
                assert_eq!(total_tokens, &None);
                assert_eq!(total_tool_use_count, &None);
            }
            other => panic!("Expected TaskCompleted, got {:?}", other),
        }
    }

    // ====================================================================
    // Hook event tests
    // ====================================================================

    #[test]
    fn test_hook_started_from_system_message() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::System {
            message: Some("Running hook...".to_string()),
            session_id: None,
            subtype: Some("hook_started".to_string()),
            hook_id: Some("hook-abc-123".to_string()),
            hook_name: Some("rule-audit.sh".to_string()),
            hook_event: Some("SessionStart".to_string()),
            output: None,
            exit_code: None,
            outcome: None,
        };

        let events = processor.process_message(msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::HookStarted {
                hook_id,
                hook_name,
                hook_event,
            } => {
                assert_eq!(hook_id, "hook-abc-123");
                assert_eq!(hook_name, "rule-audit.sh");
                assert_eq!(hook_event, "SessionStart");
            }
            other => panic!("Expected HookStarted, got {:?}", other),
        }
    }

    #[test]
    fn test_hook_completed_from_system_message() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::System {
            message: Some("Hook completed".to_string()),
            session_id: Some("sess-456".to_string()),
            subtype: Some("hook_response".to_string()),
            hook_id: Some("hook-def-456".to_string()),
            hook_name: Some("lint-fix.sh".to_string()),
            hook_event: Some("PostToolUse".to_string()),
            output: Some("Fixed 3 lint issues".to_string()),
            exit_code: Some(0),
            outcome: Some("success".to_string()),
        };

        let events = processor.process_message(msg);
        // Should emit SessionId + HookCompleted
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], StreamEvent::SessionId(id) if id == "sess-456"));
        match &events[1] {
            StreamEvent::HookCompleted {
                hook_id,
                hook_name,
                hook_event,
                output,
                exit_code,
                outcome,
            } => {
                assert_eq!(hook_id, "hook-def-456");
                assert_eq!(hook_name, "lint-fix.sh");
                assert_eq!(hook_event, "PostToolUse");
                assert_eq!(output, &Some("Fixed 3 lint issues".to_string()));
                assert_eq!(exit_code, &Some(0));
                assert_eq!(outcome, &Some("success".to_string()));
            }
            other => panic!("Expected HookCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_hook_completed_with_error() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::System {
            message: None,
            session_id: None,
            subtype: Some("hook_response".to_string()),
            hook_id: Some("hook-err-789".to_string()),
            hook_name: Some("enforce-rule-manager.sh".to_string()),
            hook_event: Some("Stop".to_string()),
            output: Some("Rule manager has pending issues".to_string()),
            exit_code: Some(1),
            outcome: Some("error".to_string()),
        };

        let events = processor.process_message(msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::HookCompleted {
                exit_code, outcome, ..
            } => {
                assert_eq!(exit_code, &Some(1));
                assert_eq!(outcome, &Some("error".to_string()));
            }
            other => panic!("Expected HookCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_hook_block_from_synthetic_user_message() {
        let mut processor = StreamProcessor::new();

        let parsed = ParsedLine {
            message: StreamMessage::User {
                message: UserMessage {
                    content: vec![UserContent::Text {
                        text: "Stop hook blocked: enforce-rule-manager.sh\nRule manager audit found issues that need fixing.".to_string(),
                    }],
                },
            },
            parent_tool_use_id: None,
            is_synthetic: true,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(parsed);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::HookBlock { reason } => {
                assert!(reason.contains("Stop hook blocked"));
                assert!(reason.contains("enforce-rule-manager.sh"));
            }
            other => panic!("Expected HookBlock, got {:?}", other),
        }
    }

    #[test]
    fn test_non_synthetic_user_text_ignored() {
        let mut processor = StreamProcessor::new();

        // Non-synthetic user text should NOT emit HookBlock
        let parsed = ParsedLine {
            message: StreamMessage::User {
                message: UserMessage {
                    content: vec![UserContent::Text {
                        text: "Some regular user text".to_string(),
                    }],
                },
            },
            parent_tool_use_id: None,
            is_synthetic: false,
            tool_use_result: None,
        };

        let events = processor.process_parsed_line(parsed);
        assert!(
            events.is_empty(),
            "Non-synthetic text should not emit events"
        );
    }

    #[test]
    fn test_hook_started_missing_required_fields() {
        let mut processor = StreamProcessor::new();

        // Missing hook_name — should NOT emit HookStarted
        let msg = StreamMessage::System {
            message: None,
            session_id: None,
            subtype: Some("hook_started".to_string()),
            hook_id: Some("hook-123".to_string()),
            hook_name: None,
            hook_event: Some("SessionStart".to_string()),
            output: None,
            exit_code: None,
            outcome: None,
        };

        let events = processor.process_message(msg);
        assert!(
            events.is_empty(),
            "HookStarted should not emit with missing hook_name"
        );
    }

    #[test]
    fn test_hook_completed_optional_fields_none() {
        let mut processor = StreamProcessor::new();

        let msg = StreamMessage::System {
            message: None,
            session_id: None,
            subtype: Some("hook_response".to_string()),
            hook_id: Some("hook-opt-1".to_string()),
            hook_name: Some("my-hook.sh".to_string()),
            hook_event: Some("PostToolUse".to_string()),
            output: None,
            exit_code: None,
            outcome: None,
        };

        let events = processor.process_message(msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            StreamEvent::HookCompleted {
                output,
                exit_code,
                outcome,
                ..
            } => {
                assert_eq!(output, &None);
                assert_eq!(exit_code, &None);
                assert_eq!(outcome, &None);
            }
            other => panic!("Expected HookCompleted, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_line_extracts_is_synthetic() {
        // Synthetic user message
        let line = r#"{"type":"user","isSynthetic":true,"message":{"content":[{"type":"text","text":"Hook blocked"}]}}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");
        assert!(parsed.is_synthetic);

        // Non-synthetic message (no isSynthetic field)
        let line2 =
            r#"{"type":"user","message":{"content":[{"type":"text","text":"Normal message"}]}}"#;
        let parsed2 = StreamProcessor::parse_line(line2).expect("Expected Some(ParsedLine)");
        assert!(!parsed2.is_synthetic);

        // Explicit isSynthetic: false
        let line3 = r#"{"type":"user","isSynthetic":false,"message":{"content":[{"type":"text","text":"Not synthetic"}]}}"#;
        let parsed3 = StreamProcessor::parse_line(line3).expect("Expected Some(ParsedLine)");
        assert!(!parsed3.is_synthetic);
    }

    #[test]
    fn test_parse_system_hook_started_json() {
        let line = r#"{"type":"system","subtype":"hook_started","hook_id":"h1","hook_name":"audit.sh","hook_event":"SessionStart","message":"Starting hook"}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");
        assert!(matches!(parsed.message, StreamMessage::System { .. }));

        let StreamMessage::System {
            subtype,
            hook_id,
            hook_name,
            hook_event,
            ..
        } = parsed.message
        else {
            unreachable!()
        };
        assert_eq!(subtype, Some("hook_started".to_string()));
        assert_eq!(hook_id, Some("h1".to_string()));
        assert_eq!(hook_name, Some("audit.sh".to_string()));
        assert_eq!(hook_event, Some("SessionStart".to_string()));
    }

    #[test]
    fn test_parse_system_hook_response_json() {
        let line = r#"{"type":"system","subtype":"hook_response","hook_id":"h2","hook_name":"lint.sh","hook_event":"PostToolUse","output":"All clean","exit_code":0,"outcome":"success"}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");

        let StreamMessage::System {
            subtype,
            hook_id,
            hook_name,
            hook_event,
            output,
            exit_code,
            outcome,
            ..
        } = parsed.message
        else {
            unreachable!()
        };
        assert_eq!(subtype, Some("hook_response".to_string()));
        assert_eq!(hook_id, Some("h2".to_string()));
        assert_eq!(hook_name, Some("lint.sh".to_string()));
        assert_eq!(hook_event, Some("PostToolUse".to_string()));
        assert_eq!(output, Some("All clean".to_string()));
        assert_eq!(exit_code, Some(0));
        assert_eq!(outcome, Some("success".to_string()));
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

    // ====================================================================
    // Team event detection tests
    // ====================================================================

    #[test]
    fn test_detect_team_created_from_tool_result() {
        let result = serde_json::json!({
            "team_name": "my-team",
            "team_file_path": "/home/user/.claude/teams/my-team.json",
            "lead_agent_id": "abc123"
        });
        let event = StreamProcessor::detect_team_event("toolu_1", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeamCreated { team_name, config_path } => {
                assert_eq!(team_name, "my-team");
                assert_eq!(config_path, "/home/user/.claude/teams/my-team.json");
            }
            other => panic!("Expected TeamCreated, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_teammate_spawned_from_tool_result() {
        let result = serde_json::json!({
            "status": "teammate_spawned",
            "name": "researcher",
            "teammate_id": "researcher@my-team",
            "agent_id": "def456",
            "model": "sonnet",
            "color": "green",
            "prompt": "Research WebSocket vs SSE transport options",
            "agent_type": "general-purpose"
        });
        let event = StreamProcessor::detect_team_event("toolu_2", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeammateSpawned { teammate_name, team_name, agent_id, model, color, prompt, agent_type } => {
                assert_eq!(teammate_name, "researcher");
                assert_eq!(team_name, "my-team");
                assert_eq!(agent_id, "def456");
                assert_eq!(model, "sonnet");
                assert_eq!(color, "green");
                assert_eq!(prompt, "Research WebSocket vs SSE transport options");
                assert_eq!(agent_type, "general-purpose");
            }
            other => panic!("Expected TeammateSpawned, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_team_message_sent_from_tool_result() {
        let result = serde_json::json!({
            "success": true,
            "recipients": ["researcher"],
            "routing": {
                "sender": "team-lead",
                "content": "Please investigate the bug"
            }
        });
        let event = StreamProcessor::detect_team_event("toolu_3", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeamMessageSent { sender, recipient, content, message_type } => {
                assert_eq!(sender, "team-lead");
                assert_eq!(recipient, Some("researcher".to_string()));
                assert_eq!(content, "Please investigate the bug");
                assert_eq!(message_type, "message");
            }
            other => panic!("Expected TeamMessageSent, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_team_message_sent_broadcast() {
        let result = serde_json::json!({
            "success": true,
            "recipients": ["researcher", "coder", "tester"],
            "routing": {
                "sender": "team-lead",
                "content": "All stop — blocking issue found"
            }
        });
        let event = StreamProcessor::detect_team_event("toolu_4", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeamMessageSent { sender, recipient, message_type, .. } => {
                assert_eq!(sender, "team-lead");
                assert!(recipient.is_none(), "Broadcast should have no single recipient");
                assert_eq!(message_type, "broadcast");
            }
            other => panic!("Expected TeamMessageSent (broadcast), got {:?}", other),
        }
    }

    #[test]
    fn test_detect_team_deleted_from_tool_result() {
        let result = serde_json::json!({
            "team_deleted": true,
            "team_name": "my-team"
        });
        let event = StreamProcessor::detect_team_event("toolu_5", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeamDeleted { team_name } => {
                assert_eq!(team_name, "my-team");
            }
            other => panic!("Expected TeamDeleted, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_team_deleted_with_deleted_flag() {
        let result = serde_json::json!({
            "deleted": true,
            "team_name": "other-team"
        });
        let event = StreamProcessor::detect_team_event("toolu_6", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeamDeleted { team_name } => {
                assert_eq!(team_name, "other-team");
            }
            other => panic!("Expected TeamDeleted, got {:?}", other),
        }
    }

    #[test]
    fn test_detect_no_team_event_for_regular_result() {
        let result = serde_json::json!({
            "output": "Hello world",
            "exit_code": 0
        });
        let event = StreamProcessor::detect_team_event("toolu_7", &result);
        assert!(event.is_none(), "Regular tool result should not produce a team event");
    }

    /// Integration test: verify team events are detected from top-level tool_use_result
    /// in the actual Claude Code stream-json format (where content is text, not structured JSON).
    #[test]
    fn test_teammate_spawned_from_real_stream_format() {
        let mut processor = StreamProcessor::new();

        // Register the Task tool call (assistant message)
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_019oPFhAfvpV3c1Zaw1S9V5e".to_string(),
                    name: "Task".to_string(),
                    input: serde_json::json!({
                        "name": "team-tabs",
                        "team_name": "tabs-vs-spaces-debate",
                        "subagent_type": "general-purpose",
                        "prompt": "You are team-tabs..."
                    }),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Actual Claude Code stream format: content is text array, tool_use_result is top-level
        let parsed = ParsedLine {
            message: StreamMessage::User {
                message: UserMessage {
                    content: vec![UserContent::ToolResult {
                        tool_use_id: "toolu_019oPFhAfvpV3c1Zaw1S9V5e".to_string(),
                        content: serde_json::json!([
                            {"type": "text", "text": "Spawned successfully.\nagent_id: team-tabs@tabs-vs-spaces-debate\nname: team-tabs"}
                        ]),
                        is_error: false,
                    }],
                },
            },
            parent_tool_use_id: None,
            is_synthetic: false,
            tool_use_result: Some(serde_json::json!({
                "status": "teammate_spawned",
                "name": "team-tabs",
                "teammate_id": "team-tabs@tabs-vs-spaces-debate",
                "agent_id": "team-tabs@tabs-vs-spaces-debate",
                "agent_type": "general-purpose",
                "model": "claude-opus-4-6",
                "color": "blue",
                "prompt": "You are team-tabs...",
                "team_name": "tabs-vs-spaces-debate"
            })),
        };

        let events = processor.process_parsed_line(parsed);

        // Should emit: TaskCompleted, ToolResultReceived, TeammateSpawned
        let teammate_event = events.iter().find(|e| matches!(e, StreamEvent::TeammateSpawned { .. }));
        assert!(teammate_event.is_some(), "Expected TeammateSpawned event from tool_use_result, got: {:?}", events);
        match teammate_event.unwrap() {
            StreamEvent::TeammateSpawned { teammate_name, team_name, model, color, prompt, .. } => {
                assert_eq!(teammate_name, "team-tabs");
                assert_eq!(team_name, "tabs-vs-spaces-debate");
                assert_eq!(model, "claude-opus-4-6");
                assert_eq!(color, "blue");
                assert_eq!(prompt, "You are team-tabs...");
            }
            _ => unreachable!(),
        }
    }

    /// Integration test: verify TeamCreated detected from top-level tool_use_result
    #[test]
    fn test_team_created_from_real_stream_format() {
        let mut processor = StreamProcessor::new();

        // Register TeamCreate tool call
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_team_create_real".to_string(),
                    name: "TeamCreate".to_string(),
                    input: serde_json::json!({"team_name": "tabs-vs-spaces-debate"}),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Real format: content is text (JSON-as-string), tool_use_result is structured
        let parsed = ParsedLine {
            message: StreamMessage::User {
                message: UserMessage {
                    content: vec![UserContent::ToolResult {
                        tool_use_id: "toolu_team_create_real".to_string(),
                        content: serde_json::json!([
                            {"type": "text", "text": "{\n  \"team_name\": \"tabs-vs-spaces-debate\",\n  \"team_file_path\": \"/home/.claude/teams/tabs-vs-spaces-debate/config.json\",\n  \"lead_agent_id\": \"team-lead@tabs-vs-spaces-debate\"\n}"}
                        ]),
                        is_error: false,
                    }],
                },
            },
            parent_tool_use_id: None,
            is_synthetic: false,
            tool_use_result: Some(serde_json::json!({
                "team_name": "tabs-vs-spaces-debate",
                "team_file_path": "/home/.claude/teams/tabs-vs-spaces-debate/config.json",
                "lead_agent_id": "team-lead@tabs-vs-spaces-debate"
            })),
        };

        let events = processor.process_parsed_line(parsed);
        let team_event = events.iter().find(|e| matches!(e, StreamEvent::TeamCreated { .. }));
        assert!(team_event.is_some(), "Expected TeamCreated from tool_use_result, got: {:?}", events);
    }

    /// Integration test: parse_line extracts tool_use_result from raw stream JSON
    #[test]
    fn test_parse_line_extracts_tool_use_result() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"toolu_xxx","type":"tool_result","content":[{"type":"text","text":"Spawned."}]}]},"parent_tool_use_id":null,"session_id":"sess1","tool_use_result":{"status":"teammate_spawned","name":"worker","agent_id":"worker@team","model":"sonnet","color":"green","prompt":"Do work","agent_type":"general-purpose","teammate_id":"worker@team","team_name":"my-team"}}"#;
        let parsed = StreamProcessor::parse_line(line).expect("Expected Some(ParsedLine)");
        assert!(parsed.tool_use_result.is_some(), "tool_use_result should be extracted");
        let tur = parsed.tool_use_result.unwrap();
        assert_eq!(tur.get("status").and_then(|s| s.as_str()), Some("teammate_spawned"));
        assert_eq!(tur.get("name").and_then(|s| s.as_str()), Some("worker"));
        assert_eq!(tur.get("team_name").and_then(|s| s.as_str()), Some("my-team"));
    }

    #[test]
    fn test_detect_no_team_event_for_string_result() {
        let result = serde_json::json!("Just a plain string result");
        let event = StreamProcessor::detect_team_event("toolu_8", &result);
        assert!(event.is_none(), "String result should not produce a team event");
    }

    #[test]
    fn test_team_event_emitted_after_tool_result_received() {
        let mut processor = StreamProcessor::new();

        // Register a tool call (simulating TeamCreate being called)
        processor.process_message(StreamMessage::Assistant {
            message: AssistantMessage {
                content: vec![AssistantContent::ToolUse {
                    id: "toolu_team_create".to_string(),
                    name: "TeamCreate".to_string(),
                    input: serde_json::json!({"team_name": "test-team"}),
                }],
                stop_reason: None,
            },
            session_id: None,
        });

        // Send tool result that looks like TeamCreate output
        let result_msg = StreamMessage::User {
            message: UserMessage {
                content: vec![UserContent::ToolResult {
                    tool_use_id: "toolu_team_create".to_string(),
                    content: serde_json::json!({
                        "team_name": "test-team",
                        "team_file_path": "/home/user/.claude/teams/test-team.json",
                        "lead_agent_id": "lead123"
                    }),
                    is_error: false,
                }],
            },
        };

        let events = processor.process_message(result_msg);

        // Should emit: ToolResultReceived, TeamCreated
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], StreamEvent::ToolResultReceived { .. }));
        match &events[1] {
            StreamEvent::TeamCreated { team_name, config_path } => {
                assert_eq!(team_name, "test-team");
                assert_eq!(config_path, "/home/user/.claude/teams/test-team.json");
            }
            other => panic!("Expected TeamCreated, got {:?}", other),
        }
    }
