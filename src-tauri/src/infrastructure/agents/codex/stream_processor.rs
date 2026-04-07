use serde::{Deserialize, Serialize};

use crate::infrastructure::agents::claude::ToolCall;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexItemError {
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodexItem {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<CodexItemError>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub aggregated_output: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub sender_thread_id: Option<String>,
    #[serde(default)]
    pub receiver_thread_ids: Option<Vec<String>>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub agents_states: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodexStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub item: Option<CodexItem>,
    #[serde(default)]
    pub usage: Option<CodexUsage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexCommandExecution {
    pub id: Option<String>,
    pub status: Option<String>,
    pub aggregated_output: Option<String>,
    pub exit_code: Option<i32>,
}

pub fn parse_codex_event_line(line: &str) -> Option<CodexStreamEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    serde_json::from_str(trimmed).ok()
}

pub fn extract_codex_agent_message(event: &CodexStreamEvent) -> Option<String> {
    if event.event_type != "item.completed" {
        return None;
    }

    let item = event.item.as_ref()?;
    if item.item_type != "agent_message" {
        return None;
    }

    item.text.clone()
}

pub fn extract_codex_tool_call(event: &CodexStreamEvent) -> Option<ToolCall> {
    let item = event.item.as_ref()?;
    if item.item_type != "mcp_tool_call" {
        return None;
    }

    let name = match (item.server.as_deref(), item.tool.as_deref()) {
        (Some(server), Some(tool)) => format!("{server}::{tool}"),
        (None, Some(tool)) => tool.to_string(),
        _ => return None,
    };

    Some(ToolCall {
        id: item.id.clone(),
        name,
        arguments: item.arguments.clone().unwrap_or_default(),
        result: item.result.clone(),
        diff_context: None,
        stats: None,
    })
}

pub fn extract_codex_command_execution(
    event: &CodexStreamEvent,
) -> Option<CodexCommandExecution> {
    let item = event.item.as_ref()?;
    if item.item_type != "command_execution" {
        return None;
    }

    Some(CodexCommandExecution {
        id: item.id.clone(),
        status: item.status.clone(),
        aggregated_output: item.aggregated_output.clone(),
        exit_code: item.exit_code,
    })
}

pub fn extract_codex_error_message(event: &CodexStreamEvent) -> Option<String> {
    let item = event.item.as_ref()?;

    match item.item_type.as_str() {
        "error" => item
            .error
            .as_ref()
            .and_then(|error| error.message.clone())
            .or_else(|| item.text.clone()),
        "mcp_tool_call" => item.error.as_ref().and_then(|error| error.message.clone()),
        _ => None,
    }
}

pub fn extract_codex_usage(event: &CodexStreamEvent) -> Option<CodexUsage> {
    if event.event_type != "turn.completed" {
        return None;
    }

    event.usage.clone()
}
