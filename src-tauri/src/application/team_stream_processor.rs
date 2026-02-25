// Teammate stdout stream processor
//
// Reads stream-json output from a spawned teammate's stdout line-by-line,
// parses events via StreamProcessor, and emits Tauri events with teammate_name
// so the frontend can route them to the correct teammate in teamStore.
//
// The function spawns a tokio task that runs until stdout closes (teammate exits),
// then updates the teammate's status to Idle or Shutdown.

use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::application::team_events;
use crate::application::team_service::TeamService;
use crate::application::team_state_tracker::{
    TeamMessageType, TeamStateTracker, TeammateCost, TeammateStatus,
};
use crate::infrastructure::agents::claude::{StreamEvent, StreamProcessor};
use crate::utils::truncate_str;

/// Start a background task that reads a teammate's stdout and emits Tauri events.
///
/// Returns a `JoinHandle` that should be stored in `TeammateHandle.stream_task`
/// so it can be aborted when the teammate is stopped.
///
/// # Arguments
/// * `stdout` - The teammate process's piped stdout
/// * `exit_signal` - Fires when the Claude process exits (from the process monitor task).
///   Breaks the read loop even if a grandchild (e.g., Node.js MCP server) holds the
///   pipe open — prevents the stream processor from blocking until the 3600s timeout.
/// * `team_name` - Name of the team this teammate belongs to
/// * `teammate_name` - Unique name of the teammate (used in event payloads)
/// * `context_type` - Chat context type (e.g. "ideation")
/// * `context_id` - Chat context ID (e.g. session ID)
/// * `app_handle` - Tauri AppHandle for emitting events to the frontend
/// * `team_tracker` - TeamStateTracker for updating teammate cost/status
/// * `team_service` - Optional TeamService for message persistence and proper event emission
pub fn start_teammate_stream<R: Runtime>(
    stdout: ChildStdout,
    exit_signal: oneshot::Receiver<()>,
    team_name: String,
    teammate_name: String,
    context_type: String,
    context_id: String,
    app_handle: AppHandle<R>,
    team_tracker: Arc<TeamStateTracker>,
    team_service: Option<Arc<TeamService>>,
) -> JoinHandle<()> {
    let span = tracing::info_span!(
        "teammate_stream",
        teammate = %teammate_name,
        team = %team_name,
    );

    tokio::spawn(async move {
        tracing::info!(
            teammate = %teammate_name,
            team = %team_name,
            "Starting teammate stdout stream processor"
        );

        // Emit agent:run_started so the frontend knows this teammate is running
        let _ = app_handle.emit(
            "agent:run_started",
            serde_json::json!({
                "teammate_name": teammate_name,
                "context_type": context_type,
                "context_id": context_id,
            }),
        );

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let mut processor = StreamProcessor::new();
        let mut lines_seen: usize = 0;
        let mut lines_parsed: usize = 0;

        // Accumulate text output for persistence on turn boundaries
        let mut text_buffer = String::new();
        let mut has_emitted_running = false;
        // Fix B: tracks TurnComplete → Idle state between turns
        let mut is_idle = false;

        // Track cumulative cost from result events
        let mut total_cost_usd: f64 = 0.0;
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;

        // Pin exit_signal so it can be used repeatedly in select!
        tokio::pin!(exit_signal);

        loop {
            // Use select! so we break when Claude exits even if a grandchild process
            // (e.g., Node.js MCP server) holds the stdout pipe open — which would
            // otherwise block next_line() indefinitely.
            let line_result = tokio::select! {
                biased;
                _ = &mut exit_signal => {
                    tracing::info!(
                        teammate = %teammate_name,
                        team = %team_name,
                        "Claude process exited — stopping stream processor (pipe inheritance guard)"
                    );
                    break;
                }
                result = lines.next_line() => result,
            };
            match line_result {
                Ok(Some(line)) => {
                    lines_seen += 1;

                    // DIAGNOSTIC: Log every raw stdout line at INFO level
                    // to verify teammate output is reaching the stream processor
                    let line_preview: &str = truncate_str(&line, 200);
                    tracing::info!(
                        teammate = %teammate_name,
                        team = %team_name,
                        lines_seen,
                        line_len = line.len(),
                        line_preview = %line_preview,
                        "[TEAMMATE_STREAM] raw stdout line"
                    );

                    if let Some(parsed) = StreamProcessor::parse_line(&line) {
                        lines_parsed += 1;
                        let stream_events = processor.process_parsed_line(parsed);

                        // DIAGNOSTIC: Log parsed event count per line
                        if !stream_events.is_empty() {
                            let event_names: Vec<&str> = stream_events.iter().map(|e| match e {
                                StreamEvent::TextChunk(_) => "TextChunk",
                                StreamEvent::Thinking(_) => "Thinking",
                                StreamEvent::ToolCallStarted { .. } => "ToolCallStarted",
                                StreamEvent::ToolCallCompleted { .. } => "ToolCallCompleted",
                                StreamEvent::ToolResultReceived { .. } => "ToolResultReceived",
                                StreamEvent::SessionId(_) => "SessionId",
                                StreamEvent::TaskStarted { .. } => "TaskStarted",
                                StreamEvent::TaskCompleted { .. } => "TaskCompleted",
                                StreamEvent::TeamMessageSent { .. } => "TeamMessageSent",
                                StreamEvent::TeamCreated { .. } => "TeamCreated",
                                StreamEvent::TeammateSpawned { .. } => "TeammateSpawned",
                                StreamEvent::TeamDeleted { .. } => "TeamDeleted",
                                StreamEvent::TurnComplete { .. } => "TurnComplete",
                                StreamEvent::HookStarted { .. } => "HookStarted",
                                StreamEvent::HookCompleted { .. } => "HookCompleted",
                                StreamEvent::HookBlock { .. } => "HookBlock",
                            }).collect();
                            tracing::info!(
                                teammate = %teammate_name,
                                team = %team_name,
                                lines_seen,
                                event_count = stream_events.len(),
                                events = ?event_names,
                                "[TEAMMATE_STREAM] parsed events"
                            );
                        }

                        for event in stream_events {
                            // Fix B: Transition back to Running when activity resumes after TurnComplete idle
                            if is_idle
                                && matches!(
                                    event,
                                    StreamEvent::TextChunk(_)
                                        | StreamEvent::Thinking(_)
                                        | StreamEvent::ToolCallStarted { .. }
                                        | StreamEvent::ToolCallCompleted { .. }
                                        | StreamEvent::ToolResultReceived { .. }
                                )
                            {
                                is_idle = false;
                                let _ = team_tracker
                                    .update_teammate_status(
                                        &team_name,
                                        &teammate_name,
                                        TeammateStatus::Running,
                                    )
                                    .await;
                                let _ = app_handle.emit(
                                    "agent:run_started",
                                    serde_json::json!({
                                        "teammate_name": teammate_name,
                                        "context_type": context_type,
                                        "context_id": context_id,
                                    }),
                                );
                            }

                            match event {
                                StreamEvent::TextChunk(text) => {
                                    // Emit "running" status on first text output
                                    if !has_emitted_running {
                                        has_emitted_running = true;
                                        tracing::info!(
                                            teammate = %teammate_name,
                                            team = %team_name,
                                            context_type = %context_type,
                                            context_id = %context_id,
                                            "[TEAMMATE_STREAM] first text chunk — emitting Running status"
                                        );
                                        let _ = team_tracker
                                            .update_teammate_status(
                                                &team_name,
                                                &teammate_name,
                                                TeammateStatus::Running,
                                            )
                                            .await;
                                        team_events::emit_teammate_status_change(
                                            &app_handle,
                                            &team_name,
                                            &teammate_name,
                                            TeammateStatus::Running,
                                            &context_type,
                                            &context_id,
                                        );
                                    }

                                    // Accumulate text for persistence on turn boundary
                                    text_buffer.push_str(&text);

                                    let text_preview: &str = truncate_str(&text, 100);
                                    tracing::info!(
                                        teammate = %teammate_name,
                                        team = %team_name,
                                        text_len = text.len(),
                                        text_preview = %text_preview,
                                        context_type = %context_type,
                                        context_id = %context_id,
                                        "[TEAMMATE_STREAM] emitting agent:chunk"
                                    );

                                    let emit_result = app_handle.emit(
                                        "agent:chunk",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "text": text,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                        }),
                                    );
                                    if let Err(ref e) = emit_result {
                                        tracing::error!(
                                            teammate = %teammate_name,
                                            error = %e,
                                            "[TEAMMATE_STREAM] agent:chunk emit FAILED"
                                        );
                                    }
                                }
                                StreamEvent::Thinking(text) => {
                                    // Emit thinking as a chunk with a marker so frontend
                                    // can distinguish if needed
                                    let _ = app_handle.emit(
                                        "agent:chunk",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "text": text,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                        }),
                                    );
                                }
                                StreamEvent::ToolCallStarted {
                                    name,
                                    id,
                                    parent_tool_use_id,
                                } => {
                                    let _ = app_handle.emit(
                                        "agent:tool_call",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "tool_name": name,
                                            "tool_id": id,
                                            "arguments": serde_json::Value::Null,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                            "parent_tool_use_id": parent_tool_use_id,
                                        }),
                                    );
                                }
                                StreamEvent::ToolCallCompleted {
                                    tool_call,
                                    parent_tool_use_id,
                                } => {
                                    let _ = app_handle.emit(
                                        "agent:tool_call",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "tool_name": tool_call.name,
                                            "tool_id": tool_call.id,
                                            "arguments": tool_call.arguments,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                            "parent_tool_use_id": parent_tool_use_id,
                                        }),
                                    );
                                }
                                StreamEvent::ToolResultReceived {
                                    tool_use_id,
                                    result,
                                    parent_tool_use_id,
                                } => {
                                    let _ = app_handle.emit(
                                        "agent:tool_call",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "tool_name": format!("result:{}", tool_use_id),
                                            "tool_id": tool_use_id,
                                            "arguments": serde_json::Value::Null,
                                            "result": result,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                            "parent_tool_use_id": parent_tool_use_id,
                                        }),
                                    );
                                }
                                StreamEvent::SessionId(_) => {
                                    // Session ID captured in processor — not needed for
                                    // teammate streaming (teammates don't use --resume)
                                }
                                StreamEvent::TaskStarted {
                                    tool_use_id,
                                    description,
                                    subagent_type,
                                    model,
                                    teammate_name: _,
                                    team_name: _,
                                } => {
                                    let _ = app_handle.emit(
                                        "agent:task_started",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "tool_use_id": tool_use_id,
                                            "description": description,
                                            "subagent_type": subagent_type,
                                            "model": model,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                        }),
                                    );
                                }
                                StreamEvent::TaskCompleted {
                                    tool_use_id,
                                    agent_id,
                                    total_duration_ms,
                                    total_tokens,
                                    total_tool_use_count,
                                } => {
                                    let _ = app_handle.emit(
                                        "agent:task_completed",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "tool_use_id": tool_use_id,
                                            "agent_id": agent_id,
                                            "total_duration_ms": total_duration_ms,
                                            "total_tokens": total_tokens,
                                            "total_tool_use_count": total_tool_use_count,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                        }),
                                    );
                                }
                                StreamEvent::TeamMessageSent {
                                    sender,
                                    recipient,
                                    content,
                                    message_type,
                                } => {
                                    // Persist message and emit proper team:message event
                                    let msg_type = match message_type.as_str() {
                                        "broadcast" => TeamMessageType::Broadcast,
                                        _ => TeamMessageType::TeammateMessage,
                                    };

                                    if let Some(ref service) = team_service {
                                        // Use TeamService for full persistence + event emission
                                        match service
                                            .add_teammate_message(
                                                &team_name,
                                                &sender,
                                                recipient.as_deref(),
                                                &content,
                                                msg_type,
                                            )
                                            .await
                                        {
                                            Ok(_) => {
                                                tracing::info!(
                                                    teammate = %teammate_name,
                                                    sender = %sender,
                                                    recipient = ?recipient,
                                                    "Teammate message persisted and emitted"
                                                );
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    error = %e,
                                                    sender = %sender,
                                                    "Failed to persist teammate message"
                                                );
                                            }
                                        }
                                    } else {
                                        // Fallback: emit event directly without persistence
                                        let _ = app_handle.emit(
                                            "team:message",
                                            serde_json::json!({
                                                "team_name": team_name,
                                                "sender": sender,
                                                "recipient": recipient,
                                                "content": content,
                                                "message_type": message_type,
                                                "context_type": context_type,
                                                "context_id": context_id,
                                            }),
                                        );
                                    }
                                }
                                StreamEvent::TurnComplete { .. } => {
                                    // Fix A: Teammate went idle between turns — update status and emit events
                                    is_idle = true;
                                    let _ = team_tracker
                                        .update_teammate_status(
                                            &team_name,
                                            &teammate_name,
                                            TeammateStatus::Idle,
                                        )
                                        .await;
                                    let _ = app_handle.emit(
                                        "agent:run_completed",
                                        serde_json::json!({
                                            "teammate_name": teammate_name,
                                            "context_type": context_type,
                                            "context_id": context_id,
                                        }),
                                    );
                                    team_events::emit_teammate_idle(
                                        &app_handle,
                                        &team_name,
                                        &teammate_name,
                                        &context_type,
                                        &context_id,
                                    );
                                }
                                StreamEvent::HookStarted { .. }
                                | StreamEvent::HookCompleted { .. }
                                | StreamEvent::HookBlock { .. }
                                | StreamEvent::TeamCreated { .. }
                                | StreamEvent::TeammateSpawned { .. }
                                | StreamEvent::TeamDeleted { .. } => {
                                    // Hook and team events from teammates are not forwarded
                                    // (hooks run on the lead, team events only relevant from lead's stream)
                                }
                            }
                        }
                    } else {
                        // DIAGNOSTIC: Line was NOT parseable as a StreamMessage
                        tracing::info!(
                            teammate = %teammate_name,
                            team = %team_name,
                            lines_seen,
                            line_len = line.len(),
                            line_preview = %truncate_str(&line, 200),
                            "[TEAMMATE_STREAM] line NOT parsed (not a stream-json message)"
                        );
                    }

                    // Check for events with cost/usage info and persist text buffer.
                    //
                    // Token sources:
                    // - "type": "result" — final turn cost + usage (authoritative)
                    // - "type": "assistant" — per-message usage for real-time updates
                    //   (emitted mid-turn when model produces each response)
                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&line) {
                        let event_type = raw.get("type").and_then(|t| t.as_str());

                        if event_type == Some("result") {
                            // Persist accumulated text as a teammate message on turn boundary
                            if !text_buffer.is_empty() {
                                if let Some(ref service) = team_service {
                                    let _ = service
                                        .add_teammate_message(
                                            &team_name,
                                            &teammate_name,
                                            None,
                                            &text_buffer,
                                            TeamMessageType::TeammateMessage,
                                        )
                                        .await;
                                }
                                text_buffer.clear();
                            }
                            // Reset running flag for next turn
                            has_emitted_running = false;
                            // Extract cost_usd from result event
                            if let Some(cost) = raw.get("cost_usd").and_then(|c| c.as_f64()) {
                                total_cost_usd += cost;
                            }

                            // Extract usage tokens if present
                            if let Some(usage) = raw.get("usage") {
                                if let Some(input) =
                                    usage.get("input_tokens").and_then(|t| t.as_u64())
                                {
                                    total_input_tokens += input;
                                }
                                if let Some(output) =
                                    usage.get("output_tokens").and_then(|t| t.as_u64())
                                {
                                    total_output_tokens += output;
                                }
                            }

                            // Update teammate cost via TeamService (which emits team:cost_update)
                            let cost = TeammateCost {
                                input_tokens: total_input_tokens,
                                output_tokens: total_output_tokens,
                                cache_creation_tokens: 0,
                                cache_read_tokens: 0,
                                estimated_usd: total_cost_usd,
                            };
                            let _ = team_tracker
                                .update_teammate_cost(&team_name, &teammate_name, cost)
                                .await;

                            // Emit cost update event
                            team_events::emit_team_cost_update(
                                &app_handle,
                                &team_name,
                                &teammate_name,
                                total_input_tokens,
                                total_output_tokens,
                                total_cost_usd,
                                &context_type,
                                &context_id,
                            );
                        } else if event_type == Some("assistant") {
                            // Real-time token updates from assistant message events.
                            // These carry a top-level `message.usage` with cumulative
                            // input/output tokens for the current turn, letting the UI
                            // show progress before the final "result" event arrives.
                            let updated = extract_assistant_usage(
                                &raw,
                                &mut total_input_tokens,
                                &mut total_output_tokens,
                            );
                            if updated {
                                let cost = TeammateCost {
                                    input_tokens: total_input_tokens,
                                    output_tokens: total_output_tokens,
                                    cache_creation_tokens: 0,
                                    cache_read_tokens: 0,
                                    estimated_usd: total_cost_usd,
                                };
                                let _ = team_tracker
                                    .update_teammate_cost(
                                        &team_name,
                                        &teammate_name,
                                        cost,
                                    )
                                    .await;

                                team_events::emit_team_cost_update(
                                    &app_handle,
                                    &team_name,
                                    &teammate_name,
                                    total_input_tokens,
                                    total_output_tokens,
                                    total_cost_usd,
                                    &context_type,
                                    &context_id,
                                );
                            }
                        }
                    }

                    // Periodic progress logging (every 50 lines at INFO level for diagnostics)
                    if lines_seen % 50 == 0 {
                        tracing::info!(
                            teammate = %teammate_name,
                            team = %team_name,
                            lines_seen,
                            lines_parsed,
                            text_buffer_len = text_buffer.len(),
                            total_cost_usd,
                            "[TEAMMATE_STREAM] progress"
                        );
                    }
                }
                Ok(None) => {
                    // Persist any remaining text buffer before closing
                    if !text_buffer.is_empty() {
                        if let Some(ref service) = team_service {
                            let _ = service
                                .add_teammate_message(
                                    &team_name,
                                    &teammate_name,
                                    None,
                                    &text_buffer,
                                    TeamMessageType::TeammateMessage,
                                )
                                .await;
                        }
                        text_buffer.clear();
                    }

                    // EOF — stdout closed, teammate process exited
                    tracing::info!(
                        teammate = %teammate_name,
                        team = %team_name,
                        lines_seen,
                        lines_parsed,
                        text_buffer_len = text_buffer.len(),
                        total_cost_usd,
                        total_input_tokens,
                        total_output_tokens,
                        "[TEAMMATE_STREAM] stdout closed (EOF) — final stats"
                    );
                    break;
                }
                Err(e) => {
                    tracing::warn!(
                        teammate = %teammate_name,
                        team = %team_name,
                        error = %e,
                        "Teammate stdout read error"
                    );
                    break;
                }
            }
        }

        // Emit agent:run_completed so the frontend knows this teammate finished
        let _ = app_handle.emit(
            "agent:run_completed",
            serde_json::json!({
                "teammate_name": teammate_name,
                "context_type": context_type,
                "context_id": context_id,
            }),
        );

        // Update teammate status to Idle (graceful exit) or Shutdown
        let new_status = TeammateStatus::Idle;
        let _ = team_tracker
            .update_teammate_status(&team_name, &teammate_name, new_status)
            .await;

        // Emit the idle event
        team_events::emit_teammate_idle(
            &app_handle,
            &team_name,
            &teammate_name,
            &context_type,
            &context_id,
        );

        tracing::info!(
            teammate = %teammate_name,
            team = %team_name,
            total_cost_usd,
            total_input_tokens,
            total_output_tokens,
            "Teammate stream processor finished"
        );
    }.instrument(span))
}


