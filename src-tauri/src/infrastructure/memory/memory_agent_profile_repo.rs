// Memory-based AgentProfileRepository implementation for testing
// Uses RwLock<HashMap> for thread-safe storage

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::agents::{AgentProfile, ProfileRole};
use crate::domain::repositories::{AgentProfileId, AgentProfileRepository};
use crate::error::AppResult;

/// In-memory implementation of AgentProfileRepository for testing
/// Uses RwLock<HashMap> for thread-safe storage
pub struct MemoryAgentProfileRepository {
    profiles: Arc<RwLock<HashMap<String, (AgentProfile, bool)>>>, // (profile, is_builtin)
}

impl Default for MemoryAgentProfileRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAgentProfileRepository {
    /// Create a new empty in-memory agent profile repository
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with pre-populated profiles (for tests)
    pub fn with_profiles(profiles: Vec<(AgentProfile, bool)>) -> Self {
        let map: HashMap<String, (AgentProfile, bool)> = profiles
            .into_iter()
            .map(|(p, is_builtin)| (p.id.clone(), (p, is_builtin)))
            .collect();
        Self {
            profiles: Arc::new(RwLock::new(map)),
        }
    }
}

#[async_trait]
impl AgentProfileRepository for MemoryAgentProfileRepository {
    async fn create(
        &self,
        id: &AgentProfileId,
        profile: &AgentProfile,
        is_builtin: bool,
    ) -> AppResult<()> {
        let mut profiles = self.profiles.write().await;
        profiles.insert(id.as_str().to_string(), (profile.clone(), is_builtin));
        Ok(())
    }

    async fn get_by_id(&self, id: &AgentProfileId) -> AppResult<Option<AgentProfile>> {
        let profiles = self.profiles.read().await;
        Ok(profiles.get(id.as_str()).map(|(p, _)| p.clone()))
    }

    async fn get_by_name(&self, name: &str) -> AppResult<Option<AgentProfile>> {
        let profiles = self.profiles.read().await;
        Ok(profiles
            .values()
            .find(|(p, _)| p.name == name)
            .map(|(p, _)| p.clone()))
    }

    async fn get_all(&self) -> AppResult<Vec<AgentProfile>> {
        let profiles = self.profiles.read().await;
        let mut result: Vec<_> = profiles.values().map(|(p, _)| p.clone()).collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_by_role(&self, role: ProfileRole) -> AppResult<Vec<AgentProfile>> {
        let profiles = self.profiles.read().await;
        let mut result: Vec<_> = profiles
            .values()
            .filter(|(p, _)| p.role == role)
            .map(|(p, _)| p.clone())
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_builtin(&self) -> AppResult<Vec<AgentProfile>> {
        let profiles = self.profiles.read().await;
        let mut result: Vec<_> = profiles
            .values()
            .filter(|(_, is_builtin)| *is_builtin)
            .map(|(p, _)| p.clone())
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn get_custom(&self) -> AppResult<Vec<AgentProfile>> {
        let profiles = self.profiles.read().await;
        let mut result: Vec<_> = profiles
            .values()
            .filter(|(_, is_builtin)| !*is_builtin)
            .map(|(p, _)| p.clone())
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }

    async fn update(&self, id: &AgentProfileId, profile: &AgentProfile) -> AppResult<()> {
        let mut profiles = self.profiles.write().await;
        if let Some((_, is_builtin)) = profiles.get(id.as_str()) {
            let is_builtin = *is_builtin;
            profiles.insert(id.as_str().to_string(), (profile.clone(), is_builtin));
        }
        Ok(())
    }

    async fn delete(&self, id: &AgentProfileId) -> AppResult<()> {
        let mut profiles = self.profiles.write().await;
        profiles.remove(id.as_str());
        Ok(())
    }

    async fn exists_by_name(&self, name: &str) -> AppResult<bool> {
        let profiles = self.profiles.read().await;
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

#[cfg(test)]
mod tests {
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
        assert_eq!(all.len(), 5);

        let builtin = repo.get_builtin().await.unwrap();
        assert_eq!(builtin.len(), 5);
    }

    #[tokio::test]
    async fn test_seed_builtin_profiles_idempotent() {
        let repo = MemoryAgentProfileRepository::new();

        repo.seed_builtin_profiles().await.unwrap();
        repo.seed_builtin_profiles().await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5);
    }
}
