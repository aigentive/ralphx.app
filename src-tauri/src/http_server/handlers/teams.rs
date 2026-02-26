// Team HTTP handlers — endpoints for MCP team tools
//
// Provides HTTP endpoints for:
// - POST /api/team/plan          — request_team_plan (validates team plan against constraints)
// - POST /api/team/plan/approve  — approve_team_plan (batch-spawns all teammates from approved plan)
// - POST /api/team/spawn         — request_teammate_spawn (validates, spawns, registers, streams)
// - POST /api/team/artifact      — create_team_artifact
// - GET  /api/team/artifacts/:session_id — get_team_artifacts
// - GET  /api/team/session_state/:session_id — get_team_session_state
// - POST /api/team/session_state — save_team_session_state

use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use tauri::Emitter;
use tracing::{error, info, warn};

use super::HttpServerState;
use crate::application::team_state_tracker::{
    PendingTeamPlan, PendingTeammate, PlanDecision, PlanDecisionTeammate, TeammateHandle,
    TeammateStatus,
};
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactRelation, ArtifactRelationId,
    ArtifactRelationType, ArtifactType, TaskId,
};
use crate::http_server::types::{
    ApproveTeamPlanRequest, ApproveTeamPlanResponse, CreateTeamArtifactRequest,
    CreateTeamArtifactResponse, GetTeamArtifactsResponse, RejectTeamPlanRequest,
    RequestTeamPlanRequest, RequestTeamPlanResponse, RequestTeammateSpawnRequest,
    RequestTeammateSpawnResponse, SaveTeamSessionStateRequest, SaveTeamSessionStateResponse,
    SpawnedTeammateInfo, TeamArtifactSummary, TeamCompositionEntry, TeamSessionStateResponse,
};
use crate::infrastructure::agents::claude::{
    get_team_constraints, team_constraints_config, validate_team_plan,
    ClaudeCodeClient, TeammateContext, TeammateSpawnConfig, TeammateSpawnRequest,
};

// ============================================================================
// POST /api/team/plan — Request approval for a team plan
// ============================================================================

/// Accept a team plan from the team lead agent.
///
/// Validates the team plan against constraints, emits a `team:plan_requested`
/// event for the frontend approval UI, then **blocks** (long-polls) until the
/// user approves or rejects. On approval the response includes spawn results
/// so the lead agent knows teammates are ready without spawning them itself.
///
/// Timeout: 5 minutes — after which the plan is auto-rejected.
pub async fn request_team_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestTeamPlanRequest>,
) -> Result<Json<RequestTeamPlanResponse>, (StatusCode, String)> {
    info!(
        process = %req.process,
        teammate_count = req.teammates.len(),
        "Team plan requested — validating constraints"
    );

    // Load constraints from ralphx.yaml
    let config = team_constraints_config();
    let constraints = get_team_constraints(config, &req.process);

    // Convert HTTP request teammates to validation type
    let spawn_requests: Vec<TeammateSpawnRequest> = req
        .teammates
        .iter()
        .map(|t| TeammateSpawnRequest {
            role: t.role.clone(),
            prompt: None,
            preset: t.preset.clone(),
            tools: t.tools.clone(),
            mcp_tools: t.mcp_tools.clone(),
            model: t.model.clone(),
            prompt_summary: Some(t.prompt_summary.clone()),
        })
        .collect();

    // Validate the plan against constraints
    let plan = validate_team_plan(&constraints, &req.process, &spawn_requests).map_err(|e| {
        warn!(process = %req.process, error = %e, "Team plan validation failed");
        (
            StatusCode::BAD_REQUEST,
            format!("Team plan validation failed: {e}"),
        )
    })?;

    let plan_id = plan.plan_id.clone();

    info!(
        plan_id = %plan_id,
        process = %req.process,
        approved_teammates = plan.teammates.len(),
        "Team plan validated — emitting for user approval (blocking until decision)"
    );

    // Store the approved plan with full prompts for batch-spawn on approval
    let pending_teammates: Vec<PendingTeammate> = req
        .teammates
        .iter()
        .zip(plan.teammates.iter())
        .map(|(req_t, approved_t)| PendingTeammate {
            role: approved_t.role.clone(),
            prompt: req_t
                .prompt
                .clone()
                .unwrap_or_else(|| req_t.prompt_summary.clone()),
            tools: approved_t.approved_tools.clone(),
            mcp_tools: approved_t.approved_mcp_tools.clone(),
            model: approved_t.approved_model.clone(),
            preset: req_t.preset.clone(),
        })
        .collect();

    state
        .team_tracker
        .store_pending_plan(PendingTeamPlan {
            plan_id: plan_id.clone(),
            context_type: req.context_type.clone(),
            context_id: req.context_id.clone(),
            process: req.process.clone(),
            teammates: pending_teammates,
            created_at: Utc::now(),
            team_name: req.team_name.clone(),
            lead_session_id: req.lead_session_id.clone(),
        })
        .await;

    // Register a watch channel for this plan (before emitting event)
    let mut rx = state.team_tracker.register_plan_channel(&plan_id).await;

    // Emit event for frontend to show approval UI (with validated plan)
    if let Some(app_handle) = &state.app_state.app_handle {
        let emit_result = app_handle.emit(
            "team:plan_requested",
            serde_json::json!({
                "plan_id": plan_id,
                "context_type": req.context_type,
                "context_id": req.context_id,
                "process": req.process,
                "teammates": req.teammates,
                "validated": true,
            }),
        );
        info!(plan_id = %plan_id, emit_ok = emit_result.is_ok(), "Emitted team:plan_requested event");
    } else {
        warn!("No app_handle available — team:plan_requested event NOT emitted");
    }

    // ── Block until user approves/rejects (5 min timeout) ──────────────
    let timeout = tokio::time::Duration::from_secs(300);
    let start = tokio::time::Instant::now();

    let decision = loop {
        // Check if a decision has been sent
        let maybe_decision: Option<crate::application::team_state_tracker::PlanDecision> = {
            let current = rx.borrow();
            current.clone()
        };

        if let Some(decision) = maybe_decision {
            break decision;
        }

        // Check timeout
        if start.elapsed() >= timeout {
            // Cleanup: remove the watch channel only — keep the pending plan alive
            // so the user can still approve it after the MCP poll times out.
            state.team_tracker.remove_plan_channel(&plan_id).await;

            return Ok(Json(RequestTeamPlanResponse {
                success: false,
                plan_id,
                team_name: None,
                teammates_spawned: vec![],
                message: "Team plan timed out waiting for user approval (5 min). Plan is still pending — user can still approve in the UI.".to_string(),
            }));
        }

        // Wait for channel signal with remaining timeout
        let remaining = timeout.saturating_sub(start.elapsed());
        match tokio::time::timeout(remaining, rx.changed()).await {
            Ok(Ok(())) => continue,
            Ok(Err(_)) => {
                // Channel closed
                state.team_tracker.remove_plan_channel(&plan_id).await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Plan approval channel closed unexpectedly".to_string(),
                ));
            }
            Err(_) => {
                // Timeout — remove the watch channel only, keep the pending plan
                // so the user can still approve it from the UI.
                state.team_tracker.remove_plan_channel(&plan_id).await;
                return Ok(Json(RequestTeamPlanResponse {
                    success: false,
                    plan_id,
                    team_name: None,
                    teammates_spawned: vec![],
                    message: "Team plan timed out waiting for user approval (5 min). Plan is still pending — user can still approve in the UI.".to_string(),
                }));
            }
        }
    };

    // Cleanup channel
    state.team_tracker.remove_plan_channel(&plan_id).await;

    // Build response from the decision
    let teammates_spawned: Vec<SpawnedTeammateInfo> = decision
        .teammates_spawned
        .iter()
        .map(|t| SpawnedTeammateInfo {
            name: t.name.clone(),
            role: t.role.clone(),
            model: t.model.clone(),
            color: t.color.clone(),
        })
        .collect();

    Ok(Json(RequestTeamPlanResponse {
        success: decision.approved,
        plan_id,
        team_name: decision.team_name,
        teammates_spawned,
        message: decision.message,
    }))
}

