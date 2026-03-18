use super::*;
use crate::testing::SqliteTestDb;

fn create_test_repo() -> (SqliteTestDb, SqliteAgentProfileRepository) {
    let db = SqliteTestDb::new("sqlite_agent_profile_repo_tests");
    let repo = SqliteAgentProfileRepository::from_shared(db.shared_conn());
    (db, repo)
}

#[tokio::test]
async fn test_create_and_get_by_id() {
    let (_db, repo) = create_test_repo();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, profile.name);
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let (_db, repo) = create_test_repo();
    let id = AgentProfileId::from_string("nonexistent");

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_get_by_name() {
    let (_db, repo) = create_test_repo();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();

    let retrieved = repo.get_by_name(&profile.name).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().role, ProfileRole::Worker);
}

#[tokio::test]
async fn test_get_by_name_not_found() {
    let (_db, repo) = create_test_repo();

    let retrieved = repo.get_by_name("Nonexistent Profile").await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_get_all() {
    let (_db, repo) = create_test_repo();

    repo.create(
        &AgentProfileId::from_string("w1"),
        &AgentProfile::worker(),
        true,
    )
    .await
    .unwrap();
    repo.create(
        &AgentProfileId::from_string("r1"),
        &AgentProfile::reviewer(),
        true,
    )
    .await
    .unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 2);
}

#[tokio::test]
async fn test_get_by_role() {
    let (_db, repo) = create_test_repo();

    repo.create(
        &AgentProfileId::from_string("w1"),
        &AgentProfile::worker(),
        true,
    )
    .await
    .unwrap();
    repo.create(
        &AgentProfileId::from_string("r1"),
        &AgentProfile::reviewer(),
        true,
    )
    .await
    .unwrap();

    let workers = repo.get_by_role(ProfileRole::Worker).await.unwrap();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].role, ProfileRole::Worker);
}

#[tokio::test]
async fn test_get_builtin_vs_custom() {
    let (_db, repo) = create_test_repo();

    repo.create(
        &AgentProfileId::from_string("w1"),
        &AgentProfile::worker(),
        true,
    )
    .await
    .unwrap();

    let mut custom_profile = AgentProfile::worker();
    custom_profile.name = "Custom Worker".to_string();
    custom_profile.id = "custom-worker".to_string();
    repo.create(&AgentProfileId::from_string("c1"), &custom_profile, false)
        .await
        .unwrap();

    let builtin = repo.get_builtin().await.unwrap();
    let custom = repo.get_custom().await.unwrap();

    assert_eq!(builtin.len(), 1);
    assert_eq!(custom.len(), 1);
    assert_eq!(builtin[0].name, "Worker");
    assert_eq!(custom[0].name, "Custom Worker");
}

#[tokio::test]
async fn test_update() {
    let (_db, repo) = create_test_repo();
    let mut profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();

    profile.description = "Updated description".to_string();
    repo.update(&id, &profile).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
    assert_eq!(retrieved.description, "Updated description");
}

#[tokio::test]
async fn test_delete() {
    let (_db, repo) = create_test_repo();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();
    repo.delete(&id).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_exists_by_name() {
    let (_db, repo) = create_test_repo();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    assert!(!repo.exists_by_name(&profile.name).await.unwrap());

    repo.create(&id, &profile, true).await.unwrap();

    assert!(repo.exists_by_name(&profile.name).await.unwrap());
}

#[tokio::test]
async fn test_seed_builtin_profiles() {
    let (_db, repo) = create_test_repo();

    repo.seed_builtin_profiles().await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 5); // worker, reviewer, supervisor, orchestrator, deep_researcher

    let builtin = repo.get_builtin().await.unwrap();
    assert_eq!(builtin.len(), 5);
}

#[tokio::test]
async fn test_seed_builtin_profiles_idempotent() {
    let (_db, repo) = create_test_repo();

    repo.seed_builtin_profiles().await.unwrap();
    repo.seed_builtin_profiles().await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 5); // Still 5, not duplicated
}

// Note: ProfileRole Display/FromStr trait implementations are tested in
// src/domain/agents/agent_profile.rs (test_profile_role_display, test_profile_role_from_str)

#[tokio::test]
async fn test_profile_json_serialization() {
    let (_db, repo) = create_test_repo();
    let profile = AgentProfile::supervisor();
    let id = AgentProfileId::from_string("supervisor-1");

    repo.create(&id, &profile, true).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();

    // Verify complex nested structures are preserved
    assert_eq!(retrieved.execution.model, profile.execution.model);
    assert_eq!(
        retrieved.execution.max_iterations,
        profile.execution.max_iterations
    );
    assert_eq!(
        retrieved.behavior.autonomy_level,
        profile.behavior.autonomy_level
    );
}
