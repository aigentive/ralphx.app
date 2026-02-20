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
        /// Teammate name if this Task spawns a team member (from args.name)
        teammate_name: Option<String>,
        /// Team name if this Task spawns a team member (from args.team_name)
        team_name: Option<String>,
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
        /// The teammate's initial task prompt (from Task tool's `prompt` field)
        prompt: String,
        /// Claude Code agent type controlling built-in tool set (e.g. "general-purpose")
        agent_type: String,
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
    /// Top-level `tool_use_result` from Claude Code stream JSON.
    /// Contains structured metadata (e.g. `{"status": "teammate_spawned", ...}`)
    /// that is NOT inside the `message.content[].content` field.
    pub tool_use_result: Option<serde_json::Value>,
}

impl StreamProcessor {
    /// Create a new stream processor
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream message without parent context (backward compat)
    pub fn process_message(&mut self, msg: StreamMessage) -> Vec<StreamEvent> {
        self.process_message_with_parent(msg, None, false, None)
    }

    /// Process a parsed line (message + parent_tool_use_id + is_synthetic + tool_use_result)
    pub fn process_parsed_line(&mut self, parsed: ParsedLine) -> Vec<StreamEvent> {
        self.process_message_with_parent(
            parsed.message,
            parsed.parent_tool_use_id,
            parsed.is_synthetic,
            parsed.tool_use_result,
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

        // Extract top-level tool_use_result before raw_value is consumed.
        // Claude Code puts structured metadata here (e.g. {"status": "teammate_spawned", ...})
        // which is separate from message.content[].content (the text result).
        let tool_use_result = raw_value
            .get("tool_use_result")
            .filter(|v| v.is_object())
            .cloned();

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
            tool_use_result,
        })
    }

    /// Process a stream message with optional parent_tool_use_id, is_synthetic, and tool_use_result context.
    /// `tool_use_result` is the top-level structured metadata from Claude Code stream JSON
    /// (e.g. `{"status": "teammate_spawned", ...}`) — distinct from `message.content[].content`.
    fn process_message_with_parent(
        &mut self,
        msg: StreamMessage,
        parent_tool_use_id: Option<String>,
        is_synthetic: bool,
        tool_use_result: Option<serde_json::Value>,
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
                                teammate_name: args
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                team_name: args
                                    .get("team_name")
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
                // Only capture session_id from top-level (lead's own) result events.
                // Teammate result events carry a non-None parent_tool_use_id; capturing
                // their session_id would overwrite the lead's session and cause --resume
                // to open the teammate's context instead of the orchestrator's.
                if parent_tool_use_id.is_none() {
                    if let Some(ref id) = session_id {
                        self.session_id = session_id.clone();
                        events.push(StreamEvent::SessionId(id.clone()));
                    }
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
                                    teammate_name: input
                                        .get("name")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    team_name: input
                                        .get("team_name")
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

                // Only capture session_id from top-level (lead's own) assistant messages.
                // Teammate assistant messages carry a non-None parent_tool_use_id; capturing
                // their session_id would overwrite the lead's session.
                if parent_tool_use_id.is_none() {
                    if let Some(ref id) = session_id {
                        self.session_id = session_id.clone();
                        events.push(StreamEvent::SessionId(id.clone()));
                    }
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
                            // Use top-level tool_use_result (structured metadata from Claude Code)
                            // falling back to content.tool_use_result or content itself
                            let empty = serde_json::Value::Null;
                            let metadata = tool_use_result.as_ref()
                                .unwrap_or_else(|| content.get("tool_use_result").unwrap_or(&empty));
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

                        // Check if this is a team event result.
                        // Use top-level tool_use_result (structured JSON from Claude Code stream)
                        // which contains the actual team metadata. The content field is just text.
                        let team_data = tool_use_result.as_ref().unwrap_or(&content);
                        if let Some(team_event) = Self::detect_team_event(&tool_use_id, team_data) {
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
                prompt: result.get("prompt").and_then(|p| p.as_str()).unwrap_or("").to_string(),
                agent_type: result.get("agent_type").and_then(|a| a.as_str()).unwrap_or("general-purpose").to_string(),
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

#[cfg(test)]
#[path = "stream_processor_tests.rs"]
mod tests;
