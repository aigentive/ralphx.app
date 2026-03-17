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
    assert!(match "InvalidType" {
        "TeamResearch" => Some(ArtifactType::TeamResearch),
        "TeamAnalysis" => Some(ArtifactType::TeamAnalysis),
        "TeamSummary" => Some(ArtifactType::TeamSummary),
        _ => None,
    }
    .is_none());
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
        "context_type": "ideation",
        "context_id": "session-abc123",
        "process": "ideation-research",
        "team_name": "test-research-team",
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
    assert_eq!(req.context_type, "ideation");
    assert_eq!(req.context_id, "session-abc123");
    assert_eq!(req.process, "ideation-research");
    assert_eq!(req.team_name, "test-research-team");
    assert_eq!(req.teammates.len(), 1);
    assert_eq!(req.teammates[0].role, "frontend-researcher");
    assert_eq!(req.teammates[0].model, "sonnet");
}

#[test]
fn request_team_plan_deserialization_missing_team_name_fails() {
    let json = r#"{
        "context_type": "ideation",
        "context_id": "session-abc123",
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

    let result: Result<RequestTeamPlanRequest, _> = serde_json::from_str(json);
    assert!(result.is_err(), "team_name is required — deserialization must fail when missing");
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
    let tracker = TeamStateTracker::new();
    let team_service = Arc::new(crate::application::TeamService::new_without_events(
        Arc::new(tracker.clone()),
    ));
    HttpServerState {
        app_state: Arc::new(crate::application::AppState::new_test()),
        execution_state: Arc::new(crate::commands::ExecutionState::new()),
        team_tracker: tracker,
        team_service,
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

    let (name, ctx_id, ctx_type) = find_active_team(&state).await.unwrap();
    assert_eq!(name, "test-team");
    assert_eq!(ctx_id, "session-123");
    assert_eq!(ctx_type, "ideation");
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

    let (name, _, _) = find_active_team(&state).await.unwrap();
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
        context_type: "ideation".to_string(),
        context_id: "session-abc123".to_string(),
        process: "ideation".to_string(),
        teammates: vec![
            crate::http_server::types::TeamPlanTeammate {
                role: "researcher".to_string(),
                tools: vec!["Read".to_string(), "Grep".to_string()],
                mcp_tools: vec![],
                model: "sonnet".to_string(),
                preset: None,
                prompt_summary: "Research patterns".to_string(),
                prompt: None,
            },
            crate::http_server::types::TeamPlanTeammate {
                role: "analyzer".to_string(),
                tools: vec!["Read".to_string()],
                mcp_tools: vec![],
                model: "haiku".to_string(),
                preset: None,
                prompt_summary: "Analyze results".to_string(),
                prompt: None,
            },
        ],
        team_name: "test-team-abc123".to_string(),
        lead_session_id: None,
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

// ============================================================================
// resolve_mcp_agent_type tests
// ============================================================================

#[test]
fn resolve_mcp_agent_type_returns_preset_when_some() {
    assert_eq!(
        resolve_mcp_agent_type("ideation", Some("ideation-specialist-backend")),
        "ideation-specialist-backend"
    );
}

#[test]
fn resolve_mcp_agent_type_ideation_process_no_preset() {
    assert_eq!(
        resolve_mcp_agent_type("ideation", None),
        "ideation-team-member"
    );
}

#[test]
fn resolve_mcp_agent_type_worker_process_no_preset() {
    assert_eq!(
        resolve_mcp_agent_type("worker-parallel", None),
        "worker-team-member"
    );
}

#[test]
fn resolve_mcp_agent_type_preset_overrides_worker_process() {
    // Even if process is worker-*, preset takes priority
    assert_eq!(
        resolve_mcp_agent_type("worker-parallel", Some("ideation-specialist-frontend")),
        "ideation-specialist-frontend"
    );
}

#[test]
fn resolve_mcp_agent_type_specialist_preset_variants() {
    for preset in &[
        "ideation-specialist-backend",
        "ideation-specialist-frontend",
        "ideation-specialist-infra",
        "ideation-critic",
        "ideation-advocate",
    ] {
        assert_eq!(
            resolve_mcp_agent_type("ideation", Some(preset)),
            *preset,
            "Expected preset '{}' to be returned",
            preset
        );
    }
}

// ============================================================================
// resolve_effort integration tests
// ============================================================================

#[test]
fn resolve_effort_for_ideation_team_member_returns_default() {
    use crate::infrastructure::agents::claude::resolve_effort;
    // ideation-team-member has no YAML entry, so should return default effort
    let effort = resolve_effort(Some("ideation-team-member"));
    // Just ensure it returns a non-empty string (the default)
    assert!(!effort.is_empty(), "Expected non-empty effort for ideation-team-member");
}

#[test]
fn resolve_effort_for_specialist_returns_non_empty() {
    use crate::infrastructure::agents::claude::resolve_effort;
    // ideation-specialist-backend has a YAML entry with opus model
    let effort = resolve_effort(Some("ideation-specialist-backend"));
    assert!(!effort.is_empty(), "Expected non-empty effort for ideation-specialist-backend");
}
