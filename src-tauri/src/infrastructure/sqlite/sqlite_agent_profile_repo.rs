// SQLite-based AgentProfileRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use crate::domain::agents::{AgentProfile, ProfileRole};
use crate::domain::repositories::{AgentProfileId, AgentProfileRepository};
use crate::error::{AppError, AppResult};

/// SQLite implementation of AgentProfileRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqliteAgentProfileRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAgentProfileRepository {
    /// Create a new SQLite agent profile repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Helper to convert ProfileRole to string for database storage
    fn role_to_string(role: &ProfileRole) -> &'static str {
        match role {
            ProfileRole::Worker => "worker",
            ProfileRole::Reviewer => "reviewer",
            ProfileRole::Supervisor => "supervisor",
            ProfileRole::Orchestrator => "orchestrator",
            ProfileRole::Researcher => "researcher",
        }
    }

    /// Helper to convert string to ProfileRole
    fn string_to_role(s: &str) -> ProfileRole {
        match s {
            "worker" => ProfileRole::Worker,
            "reviewer" => ProfileRole::Reviewer,
            "supervisor" => ProfileRole::Supervisor,
            "orchestrator" => ProfileRole::Orchestrator,
            "researcher" => ProfileRole::Researcher,
            _ => ProfileRole::Worker, // Default fallback
        }
    }
}

