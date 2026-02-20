use crate::domain::repositories::ReviewSettingsRepository;
use crate::domain::review::ReviewSettings;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SqliteReviewSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteReviewSettingsRepository {
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
impl ReviewSettingsRepository for SqliteReviewSettingsRepository {
    async fn get_settings(&self) -> Result<ReviewSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT ai_review_enabled, ai_review_auto_fix, require_fix_approval,
                    require_human_review, max_fix_attempts, max_revision_cycles
             FROM review_settings WHERE id = 1",
        )?;

        let result = stmt.query_row([], |row| {
            let ai_review_enabled: i64 = row.get(0)?;
            let ai_review_auto_fix: i64 = row.get(1)?;
            let require_fix_approval: i64 = row.get(2)?;
            let require_human_review: i64 = row.get(3)?;
            let max_fix_attempts: u32 = row.get(4)?;
            let max_revision_cycles: u32 = row.get(5)?;

            Ok(ReviewSettings {
                ai_review_enabled: ai_review_enabled != 0,
                ai_review_auto_fix: ai_review_auto_fix != 0,
                require_fix_approval: require_fix_approval != 0,
                require_human_review: require_human_review != 0,
                max_fix_attempts,
                max_revision_cycles,
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(ReviewSettings::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn update_settings(
        &self,
        settings: &ReviewSettings,
    ) -> Result<ReviewSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE review_settings
             SET ai_review_enabled = ?1,
                 ai_review_auto_fix = ?2,
                 require_fix_approval = ?3,
                 require_human_review = ?4,
                 max_fix_attempts = ?5,
                 max_revision_cycles = ?6,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = 1",
            rusqlite::params![
                settings.ai_review_enabled as i64,
                settings.ai_review_auto_fix as i64,
                settings.require_fix_approval as i64,
                settings.require_human_review as i64,
                settings.max_fix_attempts,
                settings.max_revision_cycles,
            ],
        )?;

        Ok(settings.clone())
    }
}

#[cfg(test)]
#[path = "sqlite_review_settings_repo_tests.rs"]
mod tests;
