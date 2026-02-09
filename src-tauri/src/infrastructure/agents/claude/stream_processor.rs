// Claude CLI Stream Processor
// Shared stream message parsing and content extraction for all services
// that consume Claude CLI stream-json output.

use serde::{Deserialize, Serialize};

// ============================================================================
// Stream Message Types (from Claude CLI stream-json output)
// ============================================================================

/// Parsed stream-json message from Claude CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamMessage {
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: Option<i32>,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: Option<i32>, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: Option<i32> },
    #[serde(rename = "message_start")]
    MessageStart {
        message: Option<serde_json::Value>,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: Option<serde_json::Value>,
        usage: Option<serde_json::Value>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    /// Assistant message with full content (from --verbose mode)
    #[serde(rename = "assistant")]
    Assistant {
        message: AssistantMessage,
        session_id: Option<String>,
    },
    /// Result event containing session_id for --resume support
    #[serde(rename = "result")]
    Result {
        result: Option<String>,
        session_id: Option<String>,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        errors: Vec<String>,
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        cost_usd: f64,
    },
    /// System event (e.g., init messages)
    #[serde(rename = "system")]
    System {
        message: Option<String>,
        session_id: Option<String>,
    },
    /// User message (contains tool results when using MCP)
    #[serde(rename = "user")]
    User {
        message: UserMessage,
    },
    #[serde(other)]
    Other,
}

/// User message structure (contains tool results from MCP tool execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: Vec<UserContent>,
}

/// Content block in user message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UserContent {
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(other)]
    Other,
}

/// Assistant message structure from Claude CLI verbose output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<AssistantContent>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

/// Content block in assistant message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub id: Option<String>,
    pub text: Option<String>,
    pub name: Option<String>,
    pub input: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentDelta {
    #[serde(rename = "type")]
    pub delta_type: String,
    pub text: Option<String>,
    pub partial_json: Option<String>,
}

// ============================================================================
// Tool Call Type
// ============================================================================

/// Diff context captured at ToolCallCompleted for Edit/Write tool calls.
/// Stores old file content so frontend can compute proper diffs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffContext {
    /// Previous file content (None if new file)
    pub old_content: Option<String>,
    /// Resolved file path for reference
    pub file_path: String,
}

/// Tool call extracted from the stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_context: Option<DiffContext>,
}

/// Content block item - preserves order of text and tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockItem {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: Option<String>,
        name: String,
        arguments: serde_json::Value,
        result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        diff_context: Option<serde_json::Value>,
    },
}

// ============================================================================
// Stream Events (what the processor emits)
// ============================================================================

/// Events emitted during stream processing
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text chunk received
    TextChunk(String),
    /// Thinking block from Claude's extended reasoning
    Thinking(String),
    /// Tool call started (name and id available)
    ToolCallStarted {
        name: String,
        id: Option<String>,
        parent_tool_use_id: Option<String>,
    },
    /// Tool call completed (arguments parsed)
    ToolCallCompleted {
        tool_call: ToolCall,
        parent_tool_use_id: Option<String>,
    },
    /// Tool result received (from user message with tool_result)
    ToolResultReceived {
        tool_use_id: String,
        result: serde_json::Value,
        parent_tool_use_id: Option<String>,
    },
    /// Session ID received (from Result or Assistant message)
    SessionId(String),
    /// Task subagent started (detected from Task tool_use)
    TaskStarted {
        tool_use_id: String,
        description: Option<String>,
        subagent_type: Option<String>,
        model: Option<String>,
    },
    /// Task subagent completed (detected from Task tool_result)
    TaskCompleted {
        tool_use_id: String,
        agent_id: Option<String>,
        total_duration_ms: Option<u64>,
        total_tokens: Option<u64>,
        total_tool_use_count: Option<u64>,
    },
}

// ============================================================================
// Stream Processor State
// ============================================================================

