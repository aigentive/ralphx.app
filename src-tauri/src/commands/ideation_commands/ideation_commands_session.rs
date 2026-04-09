// Session management commands

use std::path::PathBuf;
use std::sync::Arc;

use tauri::{Emitter, State};

use ralphx_domain::entities::EventType;

use crate::application::git_service::GitService;
use crate::application::session_namer_prompt::build_session_namer_prompt;
use crate::application::AppState;
use crate::application::{StopMode, TaskCleanupService};
use crate::domain::entities::plan_branch::PlanBranchStatus;
use crate::domain::entities::{
    IdeationSession, IdeationSessionId, IdeationSessionStatus, ProjectId, TaskId,
};

use super::ideation_commands_types::{
    ChatMessageResponse, CreateSessionInput, IdeationSessionResponse,
    IdeationSessionWithProgressResponse, SessionGroupCountsResponse, SessionListResponse,
    SessionWithDataResponse, TaskProposalResponse,
};

// ============================================================================
// Session Management Commands
// ============================================================================

/// Core implementation for creating an ideation session.
/// Generic over Runtime to enable unit testing with MockRuntime.
#[doc(hidden)]
pub async fn create_ideation_session_impl<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &AppState,
    input: CreateSessionInput,
) -> Result<IdeationSessionResponse, String> {
    let project_id = ProjectId::from_string(input.project_id);
    let seed_task_id = input.seed_task_id.map(TaskId::from_string);
    let team_mode_requested = input.team_mode.as_deref().is_some_and(|mode| mode != "solo");
    let team_mode_supported = crate::application::ideation_harness_availability::ideation_team_mode_supported_for_project(
        &state.agent_lane_settings_repo,
        Some(project_id.as_str()),
    )
    .await;
    let normalized_team_mode = if team_mode_requested && !team_mode_supported {
        tracing::info!(
            project_id = %project_id,
            "Downgrading ideation session team mode to solo because the primary harness does not support team mode"
        );
        Some("solo".to_string())
    } else {
        input.team_mode.clone()
    };
    let normalized_team_config_json = if team_mode_requested && !team_mode_supported {
        None
    } else {
        input.team_config
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| e.to_string())?
    };

    let mut builder = IdeationSession::builder().project_id(project_id);

    if let Some(title) = input.title {
        builder = builder.title(title);
    }

    if let Some(task_id) = seed_task_id {
        builder = builder.seed_task_id(task_id);
    }

    if let Some(ref team_mode) = normalized_team_mode {
        builder = builder.team_mode(team_mode.clone());
    }

    if let Some(config_json) = normalized_team_config_json {
        builder = builder.team_config_json(config_json);
    }

    let session = builder.build();

    let created = state
        .ideation_session_repo
        .create(session)
        .await
        .map_err(|e| e.to_string())?;

    let payload_json = serde_json::json!({
        "sessionId": created.id.to_string(),
        "projectId": created.project_id.to_string(),
    });
    let _ = app.emit("ideation:session_created", &payload_json);

    // Layer 2: persist to external_events table (non-fatal)
    if let Err(e) = state
        .external_events_repo
        .insert_event(
            "ideation:session_created",
            &created.project_id.to_string(),
            &payload_json.to_string(),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to persist IdeationSessionCreated event");
    }

    // Layer 3: webhook push (fire-and-forget, non-fatal)
    if let Some(ref publisher) = state.webhook_publisher {
        let _ = publisher
            .publish(
                EventType::IdeationSessionCreated,
                &created.project_id.to_string(),
                payload_json,
            )
            .await;
    }

    Ok(IdeationSessionResponse::from(created))
}

/// Create a new ideation session
#[tauri::command]
pub async fn create_ideation_session(
    input: CreateSessionInput,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IdeationSessionResponse, String> {
    create_ideation_session_impl(&app, &state, input).await
}

/// Get a single ideation session by ID
#[tauri::command]
pub async fn get_ideation_session(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<IdeationSessionResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);
    state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map(|opt| opt.map(IdeationSessionResponse::from))
        .map_err(|e| e.to_string())
}

/// Get session with proposals and messages
#[tauri::command]
pub async fn get_ideation_session_with_data(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<SessionWithDataResponse>, String> {
    let session_id = IdeationSessionId::from_string(id);

    // Get session
    let session = match state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(s) => s,
        None => return Ok(None),
    };

    // Get proposals
    let proposals = state
        .task_proposal_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Get messages
    let messages = state
        .chat_message_repo
        .get_by_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Some(SessionWithDataResponse {
        session: IdeationSessionResponse::from(session),
        proposals: proposals
            .into_iter()
            .map(TaskProposalResponse::from)
            .collect(),
        messages: messages
            .into_iter()
            .map(ChatMessageResponse::from)
            .collect(),
    }))
}