// ============================================================================
// POST /api/team/plan/approve — Approve a team plan and batch-spawn teammates
// ============================================================================

/// Approve a validated team plan and spawn all teammates at once.
///
/// This is the SINGLE entry point for teammate CLI process creation. After the
/// user approves the plan in the UI, this handler:
///
/// 1. Looks up the pending plan by plan_id
/// 2. Creates the team in TeamService (DB + events)
/// 3. For each teammate: generate name/color, register in DB, spawn CLI process,
///    start stdout stream processing, register process handle
/// 4. Signals the blocking `request_team_plan` handler with spawn results
/// 5. Returns the list of spawned teammates
///
/// The lead agent's `Task` tool creates in-process subagents within its own Claude
/// session, but these separate CLI processes are what the Tauri frontend tracks.
pub async fn approve_team_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<ApproveTeamPlanRequest>,
) -> Result<Json<ApproveTeamPlanResponse>, (StatusCode, String)> {
    info!(
        plan_id = %req.plan_id,
        context_type = %req.context_type,
        context_id = %req.context_id,
        "Team plan approval requested — batch-spawning teammates"
    );

    // 1. Take the pending plan (removes it from store)
    let plan = state
        .team_tracker
        .take_pending_plan(&req.plan_id)
        .await
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("No pending plan found with id '{}'", req.plan_id),
            )
        })?;

    // 2. Create team (or find existing) — via TeamService for DB persistence + events
    // team_name comes from the lead agent's TeamCreate call (required field).
    let team_name = plan.team_name.clone();
    let team_exists = state.team_service.team_exists(&team_name).await;
    if !team_exists {
        state
            .team_service
            .create_team(&team_name, &req.context_id, &req.context_type)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to create team");
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;
    }

    // 3. Resolve the working directory (worktree-aware for task contexts)
    let working_dir =
        resolve_teammate_working_dir(&state, &req.context_type, &req.context_id).await;
    info!(
        plan_id = %req.plan_id,
        context_type = %req.context_type,
        context_id = %req.context_id,
        working_dir = %working_dir.display(),
        "Resolved teammate working directory"
    );

    // 3b. Resolve project ID for RALPHX_PROJECT_ID env var on teammates
    let project_id = resolve_teammate_project_id(
        &state,
        &req.context_type,
        &req.context_id,
    )
    .await;

    // 4. Register, spawn, and stream each teammate as a separate CLI process.
    //    This is the ONLY place where teammate worker processes are created.
    //    The lead agent's Task tool creates in-process subagents within its own
    //    Claude session, but these separate CLI processes are what the frontend tracks.
    let mut spawned_teammates = Vec::new();

    for pending in &plan.teammates {
        let teammate_name = generate_unique_teammate_name(&state, &team_name, &pending.role).await;
        let teammate_color = assign_teammate_color(&state, &team_name).await;

        // Register teammate in DB via TeamService (persistence + events)
        let _ = state
            .team_service
            .add_teammate(
                &team_name,
                &teammate_name,
                &teammate_color,
                &pending.model,
                &pending.role,
            )
            .await;

        // Derive MCP agent type from process: worker-* processes use worker-team-member,
        // all others use ideation-team-member (the default in TeammateSpawnConfig::new)
        let mcp_type = if plan.process.starts_with("worker") {
            "worker-team-member"
        } else {
            "ideation-team-member"
        };

        // Use the lead agent's session ID passed through the MCP flow.
        // Fallback chain: MCP env var → Claude Code team config file → context_id.
        let source;
        let parent_session_id = if let Some(ref sid) = plan.lead_session_id {
            source = "mcp_env_var";
            sid.clone()
        } else if let Some(sid) = resolve_lead_session_from_config(&team_name) {
            source = "team_config_file";
            sid
        } else {
            source = "context_id_fallback";
            req.context_id.clone()
        };

        tracing::info!(
            parent_session_id = %parent_session_id,
            source = %source,
            lead_session_id_from_mcp = ?plan.lead_session_id,
            context_id = %req.context_id,
            team = %team_name,
            "[TEAM_SPAWN] Resolved parent_session_id for teammate (plan flow)"
        );

        // Build RalphX session context (separate from parent_session_id)
        let teammate_context = TeammateContext {
            context_id: req.context_id.clone(),
            context_type: req.context_type.clone(),
            project_id: project_id.clone(),
        };

        // Spawn an interactive CLI worker process for this teammate
        // (uses spawn_teammate_interactive for piped stdin — keeps process alive for messaging)

        // Always inject team communication tools regardless of plan specification.
        // Without these, teammates cannot send messages or coordinate via task lists.
        let mut tools = pending.tools.clone();
        for required in ["SendMessage", "TaskCreate", "TaskUpdate", "TaskList", "TaskGet"] {
            if !tools.contains(&required.to_string()) {
                tools.push(required.to_string());
            }
        }

        let spawn_config =
            TeammateSpawnConfig::new(&teammate_name, &team_name, &pending.prompt)
                .with_parent_session_id(&parent_session_id)
                .with_context(teammate_context)
                .with_model(&pending.model)
                .with_tools(tools)
                .with_mcp_tools(pending.mcp_tools.clone())
                .with_color(&teammate_color)
                .with_mcp_agent_type(mcp_type)
                .with_working_dir(working_dir.clone())
                .with_plugin_dir(working_dir.join("ralphx-plugin"));

        let client = ClaudeCodeClient::new();
        match client.spawn_teammate_interactive(spawn_config).await {
            Ok(spawn_result) => {
                info!(
                    teammate = %teammate_name,
                    team = %team_name,
                    pid = ?spawn_result.child.id(),
                    "Teammate worker process spawned in approve_team_plan"
                );

                let child_pid = spawn_result.child.id();
                let mut child = spawn_result.child;
                let stdout = child.stdout.take();

                // Drain stderr in background to capture crash messages without deadlock.
                // Logs any output at error level when the process exits.
                if let Some(stderr) = child.stderr.take() {
                    let name = teammate_name.clone();
                    let team = team_name.clone();
                    tokio::spawn(async move {
                        use tokio::io::{AsyncBufReadExt, BufReader};
                        let mut lines = BufReader::new(stderr).lines();
                        let mut output = Vec::new();
                        while let Ok(Some(line)) = lines.next_line().await {
                            output.push(line);
                        }
                        if !output.is_empty() {
                            tracing::error!(
                                teammate = %name,
                                team = %team,
                                stderr = %output.join("\n"),
                                "Teammate process stderr (crash/MCP init error)"
                            );
                        }
                    });
                }

                // Process monitor: owns child, signals stream processor when Claude exits.
                // Prevents 3600s timeout when a grandchild (e.g., Node.js MCP server) inherits
                // the stdout pipe and holds it open after Claude exits.
                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
                let (exit_tx, exit_rx) = tokio::sync::oneshot::channel::<()>();
                {
                    let monitor_name = teammate_name.clone();
                    let monitor_team = team_name.clone();
                    tokio::spawn(async move {
                        tokio::select! {
                            biased;
                            _ = kill_rx => {
                                tracing::info!(
                                    teammate = %monitor_name,
                                    team = %monitor_team,
                                    "Teammate process kill signal received"
                                );
                                let _ = child.kill().await;
                                let _ = child.wait().await;
                            }
                            status = child.wait() => {
                                tracing::info!(
                                    teammate = %monitor_name,
                                    team = %monitor_team,
                                    status = ?status,
                                    "Teammate process exited naturally"
                                );
                            }
                        }
                        // Signal stream processor to stop (pipe inheritance guard)
                        let _ = exit_tx.send(());
                    });
                }

                // Start background stream processor for teammate stdout
                let stream_task = match (stdout, &state.app_state.app_handle) {
                    (Some(stdout), Some(app_handle)) => Some(
                        crate::application::team_stream_processor::start_teammate_stream(
                            stdout,
                            exit_rx,
                            team_name.clone(),
                            teammate_name.clone(),
                            req.context_type.clone(),
                            req.context_id.clone(),
                            app_handle.clone(),
                            std::sync::Arc::new(state.team_tracker.clone()),
                            Some(state.team_service.clone()),
                            Some(std::sync::Arc::clone(&state.app_state.chat_conversation_repo)),
                            Some(std::sync::Arc::clone(&state.app_state.chat_message_repo)),
                            Some(std::sync::Arc::clone(&state.app_state.interactive_process_registry)),
                            Some(std::sync::Arc::clone(&state.execution_state)),
                        ),
                    ),
                    _ => {
                        warn!(
                            teammate = %teammate_name,
                            "No stdout/app_handle for teammate stream processing"
                        );
                        None
                    }
                };

                let handle = TeammateHandle {
                    kill_tx: Some(kill_tx),
                    stream_task,
                    stdin: Some(spawn_result.stdin),
                    child_pid,
                };

                let _ = state
                    .team_service
                    .set_teammate_handle(&team_name, &teammate_name, handle)
                    .await;

                let _ = state
                    .team_service
                    .update_teammate_status(&team_name, &teammate_name, TeammateStatus::Running)
                    .await;
            }
            Err(e) => {
                error!(
                    teammate = %teammate_name,
                    team = %team_name,
                    error = %e,
                    "Failed to spawn teammate worker process"
                );

                let _ = state
                    .team_service
                    .update_teammate_status(&team_name, &teammate_name, TeammateStatus::Failed)
                    .await;

                if let Some(app_handle) = &state.app_state.app_handle {
                    let _ = app_handle.emit(
                        "team:teammate_spawn_failed",
                        serde_json::json!({
                            "team_name": team_name,
                            "teammate_name": teammate_name,
                            "error": e.to_string(),
                        }),
                    );
                }
            }
        }

        spawned_teammates.push(SpawnedTeammateInfo {
            name: teammate_name,
            role: pending.role.clone(),
            model: pending.model.clone(),
            color: teammate_color,
        });
    }

    let spawned_count = spawned_teammates.len();
    let total_count = plan.teammates.len();

    info!(
        plan_id = %req.plan_id,
        team = %team_name,
        spawned = spawned_count,
        total = total_count,
        "Team plan approved — teammates spawned as CLI worker processes"
    );

    // Signal the blocking request_team_plan handler with the spawn results
    let decision_teammates: Vec<PlanDecisionTeammate> = spawned_teammates
        .iter()
        .map(|t| PlanDecisionTeammate {
            name: t.name.clone(),
            role: t.role.clone(),
            model: t.model.clone(),
            color: t.color.clone(),
        })
        .collect();

    state
        .team_tracker
        .resolve_plan(
            &req.plan_id,
            PlanDecision {
                approved: spawned_count > 0,
                team_name: Some(team_name.clone()),
                teammates_spawned: decision_teammates,
                message: format!(
                    "{}/{} teammates registered successfully",
                    spawned_count, total_count
                ),
            },
        )
        .await;

    Ok(Json(ApproveTeamPlanResponse {
        success: spawned_count > 0,
        team_name,
        teammates_spawned: spawned_teammates,
        message: format!(
            "{}/{} teammates registered successfully",
            spawned_count, total_count
        ),
    }))
}

