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
use tokio::task::JoinHandle;

use crate::application::team_events;
use crate::application::team_state_tracker::{TeamStateTracker, TeammateCost, TeammateStatus};
use crate::infrastructure::agents::claude::{StreamEvent, StreamProcessor};

/// Start a background task that reads a teammate's stdout and emits Tauri events.
///
/// Returns a `JoinHandle` that should be stored in `TeammateHandle.stream_task`
/// so it can be aborted when the teammate is stopped.
///
/// # Arguments
/// * `stdout` - The teammate process's piped stdout
/// * `team_name` - Name of the team this teammate belongs to
/// * `teammate_name` - Unique name of the teammate (used in event payloads)
/// * `context_type` - Chat context type (e.g. "ideation")
/// * `context_id` - Chat context ID (e.g. session ID)
/// * `app_handle` - Tauri AppHandle for emitting events to the frontend
/// * `team_tracker` - TeamStateTracker for updating teammate cost/status
pub fn start_teammate_stream<R: Runtime>(
    stdout: ChildStdout,
    team_name: String,
    teammate_name: String,
    context_type: String,
    context_id: String,
    app_handle: AppHandle<R>,
    team_tracker: Arc<TeamStateTracker>,
) -> JoinHandle<()> {
    let span = tracing::info_span!(
        "teammate_stream",
        teammate = %teammate_name,
        team = %team_name,
    );

    tokio::spawn(async move {
        let _guard = span.enter();

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

        // Track cumulative cost from result events
        let mut total_cost_usd: f64 = 0.0;
        let mut total_input_tokens: u64 = 0;
        let mut total_output_tokens: u64 = 0;

        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    lines_seen += 1;

                    if let Some(parsed) = StreamProcessor::parse_line(&line) {
                        lines_parsed += 1;
                        let stream_events = processor.process_parsed_line(parsed);

                        for event in stream_events {
                            match event {
                                StreamEvent::TextChunk(text) => {
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
                                StreamEvent::HookStarted { .. }
                                | StreamEvent::HookCompleted { .. }
                                | StreamEvent::HookBlock { .. }
                                | StreamEvent::TeamCreated { .. }
                                | StreamEvent::TeammateSpawned { .. }
                                | StreamEvent::TeamMessageSent { .. }
                                | StreamEvent::TeamDeleted { .. } => {
                                    // Hook and team events from teammates are not forwarded
                                    // (hooks run on the lead, team events only relevant from lead's stream)
                                }
                            }
                        }
                    }

                    // Check for result events with cost info by re-parsing the line
                    // (StreamProcessor captures this in its state but doesn't emit
                    // a dedicated cost event, so we parse it separately)
                    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&line) {
                        if raw.get("type").and_then(|t| t.as_str()) == Some("result") {
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
                        }
                    }

                    // Periodic progress logging
                    if lines_seen % 100 == 0 {
                        tracing::debug!(
                            teammate = %teammate_name,
                            lines_seen,
                            lines_parsed,
                            "Teammate stream progress"
                        );
                    }
                }
                Ok(None) => {
                    // EOF — stdout closed, teammate process exited
                    tracing::info!(
                        teammate = %teammate_name,
                        team = %team_name,
                        lines_seen,
                        lines_parsed,
                        "Teammate stdout closed (process exited)"
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_compiles() {
        // Verify the module compiles and types are accessible
        fn _assert_fn_signature() {
            fn _check(
                _stdout: ChildStdout,
                _team_name: String,
                _teammate_name: String,
                _context_type: String,
                _context_id: String,
                _app_handle: AppHandle,
                _team_tracker: Arc<TeamStateTracker>,
            ) -> JoinHandle<()> {
                unimplemented!()
            }
            let _ = _check;
        }
    }
}
