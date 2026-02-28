use super::DbConnection;
use crate::domain::entities::app_state::AppSettings;
use crate::domain::entities::ProjectId;
use crate::domain::repositories::AppStateRepository;
use crate::error::AppError;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SQLite implementation of AppStateRepository
/// Manages the singleton app_state table (id=1)
pub struct SqliteAppStateRepository {
    db: DbConnection,
}

impl SqliteAppStateRepository {
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
impl AppStateRepository for SqliteAppStateRepository {
    async fn get(&self) -> Result<AppSettings, Box<dyn std::error::Error>> {
        self.db
            .run(move |conn| {
                let result = conn.query_row(
                    "SELECT active_project_id FROM app_state WHERE id = 1",
                    [],
                    |row| {
                        let active_project_id: Option<String> = row.get(0)?;
                        Ok(AppSettings {
                            active_project_id: active_project_id.map(ProjectId::from_string),
                        })
                    },
                );

                match result {
                    Ok(settings) => Ok(settings),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AppSettings::default()),
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn set_active_project(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_id_str = project_id.map(|p| p.as_str().to_string());

        self.db
            .run(move |conn| {
                conn.execute(
                    "UPDATE app_state SET active_project_id = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = 1",
                    rusqlite::params![project_id_str],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

#[cfg(test)]
#[path = "sqlite_app_state_repo_tests.rs"]
mod tests;
