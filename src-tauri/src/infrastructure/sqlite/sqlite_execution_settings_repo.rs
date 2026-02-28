use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::ProjectId;
use crate::domain::execution::{ExecutionSettings, GlobalExecutionSettings};
use crate::domain::repositories::{ExecutionSettingsRepository, GlobalExecutionSettingsRepository};
use crate::error::AppError;

/// Maximum allowed value for global_max_concurrent
const GLOBAL_MAX_CONCURRENT_LIMIT: u32 = 50;

pub struct SqliteExecutionSettingsRepository {
    db: DbConnection,
}

impl SqliteExecutionSettingsRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl ExecutionSettingsRepository for SqliteExecutionSettingsRepository {
    /// Get execution settings for a project.
    /// If project_id is None, returns global defaults (id=1, project_id IS NULL).
    /// If project_id is Some but no project-specific settings exist, returns global defaults.
    async fn get_settings(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let project_id_str = project_id.map(|p| p.as_str().to_string());
        self.db
            .run(move |conn| {
                if let Some(ref pid) = project_id_str {
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
                        Err(rusqlite::Error::QueryReturnedNoRows) => {}
                        Err(e) => return Err(AppError::Database(e.to_string())),
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
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Update execution settings for a project.
    /// If project_id is None, updates global defaults.
    /// If project_id is Some and no project-specific settings exist, creates them.
    async fn update_settings(
        &self,
        project_id: Option<&ProjectId>,
        settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let project_id_str = project_id.map(|p| p.as_str().to_string());
        let settings = settings.clone();
        self.db
            .run(move |conn| {
                if let Some(ref pid) = project_id_str {
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

                Ok(settings)
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

/// SQLite implementation of GlobalExecutionSettingsRepository
/// Manages the global_execution_settings singleton table
pub struct SqliteGlobalExecutionSettingsRepository {
    db: DbConnection,
}

impl SqliteGlobalExecutionSettingsRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            db: DbConnection::new(conn),
        }
    }

    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            db: DbConnection::from_shared(conn),
        }
    }
}

#[async_trait]
impl GlobalExecutionSettingsRepository for SqliteGlobalExecutionSettingsRepository {
    async fn get_settings(&self) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        self.db
            .run(move |conn| {
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
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        Ok(GlobalExecutionSettings::default())
                    }
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn update_settings(
        &self,
        settings: &GlobalExecutionSettings,
    ) -> Result<GlobalExecutionSettings, Box<dyn std::error::Error>> {
        let clamped_max = settings
            .global_max_concurrent
            .min(GLOBAL_MAX_CONCURRENT_LIMIT);
        self.db
            .run(move |conn| {
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
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

#[cfg(test)]
#[path = "sqlite_execution_settings_repo_tests.rs"]
mod tests;