#[async_trait]
impl AgentProfileRepository for SqliteAgentProfileRepository {
    async fn create(&self, id: &AgentProfileId, profile: &AgentProfile, is_builtin: bool) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let profile_json = serde_json::to_string(profile)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                id.as_str(),
                profile.name,
                Self::role_to_string(&profile.role),
                profile_json,
                if is_builtin { 1 } else { 0 },
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_by_id(&self, id: &AgentProfileId) -> AppResult<Option<AgentProfile>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT profile_json FROM agent_profiles WHERE id = ?1",
            [id.as_str()],
            |row| {
                let json: String = row.get(0)?;
                Ok(json)
            },
        );

        match result {
            Ok(json) => {
                let profile: AgentProfile = serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))?;
                Ok(Some(profile))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_by_name(&self, name: &str) -> AppResult<Option<AgentProfile>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT profile_json FROM agent_profiles WHERE name = ?1",
            [name],
            |row| {
                let json: String = row.get(0)?;
                Ok(json)
            },
        );

        match result {
            Ok(json) => {
                let profile: AgentProfile = serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))?;
                Ok(Some(profile))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_all(&self) -> AppResult<Vec<AgentProfile>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT profile_json FROM agent_profiles ORDER BY name ASC")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let profiles = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        profiles
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))
            })
            .collect()
    }

    async fn get_by_role(&self, role: ProfileRole) -> AppResult<Vec<AgentProfile>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT profile_json FROM agent_profiles WHERE role = ?1 ORDER BY name ASC")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let profiles = stmt
            .query_map([Self::role_to_string(&role)], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        profiles
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))
            })
            .collect()
    }

    async fn get_builtin(&self) -> AppResult<Vec<AgentProfile>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT profile_json FROM agent_profiles WHERE is_builtin = 1 ORDER BY name ASC")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let profiles = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        profiles
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))
            })
            .collect()
    }

    async fn get_custom(&self) -> AppResult<Vec<AgentProfile>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare("SELECT profile_json FROM agent_profiles WHERE is_builtin = 0 ORDER BY name ASC")
            .map_err(|e| AppError::Database(e.to_string()))?;

        let profiles = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        profiles
            .into_iter()
            .map(|json| {
                serde_json::from_str(&json)
                    .map_err(|e| AppError::Database(format!("JSON deserialization error: {}", e)))
            })
            .collect()
    }

    async fn update(&self, id: &AgentProfileId, profile: &AgentProfile) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let profile_json = serde_json::to_string(profile)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "UPDATE agent_profiles SET name = ?2, role = ?3, profile_json = ?4, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            rusqlite::params![
                id.as_str(),
                profile.name,
                Self::role_to_string(&profile.role),
                profile_json,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, id: &AgentProfileId) -> AppResult<()> {
        let conn = self.conn.lock().await;

        conn.execute("DELETE FROM agent_profiles WHERE id = ?1", [id.as_str()])
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn exists_by_name(&self, name: &str) -> AppResult<bool> {
        let conn = self.conn.lock().await;

        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_profiles WHERE name = ?1",
                [name],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(count > 0)
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
    use crate::infrastructure::sqlite::connection::open_memory_connection;
    use crate::infrastructure::sqlite::migrations::run_migrations;

    async fn create_test_repo() -> SqliteAgentProfileRepository {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        SqliteAgentProfileRepository::new(conn)
    }

    #[tokio::test]
    async fn test_create_and_get_by_id() {
        let repo = create_test_repo().await;
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, profile.name);
    }

    #[tokio::test]
    async fn test_get_by_id_not_found() {
        let repo = create_test_repo().await;
        let id = AgentProfileId::from_string("nonexistent");

        let retrieved = repo.get_by_id(&id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let repo = create_test_repo().await;
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();

        let retrieved = repo.get_by_name(&profile.name).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().role, ProfileRole::Worker);
    }

    #[tokio::test]
    async fn test_get_by_name_not_found() {
        let repo = create_test_repo().await;

        let retrieved = repo.get_by_name("Nonexistent Profile").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_all() {
        let repo = create_test_repo().await;

        repo.create(&AgentProfileId::from_string("w1"), &AgentProfile::worker(), true)
            .await
            .unwrap();
        repo.create(&AgentProfileId::from_string("r1"), &AgentProfile::reviewer(), true)
            .await
            .unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_role() {
        let repo = create_test_repo().await;

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
    async fn test_get_builtin_vs_custom() {
        let repo = create_test_repo().await;

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
        assert_eq!(builtin[0].name, "Worker");
        assert_eq!(custom[0].name, "Custom Worker");
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_test_repo().await;
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
        let repo = create_test_repo().await;
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        repo.create(&id, &profile, true).await.unwrap();
        repo.delete(&id).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_exists_by_name() {
        let repo = create_test_repo().await;
        let profile = AgentProfile::worker();
        let id = AgentProfileId::from_string("worker-1");

        assert!(!repo.exists_by_name(&profile.name).await.unwrap());

        repo.create(&id, &profile, true).await.unwrap();

        assert!(repo.exists_by_name(&profile.name).await.unwrap());
    }

    #[tokio::test]
    async fn test_seed_builtin_profiles() {
        let repo = create_test_repo().await;

        repo.seed_builtin_profiles().await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5); // worker, reviewer, supervisor, orchestrator, deep_researcher

        let builtin = repo.get_builtin().await.unwrap();
        assert_eq!(builtin.len(), 5);
    }

    #[tokio::test]
    async fn test_seed_builtin_profiles_idempotent() {
        let repo = create_test_repo().await;

        repo.seed_builtin_profiles().await.unwrap();
        repo.seed_builtin_profiles().await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 5); // Still 5, not duplicated
    }

    #[tokio::test]
    async fn test_role_to_string_conversions() {
        assert_eq!(SqliteAgentProfileRepository::role_to_string(&ProfileRole::Worker), "worker");
        assert_eq!(SqliteAgentProfileRepository::role_to_string(&ProfileRole::Reviewer), "reviewer");
        assert_eq!(SqliteAgentProfileRepository::role_to_string(&ProfileRole::Supervisor), "supervisor");
        assert_eq!(SqliteAgentProfileRepository::role_to_string(&ProfileRole::Orchestrator), "orchestrator");
        assert_eq!(SqliteAgentProfileRepository::role_to_string(&ProfileRole::Researcher), "researcher");
    }

    #[tokio::test]
    async fn test_string_to_role_conversions() {
        assert_eq!(SqliteAgentProfileRepository::string_to_role("worker"), ProfileRole::Worker);
        assert_eq!(SqliteAgentProfileRepository::string_to_role("reviewer"), ProfileRole::Reviewer);
        assert_eq!(SqliteAgentProfileRepository::string_to_role("supervisor"), ProfileRole::Supervisor);
        assert_eq!(SqliteAgentProfileRepository::string_to_role("orchestrator"), ProfileRole::Orchestrator);
        assert_eq!(SqliteAgentProfileRepository::string_to_role("researcher"), ProfileRole::Researcher);
        assert_eq!(SqliteAgentProfileRepository::string_to_role("unknown"), ProfileRole::Worker); // Default
    }

    #[tokio::test]
    async fn test_profile_json_serialization() {
        let repo = create_test_repo().await;
        let profile = AgentProfile::supervisor();
        let id = AgentProfileId::from_string("supervisor-1");

        repo.create(&id, &profile, true).await.unwrap();

        let retrieved = repo.get_by_id(&id).await.unwrap().unwrap();

        // Verify complex nested structures are preserved
        assert_eq!(retrieved.execution.model, profile.execution.model);
        assert_eq!(retrieved.execution.max_iterations, profile.execution.max_iterations);
        assert_eq!(retrieved.behavior.autonomy_level, profile.behavior.autonomy_level);
    }
}
