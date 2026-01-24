// Agent profile repository trait - domain layer abstraction
//
// This trait defines the contract for agent profile persistence.
// Agent profiles define how different agent types behave.

use async_trait::async_trait;

use crate::domain::agents::{AgentProfile, ProfileRole};
use crate::error::AppResult;

/// Unique identifier for agent profiles
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AgentProfileId(String);

impl AgentProfileId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AgentProfileId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AgentProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Repository trait for AgentProfile persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait AgentProfileRepository: Send + Sync {
    /// Create a new agent profile
    async fn create(&self, id: &AgentProfileId, profile: &AgentProfile, is_builtin: bool) -> AppResult<()>;

    /// Get agent profile by ID
    async fn get_by_id(&self, id: &AgentProfileId) -> AppResult<Option<AgentProfile>>;

    /// Get agent profile by name
    async fn get_by_name(&self, name: &str) -> AppResult<Option<AgentProfile>>;

    /// Get all agent profiles
    async fn get_all(&self) -> AppResult<Vec<AgentProfile>>;

    /// Get agent profiles by role
    async fn get_by_role(&self, role: ProfileRole) -> AppResult<Vec<AgentProfile>>;

    /// Get only built-in agent profiles
    async fn get_builtin(&self) -> AppResult<Vec<AgentProfile>>;

    /// Get only custom (non-builtin) agent profiles
    async fn get_custom(&self) -> AppResult<Vec<AgentProfile>>;

    /// Update an agent profile
    async fn update(&self, id: &AgentProfileId, profile: &AgentProfile) -> AppResult<()>;

    /// Delete an agent profile
    async fn delete(&self, id: &AgentProfileId) -> AppResult<()>;

    /// Check if a profile with the given name exists
    async fn exists_by_name(&self, name: &str) -> AppResult<bool>;

    /// Seed built-in profiles (idempotent)
    async fn seed_builtin_profiles(&self) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agents::AgentProfile;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    // Mock implementation for testing trait object usage
    struct MockAgentProfileRepository {
        profiles: RwLock<HashMap<String, (AgentProfile, bool)>>,
    }

    impl MockAgentProfileRepository {
        fn new() -> Self {
            Self {
                profiles: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl AgentProfileRepository for MockAgentProfileRepository {
        async fn create(&self, id: &AgentProfileId, profile: &AgentProfile, is_builtin: bool) -> AppResult<()> {
            let mut profiles = self.profiles.write().unwrap();
            profiles.insert(id.as_str().to_string(), (profile.clone(), is_builtin));
            Ok(())
        }

        async fn get_by_id(&self, id: &AgentProfileId) -> AppResult<Option<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles.get(id.as_str()).map(|(p, _)| p.clone()))
        }

        async fn get_by_name(&self, name: &str) -> AppResult<Option<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles.values().find(|(p, _)| p.name == name).map(|(p, _)| p.clone()))
        }