// ============================================================================
// POST /api/team/plan/reject — Reject a team plan
// ============================================================================

/// Reject a team plan, signaling the blocking `request_team_plan` handler.
///
/// Called by the frontend when the user clicks Reject on the plan approval UI.
/// Removes the pending plan and signals the watch channel with a rejection.
pub async fn reject_team_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<RejectTeamPlanRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    info!(plan_id = %req.plan_id, "Team plan rejected by user");

    // Remove the pending plan
    state.team_tracker.take_pending_plan(&req.plan_id).await;

    // Signal the blocking handler with rejection
    state
        .team_tracker
        .resolve_plan(
            &req.plan_id,
            PlanDecision {
                approved: false,
                team_name: None,
                teammates_spawned: vec![],
                message: "Team plan rejected by user".to_string(),
            },
        )
        .await;

    Ok(StatusCode::OK)
}

// ============================================================================
// POST /api/team/spawn — Request to spawn a single teammate
// ============================================================================

/// Spawn a teammate agent process.
///
/// Validates the request against team constraints, spawns an interactive Claude
/// process, registers it in TeamStateTracker, and starts background stdout streaming.
///
/// Flow:
/// 1. Load constraints from ralphx.yaml
/// 2. Validate model ≤ model_cap, tools ∩ allowed_tools, teammate_count < max
/// 3. Find active team (or return error)
/// 4. Spawn via ClaudeCodeClient::spawn_teammate_interactive()
/// 5. Register TeammateHandle in tracker
/// 6. Emit team:teammate_spawned event
pub async fn request_teammate_spawn(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestTeammateSpawnRequest>,
) -> Result<Json<RequestTeammateSpawnResponse>, (StatusCode, String)> {
    info!(
        role = %req.role,
        model = %req.model,
        tool_count = req.tools.len(),
        mcp_tool_count = req.mcp_tools.len(),
        "Teammate spawn requested — validating and spawning"
    );

    // 1. Validate against constraints (use "ideation" as default process)
    let config = team_constraints_config();
    let constraints = get_team_constraints(config, "ideation");

    // Validate individual teammate spawn request
    let spawn_req = TeammateSpawnRequest {
        role: req.role.clone(),
        prompt: Some(req.prompt.clone()),
        preset: req.preset.clone(),
        tools: req.tools.clone(),
        mcp_tools: req.mcp_tools.clone(),
        model: req.model.clone(),
        prompt_summary: None,
    };

    // Validate as a single-teammate plan
    let _approved = validate_team_plan(&constraints, "ideation", &[spawn_req]).map_err(|e| {
        warn!(role = %req.role, error = %e, "Teammate spawn validation failed");
        (
            StatusCode::BAD_REQUEST,
            format!("Spawn validation failed: {e}"),
        )
    })?;

    // 2. Find the active team
    let (team_name, context_id, context_type) = find_active_team(&state).await.map_err(|e| {
        error!(error = %e, "No active team found for teammate spawn");
        (StatusCode::CONFLICT, e)
    })?;

    // Resolve working directory (worktree-aware for task contexts)
    let working_dir = resolve_teammate_working_dir(&state, &context_type, &context_id).await;

    // 3. Generate unique teammate name (add suffix for uniqueness)
    let teammate_name = generate_unique_teammate_name(&state, &team_name, &req.role).await;
    let teammate_color = assign_teammate_color(&state, &team_name).await;

    // 4. Check teammate count against constraint max
    let current_count = state
        .team_service
        .get_teammate_count(&team_name)
        .await
        .unwrap_or(0);
    if current_count >= constraints.max_teammates as usize {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Max teammates exceeded: {current_count} >= {}",
                constraints.max_teammates
            ),
        ));
    }

    // 5. Register teammate via TeamService (persists to DB + emits events)
    state
        .team_service
        .add_teammate(
            &team_name,
            &teammate_name,
            &teammate_color,
            &req.model,
            &req.role,
        )
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to register teammate");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // 6. Build spawn config and spawn the process.
    //    Resolve lead session ID from Claude Code team config, falling back to context_id.
    let source;
    let parent_session_id = if let Some(sid) = resolve_lead_session_from_config(&team_name) {
        source = "team_config_file";
        sid
    } else {
        source = "context_id_fallback";
        context_id.clone()
    };

    tracing::info!(
        parent_session_id = %parent_session_id,
        source = %source,
        context_id = %context_id,
        team = %team_name,
        "[TEAM_SPAWN] Resolved parent_session_id for teammate (ad-hoc flow)"
    );

    // Resolve project ID for RALPHX_PROJECT_ID env var on teammates
    let project_id = resolve_teammate_project_id(
        &state,
        &context_type,
        &context_id,
    )
    .await;

    let teammate_context = TeammateContext {
        context_id: context_id.clone(),
        context_type: context_type.clone(),
        project_id,
    };

    // Always inject team communication tools regardless of spawn request specification.
    // Without these, teammates cannot send messages or coordinate via task lists.
    let mut tools = req.tools.clone();
    for required in ["SendMessage", "TaskCreate", "TaskUpdate", "TaskList", "TaskGet"] {
        if !tools.contains(&required.to_string()) {
            tools.push(required.to_string());
        }
    }

    let spawn_config =
        TeammateSpawnConfig::new(&teammate_name, &team_name, &req.prompt)
            .with_parent_session_id(&parent_session_id)
            .with_context(teammate_context)
            .with_model(&req.model)
            .with_tools(tools)
            .with_mcp_tools(req.mcp_tools.clone())
            .with_color(&teammate_color)
            .with_working_dir(working_dir.clone())
            .with_plugin_dir(working_dir.join("ralphx-plugin"));

    let client = ClaudeCodeClient::new();
    match client.spawn_teammate_interactive(spawn_config).await {
        Ok(spawn_result) => {
            info!(
                teammate = %teammate_name,
                team = %team_name,
                pid = ?spawn_result.child.id(),
                "Teammate process spawned successfully"
            );

            // 7. Take stdout/stderr from child for stream processing, then create handle
            let child_pid = spawn_result.child.id();
            let mut child = spawn_result.child;
            let stdout = child.stdout.take();

            // Drain stderr in background to capture crash messages without deadlock.
            // Logs any output at error level when the process exits.
            if let Some(stderr) = child.stderr.take() {
                let name = teammate_name.clone();
                let team = team_name.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncBufReadExt, BufReader};
                    let mut lines = BufReader::new(stderr).lines();
                    let mut output = Vec::new();
                    while let Ok(Some(line)) = lines.next_line().await {
                        output.push(line);
                    }
                    if !output.is_empty() {
                        tracing::error!(
                            teammate = %name,
                            team = %team,
                            stderr = %output.join("\n"),
                            "Teammate process stderr (crash/MCP init error)"
                        );
                    }
                });
            }

            // Process monitor: owns child, signals stream processor when Claude exits.
            // Prevents 3600s timeout when a grandchild (e.g., Node.js MCP server) inherits
            // the stdout pipe and holds it open after Claude exits.
            let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
            let (exit_tx, exit_rx) = tokio::sync::oneshot::channel::<()>();
            {
                let monitor_name = teammate_name.clone();
                let monitor_team = team_name.clone();
                tokio::spawn(async move {
                    tokio::select! {
                        biased;
                        _ = kill_rx => {
                            tracing::info!(
                                teammate = %monitor_name,
                                team = %monitor_team,
                                "Teammate process kill signal received"
                            );
                            let _ = child.kill().await;
                            let _ = child.wait().await;
                        }
                        status = child.wait() => {
                            tracing::info!(
                                teammate = %monitor_name,
                                team = %monitor_team,
                                status = ?status,
                                "Teammate process exited naturally"
                            );
                        }
                    }
                    // Signal stream processor to stop (pipe inheritance guard)
                    let _ = exit_tx.send(());
                });
            }

            // 8. Start background stream processor if we have both stdout and app_handle
            let stream_task = match (stdout, &state.app_state.app_handle) {
                (Some(stdout), Some(app_handle)) => Some(
                    crate::application::team_stream_processor::start_teammate_stream(
                        stdout,
                        exit_rx,
                        team_name.clone(),
                        teammate_name.clone(),
                        "ideation".to_string(),
                        context_id.clone(),
                        app_handle.clone(),
                        std::sync::Arc::new(state.team_tracker.clone()),
                        Some(state.team_service.clone()),
                        Some(std::sync::Arc::clone(&state.app_state.chat_conversation_repo)),
                        Some(std::sync::Arc::clone(&state.app_state.chat_message_repo)),
                        Some(std::sync::Arc::clone(&state.app_state.interactive_process_registry)),
                        Some(std::sync::Arc::clone(&state.execution_state)),
                    ),
                ),
                (None, _) => {
                    warn!(
                        teammate = %teammate_name,
                        "No stdout pipe available for teammate stream processing"
                    );
                    None
                }
                (_, None) => {
                    warn!(
                        teammate = %teammate_name,
                        "No AppHandle available for teammate event emission"
                    );
                    None
                }
            };

            // 9. Create TeammateHandle and register via TeamService
            let handle = TeammateHandle {
                kill_tx: Some(kill_tx),
                stream_task,
                stdin: Some(spawn_result.stdin),
                child_pid,
            };

            state
                .team_service
                .set_teammate_handle(&team_name, &teammate_name, handle)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to set teammate handle");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                })?;

            // 10. Update status to Running (via TeamService for persistence + events)
            let _ = state
                .team_service
                .update_teammate_status(&team_name, &teammate_name, TeammateStatus::Running)
                .await;

            Ok(Json(RequestTeammateSpawnResponse {
                success: true,
                message: format!(
                    "Teammate '{}' spawned for team '{}'",
                    teammate_name, team_name
                ),
                teammate_name,
            }))
        }
        Err(e) => {
            // Spawn failed — update status and return error
            warn!(
                teammate = %teammate_name,
                team = %team_name,
                error = %e,
                "Teammate spawn failed"
            );

            let _ = state
                .team_service
                .update_teammate_status(&team_name, &teammate_name, TeammateStatus::Failed)
                .await;

            if let Some(app_handle) = &state.app_state.app_handle {
                let _ = app_handle.emit(
                    "team:teammate_spawn_failed",
                    serde_json::json!({
                        "team_name": team_name,
                        "teammate_name": teammate_name,
                        "error": e.to_string(),
                    }),
                );
            }

            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Teammate spawn failed: {e}"),
            ))
        }
    }
}

