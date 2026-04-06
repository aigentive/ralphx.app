use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::Connection;

use super::DbConnection;
use crate::domain::entities::ProjectId;
use crate::domain::ideation::model_settings::{IdeationModelSettings, ModelLevel};
use ralphx_domain::repositories::IdeationModelSettingsRepository;
use crate::error::AppError;

pub struct SqliteIdeationModelSettingsRepository {
    db: DbConnection,
}

impl SqliteIdeationModelSettingsRepository {
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
impl IdeationModelSettingsRepository for SqliteIdeationModelSettingsRepository {
    async fn get_global(
        &self,
    ) -> Result<Option<IdeationModelSettings>, Box<dyn std::error::Error>> {
        self.db
            .run(move |conn| {
                let result = conn.query_row(
                    "SELECT id, project_id, primary_model, verifier_model, verifier_subagent_model, updated_at
                     FROM ideation_model_settings WHERE project_id IS NULL",
                    [],
                    parse_row,
                );

                match result {
                    Ok(settings) => Ok(Some(settings)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn get_for_project(
        &self,
        project_id: &str,
    ) -> Result<Option<IdeationModelSettings>, Box<dyn std::error::Error>> {
        let pid = project_id.to_string();
        self.db
            .run(move |conn| {
                let result = conn.query_row(
                    "SELECT id, project_id, primary_model, verifier_model, verifier_subagent_model, updated_at
                     FROM ideation_model_settings WHERE project_id = ?1",
                    rusqlite::params![pid.as_str()],
                    parse_row,
                );

                match result {
                    Ok(settings) => Ok(Some(settings)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(AppError::Database(e.to_string())),
                }
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn upsert_global(
        &self,
        primary_model: &str,
        verifier_model: &str,
        verifier_subagent_model: &str,
    ) -> Result<IdeationModelSettings, Box<dyn std::error::Error>> {
        let primary_model_owned = primary_model.to_string();
        let verifier_model_owned = verifier_model.to_string();
        let verifier_subagent_model_owned = verifier_subagent_model.to_string();

        self.db
            .run(move |conn| {
                let exists: bool = {
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM ideation_model_settings WHERE project_id IS NULL",
                        [],
                        |row| row.get(0),
                    )?;
                    count > 0
                };

                if exists {
                    conn.execute(
                        "UPDATE ideation_model_settings
                         SET primary_model = ?1,
                             verifier_model = ?2,
                             verifier_subagent_model = ?3,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                         WHERE project_id IS NULL",
                        rusqlite::params![
                            primary_model_owned.as_str(),
                            verifier_model_owned.as_str(),
                            verifier_subagent_model_owned.as_str(),
                        ],
                    )?;
                } else {
                    conn.execute(
                        "INSERT INTO ideation_model_settings (project_id, primary_model, verifier_model, verifier_subagent_model, updated_at)
                         VALUES (NULL, ?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
                        rusqlite::params![
                            primary_model_owned.as_str(),
                            verifier_model_owned.as_str(),
                            verifier_subagent_model_owned.as_str(),
                        ],
                    )?;
                }

                let result = conn.query_row(
                    "SELECT id, project_id, primary_model, verifier_model, verifier_subagent_model, updated_at
                     FROM ideation_model_settings WHERE project_id IS NULL",
                    [],
                    parse_row,
                );

                result.map_err(|e| AppError::Database(e.to_string()))
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn upsert_for_project(
        &self,
        project_id: &str,
        primary_model: &str,
        verifier_model: &str,
        verifier_subagent_model: &str,
    ) -> Result<IdeationModelSettings, Box<dyn std::error::Error>> {
        let pid = project_id.to_string();
        let primary_model_owned = primary_model.to_string();
        let verifier_model_owned = verifier_model.to_string();
        let verifier_subagent_model_owned = verifier_subagent_model.to_string();

        self.db
            .run(move |conn| {
                let exists: bool = {
                    let count: i64 = conn.query_row(
                        "SELECT COUNT(*) FROM ideation_model_settings WHERE project_id = ?1",
                        rusqlite::params![pid.as_str()],
                        |row| row.get(0),
                    )?;
                    count > 0
                };

                if exists {
                    conn.execute(
                        "UPDATE ideation_model_settings
                         SET primary_model = ?1,
                             verifier_model = ?2,
                             verifier_subagent_model = ?3,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
                         WHERE project_id = ?4",
                        rusqlite::params![
                            primary_model_owned.as_str(),
                            verifier_model_owned.as_str(),
                            verifier_subagent_model_owned.as_str(),
                            pid.as_str(),
                        ],
                    )?;
                } else {
                    conn.execute(
                        "INSERT INTO ideation_model_settings (project_id, primary_model, verifier_model, verifier_subagent_model, updated_at)
                         VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now'))",
                        rusqlite::params![
                            pid.as_str(),
                            primary_model_owned.as_str(),
                            verifier_model_owned.as_str(),
                            verifier_subagent_model_owned.as_str(),
                        ],
                    )?;
                }

                let result = conn.query_row(
                    "SELECT id, project_id, primary_model, verifier_model, verifier_subagent_model, updated_at
                     FROM ideation_model_settings WHERE project_id = ?1",
                    rusqlite::params![pid.as_str()],
                    parse_row,
                );

                result.map_err(|e| AppError::Database(e.to_string()))
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

fn parse_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdeationModelSettings> {
    let id: i64 = row.get(0)?;
    let project_id_str: Option<String> = row.get(1)?;
    let primary_model_str: String = row.get(2)?;
    let verifier_model_str: String = row.get(3)?;
    let verifier_subagent_model_str: String = row.get(4)?;
    let updated_at_str: String = row.get(5)?;

    let project_id = project_id_str.map(ProjectId);
    let primary_model =
        ModelLevel::from_str(&primary_model_str).unwrap_or(ModelLevel::Inherit);
    let verifier_model =
        ModelLevel::from_str(&verifier_model_str).unwrap_or(ModelLevel::Inherit);
    let verifier_subagent_model =
        ModelLevel::from_str(&verifier_subagent_model_str).unwrap_or(ModelLevel::Inherit);
    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(IdeationModelSettings {
        id,
        project_id,
        primary_model,
        verifier_model,
        verifier_subagent_model,
        updated_at,
    })
}

#[cfg(test)]
#[path = "sqlite_ideation_model_settings_repo_tests.rs"]
mod tests;
