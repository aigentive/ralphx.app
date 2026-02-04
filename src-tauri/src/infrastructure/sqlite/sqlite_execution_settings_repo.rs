use crate::domain::entities::ProjectId;
use crate::domain::execution::{ExecutionSettings, GlobalExecutionSettings};
use crate::domain::repositories::{
    ExecutionSettingsRepository, GlobalExecutionSettingsRepository,
};
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum allowed value for global_max_concurrent
const GLOBAL_MAX_CONCURRENT_LIMIT: u32 = 50;

pub struct SqliteExecutionSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteExecutionSettingsRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl ExecutionSettingsRepository for SqliteExecutionSettingsRepository {
    /// Get execution settings for a project
    /// Phase 82: If project_id is None, returns global defaults (id=1, project_id IS NULL)
    /// If project_id is Some but no project-specific settings exist, returns global defaults
    async fn get_settings(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        if let Some(pid) = project_id {
            // Try to get project-specific settings first
            let mut stmt = conn.prepare(
                "SELECT max_concurrent_tasks, auto_commit, pause_on_failure
                 FROM execution_settings WHERE project_id = ?1",
            )?;

            let result = stmt.query_row([pid.as_str()], |row| {
                let max_concurrent_tasks: i64 = row.get(0)?;
                let auto_commit: i64 = row.get(1)?;
                let pause_on_failure: i64 = row.get(2)?;

                Ok(ExecutionSettings {
                    max_concurrent_tasks: max_concurrent_tasks as u32,
                    auto_commit: auto_commit != 0,
                    pause_on_failure: pause_on_failure != 0,
                })
            });

            match result {
                Ok(settings) => return Ok(settings),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    // Fall through to get global defaults
                }
                Err(e) => return Err(Box::new(e)),
            }
        }

        // Get global defaults (id=1, project_id IS NULL)
        let mut stmt = conn.prepare(
            "SELECT max_concurrent_tasks, auto_commit, pause_on_failure
             FROM execution_settings WHERE id = 1 AND project_id IS NULL",
        )?;

