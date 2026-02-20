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
    async fn create(
        &self,
        id: &AgentProfileId,
        profile: &AgentProfile,
        is_builtin: bool,
    ) -> AppResult<()>;

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
#[path = "agent_profile_repository_tests.rs"]
mod tests;