// ============================================================================
// Spawn helpers
// ============================================================================

/// Context types where context_id is a task ID (worktree resolution applies).
const TASK_CONTEXT_TYPES: &[&str] = &["task_execution", "task", "review", "merge"];

/// Resolve the working directory for a teammate spawn.
///
/// Mirrors `chat_service_context::resolve_working_directory` — every context type
/// resolves through the project's `working_directory` so teammates always use the
/// same CWD as the lead agent.
async fn resolve_teammate_working_dir(
    state: &HttpServerState,
    context_type: &str,
    context_id: &str,
) -> PathBuf {
    // Ideation context: session → project → project.working_directory
    if context_type == "ideation" {
        use crate::domain::entities::IdeationSessionId;
        let session_id = IdeationSessionId::from_string(context_id);
        if let Ok(Some(session)) = state
            .app_state
            .ideation_session_repo
            .get_by_id(&session_id)
            .await
        {
            if let Ok(Some(project)) = state
                .app_state
                .project_repo
                .get_by_id(&session.project_id)
                .await
            {
                return PathBuf::from(&project.working_directory);
            }
        }
        warn!(
            context_type,
            context_id,
            "Teammate working dir: ideation session/project lookup failed — using default"
        );
        return default_working_dir();
    }

    // Project context: project → project.working_directory
    if context_type == "project" {
        use crate::domain::entities::ProjectId;
        let project_id = ProjectId::from_string(context_id.to_string());
        if let Ok(Some(project)) = state.app_state.project_repo.get_by_id(&project_id).await {
            return PathBuf::from(&project.working_directory);
        }
        warn!(
            context_type,
            context_id,
            "Teammate working dir: project lookup failed — using default"
        );
        return default_working_dir();
    }

    // Task-related contexts: task → worktree_path or project.working_directory
    if !TASK_CONTEXT_TYPES.contains(&context_type) {
        return default_working_dir();
    }

    let task_id = TaskId(context_id.to_string());

    let task = match state.app_state.task_repo.get_by_id(&task_id).await {
        Ok(Some(task)) => task,
        Ok(None) => {
            warn!(
                context_id = context_id,
                "Teammate working dir: task not found — using default"
            );
            return default_working_dir();
        }
        Err(e) => {
            warn!(
                context_id = context_id,
                error = %e,
                "Teammate working dir: task lookup failed — using default"
            );
            return default_working_dir();
        }
    };

    let project = match state
        .app_state
        .project_repo
        .get_by_id(&task.project_id)
        .await
    {
        Ok(Some(project)) => project,
        Ok(None) => {
            warn!(
                project_id = %task.project_id,
                "Teammate working dir: project not found — using default"
            );
            return default_working_dir();
        }
        Err(e) => {
            warn!(
                project_id = %task.project_id,
                error = %e,
                "Teammate working dir: project lookup failed — using default"
            );
            return default_working_dir();
        }
    };

    if let Some(ref wt_path) = task.worktree_path {
        info!(
            task_id = context_id,
            worktree_path = wt_path,
            "Teammate working dir: using task worktree path"
        );
        PathBuf::from(wt_path)
    } else {
        // No worktree — use the project's working directory (repo root)
        PathBuf::from(&project.working_directory)
    }
}

