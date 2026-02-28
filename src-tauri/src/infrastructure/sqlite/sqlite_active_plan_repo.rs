use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::{IdeationSessionId, ProjectId};
use crate::domain::repositories::ActivePlanRepository;
use crate::error::AppError;

/// SQLite implementation of ActivePlanRepository
/// Manages the project_active_plan table with validation
pub struct SqliteActivePlanRepository {
    db: DbConnection,
}

impl SqliteActivePlanRepository {
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
impl ActivePlanRepository for SqliteActivePlanRepository {
    async fn get(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<IdeationSessionId>, Box<dyn std::error::Error>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT ideation_session_id FROM project_active_plan WHERE project_id = ?1",
                )?;
                let result = stmt.query_row([project_id.as_str()], |row| {
                    let session_id: String = row.get(0)?;
                    Ok(IdeationSessionId::from_string(session_id))
                });
                match result {
                    Ok(session_id) => Ok(Some(session_id)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn set(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_id = project_id.as_str().to_string();
        let session_id = ideation_session_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let is_valid: bool = conn.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM ideation_sessions
                        WHERE id = ?1
                          AND project_id = ?2
                          AND status = 'accepted'
                          AND converted_at IS NOT NULL
                    )",
                    [session_id.as_str(), project_id.as_str()],
                    |row| row.get(0),
                )?;

                if !is_valid {
                    return Err(AppError::Validation(
                        "Session must be accepted and belong to the project".to_string(),
                    ));
                }

                conn.execute(
                    "INSERT INTO project_active_plan (project_id, ideation_session_id, updated_at)
                     VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))
                     ON CONFLICT(project_id) DO UPDATE SET
                         ideation_session_id = excluded.ideation_session_id,
                         updated_at = excluded.updated_at",
                    [project_id.as_str(), session_id.as_str()],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn clear(&self, project_id: &ProjectId) -> Result<(), Box<dyn std::error::Error>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "DELETE FROM project_active_plan WHERE project_id = ?1",
                    [project_id.as_str()],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn exists(&self, project_id: &ProjectId) -> Result<bool, Box<dyn std::error::Error>> {
        let project_id = project_id.as_str().to_string();
        self.db
            .run(move |conn| {
                let exists: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM project_active_plan WHERE project_id = ?1)",
                    [project_id.as_str()],
                    |row| row.get(0),
                )?;
                Ok(exists)
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn record_selection(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
        source: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let project_id = project_id.as_str().to_string();
        let session_id = ideation_session_id.as_str().to_string();
        let source = source.to_string();
        self.db
            .run(move |conn| {
                conn.execute(
                    "INSERT INTO plan_selection_stats (project_id, ideation_session_id, selected_count, last_selected_at, last_selected_source)
                     VALUES (?1, ?2, 1, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'), ?3)
                     ON CONFLICT(project_id, ideation_session_id) DO UPDATE SET
                         selected_count = selected_count + 1,
                         last_selected_at = excluded.last_selected_at,
                         last_selected_source = excluded.last_selected_source",
                    [project_id.as_str(), session_id.as_str(), source.as_str()],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

#[cfg(test)]
#[path = "sqlite_active_plan_repo_tests.rs"]
mod tests;
