use crate::domain::entities::app_state::AppSettings;
use crate::domain::entities::ProjectId;
use crate::domain::repositories::AppStateRepository;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

/// SQLite implementation of AppStateRepository
/// Manages the singleton app_state table (id=1)
pub struct SqliteAppStateRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteAppStateRepository {
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
impl AppStateRepository for SqliteAppStateRepository {
    async fn get(&self) -> Result<AppSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare("SELECT active_project_id FROM app_state WHERE id = 1")?;

        let result = stmt.query_row([], |row| {
            let active_project_id: Option<String> = row.get(0)?;
            Ok(AppSettings {
                active_project_id: active_project_id.map(ProjectId::from_string),
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AppSettings::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn set_active_project(
        &self,
        project_id: Option<&ProjectId>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE app_state SET active_project_id = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now') WHERE id = 1",
            rusqlite::params![project_id.map(|p| p.as_str())],
        )?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "sqlite_app_state_repo_tests.rs"]
mod tests;