        async fn get_all(&self) -> AppResult<Vec<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles.values().map(|(p, _)| p.clone()).collect())
        }

        async fn get_by_role(&self, role: ProfileRole) -> AppResult<Vec<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles
                .values()
                .filter(|(p, _)| p.role == role)
                .map(|(p, _)| p.clone())
                .collect())
        }

        async fn get_builtin(&self) -> AppResult<Vec<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles
                .values()
                .filter(|(_, is_builtin)| *is_builtin)
                .map(|(p, _)| p.clone())
                .collect())
        }

        async fn get_custom(&self) -> AppResult<Vec<AgentProfile>> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles
                .values()
                .filter(|(_, is_builtin)| !*is_builtin)
                .map(|(p, _)| p.clone())
                .collect())
        }

        async fn update(&self, id: &AgentProfileId, profile: &AgentProfile) -> AppResult<()> {
            let mut profiles = self.profiles.write().unwrap();
            if let Some((_, is_builtin)) = profiles.get(id.as_str()) {
                let is_builtin = *is_builtin;
                profiles.insert(id.as_str().to_string(), (profile.clone(), is_builtin));
            }
            Ok(())
        }

        async fn delete(&self, id: &AgentProfileId) -> AppResult<()> {
            let mut profiles = self.profiles.write().unwrap();
            profiles.remove(id.as_str());
            Ok(())
        }

        async fn exists_by_name(&self, name: &str) -> AppResult<bool> {
            let profiles = self.profiles.read().unwrap();
            Ok(profiles.values().any(|(p, _)| p.name == name))
        }

        async fn seed_builtin_profiles(&self) -> AppResult<()> {
            for profile in AgentProfile::builtin_profiles() {
                if !self.exists_by_name(&profile.name).await? {
                    let id = AgentProfileId::from_string(&profile.id);
                    self.create(&id, &profile, true).await?;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn test_agent_profile_id_new() {
        let id1 = AgentProfileId::new();
        let id2 = AgentProfileId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_agent_profile_id_from_string() {
        let id = AgentProfileId::from_string("test-id");
        assert_eq!(id.as_str(), "test-id");
    }

    #[test]
    fn test_agent_profile_id_display() {
        let id = AgentProfileId::from_string("test-id");
        assert_eq!(format!("{}", id), "test-id");
    }

    #[test]
    fn test_agent_profile_repository_trait_can_be_object_safe() {
        let repo: Arc<dyn AgentProfileRepository> = Arc::new(MockAgentProfileRepository::new());
        assert!(Arc::strong_count(&repo) == 1);
    }

    #[tokio::test]
    async fn test_mock_repository_create_and_get() {
        let repo = MockAgentProfileRepository::new();
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, profile.name);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_name() {
        let repo = MockAgentProfileRepository::new();
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();

        let retrieved = repo.get_by_name(&profile.name).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().role, ProfileRole::Worker);
    }

    #[tokio::test]
    async fn test_mock_repository_get_by_role() {
        let repo = MockAgentProfileRepository::new();

        repo.create(&AgentProfileId::from_string("w1"), &AgentProfile::worker(), true)
            .await
            .unwrap();
        repo.create(&AgentProfileId::from_string("r1"), &AgentProfile::reviewer(), true)
            .await
            .unwrap();

        let workers = repo.get_by_role(ProfileRole::Worker).await.unwrap();
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].role, ProfileRole::Worker);
    }

    #[tokio::test]
    async fn test_mock_repository_get_builtin_vs_custom() {
        let repo = MockAgentProfileRepository::new();

        repo.create(&AgentProfileId::from_string("w1"), &AgentProfile::worker(), true)
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
    }

    #[tokio::test]
    async fn test_mock_repository_update() {
        let repo = MockAgentProfileRepository::new();
        let mut profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();

        profile.description = "Updated description".to_string();
        repo.update(&id, &profile).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.description, "Updated description");
    }

    #[tokio::test]
    async fn test_mock_repository_delete() {
        let repo = MockAgentProfileRepository::new();
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();
        repo.delete(&id).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_mock_repository_exists_by_name() {
        let repo = MockAgentProfileRepository::new();
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        assert!(!repo.exists_by_name(&profile.name).await.unwrap());

        repo.create(&id, &profile, true).await.unwrap();

        assert!(repo.exists_by_name(&profile.name).await.unwrap());
    }

    #[tokio::test]
    async fn test_mock_repository_seed_builtin_profiles() {
        let repo = MockAgentProfileRepository::new();

        repo.seed_builtin_profiles().await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5); // worker, reviewer, supervisor, orchestrator, deep_researcher
    }

    #[tokio::test]
    async fn test_mock_repository_seed_builtin_profiles_idempotent() {
        let repo = MockAgentProfileRepository::new();

        repo.seed_builtin_profiles().await.unwrap();
        repo.seed_builtin_profiles().await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5); // Still 5, not duplicated
    }
}
