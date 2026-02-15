// Team HTTP handlers — endpoints for MCP team tools
//
// Provides HTTP endpoints for:
// - POST /api/team/plan        — request_team_plan (validates team plan against constraints)
// - POST /api/team/spawn       — request_teammate_spawn (validates, spawns, registers, streams)
// - POST /api/team/artifact    — create_team_artifact
// - GET  /api/team/artifacts/:session_id — get_team_artifacts
// - GET  /api/team/session_state/:session_id — get_team_session_state
// - POST /api/team/session_state — save_team_session_state

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tauri::Emitter;
use tracing::{error, info, warn};

use super::HttpServerState;
use crate::application::team_state_tracker::{TeammateHandle, TeammateStatus};
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactRelation,
    ArtifactRelationId, ArtifactRelationType, ArtifactType,
};
use crate::http_server::types::{
    CreateTeamArtifactRequest, CreateTeamArtifactResponse, GetTeamArtifactsResponse,
    RequestTeamPlanRequest, RequestTeamPlanResponse, RequestTeammateSpawnRequest,
    RequestTeammateSpawnResponse, SaveTeamSessionStateRequest, SaveTeamSessionStateResponse,
    TeamArtifactSummary, TeamCompositionEntry, TeamSessionStateResponse,
};
use crate::infrastructure::agents::claude::{
    get_team_constraints, team_constraints_config, validate_team_plan, ClaudeCodeClient,
    TeammateSpawnConfig, TeammateSpawnRequest,
};

// ============================================================================
// POST /api/team/plan — Request approval for a team plan
// ============================================================================