/// List all ideation sessions for a project, optionally filtered by purpose
#[tauri::command]
pub async fn list_ideation_sessions(
    project_id: String,
    purpose: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<IdeationSessionResponse>, String> {
    let project_id = ProjectId::from_string(project_id);
    state
        .ideation_session_repo
        .get_by_project(&project_id)
        .await
        .map(|sessions| {
            sessions
                .into_iter()
                .filter(|s| {
                    if let Some(ref p) = purpose {
                        s.session_purpose.to_string() == p.as_str()
                    } else {
                        true
                    }
                })
                .map(IdeationSessionResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Archive an ideation session
#[tauri::command]
pub async fn archive_ideation_session(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id.clone());

    // Get session to retrieve project_id
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {}", id))?;

    // 1. Get all tasks for this session
    let tasks = state
        .task_repo
        .get_by_ideation_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Stop ideation agent and clean up tasks (archive them, stop agents, clean git)
    let cleanup_service = TaskCleanupService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    )
    .with_interactive_process_registry(Arc::clone(&state.interactive_process_registry));

    // Stop the ideation session agent first
    let found = cleanup_service.stop_ideation_session_agent(&id).await;
    if !found {
        tracing::debug!(
            session_id = id.as_str(),
            "archive_ideation_session: no running agent found in IPR (expected for sessions without active agents)"
        );
    }

    // Archive all session tasks (stop agents, clean git branches/worktrees, archive in DB)
    let report = cleanup_service
        .cleanup_tasks(&tasks, StopMode::DirectStop, true)
        .await;
    if !report.errors.is_empty() {
        tracing::warn!(
            session_id = id.as_str(),
            errors = ?report.errors,
            "Some tasks failed during session archive cleanup"
        );
    }

    // 3. Clean up plan branch (best-effort)
    if let Ok(Some(plan_branch)) = state.plan_branch_repo.get_by_session_id(&session_id).await {
        // Best-effort delete the git feature branch
        let project = state
            .project_repo
            .get_by_id(&session.project_id)
            .await
            .ok()
            .flatten();

        if let Some(project) = project {
            let repo_path = PathBuf::from(&project.working_directory);
            if let Err(e) =
                GitService::delete_feature_branch(&repo_path, &plan_branch.branch_name).await
            {
                tracing::warn!(
                    branch = plan_branch.branch_name.as_str(),
                    error = %e,
                    "Failed to delete git feature branch during session archive (best-effort)"
                );
            }
        }

        // Mark plan branch as Abandoned
        if let Err(e) = state
            .plan_branch_repo
            .update_status(&plan_branch.id, PlanBranchStatus::Abandoned)
            .await
        {
            tracing::warn!(
                plan_branch_id = plan_branch.id.as_str(),
                error = %e,
                "Failed to mark plan branch as Abandoned during session archive"
            );
        }
    }

    // 4. Archive the session
    state
        .ideation_session_repo
        .update_status(&session_id, IdeationSessionStatus::Archived)
        .await
        .map_err(|e| e.to_string())
}

/// Delete an ideation session with cascade: stop active agents, delete tasks, clean up plan branch
#[tauri::command]
pub async fn delete_ideation_session(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let session_id = IdeationSessionId::from_string(id.clone());

    // Get session to retrieve project_id for events
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {}", id))?;

    // 1. Get all tasks for this session
    let tasks = state
        .task_repo
        .get_by_ideation_session(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // 2. Clean up tasks: stop agents, clean git branches/worktrees, delete from DB, emit events
    let cleanup_service = TaskCleanupService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    )
    .with_interactive_process_registry(Arc::clone(&state.interactive_process_registry));
    let report = cleanup_service
        .cleanup_tasks(&tasks, StopMode::DirectStop, true)
        .await;
    if !report.errors.is_empty() {
        tracing::warn!(
            session_id = id.as_str(),
            errors = ?report.errors,
            "Some tasks failed during session deletion cleanup"
        );
    }

    // 3. Clean up plan branch (best-effort)
    if let Ok(Some(plan_branch)) = state.plan_branch_repo.get_by_session_id(&session_id).await {
        // Best-effort delete the git feature branch
        let project = state
            .project_repo
            .get_by_id(&session.project_id)
            .await
            .ok()
            .flatten();

        if let Some(project) = project {
            let repo_path = PathBuf::from(&project.working_directory);
            if let Err(e) =
                GitService::delete_feature_branch(&repo_path, &plan_branch.branch_name).await
            {
                tracing::warn!(
                    branch = plan_branch.branch_name.as_str(),
                    error = %e,
                    "Failed to delete git feature branch during session deletion (best-effort)"
                );
            }
        }

        // Mark plan branch as Abandoned
        if let Err(e) = state
            .plan_branch_repo
            .update_status(&plan_branch.id, PlanBranchStatus::Abandoned)
            .await
        {
            tracing::warn!(
                plan_branch_id = plan_branch.id.as_str(),
                error = %e,
                "Failed to mark plan branch as Abandoned during session deletion"
            );
        }
    }

    // 4. Delete the session (existing CASCADE handles proposals/messages)
    state
        .ideation_session_repo
        .delete(&session_id)
        .await
        .map_err(|e| e.to_string())
}

/// Reopen an accepted/archived ideation session back to Active.
///
/// Cleanup: stops running agents, deletes tasks, cleans git branches/worktrees,
/// clears proposal task links, resets session to Active.
/// Emits ideation:session_reopened and task:list_changed events.
#[tauri::command]
pub async fn reopen_ideation_session(
    id: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use crate::application::session_reopen_service::SessionReopenService;
    use crate::application::task_cleanup_service::TaskCleanupService;

    let session_id = IdeationSessionId::from_string(id.clone());

    // Get project_id for events before reopening
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found: {}", id))?;
    let project_id_str = session.project_id.as_str().to_string();

    // Stop any running ideation agent before reopening (separate instance; task_cleanup is consumed
    // by SessionReopenService::new() and all deps are Arc<> clones so two instances are cheap)
    let stop_cleanup = TaskCleanupService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    )
    .with_interactive_process_registry(Arc::clone(&state.interactive_process_registry));
    let found = stop_cleanup.stop_ideation_session_agent(&id).await;
    if !found {
        tracing::debug!(
            session_id = id.as_str(),
            "reopen_ideation_session: no running agent found in IPR (expected for sessions without active agents)"
        );
    }

    let task_cleanup = TaskCleanupService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.running_agent_registry),
        Some(app.clone()),
    )
    .with_interactive_process_registry(Arc::clone(&state.interactive_process_registry));

    let service = SessionReopenService::new(
        Arc::clone(&state.task_repo),
        Arc::clone(&state.task_proposal_repo),
        Arc::clone(&state.ideation_session_repo),
        Arc::clone(&state.plan_branch_repo),
        Arc::clone(&state.project_repo),
        Arc::clone(&state.execution_plan_repo),
        task_cleanup,
    );

    service
        .reopen(&session_id, &state)
        .await
        .map_err(|e| e.to_string())?;

    // Emit events for real-time UI updates
    let _ = app.emit(
        "ideation:session_reopened",
        serde_json::json!({
            "sessionId": id,
            "projectId": project_id_str,
        }),
    );
    let _ = app.emit(
        "task:list_changed",
        serde_json::json!({
            "projectId": project_id_str,
        }),
    );

    Ok(())
}