        let result = stmt.query_row([], |row| {
            let max_concurrent_tasks: i64 = row.get(0)?;
            let auto_commit: i64 = row.get(1)?;
            let pause_on_failure: i64 = row.get(2)?;

            Ok(ExecutionSettings {
                max_concurrent_tasks: max_concurrent_tasks as u32,
                auto_commit: auto_commit != 0,
                pause_on_failure: pause_on_failure != 0,
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ExecutionSettings::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Update execution settings for a project
    /// Phase 82: If project_id is None, updates global defaults
    /// If project_id is Some and no project-specific settings exist, creates them
    async fn update_settings(
        &self,
        project_id: Option<&ProjectId>,
        settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        if let Some(pid) = project_id {
            // Try to update existing project-specific settings
            let rows_updated = conn.execute(
                "UPDATE execution_settings
                 SET max_concurrent_tasks = ?1,
                     auto_commit = ?2,
                     pause_on_failure = ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                 WHERE project_id = ?4",
                rusqlite::params![
                    settings.max_concurrent_tasks as i64,
                    settings.auto_commit as i64,
                    settings.pause_on_failure as i64,
                    pid.as_str(),
                ],
            )?;

            if rows_updated == 0 {
                // No existing settings for this project, insert new row
                conn.execute(
                    "INSERT INTO execution_settings (max_concurrent_tasks, auto_commit, pause_on_failure, updated_at, project_id)
                     VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), ?4)",
                    rusqlite::params![
                        settings.max_concurrent_tasks as i64,
                        settings.auto_commit as i64,
                        settings.pause_on_failure as i64,
                        pid.as_str(),
                    ],
                )?;
            }
        } else {
            // Update global defaults (id=1, project_id IS NULL)
            conn.execute(
                "UPDATE execution_settings
                 SET max_concurrent_tasks = ?1,
                     auto_commit = ?2,
                     pause_on_failure = ?3,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                 WHERE id = 1 AND project_id IS NULL",
                rusqlite::params![
                    settings.max_concurrent_tasks as i64,
                    settings.auto_commit as i64,
                    settings.pause_on_failure as i64,
                ],
            )?;
        }

        Ok(settings.clone())
    }
}

/// SQLite implementation of GlobalExecutionSettingsRepository
/// Phase 82: Manages the global_execution_settings singleton table
pub struct SqliteGlobalExecutionSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteGlobalExecutionSettingsRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl GlobalExecutionSettingsRepository for SqliteGlobalExecutionSettingsRepository {
    async fn get_settings(&self) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT global_max_concurrent FROM global_execution_settings WHERE id = 1",
        )?;

        let result = stmt.query_row([], |row| {
            let global_max_concurrent: i64 = row.get(0)?;
            Ok(GlobalExecutionSettings {
                global_max_concurrent: global_max_concurrent as u32,
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(GlobalExecutionSettings::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn update_settings(
        &self,
        settings: &GlobalExecutionSettings,
    ) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        // Enforce max limit of 50
        let clamped_max = settings.global_max_concurrent.min(GLOBAL_MAX_CONCURRENT_LIMIT);

        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE global_execution_settings
             SET global_max_concurrent = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = 1",
            rusqlite::params![clamped_max as i64],
        )?;

        Ok(GlobalExecutionSettings {
            global_max_concurrent: clamped_max,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    #[tokio::test]
    async fn test_get_default_global_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteExecutionSettingsRepository::new(conn);

        // Get global defaults (project_id = None)
        let settings = repo.get_settings(None).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_update_global_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteExecutionSettingsRepository::new(conn);

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 4,
            auto_commit: false,
            pause_on_failure: false,
        };

        // Update global defaults
        let updated = repo.update_settings(None, &new_settings).await.unwrap();
        assert_eq!(updated.max_concurrent_tasks, 4);
        assert!(!updated.auto_commit);
        assert!(!updated.pause_on_failure);

        // Verify persistence
        let retrieved = repo.get_settings(None).await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 4);
        assert!(!retrieved.auto_commit);
        assert!(!retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_per_project_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteExecutionSettingsRepository::new(conn);

        let project_id = ProjectId::from_string("test-project-123".to_string());

        // Initially, get_settings for a project should return global defaults
        let settings = repo.get_settings(Some(&project_id)).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2); // global default

        // Create project-specific settings
        let project_settings = ExecutionSettings {
            max_concurrent_tasks: 5,
            auto_commit: false,
            pause_on_failure: true,
        };

        repo.update_settings(Some(&project_id), &project_settings)
            .await
            .unwrap();

        // Now get_settings should return project-specific values
        let retrieved = repo.get_settings(Some(&project_id)).await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 5);
        assert!(!retrieved.auto_commit);
        assert!(retrieved.pause_on_failure);

        // Global settings should remain unchanged
        let global = repo.get_settings(None).await.unwrap();
        assert_eq!(global.max_concurrent_tasks, 2);
    }

    #[tokio::test]
    async fn test_global_execution_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteGlobalExecutionSettingsRepository::new(conn);

        // Get default global settings
        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.global_max_concurrent, 20);

        // Update global settings
        let new_settings = GlobalExecutionSettings {
            global_max_concurrent: 30,
        };
        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.global_max_concurrent, 30);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.global_max_concurrent, 30);
    }

    #[tokio::test]
    async fn test_global_max_concurrent_capped_at_50() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteGlobalExecutionSettingsRepository::new(conn);

        // Try to set above max
        let new_settings = GlobalExecutionSettings {
            global_max_concurrent: 100,
        };
        let updated = repo.update_settings(&new_settings).await.unwrap();

        // Should be clamped to 50
        assert_eq!(updated.global_max_concurrent, 50);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.global_max_concurrent, 50);
    }

    #[tokio::test]
    async fn test_shared_connection() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let repo = SqliteExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn));

        let settings = repo.get_settings(None).await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
    }
}
