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
}

#[async_trait]
impl AgentProfileRepository for SqliteAgentProfileRepository {
    async fn create(
        &self,
        id: &AgentProfileId,
        profile: &AgentProfile,
        is_builtin: bool,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        let profile_json = serde_json::to_string(profile)
            .map_err(|e| AppError::Database(format!("JSON serialization error: {}", e)))?;

        conn.execute(
            "INSERT INTO agent_profiles (id, name, role, profile_json, is_builtin)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                id.as_str(),
                profile.name,
                profile.role.to_string(),
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
                let profile: AgentProfile = serde_json::from_str(&json).map_err(|e| {
                    AppError::Database(format!("JSON deserialization error: {}", e))
                })?;
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
                let profile: AgentProfile = serde_json::from_str(&json).map_err(|e| {
                    AppError::Database(format!("JSON deserialization error: {}", e))
                })?;
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
            .query_map([role.to_string()], |row| {
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
            .prepare(
                "SELECT profile_json FROM agent_profiles WHERE is_builtin = 1 ORDER BY name ASC",
            )
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
            .prepare(
                "SELECT profile_json FROM agent_profiles WHERE is_builtin = 0 ORDER BY name ASC",
            )
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
            "UPDATE agent_profiles SET name = ?2, role = ?3, profile_json = ?4, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = ?1",
            rusqlite::params![
                id.as_str(),
                profile.name,
                profile.role.to_string(),
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
#[path = "sqlite_agent_profile_repo_tests.rs"]
mod tests;