/// Fallback working directory (same as TeammateSpawnConfig::new default).
fn default_working_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Resolve the project ID for a teammate's context, mirroring the lead agent's
/// RALPHX_PROJECT_ID resolution from `chat_service_context::resolve_project_id`.
async fn resolve_teammate_project_id(
    state: &HttpServerState,
    context_type: &str,
    context_id: &str,
) -> Option<String> {
    use crate::domain::entities::chat_conversation::ChatContextType;
    let ct = context_type.parse::<ChatContextType>().ok()?;
    crate::application::chat_service::chat_service_context::resolve_project_id(
        ct,
        context_id,
        state.app_state.task_repo.clone(),
        state.app_state.ideation_session_repo.clone(),
    )
    .await
}

/// Color palette for teammate distinction
const TEAMMATE_COLORS: &[&str] = &["blue", "green", "cyan", "magenta", "yellow"];

/// Find the first active team via TeamService.
/// Returns (team_name, context_id, context_type).
async fn find_active_team(state: &HttpServerState) -> Result<(String, String, String), String> {
    let teams = state.team_service.list_teams().await;
    for team_name in &teams {
        if let Ok(status) = state.team_service.get_team_status(team_name).await {
            let phase = status.phase;
            if phase == crate::application::team_state_tracker::TeamPhase::Active
                || phase == crate::application::team_state_tracker::TeamPhase::Forming
            {
                return Ok((team_name.clone(), status.context_id, status.context_type));
            }
        }
    }
    Err("No active team found. Create a team before spawning teammates.".to_string())
}

