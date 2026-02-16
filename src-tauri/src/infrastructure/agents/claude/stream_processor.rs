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
    ContentBlockDelta {
        index: Option<i32>,
        delta: ContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: Option<i32> },
    #[serde(rename = "message_start")]
    MessageStart { message: Option<serde_json::Value> },
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
    /// System event (e.g., init messages, hook events)
    #[serde(rename = "system")]
    System {
        message: Option<String>,
        session_id: Option<String>,
        #[serde(default)]
        subtype: Option<String>,
        #[serde(default)]
        hook_id: Option<String>,
        #[serde(default)]
        hook_name: Option<String>,
        #[serde(default)]
        hook_event: Option<String>,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        exit_code: Option<i32>,
        #[serde(default)]
        outcome: Option<String>,
    },
    /// User message (contains tool results when using MCP)
    #[serde(rename = "user")]
    User { message: UserMessage },
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
    #[serde(rename = "text")]
    Text { text: String },
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
    /// Hook started (from system message with subtype "hook_started")
    HookStarted {
        hook_id: String,
        hook_name: String,
        hook_event: String,
    },
    /// Hook completed (from system message with subtype "hook_response")
    HookCompleted {
        hook_id: String,
        hook_name: String,
        hook_event: String,
        output: Option<String>,
        exit_code: Option<i32>,
        outcome: Option<String>,
    },
    /// Hook block (from synthetic user message with text content)
    HookBlock { reason: String },
    /// Team created by lead (from TeamCreate tool result)
    TeamCreated {
        team_name: String,
        config_path: String,
    },
    /// In-process teammate spawned by lead (from Task tool result with teammate_spawned status)
    TeammateSpawned {
        teammate_name: String,
        team_name: String,
        agent_id: String,
        model: String,
        color: String,
    },
    /// Team message sent (from SendMessage tool result)
    TeamMessageSent {
        sender: String,
        recipient: Option<String>,
        content: String,
        message_type: String,
    },
    /// Team deleted (from TeamDelete tool result)
    TeamDeleted {
        team_name: String,
    },
}

// ============================================================================
// Stream Processor State
// ============================================================================

/// Parse a `<usage>...</usage>` block from text to extract task completion stats.
///
/// The Claude CLI Task tool result format is:
/// ```text
/// [subagent text output]
/// agentId: abc1234 (for resuming...)
/// <usage>total_tokens: 12345
/// tool_uses: 8
/// duration_ms: 45000</usage>
/// ```
fn parse_usage_text(text: &str) -> (Option<String>, Option<u64>, Option<u64>, Option<u64>) {
    let agent_id = text.find("agentId:").and_then(|start| {
        let after = &text[start + "agentId:".len()..];
        let trimmed = after.trim_start();
        // agentId is a hex string, take chars until non-hex
        let end = trimmed
            .find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(trimmed.len());
        if end > 0 {
            Some(trimmed[..end].to_string())
        } else {
            None
        }
    });

    let (duration_ms, total_tokens, tool_use_count) =
        if let Some(usage_start) = text.find("<usage>") {
            let usage_end = text.find("</usage>").unwrap_or(text.len());
            let usage_block = &text[usage_start + "<usage>".len()..usage_end];

            let duration = extract_stat(usage_block, "duration_ms:");
            let tokens = extract_stat(usage_block, "total_tokens:");
            let tools = extract_stat(usage_block, "tool_uses:");

            (duration, tokens, tools)
        } else {
            (None, None, None)
        };

    (agent_id, duration_ms, total_tokens, tool_use_count)
}

/// Extract a numeric stat value from a line like "key: 12345"
fn extract_stat(block: &str, key: &str) -> Option<u64> {
    block.find(key).and_then(|start| {
        let after = &block[start + key.len()..];
        let trimmed = after.trim_start();
        let end = trimmed
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(trimmed.len());
        if end > 0 {
            trimmed[..end].parse::<u64>().ok()
        } else {
            None
        }
    })
}