/// Accept a team plan from the team lead agent.
///
/// Validates the team plan against constraints from ralphx.yaml, then emits
/// a team:plan_requested event for user approval with validation results.
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
        (StatusCode::BAD_REQUEST, format!("Team plan validation failed: {e}"))
    })?;

    let plan_id = plan.plan_id.clone();

    info!(
        plan_id = %plan_id,
        process = %req.process,
        approved_teammates = plan.teammates.len(),
        "Team plan validated — emitting for user approval"
    );

    // Emit event for frontend to show approval UI (with validated plan)
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "team:plan_requested",
            serde_json::json!({
                "plan_id": plan_id,
                "process": req.process,
                "teammates": req.teammates,
                "validated": true,
            }),
        );
    }

    Ok(Json(RequestTeamPlanResponse {
        success: true,
        plan_id,
        message: format!(
            "Team plan validated and submitted with {} teammates for '{}' process",
            req.teammates.len(),
            req.process
        ),
    }))
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
        (StatusCode::BAD_REQUEST, format!("Spawn validation failed: {e}"))
    })?;

    // 2. Find the active team
    let (team_name, context_id) = find_active_team(&state).await.map_err(|e| {
        error!(error = %e, "No active team found for teammate spawn");
        (StatusCode::CONFLICT, e)
    })?;

    // 3. Generate unique teammate name (add suffix for uniqueness)
    let teammate_name = generate_unique_teammate_name(&state, &team_name, &req.role).await;
    let teammate_color = assign_teammate_color(&state, &team_name).await;

    // 4. Check teammate count against constraint max
    let current_count = state
        .team_tracker
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

    // 5. Register teammate in tracker (status: Spawning)
    state
        .team_tracker
        .add_teammate(&team_name, &teammate_name, &teammate_color, &req.model, &req.role)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to register teammate in tracker");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // 6. Build spawn config and spawn the process
    let spawn_config = TeammateSpawnConfig::new(
        &teammate_name,
        &team_name,
        &context_id,
        &req.prompt,
    )
    .with_model(&req.model)
    .with_tools(req.tools.clone())
    .with_mcp_tools(req.mcp_tools.clone())
    .with_color(&teammate_color);

    let client = ClaudeCodeClient::new();
    match client.spawn_teammate_interactive(spawn_config).await {
        Ok(spawn_result) => {
            info!(
                teammate = %teammate_name,
                team = %team_name,
                pid = ?spawn_result.child.id(),
                "Teammate process spawned successfully"
            );

            // 7. Create TeammateHandle and register in tracker
            let handle = TeammateHandle {
                child: spawn_result.child,
                stream_task: None,
                stdin: Some(spawn_result.stdin),
            };

            state
                .team_tracker
                .set_teammate_handle(&team_name, &teammate_name, handle)
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to set teammate handle");
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                })?;

            // 8. Update status to Running
            let _ = state
                .team_tracker
                .update_teammate_status(&team_name, &teammate_name, TeammateStatus::Running)
                .await;

            // 9. Emit spawned event
            if let Some(app_handle) = &state.app_state.app_handle {
                let _ = app_handle.emit(
                    "team:teammate_spawned",
                    serde_json::json!({
                        "team_name": team_name,
                        "teammate_name": teammate_name,
                        "role": req.role,
                        "model": req.model,
                        "color": teammate_color,
                    }),
                );
            }

            Ok(Json(RequestTeammateSpawnResponse {
                success: true,
                message: format!("Teammate '{}' spawned for team '{}'", teammate_name, team_name),
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
                .team_tracker
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

/// Color palette for teammate distinction
const TEAMMATE_COLORS: &[&str] = &["blue", "green", "cyan", "magenta", "yellow"];

/// Find the first active team in the tracker.
/// Returns (team_name, context_id).
async fn find_active_team(state: &HttpServerState) -> Result<(String, String), String> {
    let teams = state.team_tracker.list_teams().await;
    for team_name in &teams {
        if let Ok(status) = state.team_tracker.get_team_status(team_name).await {
            let phase = status.phase;
            if phase == crate::application::team_state_tracker::TeamPhase::Active
                || phase == crate::application::team_state_tracker::TeamPhase::Forming
            {
                return Ok((team_name.clone(), status.context_id));
            }
        }
    }
    Err("No active team found. Create a team before spawning teammates.".to_string())
}

/// Generate a unique teammate name, appending a suffix if needed.
async fn generate_unique_teammate_name(
    state: &HttpServerState,
    team_name: &str,
    role: &str,
) -> String {
    let base_name = role.to_string();
    if let Ok(status) = state.team_tracker.get_team_status(team_name).await {
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
        .team_tracker
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
            TeamArtifactSummary {
                id: a.id.to_string(),
                name: a.name.clone(),
                artifact_type: format!("{:?}", a.artifact_type),
                version: a.metadata.version,
                content_preview,
                created_at: a.metadata.created_at.to_rfc3339(),
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
    // Check active teams in the tracker
    let teams = state.team_tracker.list_teams().await;

    // Look for a team that matches this session_id (teams use session_id as context)
    for team_name in &teams {
        if let Ok(status) = state.team_tracker.get_team_status(team_name).await {
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
mod tests {
    use super::*;
    use crate::application::team_state_tracker::TeamStateTracker;

    #[test]
    fn team_artifact_type_mapping() {
        // Verify the string → ArtifactType mapping
        assert!(matches!(
            match "TeamResearch" {
                "TeamResearch" => Some(ArtifactType::TeamResearch),
                "TeamAnalysis" => Some(ArtifactType::TeamAnalysis),
                "TeamSummary" => Some(ArtifactType::TeamSummary),
                _ => None,
            },
            Some(ArtifactType::TeamResearch)
        ));

        assert!(matches!(
            match "TeamAnalysis" {
                "TeamResearch" => Some(ArtifactType::TeamResearch),
                "TeamAnalysis" => Some(ArtifactType::TeamAnalysis),
                "TeamSummary" => Some(ArtifactType::TeamSummary),
                _ => None,
            },
            Some(ArtifactType::TeamAnalysis)
        ));

        assert!(matches!(
            match "TeamSummary" {
                "TeamResearch" => Some(ArtifactType::TeamResearch),
                "TeamAnalysis" => Some(ArtifactType::TeamAnalysis),
                "TeamSummary" => Some(ArtifactType::TeamSummary),
                _ => None,
            },
            Some(ArtifactType::TeamSummary)
        ));

        // Invalid type
        assert!(
            match "InvalidType" {
                "TeamResearch" => Some(ArtifactType::TeamResearch),
                "TeamAnalysis" => Some(ArtifactType::TeamAnalysis),
                "TeamSummary" => Some(ArtifactType::TeamSummary),
                _ => None,
            }
            .is_none()
        );
    }

    #[test]
    fn team_composition_serialization() {
        let entry = TeamCompositionEntry {
            name: "researcher".to_string(),
            role: "explore".to_string(),
            prompt: "Research the topic".to_string(),
            model: "sonnet".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let parsed: TeamCompositionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "researcher");
        assert_eq!(parsed.role, "explore");
        assert_eq!(parsed.model, "sonnet");
    }

    #[test]
    fn request_teammate_spawn_deserialization() {
        let json = r#"{
            "role": "frontend-researcher",
            "prompt": "Research React patterns",
            "model": "sonnet",
            "tools": ["Read", "Grep", "Glob"],
            "mcp_tools": ["get_session_plan"],
            "preset": null
        }"#;

        let req: RequestTeammateSpawnRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, "frontend-researcher");
        assert_eq!(req.model, "sonnet");
        assert_eq!(req.tools.len(), 3);
        assert_eq!(req.mcp_tools.len(), 1);
        assert!(req.preset.is_none());
    }

    #[test]
    fn request_team_plan_deserialization() {
        let json = r#"{
            "process": "ideation-research",
            "teammates": [
                {
                    "role": "frontend-researcher",
                    "tools": ["Read", "Grep"],
                    "mcp_tools": ["get_session_plan"],
                    "model": "sonnet",
                    "prompt_summary": "Research React component patterns"
                }
            ]
        }"#;

        let req: RequestTeamPlanRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.process, "ideation-research");
        assert_eq!(req.teammates.len(), 1);
        assert_eq!(req.teammates[0].role, "frontend-researcher");
        assert_eq!(req.teammates[0].model, "sonnet");
    }

    #[test]
    fn save_team_session_state_deserialization() {
        let json = r#"{
            "session_id": "session-123",
            "team_composition": [
                {
                    "name": "researcher",
                    "role": "explore",
                    "prompt": "Research the topic",
                    "model": "sonnet"
                }
            ],
            "phase": "EXPLORE",
            "artifact_ids": ["art-1", "art-2"]
        }"#;

        let req: SaveTeamSessionStateRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.session_id, "session-123");
        assert_eq!(req.team_composition.len(), 1);
        assert_eq!(req.phase, "EXPLORE");
        assert_eq!(req.artifact_ids.as_ref().unwrap().len(), 2);
    }

    // ── Color palette tests ──────────────────────────────────────────

    #[test]
    fn teammate_colors_rotate() {
        assert_eq!(TEAMMATE_COLORS[0], "blue");
        assert_eq!(TEAMMATE_COLORS[1], "green");
        assert_eq!(TEAMMATE_COLORS.len(), 5);
        // Rotation wraps around
        assert_eq!(TEAMMATE_COLORS[5 % TEAMMATE_COLORS.len()], "blue");
    }

    // ── find_active_team tests ───────────────────────────────────────

    fn test_state() -> HttpServerState {
        use std::sync::Arc;
        HttpServerState {
            app_state: Arc::new(crate::application::AppState::new_test()),
            execution_state: Arc::new(crate::commands::ExecutionState::new()),
            team_tracker: TeamStateTracker::new(),
        }
    }

    #[tokio::test]
    async fn test_find_active_team_none_found() {
        let state = test_state();
        let result = find_active_team(&state).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No active team found"));
    }

    #[tokio::test]
    async fn test_find_active_team_forming() {
        let state = test_state();
        state
            .team_tracker
            .create_team("test-team", "session-123", "ideation")
            .await
            .unwrap();

        let (name, ctx_id) = find_active_team(&state).await.unwrap();
        assert_eq!(name, "test-team");
        assert_eq!(ctx_id, "session-123");
    }

    #[tokio::test]
    async fn test_find_active_team_active() {
        let state = test_state();
        state
            .team_tracker
            .create_team("my-team", "ctx-1", "ideation")
            .await
            .unwrap();
        // Adding a teammate transitions Forming → Active
        state
            .team_tracker
            .add_teammate("my-team", "worker", "#ff0000", "sonnet", "code")
            .await
            .unwrap();

        let (name, _) = find_active_team(&state).await.unwrap();
        assert_eq!(name, "my-team");
    }

    #[tokio::test]
    async fn test_find_active_team_skips_disbanded() {
        let state = test_state();
        state
            .team_tracker
            .create_team("old-team", "ctx-1", "ideation")
            .await
            .unwrap();
        state.team_tracker.disband_team("old-team").await.unwrap();

        let result = find_active_team(&state).await;
        assert!(result.is_err());
    }

    // ── generate_unique_teammate_name tests ──────────────────────────

    #[tokio::test]
    async fn test_unique_name_no_collision() {
        let state = test_state();
        state
            .team_tracker
            .create_team("t1", "ctx", "ideation")
            .await
            .unwrap();

        let name = generate_unique_teammate_name(&state, "t1", "researcher").await;
        assert_eq!(name, "researcher");
    }

    #[tokio::test]
    async fn test_unique_name_with_collision() {
        let state = test_state();
        state
            .team_tracker
            .create_team("t1", "ctx", "ideation")
            .await
            .unwrap();
        state
            .team_tracker
            .add_teammate("t1", "researcher", "#blue", "sonnet", "explore")
            .await
            .unwrap();

        let name = generate_unique_teammate_name(&state, "t1", "researcher").await;
        assert_eq!(name, "researcher-2");
    }

    #[tokio::test]
    async fn test_unique_name_multiple_collisions() {
        let state = test_state();
        state
            .team_tracker
            .create_team("t1", "ctx", "ideation")
            .await
            .unwrap();
        state
            .team_tracker
            .add_teammate("t1", "coder", "#blue", "sonnet", "code")
            .await
            .unwrap();
        state
            .team_tracker
            .add_teammate("t1", "coder-2", "#green", "sonnet", "code")
            .await
            .unwrap();

        let name = generate_unique_teammate_name(&state, "t1", "coder").await;
        assert_eq!(name, "coder-3");
    }

    // ── assign_teammate_color tests ─────────────────────────────────

    #[tokio::test]
    async fn test_assign_color_first_teammate() {
        let state = test_state();
        state
            .team_tracker
            .create_team("t1", "ctx", "ideation")
            .await
            .unwrap();

        let color = assign_teammate_color(&state, "t1").await;
        assert_eq!(color, "blue");
    }

    #[tokio::test]
    async fn test_assign_color_rotates() {
        let state = test_state();
        state
            .team_tracker
            .create_team("t1", "ctx", "ideation")
            .await
            .unwrap();
        state
            .team_tracker
            .add_teammate("t1", "a", "#blue", "sonnet", "code")
            .await
            .unwrap();

        let color = assign_teammate_color(&state, "t1").await;
        assert_eq!(color, "green");
    }

    // ── Team plan validation integration test ────────────────────────

    #[test]
    fn team_plan_request_converts_to_spawn_requests() {
        let req = RequestTeamPlanRequest {
            process: "ideation".to_string(),
            teammates: vec![
                crate::http_server::types::TeamPlanTeammate {
                    role: "researcher".to_string(),
                    tools: vec!["Read".to_string(), "Grep".to_string()],
                    mcp_tools: vec![],
                    model: "sonnet".to_string(),
                    preset: None,
                    prompt_summary: "Research patterns".to_string(),
                },
                crate::http_server::types::TeamPlanTeammate {
                    role: "analyzer".to_string(),
                    tools: vec!["Read".to_string()],
                    mcp_tools: vec![],
                    model: "haiku".to_string(),
                    preset: None,
                    prompt_summary: "Analyze results".to_string(),
                },
            ],
        };

        // Convert to spawn requests
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

        assert_eq!(spawn_requests.len(), 2);
        assert_eq!(spawn_requests[0].role, "researcher");
        assert_eq!(spawn_requests[1].model, "haiku");
    }
}