/// Read the lead's Claude Code session ID from the team config file.
///
/// Claude Code's `TeamCreate` tool writes `~/.claude/teams/{name}/config.json`
/// with a `leadSessionId` field. This is the most reliable source when the
/// `RALPHX_LEAD_SESSION_ID` env var wasn't set (first spawn, session_id not yet known).
fn resolve_lead_session_from_config(team_name: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let config_path = PathBuf::from(home)
        .join(".claude/teams")
        .join(team_name)
        .join("config.json");
    let content = std::fs::read_to_string(&config_path).ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    let session_id = config.get("leadSessionId")?.as_str().map(|s| s.to_string());
    if session_id.is_some() {
        tracing::info!(
            team = %team_name,
            config_path = %config_path.display(),
            "[TEAM_SPAWN] Resolved leadSessionId from Claude Code team config"
        );
    }
    session_id
}

/// Generate a unique teammate name, appending a suffix if needed.
async fn generate_unique_teammate_name(
    state: &HttpServerState,
    team_name: &str,
    role: &str,
) -> String {
    let base_name = role.to_string();
    if let Ok(status) = state.team_service.get_team_status(team_name).await {
        let existing_names: Vec<&str> = status.teammates.iter().map(|t| t.name.as_str()).collect();
        if !existing_names.contains(&base_name.as_str()) {
            return base_name;
        }
        // Find next available suffix
        for i in 2..=99 {
            let candidate = format!("{}-{}", base_name, i);
            if !existing_names.contains(&candidate.as_str()) {
                return candidate;
            }
        }
    }
    base_name
}

