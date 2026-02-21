// Stream parsing and detection functions
// Stateless parsing helpers for extracting data from Claude CLI stream-json output

use super::types::{ParsedLine, StreamEvent, StreamMessage};

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
pub(crate) fn parse_usage_text(text: &str) -> (Option<String>, Option<u64>, Option<u64>, Option<u64>) {
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
pub(crate) fn extract_stat(block: &str, key: &str) -> Option<u64> {
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
pub(crate) fn value_to_text(value: &serde_json::Value) -> String {
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

/// Parse a stream-json line, extracting parent_tool_use_id from the top-level JSON envelope
pub(crate) fn parse_line(line: &str) -> Option<ParsedLine> {
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

/// Detect team-related events from tool result JSON.
///
/// Checks whether a tool result corresponds to TeamCreate, TeammateSpawned,
/// SendMessage, or TeamDelete and returns the appropriate StreamEvent.
pub(crate) fn detect_team_event(_tool_use_id: &str, result: &serde_json::Value) -> Option<StreamEvent> {
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