/// Convert a serde_json::Value to a flat text string for usage tag parsing.
/// Handles: plain strings, arrays of content blocks (with "text" type), and JSON objects.
fn value_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text")
                        .and_then(|t| t.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

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

/// Parsed line with optional parent_tool_use_id and is_synthetic extracted from top-level JSON
pub struct ParsedLine {
    pub message: StreamMessage,
    pub parent_tool_use_id: Option<String>,
    pub is_synthetic: bool,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream message without parent context (backward compat)
    pub fn process_message(&mut self, msg: StreamMessage) -> Vec<StreamEvent> {
        self.process_message_with_parent(msg, None, false)
    }

    /// Process a parsed line (message + parent_tool_use_id + is_synthetic)
    pub fn process_parsed_line(&mut self, parsed: ParsedLine) -> Vec<StreamEvent> {
        self.process_message_with_parent(
            parsed.message,
            parsed.parent_tool_use_id,
            parsed.is_synthetic,
        )
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

        // First parse raw JSON so we can extract metadata and support envelope drift.
        let raw_value = serde_json::from_str::<serde_json::Value>(candidate).ok()?;
        let parent_tool_use_id = raw_value
            .get("parent_tool_use_id")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());
        let parent_tool_use_id = parent_tool_use_id.or_else(|| {
            raw_value
                .get("message")
                .and_then(|m| m.get("parent_tool_use_id"))
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
        });
        let is_synthetic = raw_value
            .get("isSynthetic")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        // Parse either direct event objects ({type: ...}) or wrapped envelopes
        // ({message: {type: ...}}, {data: {type: ...}}, {event: {type: ...}}).
        let message_value = if raw_value.get("type").is_some() {
            raw_value
        } else if let Some(inner) = raw_value.get("message").filter(|v| v.is_object()) {
            inner.clone()
        } else if let Some(inner) = raw_value.get("data").filter(|v| v.is_object()) {
            inner.clone()
        } else if let Some(inner) = raw_value.get("event").filter(|v| v.is_object()) {
            inner.clone()
        } else {
            return None;
        };

        let message: StreamMessage = serde_json::from_value(message_value).ok()?;
        Some(ParsedLine {
            message,
            parent_tool_use_id,
            is_synthetic,
        })
    }

    /// Process a stream message with optional parent_tool_use_id and is_synthetic context
    fn process_message_with_parent(
        &mut self,
        msg: StreamMessage,
        parent_tool_use_id: Option<String>,
        is_synthetic: bool,
    ) -> Vec<StreamEvent> {
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
                        parent_tool_use_id: parent_tool_use_id.clone(),
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

                    // Detect Task tool_use and emit TaskStarted (streaming mode)
                    if self.current_tool_name == "Task" {
                        if let Some(ref tool_id) = self.current_tool_id {
                            events.push(StreamEvent::TaskStarted {
                                tool_use_id: tool_id.clone(),
                                description: args
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                subagent_type: args
                                    .get("subagent_type")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                model: args
                                    .get("model")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }

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

                    events.push(StreamEvent::ToolCallCompleted {
                        tool_call,
                        parent_tool_use_id: parent_tool_use_id.clone(),
                    });

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
            StreamMessage::Assistant {
                message,
                session_id,
            } => {
                // Handle --verbose mode assistant messages (full content in one message)
                for content in message.content {
                    match content {
                        AssistantContent::Text { text } => {
                            self.response_text.push_str(&text);
                            // Add as content block directly (verbose mode gives us complete blocks)
                            self.content_blocks
                                .push(ContentBlockItem::Text { text: text.clone() });
                            events.push(StreamEvent::TextChunk(text));
                        }
                        AssistantContent::ToolUse { id, name, input } => {
                            // Detect Task tool_use and emit TaskStarted
                            if name == "Task" {
                                events.push(StreamEvent::TaskStarted {
                                    tool_use_id: id.clone(),
                                    description: input
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    subagent_type: input
                                        .get("subagent_type")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    model: input
                                        .get("model")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                });
                            }

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
                            events.push(StreamEvent::ToolCallCompleted {
                                tool_call,
                                parent_tool_use_id: parent_tool_use_id.clone(),
                            });
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
            StreamMessage::System {
                session_id,
                subtype,
                hook_id,
                hook_name,
                hook_event,
                output,
                exit_code,
                outcome,
                ..
            } => {
                if let Some(ref id) = session_id {
                    self.session_id = session_id.clone();
                    events.push(StreamEvent::SessionId(id.clone()));
                }

                match subtype.as_deref() {
                    Some("hook_started") => {
                        if let (Some(hid), Some(hname), Some(hevent)) =
                            (hook_id, hook_name, hook_event)
                        {
                            events.push(StreamEvent::HookStarted {
                                hook_id: hid,
                                hook_name: hname,
                                hook_event: hevent,
                            });
                        }
                    }
                    Some("hook_response") => {
                        if let (Some(hid), Some(hname), Some(hevent)) =
                            (hook_id, hook_name, hook_event)
                        {
                            events.push(StreamEvent::HookCompleted {
                                hook_id: hid,
                                hook_name: hname,
                                hook_event: hevent,
                                output,
                                exit_code,
                                outcome,
                            });
                        }
                    }
                    _ => {}
                }
            }
            StreamMessage::User { message } => {
                // Handle tool results and synthetic hook blocks
                for content in message.content {
                    // Synthetic user text messages are hook block notifications
                    if is_synthetic {
                        if let UserContent::Text { text } = &content {
                            events.push(StreamEvent::HookBlock {
                                reason: text.clone(),
                            });
                            continue;
                        }
                    }

                    if let UserContent::ToolResult {
                        tool_use_id,
                        content,
                        is_error: _,
                    } = content
                    {
                        // Check if this is a Task tool_result by finding the matching tool_call
                        let is_task_result = self
                            .tool_calls
                            .iter()
                            .any(|tc| tc.id.as_ref() == Some(&tool_use_id) && tc.name == "Task");

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

                        // Emit TaskCompleted if this is a Task tool_result
                        if is_task_result {
                            // Try JSON extraction first (tool_use_result sub-object or content itself)
                            let metadata = content.get("tool_use_result").unwrap_or(&content);
                            let json_agent_id = metadata
                                .get("agentId")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let json_duration =
                                metadata.get("totalDurationMs").and_then(|v| v.as_u64());
                            let json_tokens = metadata.get("totalTokens").and_then(|v| v.as_u64());
                            let json_tools =
                                metadata.get("totalToolUseCount").and_then(|v| v.as_u64());

                            let has_json_stats = json_duration.is_some()
                                || json_tokens.is_some()
                                || json_tools.is_some();

                            // Fall back to text-based <usage> tag parsing if JSON extraction found nothing
                            let (agent_id, total_duration_ms, total_tokens, total_tool_use_count) =
                                if has_json_stats {
                                    (json_agent_id, json_duration, json_tokens, json_tools)
                                } else {
                                    let text = value_to_text(&content);
                                    let (text_agent, text_dur, text_tok, text_tools) =
                                        parse_usage_text(&text);
                                    (json_agent_id.or(text_agent), text_dur, text_tok, text_tools)
                                };

                            events.push(StreamEvent::TaskCompleted {
                                tool_use_id: tool_use_id.clone(),
                                agent_id,
                                total_duration_ms,
                                total_tokens,
                                total_tool_use_count,
                            });
                        }

                        // Emit event with updated tool call
                        events.push(StreamEvent::ToolResultReceived {
                            tool_use_id: tool_use_id.clone(),
                            result: content.clone(),
                            parent_tool_use_id: parent_tool_use_id.clone(),
                        });

                        // Check if this is a team event result
                        if let Some(team_event) = Self::detect_team_event(&tool_use_id, &content) {
                            events.push(team_event);
                        }
                    }
                }
            }
            _ => {}
        }

        events
    }

    /// Detect team-related events from tool result JSON.
    ///
    /// Checks whether a tool result corresponds to TeamCreate, TeammateSpawned,
    /// SendMessage, or TeamDelete and returns the appropriate StreamEvent.
    fn detect_team_event(_tool_use_id: &str, result: &serde_json::Value) -> Option<StreamEvent> {
        // TeamCreate result: { "team_name": "...", "team_file_path": "...", "lead_agent_id": "..." }
        if result.get("team_file_path").is_some() && result.get("lead_agent_id").is_some() {
            return Some(StreamEvent::TeamCreated {
                team_name: result["team_name"].as_str().unwrap_or("").to_string(),
                config_path: result["team_file_path"].as_str().unwrap_or("").to_string(),
            });
        }

        // TeammateSpawned: { "status": "teammate_spawned", "name": "...", "agent_id": "...", ... }
        if result.get("status").and_then(|s| s.as_str()) == Some("teammate_spawned") {
            return Some(StreamEvent::TeammateSpawned {
                teammate_name: result["name"].as_str().unwrap_or("").to_string(),
                team_name: result.get("teammate_id").and_then(|id| {
                    id.as_str().and_then(|s| s.split('@').nth(1))
                }).unwrap_or("").to_string(),
                agent_id: result["agent_id"].as_str().unwrap_or("").to_string(),
                model: result["model"].as_str().unwrap_or("").to_string(),
                color: result.get("color").and_then(|c| c.as_str()).unwrap_or("blue").to_string(),
            });
        }

        // SendMessage result: { "success": true, "recipients": [...], "routing": { "sender": "...", "content": "..." } }
        if result.get("success").and_then(|s| s.as_bool()) == Some(true) && result.get("routing").is_some() {
            let routing = &result["routing"];
            let recipients = result.get("recipients").and_then(|r| r.as_array());
            return Some(StreamEvent::TeamMessageSent {
                sender: routing.get("sender").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                recipient: if recipients.map_or(false, |r| r.len() == 1) {
                    recipients.and_then(|r| r[0].as_str()).map(|s| s.to_string())
                } else { None },
                content: routing.get("content").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                message_type: if recipients.map_or(false, |r| r.len() > 1) { "broadcast" } else { "message" }.to_string(),
            });
        }

        // TeamDelete: look for deletion confirmation
        if result.get("team_deleted").is_some() || result.get("deleted").and_then(|d| d.as_bool()) == Some(true) {
            return Some(StreamEvent::TeamDeleted {
                team_name: result.get("team_name").and_then(|s| s.as_str()).unwrap_or("").to_string(),
            });
        }

        None
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
            } => {
                assert_eq!(tool_use_id, "toolu_task1");
                assert_eq!(description, &Some("Search codebase".to_string()));
                assert_eq!(subagent_type, &Some("Explore".to_string()));
                assert_eq!(model, &Some("sonnet".to_string()));
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
            } => {
                assert_eq!(tool_use_id, "toolu_task2");
                assert_eq!(description, &Some("Run tests".to_string()));
                assert_eq!(subagent_type, &Some("Bash".to_string()));
                assert_eq!(model, &Some("haiku".to_string()));
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
            "color": "green"
        });
        let event = StreamProcessor::detect_team_event("toolu_2", &result);
        assert!(event.is_some());
        match event.unwrap() {
            StreamEvent::TeammateSpawned { teammate_name, team_name, agent_id, model, color } => {
                assert_eq!(teammate_name, "researcher");
                assert_eq!(team_name, "my-team");
                assert_eq!(agent_id, "def456");
                assert_eq!(model, "sonnet");
                assert_eq!(color, "green");
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
}