/// Assign a color from the palette based on current teammate count.
async fn assign_teammate_color(state: &HttpServerState, team_name: &str) -> String {
    let count = state
        .team_service
        .get_teammate_count(team_name)
        .await
        .unwrap_or(0);
    TEAMMATE_COLORS[count % TEAMMATE_COLORS.len()].to_string()
}

// ============================================================================
// POST /api/team/artifact — Create a team artifact
// ============================================================================

/// Create a team artifact in the 'team-findings' bucket.
///
/// Accepts: { session_id, title, content, artifact_type, related_artifact_id? }
/// Maps artifact_type strings to ArtifactType variants.
pub async fn create_team_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreateTeamArtifactRequest>,
) -> Result<Json<CreateTeamArtifactResponse>, (StatusCode, String)> {
    // Map team artifact types to ArtifactType
    let artifact_type = match req.artifact_type.as_str() {
        "TeamResearch" => ArtifactType::TeamResearch,
        "TeamAnalysis" => ArtifactType::TeamAnalysis,
        "TeamSummary" => ArtifactType::TeamSummary,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid artifact_type: '{}'. Valid: TeamResearch, TeamAnalysis, TeamSummary",
                    other
                ),
            ));
        }
    };

    // Create the artifact
    let mut artifact = Artifact::new_inline(&req.title, artifact_type, &req.content, "team-lead");

    // Set bucket to team-findings
    artifact.bucket_id = Some(ArtifactBucketId::from_string("team-findings"));

    // Store team metadata with session_id
    artifact.metadata.team_metadata = Some(crate::domain::entities::TeamArtifactMetadata {
        team_name: "team".to_string(),
        author_teammate: "team-lead".to_string(),
        session_id: Some(req.session_id.clone()),
        team_phase: None,
    });

    let artifact_id = artifact.id.to_string();

    state
        .app_state
        .artifact_repo
        .create(artifact)
        .await
        .map_err(|e| {
            error!("Failed to create team artifact: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // Link to related artifact if provided
    if let Some(related_id) = &req.related_artifact_id {
        let relation = ArtifactRelation {
            id: ArtifactRelationId::new(),
            from_artifact_id: ArtifactId::from_string(artifact_id.clone()),
            to_artifact_id: ArtifactId::from_string(related_id.clone()),
            relation_type: ArtifactRelationType::RelatedTo,
        };
        let _ = state.app_state.artifact_repo.add_relation(relation).await;
    }

    info!(
        artifact_id = %artifact_id,
        session_id = %req.session_id,
        artifact_type = %req.artifact_type,
        "Team artifact created"
    );

    // Emit Tauri event so the frontend can live-update artifact lists
    if let Some(app_handle) = &state.app_state.app_handle {
        use crate::application::chat_service::{events, TeamArtifactCreatedPayload};
        let _ = app_handle.emit(
            events::TEAM_ARTIFACT_CREATED,
            TeamArtifactCreatedPayload {
                artifact_id: artifact_id.clone(),
                session_id: req.session_id.clone(),
                artifact_type: req.artifact_type.clone(),
                title: req.title.clone(),
            },
        );
    }

    Ok(Json(CreateTeamArtifactResponse { artifact_id }))
}

// ============================================================================
// GET /api/team/artifacts/:session_id — Get team artifacts for a session
// ============================================================================

/// Retrieve all team artifacts for a given session.
///
/// Filters artifacts in the 'team-findings' bucket by session_id in custom metadata.
pub async fn get_team_artifacts(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<GetTeamArtifactsResponse>, (StatusCode, String)> {
    // Get all artifacts from the team-findings bucket
    let bucket_id = ArtifactBucketId::from_string("team-findings");
    let artifacts = state
        .app_state
        .artifact_repo
        .get_by_bucket(&bucket_id)
        .await
        .map_err(|e| {
            error!("Failed to get team artifacts: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // Filter by session_id in team metadata
    let filtered: Vec<TeamArtifactSummary> = artifacts
        .into_iter()
        .filter(|a| {
            a.metadata
                .team_metadata
                .as_ref()
                .and_then(|tm| tm.session_id.as_deref())
                == Some(session_id.as_str())
        })
        .map(|a| {
            let content_preview = match &a.content {
                ArtifactContent::Inline { text } => {
                    if text.chars().count() <= 200 {
                        text.clone()
                    } else {
                        let truncated: String = text.chars().take(200).collect();
                        format!("{truncated}...")
                    }
                }
                ArtifactContent::File { path } => format!("[File: {}]", path),
            };
            let author_teammate = a
                .metadata
                .team_metadata
                .as_ref()
                .map(|tm| tm.author_teammate.clone());
            TeamArtifactSummary {
                id: a.id.to_string(),
                name: a.name.clone(),
                artifact_type: format!("{:?}", a.artifact_type),
                version: a.metadata.version,
                content_preview,
                created_at: a.metadata.created_at.to_rfc3339(),
                author_teammate,
            }
        })
        .collect();

    let count = filtered.len();
    Ok(Json(GetTeamArtifactsResponse {
        artifacts: filtered,
        count,
    }))
}

// ============================================================================
// GET /api/team/session_state/:session_id — Get team session state
// ============================================================================

/// Retrieve team session state from the in-memory tracker.
///
/// Checks active teams in TeamStateTracker for a match. Returns "none" phase
/// if no active team is found for this session.
pub async fn get_team_session_state(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<TeamSessionStateResponse>, (StatusCode, String)> {
    // Check active teams via TeamService
    let teams = state.team_service.list_teams().await;

    // Look for a team that matches this session_id (teams use session_id as context)
    for team_name in &teams {
        if let Ok(status) = state.team_service.get_team_status(team_name).await {
            // Check if this team's context matches the session_id
            if status.context_id == session_id {
                let team_composition: Vec<TeamCompositionEntry> = status
                    .teammates
                    .iter()
                    .map(|t| TeamCompositionEntry {
                        name: t.name.clone(),
                        role: t.role.clone(),
                        prompt: String::new(),
                        model: t.model.clone(),
                    })
                    .collect();

                return Ok(Json(TeamSessionStateResponse {
                    session_id,
                    team_name: Some(team_name.clone()),
                    phase: status.phase.to_string(),
                    team_composition,
                    artifact_ids: vec![],
                }));
            }
        }
    }

    // No active team found for this session
    Ok(Json(TeamSessionStateResponse {
        session_id,
        team_name: None,
        phase: "none".to_string(),
        team_composition: vec![],
        artifact_ids: vec![],
    }))
}

// ============================================================================
// POST /api/team/session_state — Save team session state
// ============================================================================

/// Save team session state via event emission.
///
/// Emits a team:session_state_saved event for the frontend to persist.
/// The in-memory state is tracked by TeamStateTracker.
pub async fn save_team_session_state(
    State(state): State<HttpServerState>,
    Json(req): Json<SaveTeamSessionStateRequest>,
) -> Result<Json<SaveTeamSessionStateResponse>, (StatusCode, String)> {
    // Emit event for frontend/persistence layer
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "team:session_state_saved",
            serde_json::json!({
                "session_id": req.session_id,
                "phase": req.phase,
                "team_composition": req.team_composition,
                "artifact_ids": req.artifact_ids,
            }),
        );
    }

    info!(
        session_id = %req.session_id,
        phase = %req.phase,
        teammates = req.team_composition.len(),
        "Team session state saved"
    );

    Ok(Json(SaveTeamSessionStateResponse {
        success: true,
        message: format!(
            "Team session state saved for session '{}' (phase: {}, {} teammates)",
            req.session_id,
            req.phase,
            req.team_composition.len()
        ),
    }))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[path = "teams_tests.rs"]
mod tests;
