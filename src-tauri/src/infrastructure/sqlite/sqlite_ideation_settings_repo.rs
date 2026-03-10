use super::DbConnection;
use crate::domain::ideation::{IdeationPlanMode, IdeationSettings};
use crate::domain::repositories::IdeationSettingsRepository;
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SqliteIdeationSettingsRepository {
    db: DbConnection,
}

impl SqliteIdeationSettingsRepository {
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

/// Synchronous helper for reading settings inside a transaction closure.
pub(crate) fn get_settings_sync(conn: &Connection) -> AppResult<IdeationSettings> {
    let result = conn.query_row(
        "SELECT plan_mode, require_plan_approval, suggest_plans_for_complex, auto_link_proposals,
                require_verification_for_accept, require_verification_for_proposals
         FROM ideation_settings WHERE id = 1
         LIMIT 1",
        [],
        |row| {
            let plan_mode_str: String = row.get(0)?;
            let require_plan_approval: i64 = row.get(1)?;
            let suggest_plans_for_complex: i64 = row.get(2)?;
            let auto_link_proposals: i64 = row.get(3)?;
            let require_verification_for_accept: i64 = row.get(4)?;
            let require_verification_for_proposals: i64 = row.get(5)?;

            let plan_mode = match plan_mode_str.as_str() {
                "required" => IdeationPlanMode::Required,
                "optional" => IdeationPlanMode::Optional,
                "parallel" => IdeationPlanMode::Parallel,
                _ => IdeationPlanMode::default(),
            };

            Ok(IdeationSettings {
                plan_mode,
                require_plan_approval: require_plan_approval != 0,
                suggest_plans_for_complex: suggest_plans_for_complex != 0,
                auto_link_proposals: auto_link_proposals != 0,
                require_verification_for_accept: require_verification_for_accept != 0,
                require_verification_for_proposals: require_verification_for_proposals != 0,
            })
        },
    );

    match result {
        Ok(settings) => Ok(settings),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(IdeationSettings::default()),
        Err(e) => Err(AppError::Database(e.to_string())),
    }
}

#[async_trait]
impl IdeationSettingsRepository for SqliteIdeationSettingsRepository {
    async fn get_settings(&self) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        self.db
            .run(move |conn| get_settings_sync(conn))
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn update_settings(
        &self,
        settings: &IdeationSettings,
    ) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        let settings = settings.clone();

        self.db
            .run(move |conn| {
                let plan_mode_str = match settings.plan_mode {
                    IdeationPlanMode::Required => "required",
                    IdeationPlanMode::Optional => "optional",
                    IdeationPlanMode::Parallel => "parallel",
                };

                conn.execute(
                    "UPDATE ideation_settings
             SET plan_mode = ?1,
                 require_plan_approval = ?2,
                 suggest_plans_for_complex = ?3,
                 auto_link_proposals = ?4,
                 require_verification_for_accept = ?5,
                 require_verification_for_proposals = ?6,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = 1",
                    rusqlite::params![
                        plan_mode_str,
                        settings.require_plan_approval as i64,
                        settings.suggest_plans_for_complex as i64,
                        settings.auto_link_proposals as i64,
                        settings.require_verification_for_accept as i64,
                        settings.require_verification_for_proposals as i64,
                    ],
                )?;

                Ok(settings)
            })
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

#[cfg(test)]
#[path = "sqlite_ideation_settings_repo_tests.rs"]
mod tests;
