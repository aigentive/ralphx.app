use super::*;

#[tokio::test]
async fn test_create_and_get_by_id() {
    let repo = MemoryAgentProfileRepository::new();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, profile.name);
}

#[tokio::test]
async fn test_get_by_id_not_found() {
    let repo = MemoryAgentProfileRepository::new();
    let id = AgentProfileId::from_string("nonexistent");

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_get_by_name() {
    let repo = MemoryAgentProfileRepository::new();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();

    let retrieved = repo.get_by_name(&profile.name).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().role, ProfileRole::Worker);
}

#[tokio::test]
async fn test_get_all() {
    let repo = MemoryAgentProfileRepository::new();

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
    let repo = MemoryAgentProfileRepository::new();

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
    let repo = MemoryAgentProfileRepository::new();

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
    let repo = MemoryAgentProfileRepository::new();
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
    let repo = MemoryAgentProfileRepository::new();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    repo.create(&id, &profile, true).await.unwrap();
    repo.delete(&id).await.unwrap();

    let retrieved = repo.get_by_id(&id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_exists_by_name() {
    let repo = MemoryAgentProfileRepository::new();
    let profile = AgentProfile::worker();
    let id = AgentProfileId::from_string("worker-1");

    assert!(!repo.exists_by_name(&profile.name).await.unwrap());

    repo.create(&id, &profile, true).await.unwrap();

    assert!(repo.exists_by_name(&profile.name).await.unwrap());
}

#[tokio::test]
async fn test_seed_builtin_profiles() {
    let repo = MemoryAgentProfileRepository::new();

    repo.seed_builtin_profiles().await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 4);

    let builtin = repo.get_builtin().await.unwrap();
    assert_eq!(builtin.len(), 4);
}

#[tokio::test]
async fn test_seed_builtin_profiles_idempotent() {
    let repo = MemoryAgentProfileRepository::new();

    repo.seed_builtin_profiles().await.unwrap();
    repo.seed_builtin_profiles().await.unwrap();

    let all = repo.get_all().await.unwrap();
    assert_eq!(all.len(), 4);
}
