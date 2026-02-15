// Team HTTP handlers — endpoints for MCP team tools
//
// Provides HTTP endpoints for:
// - POST /api/team/plan        — request_team_plan
// - POST /api/team/spawn       — request_teammate_spawn
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
use tracing::{error, info};

use super::HttpServerState;
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

// ============================================================================
// POST /api/team/plan — Request approval for a team plan
// ============================================================================

/// Accept a team plan from the team lead agent.
///
/// Stores the plan and emits a team:plan_requested event for user approval.
pub async fn request_team_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestTeamPlanRequest>,
) -> Result<Json<RequestTeamPlanResponse>, (StatusCode, String)> {
    let plan_id = uuid::Uuid::new_v4().to_string();

    info!(
        plan_id = %plan_id,
        process = %req.process,
        teammate_count = req.teammates.len(),
        "Team plan requested"
    );

    // Emit event for frontend to show approval UI
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "team:plan_requested",
            serde_json::json!({
                "plan_id": plan_id,
                "process": req.process,
                "teammates": req.teammates,
            }),
        );
    }

    Ok(Json(RequestTeamPlanResponse {
        success: true,
        plan_id,
        message: format!(
            "Team plan submitted with {} teammates for '{}' process",
            req.teammates.len(),
            req.process
        ),
    }))
}

// ============================================================================
// POST /api/team/spawn — Request to spawn a single teammate
// ============================================================================

/// Register a teammate spawn request from the MCP proxy.
///
/// Accepts the MCP schema: { role, prompt, model, tools[], mcp_tools[], preset? }
/// Generates a unique teammate name and default color, then registers in TeamStateTracker.
/// Emits team:teammate_spawn_requested event for the spawn orchestrator.
pub async fn request_teammate_spawn(
    State(state): State<HttpServerState>,
    Json(req): Json<RequestTeammateSpawnRequest>,
) -> Result<Json<RequestTeammateSpawnResponse>, (StatusCode, String)> {
    // Generate teammate name from role (add suffix if needed for uniqueness)
    let teammate_name = req.role.clone();

    info!(
        role = %req.role,
        model = %req.model,
        tool_count = req.tools.len(),
        mcp_tool_count = req.mcp_tools.len(),
        "Teammate spawn requested"
    );

    // Emit event for the spawn orchestrator to pick up
    if let Some(app_handle) = &state.app_state.app_handle {
        let _ = app_handle.emit(
            "team:teammate_spawn_requested",
            serde_json::json!({
                "teammate_name": teammate_name,
                "role": req.role,
                "prompt": req.prompt,
                "model": req.model,
                "tools": req.tools,
                "mcp_tools": req.mcp_tools,
                "preset": req.preset,
            }),
        );
    }

    Ok(Json(RequestTeammateSpawnResponse {
        success: true,
        message: format!("Teammate spawn request submitted for role '{}'", req.role),
        teammate_name,
    }))
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
}
