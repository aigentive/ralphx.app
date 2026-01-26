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

/// Tool call extracted from the stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
}

// ============================================================================
// Stream Events (what the processor emits)
// ============================================================================

/// Events emitted during stream processing
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text chunk received
    TextChunk(String),
    /// Tool call started (name and id available)
    ToolCallStarted {
        name: String,
        id: Option<String>,
    },
    /// Tool call completed (arguments parsed)
    ToolCallCompleted(ToolCall),
    /// Tool result received (from user message with tool_result)
    ToolResultReceived {
        tool_use_id: String,
        result: serde_json::Value,
    },
    /// Session ID received (from Result or Assistant message)
    SessionId(String),
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
    /// Accumulated response text
    pub response_text: String,
    /// Completed tool calls
    pub tool_calls: Vec<ToolCall>,
    /// Claude session ID for --resume
    pub session_id: Option<String>,

    // Internal state for partial tool calls
    current_tool_name: String,
    current_tool_id: Option<String>,
    current_tool_input: String,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a stream-json line
    pub fn parse_line(line: &str) -> Option<StreamMessage> {
        serde_json::from_str(line).ok()
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
                    self.current_tool_name = content_block.name.unwrap_or_default();
                    self.current_tool_id = content_block.id;
                    self.current_tool_input.clear();

                    events.push(StreamEvent::ToolCallStarted {
                        name: self.current_tool_name.clone(),
                        id: self.current_tool_id.clone(),
                    });
                }
            }
            StreamMessage::ContentBlockDelta { delta, .. } => {
                if delta.delta_type == "text_delta" {
                    if let Some(text) = delta.text {
                        self.response_text.push_str(&text);
                        events.push(StreamEvent::TextChunk(text));
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
                        arguments: args,
                        result: None,
                    };

                    self.tool_calls.push(tool_call.clone());
                    events.push(StreamEvent::ToolCallCompleted(tool_call));

                    // Reset tool state
                    self.current_tool_name.clear();
                    self.current_tool_id = None;
                    self.current_tool_input.clear();
                }
            }
            StreamMessage::Result { session_id, .. } => {
                if let Some(ref id) = session_id {
                    self.session_id = session_id.clone();
                    events.push(StreamEvent::SessionId(id.clone()));
                }
            }
            StreamMessage::Assistant { message, session_id } => {
                // Handle --verbose mode assistant messages (full content in one message)
                for content in message.content {
                    match content {
                        AssistantContent::Text { text } => {
                            self.response_text.push_str(&text);
                            events.push(StreamEvent::TextChunk(text));
                        }
                        AssistantContent::ToolUse { id, name, input } => {
                            let tool_call = ToolCall {
                                id: Some(id.clone()),
                                name: name.clone(),
                                arguments: input,
                                result: None,
                            };

                            self.tool_calls.push(tool_call.clone());
                            events.push(StreamEvent::ToolCallCompleted(tool_call));
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
                            // Emit event with updated tool call
                            events.push(StreamEvent::ToolResultReceived {
                                tool_use_id,
                                result: content,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        events
    }

    /// Get the final result after stream is complete
    pub fn finish(self) -> StreamResult {
        StreamResult {
            response_text: self.response_text,
            tool_calls: self.tool_calls,
            session_id: self.session_id,
        }
    }
}

/// Final result from processing a stream
#[derive(Debug, Clone)]
pub struct StreamResult {
    pub response_text: String,
    pub tool_calls: Vec<ToolCall>,
    pub session_id: Option<String>,
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

        assert!(msg.is_some());
        if let Some(StreamMessage::ContentBlockDelta { delta, .. }) = msg {
            assert_eq!(delta.delta_type, "text_delta");
            assert_eq!(delta.text, Some("Hello".to_string()));
        } else {
            panic!("Expected ContentBlockDelta");
        }
    }

    #[test]
    fn test_parse_tool_use_start() {
        let line = r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"toolu_123","name":"create_task_proposal"}}"#;
        let msg = StreamProcessor::parse_line(line);

        assert!(msg.is_some());
        if let Some(StreamMessage::ContentBlockStart { content_block, .. }) = msg {
            assert_eq!(content_block.block_type, "tool_use");
            assert_eq!(content_block.name, Some("create_task_proposal".to_string()));
            assert_eq!(content_block.id, Some("toolu_123".to_string()));
        } else {
            panic!("Expected ContentBlockStart");
        }
    }

    #[test]
    fn test_parse_result() {
        let line = r#"{"type":"result","session_id":"550e8400-e29b-41d4-a716-446655440000","result":"Done","is_error":false,"cost_usd":0.05}"#;
        let msg = StreamProcessor::parse_line(line);

        assert!(msg.is_some());
        if let Some(StreamMessage::Result { session_id, .. }) = msg {
            assert_eq!(session_id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
        } else {
            panic!("Expected Result");
        }
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}],"stop_reason":"end_turn"},"session_id":"sess-123"}"#;
        let msg = StreamProcessor::parse_line(line);

        assert!(msg.is_some());
        if let Some(StreamMessage::Assistant { message, session_id }) = msg {
            assert_eq!(session_id, Some("sess-123".to_string()));
            assert_eq!(message.content.len(), 1);
            if let AssistantContent::Text { text } = &message.content[0] {
                assert_eq!(text, "Hello world");
            } else {
                panic!("Expected Text content");
            }
        } else {
            panic!("Expected Assistant message");
        }
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
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        assert!(json.contains("toolu_01ABC"));
        assert!(json.contains("create_task_proposal"));

        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "create_task_proposal");
    }
}