/// Accumulator for processing Claude CLI stream output
///
/// This struct handles the stateful parsing of stream-json output,
/// tracking partial tool calls and accumulated text.
#[derive(Debug, Default)]
pub struct StreamProcessor {
    /// Accumulated response text (legacy, for backward compat)
    pub response_text: String,
    /// Completed tool calls (legacy, for backward compat)
    pub tool_calls: Vec<ToolCall>,
    /// Content blocks in order (text and tool calls interleaved)
    pub content_blocks: Vec<ContentBlockItem>,
    /// Claude session ID for --resume
    pub session_id: Option<String>,
    /// Indicates the result message reported an error
    pub result_is_error: bool,
    /// Errors from the result message (if any)
    pub result_errors: Vec<String>,
    /// Optional error subtype from the result message
    pub result_subtype: Option<String>,

    // Internal state for partial tool calls
    current_tool_name: String,
    current_tool_id: Option<String>,
    current_tool_input: String,
    // Track current text accumulation for content blocks
    current_text_block: String,
    // Track if we're in a thinking block (streaming mode)
    in_thinking_block: bool,
    // Accumulated thinking text during streaming
    current_thinking_block: String,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self::default()
    }

    /// Parsed line with optional parent_tool_use_id extracted from top-level JSON
    pub struct ParsedLine {
        pub message: StreamMessage,
        pub parent_tool_use_id: Option<String>,
    }

    /// Parse a stream-json line, extracting parent_tool_use_id from the top-level JSON envelope
    pub fn parse_line(line: &str) -> Option<ParsedLine> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "[DONE]" {
            return None;
        }

        let candidate = if let Some(rest) = trimmed.strip_prefix("data:") {
            rest.trim()
        } else {
            trimmed
        };

        // First extract parent_tool_use_id from raw JSON before typed parsing
        let parent_tool_use_id = serde_json::from_str::<serde_json::Value>(candidate)
            .ok()
            .and_then(|v| {
                v.get("parent_tool_use_id")
                    .and_then(|p| p.as_str())
                    .map(|s| s.to_string())
            });

        let message: StreamMessage = serde_json::from_str(candidate).ok()?;
        Some(ParsedLine {
            message,
            parent_tool_use_id,
        })
    }

    /// Process a stream message and return events
    ///
    /// The returned events can be used by callers to emit Tauri events
    /// or perform other actions as needed.
    pub fn process_message(&mut self, msg: StreamMessage) -> Vec<StreamEvent> {
        let mut events = Vec::new();

        match msg {
            StreamMessage::ContentBlockStart { content_block, .. } => {
                if content_block.block_type == "tool_use" {
                    // Flush any accumulated text before starting tool call
                    if !self.current_text_block.is_empty() {
                        self.content_blocks.push(ContentBlockItem::Text {
                            text: self.current_text_block.clone(),
                        });
                        self.current_text_block.clear();
                    }

                    self.current_tool_name = content_block.name.unwrap_or_default();
                    self.current_tool_id = content_block.id;
                    self.current_tool_input.clear();

                    events.push(StreamEvent::ToolCallStarted {
                        name: self.current_tool_name.clone(),
                        id: self.current_tool_id.clone(),
                    });
                } else if content_block.block_type == "thinking" {
                    // Start of a thinking block - mark state for thinking delta handling
                    self.in_thinking_block = true;
                    self.current_thinking_block.clear();
                }
            }
            StreamMessage::ContentBlockDelta { delta, .. } => {
                if delta.delta_type == "text_delta" {
                    if let Some(text) = delta.text {
                        self.response_text.push_str(&text);
                        self.current_text_block.push_str(&text);
                        events.push(StreamEvent::TextChunk(text));
                    }
                } else if delta.delta_type == "thinking_delta" {
                    // Thinking block delta - accumulate and emit
                    if let Some(text) = delta.text {
                        self.current_thinking_block.push_str(&text);
                        events.push(StreamEvent::Thinking(text));
                    }
                } else if delta.delta_type == "input_json_delta" {
                    if let Some(json) = delta.partial_json {
                        self.current_tool_input.push_str(&json);
                    }
                }
            }
            StreamMessage::ContentBlockStop { .. } => {
                if !self.current_tool_name.is_empty() {
                    // Parse tool arguments
                    let args: serde_json::Value =
                        serde_json::from_str(&self.current_tool_input).unwrap_or_default();

                    let tool_call = ToolCall {
                        id: self.current_tool_id.clone(),
                        name: self.current_tool_name.clone(),
                        arguments: args.clone(),
                        result: None,
                        diff_context: None,
                    };

                    self.tool_calls.push(tool_call.clone());

                    // Add to content blocks in order
                    self.content_blocks.push(ContentBlockItem::ToolUse {
                        id: self.current_tool_id.clone(),
                        name: self.current_tool_name.clone(),
                        arguments: args,
                        result: None,
                        diff_context: None,
                    });

                    events.push(StreamEvent::ToolCallCompleted(tool_call));

                    // Reset tool state
                    self.current_tool_name.clear();
                    self.current_tool_id = None;
                    self.current_tool_input.clear();
                } else if self.in_thinking_block {
                    // End of thinking block - reset state
                    // (thinking content was already emitted as chunks)
                    self.in_thinking_block = false;
                    self.current_thinking_block.clear();
                }
            }
            StreamMessage::Result {
                session_id,
                is_error,
                errors,
                subtype,
                ..
            } => {
                if let Some(ref id) = session_id {
                    self.session_id = session_id.clone();
                    events.push(StreamEvent::SessionId(id.clone()));
                }
                if is_error {
                    self.result_is_error = true;
                    if !errors.is_empty() {
                        self.result_errors = errors;
                    }
                    if subtype.is_some() {
                        self.result_subtype = subtype;
                    }
                }
            }
            StreamMessage::Assistant { message, session_id } => {
                // Handle --verbose mode assistant messages (full content in one message)
                for content in message.content {
                    match content {
                        AssistantContent::Text { text } => {
                            self.response_text.push_str(&text);
                            // Add as content block directly (verbose mode gives us complete blocks)
                            self.content_blocks.push(ContentBlockItem::Text {
                                text: text.clone(),
                            });
                            events.push(StreamEvent::TextChunk(text));
                        }
                        AssistantContent::ToolUse { id, name, input } => {
                            let tool_call = ToolCall {
                                id: Some(id.clone()),
                                name: name.clone(),
                                arguments: input.clone(),
                                result: None,
                                diff_context: None,
                            };

                            self.tool_calls.push(tool_call.clone());
                            // Add as content block
                            self.content_blocks.push(ContentBlockItem::ToolUse {
                                id: Some(id),
                                name,
                                arguments: input,
                                result: None,
                                diff_context: None,
                            });
                            events.push(StreamEvent::ToolCallCompleted(tool_call));
                        }
                        AssistantContent::Thinking { thinking } => {
                            // Emit complete thinking block from verbose mode
                            events.push(StreamEvent::Thinking(thinking));
                        }
                        AssistantContent::Other => {}
                    }
                }

                // Capture session_id if present
                if let Some(ref id) = session_id {
                    self.session_id = session_id.clone();
                    events.push(StreamEvent::SessionId(id.clone()));
                }
            }
            StreamMessage::System { session_id, .. } => {
                if let Some(ref id) = session_id {
                    self.session_id = session_id.clone();
                    events.push(StreamEvent::SessionId(id.clone()));
                }
            }
            StreamMessage::User { message } => {
                // Handle tool results from MCP tool execution
                for content in message.content {
                    if let UserContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error: _,
                    } = content
                    {
                        // Find the tool call by ID and update its result
                        if let Some(tool_call) = self
                            .tool_calls
                            .iter_mut()
                            .find(|tc| tc.id.as_ref() == Some(&tool_use_id))
                        {
                            tool_call.result = Some(content.clone());
                        }

                        // Also update the content_blocks array
                        for block in self.content_blocks.iter_mut() {
                            if let ContentBlockItem::ToolUse { id, result, .. } = block {
                                if id.as_ref() == Some(&tool_use_id) {
                                    *result = Some(content.clone());
                                    break;
                                }
                            }
                        }

                        // Emit event with updated tool call
                        events.push(StreamEvent::ToolResultReceived {
                            tool_use_id,
                            result: content,
                        });
                    }
                }
            }
            _ => {}
        }

        events
    }

    /// Get the final result after stream is complete
    pub fn finish(mut self) -> StreamResult {
        // Flush any remaining text as a content block
        if !self.current_text_block.is_empty() {
            self.content_blocks.push(ContentBlockItem::Text {
                text: self.current_text_block,
            });
        }

        StreamResult {
            response_text: self.response_text,
            tool_calls: self.tool_calls,
            content_blocks: self.content_blocks,
            session_id: self.session_id,
            is_error: self.result_is_error,
            errors: self.result_errors,
            error_subtype: self.result_subtype,
        }
    }
}

