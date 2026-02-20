use super::*;
use crate::domain::entities::{IdeationSessionId, IdeationSessionStatus, ProjectId};

fn make_session(team_mode: Option<&str>) -> IdeationSession {
    IdeationSession {
        id: IdeationSessionId::new(),
        project_id: ProjectId("proj-1".to_string()),
        title: None,
        status: IdeationSessionStatus::Active,
        plan_artifact_id: None,
        seed_task_id: None,
        parent_session_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode: team_mode.map(|s| s.to_string()),
        team_config_json: None,
        title_source: None,
    }
}

#[test]
fn test_session_is_team_mode_research_returns_true() {
    let session = make_session(Some("research"));
    assert!(session_is_team_mode(&session));
}

#[test]
fn test_session_is_team_mode_debate_returns_true() {
    let session = make_session(Some("debate"));
    assert!(session_is_team_mode(&session));
}

#[test]
fn test_session_is_team_mode_solo_returns_false() {
    let session = make_session(Some("solo"));
    assert!(!session_is_team_mode(&session));
}

#[test]
fn test_session_is_team_mode_none_returns_false() {
    let session = make_session(None);
    assert!(!session_is_team_mode(&session));
}
