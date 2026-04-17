use ralphx_lib::application::AppState;
use ralphx_lib::commands::agent_profile_commands::AgentProfileResponse;
use ralphx_lib::domain::agents::{AgentProfile, ProfileRole};
use ralphx_lib::domain::repositories::AgentProfileId;

fn setup_test_state() -> AppState {
    AppState::new_test()
}

#[tokio::test]
async fn test_list_agent_profiles_empty() {
    let state = setup_test_state();

    let profiles = state.agent_profile_repo.get_all().await.unwrap();
    assert!(profiles.is_empty());
}

#[tokio::test]
async fn test_seed_and_list_builtin_profiles() {
    let state = setup_test_state();

    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .unwrap();

    let profiles = state.agent_profile_repo.get_all().await.unwrap();
    assert_eq!(profiles.len(), 4);
}

#[tokio::test]
async fn test_get_agent_profile_by_id() {
    let state = setup_test_state();

    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .unwrap();

    let profile_id = AgentProfileId::from_string("worker");
    let profile = state
        .agent_profile_repo
        .get_by_id(&profile_id)
        .await
        .unwrap();
    assert!(profile.is_some());
    assert_eq!(profile.unwrap().name, "Worker");
}

#[tokio::test]
async fn test_get_agent_profiles_by_role() {
    let state = setup_test_state();

    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .unwrap();

    let workers = state
        .agent_profile_repo
        .get_by_role(ProfileRole::Worker)
        .await
        .unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].role, ProfileRole::Worker);
}

#[tokio::test]
async fn test_get_builtin_profiles() {
    let state = setup_test_state();

    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .unwrap();

    let builtin = state.agent_profile_repo.get_builtin().await.unwrap();
    assert_eq!(builtin.len(), 4);
}

#[tokio::test]
async fn test_agent_profile_response_serialization() {
    let profile = AgentProfile::worker();
    let response = AgentProfileResponse::from(profile);

    assert_eq!(response.name, "Worker");
    assert_eq!(response.role, "worker");
    assert_eq!(response.execution.model, "sonnet");

    // Verify it serializes to JSON
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"name\":\"Worker\""));
    assert!(json.contains("\"role\":\"worker\""));
}

#[tokio::test]
async fn test_all_builtin_profiles_have_unique_ids() {
    let state = setup_test_state();

    state
        .agent_profile_repo
        .seed_builtin_profiles()
        .await
        .unwrap();

    let profiles = state.agent_profile_repo.get_all().await.unwrap();
    let ids: Vec<_> = profiles.iter().map(|p| &p.id).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique_ids.len());
}