/// Final result from processing a stream
#[derive(Debug, Clone)]
pub struct StreamResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    /// Content blocks in order (text and tool calls interleaved)
    pub content_blocks: Vec<ContentBlockItem>,
    pub session_id: Option<String>,
    pub is_error: bool,
    pub errors: Vec<String>,
    pub error_subtype: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_text_delta() {
        let line = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(
            matches!(msg, StreamMessage::ContentBlockDelta { .. }),
            "Expected ContentBlockDelta, got different variant"
        );
        let StreamMessage::ContentBlockDelta { delta, .. } = msg else {
            unreachable!()
        };
        assert_eq!(delta.delta_type, "text_delta");
        assert_eq!(delta.text, Some("Hello".to_string()));
    }

    #[test]
    fn test_parse_line_with_data_prefix() {
        let line = r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"Hi"}}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(matches!(msg, StreamMessage::ContentBlockDelta { .. }));
    }

    #[test]
    fn test_parse_tool_use_start() {
        let line = r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"toolu_123","name":"create_task_proposal"}}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(
            matches!(msg, StreamMessage::ContentBlockStart { .. }),
            "Expected ContentBlockStart, got different variant"
        );
        let StreamMessage::ContentBlockStart { content_block, .. } = msg else {
            unreachable!()
        };
        assert_eq!(content_block.block_type, "tool_use");
        assert_eq!(content_block.name, Some("create_task_proposal".to_string()));
        assert_eq!(content_block.id, Some("toolu_123".to_string()));
    }

    #[test]
    fn test_parse_result() {
        let line = r#"{"type":"result","session_id":"550e8400-e29b-41d4-a716-446655440000","result":"Done","is_error":false,"cost_usd":0.05}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(
            matches!(msg, StreamMessage::Result { .. }),
            "Expected Result, got different variant"
        );
        let StreamMessage::Result { session_id, .. } = msg else {
            unreachable!()
        };
        assert_eq!(session_id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}],"stop_reason":"end_turn"},"session_id":"sess-123"}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(
            matches!(msg, StreamMessage::Assistant { .. }),
            "Expected Assistant message, got different variant"
        );
        let StreamMessage::Assistant { message, session_id } = msg else {
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
        assert!(matches!(events3[0], StreamEvent::ToolCallCompleted(_)));

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
                    AssistantContent::Text { text: "Here's my response".to_string() },
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
        assert!(matches!(&events[1], StreamEvent::ToolCallCompleted(tc) if tc.name == "search"));
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
        assert!(matches!(&events1[0], StreamEvent::ToolCallCompleted(tc) if tc.name == "bash"));

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
        assert!(matches!(&events[0], StreamEvent::Thinking(t) if t == "Deep analysis of the problem..."));
        assert!(matches!(&events[1], StreamEvent::TextChunk(t) if t == "Here's my answer."));
        assert!(matches!(&events[2], StreamEvent::SessionId(id) if id == "sess-456"));
    }

    #[test]
    fn test_parse_thinking_content() {
        // Test parsing thinking content from assistant message JSON
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"Let me think..."}],"stop_reason":"end_turn"},"session_id":"sess-789"}"#;
        let msg = StreamProcessor::parse_line(line);

        let msg = msg.expect("Expected Some(StreamMessage)");
        assert!(
            matches!(msg, StreamMessage::Assistant { .. }),
            "Expected Assistant message, got different variant"
        );
        let StreamMessage::Assistant { message, .. } = msg else {
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
}