/// Extract usage tokens from a `"type": "assistant"` event's `message.usage` field.
///
/// Claude Code emits assistant events with cumulative usage per-message:
/// ```json
/// {"type":"assistant","message":{"usage":{"input_tokens":1234,"output_tokens":567},...},...}
/// ```
///
/// Updates the running totals only if the new values exceed the current totals
/// (usage is cumulative within a turn, so later messages have higher counts).
///
/// Returns `true` if the totals were updated (i.e., the UI should be notified).
fn extract_assistant_usage(
    raw: &serde_json::Value,
    total_input: &mut u64,
    total_output: &mut u64,
) -> bool {
    let usage = raw
        .get("message")
        .and_then(|m| m.get("usage"));
    let Some(usage) = usage else {
        return false;
    };

    let input = usage.get("input_tokens").and_then(|t| t.as_u64()).unwrap_or(0);
    let output = usage.get("output_tokens").and_then(|t| t.as_u64()).unwrap_or(0);

    // Only update if the new cumulative values exceed current totals.
    // Assistant usage is cumulative within a turn — later messages have higher counts.
    if input > *total_input || output > *total_output {
        if input > *total_input {
            *total_input = input;
        }
        if output > *total_output {
            *total_output = output;
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
#[path = "team_stream_processor_tests.rs"]
mod tests;