/// Update the title of an ideation session
///
/// Sets or clears the session title and emits a real-time event for UI updates.
/// This is used by the session-namer agent for auto-generated titles and
/// by the frontend for manual renames.
#[tauri::command]
pub async fn update_ideation_session_title(
    id: String,
    title: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<IdeationSessionResponse, String> {
    let session_id = IdeationSessionId::from_string(id.clone());

    // Update the title in the database (UI rename = user-set title)
    state
        .ideation_session_repo
        .update_title(&session_id, title.clone(), "user")
        .await
        .map_err(|e| e.to_string())?;

    // Get the updated session to return
    let session = state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session not found after update: {}", id))?;

    // Emit event for real-time UI updates
    let _ = app.emit(
        "ideation:session_title_updated",
        serde_json::json!({
            "sessionId": id,
            "title": title,
        }),
    );

    Ok(IdeationSessionResponse::from(session))
}

/// Spawn the session-namer agent to auto-generate a title for the session
///
/// This is a fire-and-forget operation that spawns a background agent.
/// The agent will call the update_session_title MCP tool when complete,
/// which will emit an event for real-time UI updates.
#[tauri::command]
pub async fn spawn_session_namer(
    session_id: String,
    first_message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::application::harness_runtime_registry::{
        default_repo_root_working_directory, resolve_default_harness_plugin_dir,
    };
    use crate::domain::agents::{AgentConfig, AgentRole};
    use crate::domain::entities::IdeationSessionId;
    use crate::infrastructure::agents::claude::{agent_names, mcp_agent_type};

    // Build the prompt with session context (XML-delineated to prevent injection)
    let prompt = build_session_namer_prompt(&format!(
        "<session_id>{}</session_id>\n<user_message>{}</user_message>",
        session_id, first_message
    ));

    // Get the working directory (project root)
    let working_directory = default_repo_root_working_directory();
    let plugin_dir = resolve_default_harness_plugin_dir(&working_directory);

    // Set RALPHX_AGENT_TYPE so MCP server grants access to update_session_title tool
    let mut env = std::collections::HashMap::new();
    env.insert(
        "RALPHX_AGENT_TYPE".to_string(),
        mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string(),
    );

    let project_id = state
        .ideation_session_repo
        .get_by_id(&IdeationSessionId::from_string(session_id.clone()))
        .await
        .map_err(|e| e.to_string())?
        .map(|session| session.project_id.as_str().to_string());
    let runtime = state
        .resolve_ideation_background_agent_runtime(project_id.as_deref())
        .await;

    let config = AgentConfig {
        role: AgentRole::Custom(mcp_agent_type(agent_names::AGENT_SESSION_NAMER).to_string()),
        prompt,
        working_directory,
        plugin_dir: Some(plugin_dir),
        agent: Some(agent_names::AGENT_SESSION_NAMER.to_string()),
        model: runtime.model,
        harness: runtime.harness,
        logical_effort: runtime.logical_effort,
        approval_policy: runtime.approval_policy,
        sandbox_mode: runtime.sandbox_mode,
        max_tokens: None,
        timeout_secs: Some(60), // 60 second timeout for title generation
        env,
    };

    // Clone the agent client for the background task
    let agent_client = Arc::clone(&runtime.client);

    // Spawn in background (fire-and-forget)
    tokio::spawn(async move {
        match agent_client.spawn_agent(config).await {
            Ok(handle) => {
                // Wait for completion in the background
                if let Err(e) = agent_client.wait_for_completion(&handle).await {
                    tracing::warn!("Session namer agent failed: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to spawn session namer agent: {}", e);
            }
        }
    });

    Ok(())
}

/// Get child sessions for a parent session, with optional purpose filter.
/// Returns all non-archived children. When purpose is provided, filters by session_purpose.
#[tauri::command]
pub async fn get_child_sessions(
    session_id: String,
    purpose: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<IdeationSessionResponse>, String> {
    let parent_id = IdeationSessionId::from_string(session_id);
    state
        .ideation_session_repo
        .get_children(&parent_id)
        .await
        .map(|mut sessions| {
            // Exclude archived children
            sessions.retain(|s| {
                use crate::domain::entities::ideation::IdeationSessionStatus;
                s.status != IdeationSessionStatus::Archived
            });
            // Optionally filter by purpose
            if let Some(ref p) = purpose {
                sessions.retain(|s| &s.session_purpose.to_string() == p);
            }
            sessions
                .into_iter()
                .map(IdeationSessionResponse::from)
                .collect()
        })
        .map_err(|e| e.to_string())
}

/// Get group counts for all 5 session display groups for a project
///
/// Returns counts for: drafts (active sessions), in_progress (accepted + has active tasks),
/// accepted (accepted + no active tasks), done (accepted + all tasks terminal), archived.
/// Optional `search` filters by case-insensitive title substring match.
#[tauri::command]
pub async fn get_session_group_counts(
    project_id: String,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<SessionGroupCountsResponse, String> {
    let project_id = crate::domain::entities::ProjectId::from_string(project_id);
    let counts = state
        .ideation_session_repo
        .get_group_counts(&project_id, search.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(SessionGroupCountsResponse {
        drafts: counts.drafts,
        in_progress: counts.in_progress,
        accepted: counts.accepted,
        done: counts.done,
        archived: counts.archived,
    })
}

/// List paginated sessions for a specific group
///
/// Valid groups: "drafts", "in_progress", "accepted", "done", "archived".
/// Returns sessions with server-computed progress data and parent session title.
/// Optional `search` filters by case-insensitive title substring match.
#[tauri::command]
pub async fn list_sessions_by_group(
    project_id: String,
    group: String,
    offset: Option<u32>,
    limit: Option<u32>,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<SessionListResponse, String> {
    // Validate group early for a clear error message
    match group.as_str() {
        "drafts" | "in_progress" | "accepted" | "done" | "archived" => {}
        _ => {
            return Err(format!(
                "Unknown session group: '{}'. Valid groups: drafts, in_progress, accepted, done, archived",
                group
            ))
        }
    }

    let project_id = crate::domain::entities::ProjectId::from_string(project_id);
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(20);

    let (sessions, total) = state
        .ideation_session_repo
        .list_by_group(&project_id, &group, offset, limit, search.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let has_more = (offset + sessions.len() as u32) < total;

    let response_sessions = sessions
        .into_iter()
        .map(IdeationSessionWithProgressResponse::from)
        .collect();

    Ok(SessionListResponse {
        sessions: response_sessions,
        total,
        has_more,
        offset,
    })
}
