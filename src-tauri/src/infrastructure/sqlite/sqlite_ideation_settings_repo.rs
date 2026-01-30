use crate::domain::ideation::{IdeationPlanMode, IdeationSettings};
use crate::domain::repositories::IdeationSettingsRepository;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SqliteIdeationSettingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteIdeationSettingsRepository {
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
impl IdeationSettingsRepository for SqliteIdeationSettingsRepository {
    async fn get_settings(&self) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT plan_mode, require_plan_approval, suggest_plans_for_complex, auto_link_proposals
             FROM ideation_settings WHERE id = 1",
        )?;

        let result = stmt.query_row([], |row| {
            let plan_mode_str: String = row.get(0)?;
            let require_plan_approval: i64 = row.get(1)?;
            let suggest_plans_for_complex: i64 = row.get(2)?;
            let auto_link_proposals: i64 = row.get(3)?;

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
            })
        });

        match result {
            Ok(settings) => Ok(settings),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(IdeationSettings::default()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn update_settings(
        &self,
        settings: &IdeationSettings,
    ) -> Result<IdeationSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

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
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = 1",
            rusqlite::params![
                plan_mode_str,
                settings.require_plan_approval as i64,
                settings.suggest_plans_for_complex as i64,
                settings.auto_link_proposals as i64,
            ],
        )?;

        Ok(settings.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::{open_memory_connection, run_migrations};

    #[tokio::test]
    async fn test_get_default_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteIdeationSettingsRepository::new(conn);

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.plan_mode, IdeationPlanMode::Optional);
        assert!(!settings.require_plan_approval);
        assert!(settings.suggest_plans_for_complex);
        assert!(settings.auto_link_proposals);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteIdeationSettingsRepository::new(conn);

        let new_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Required,
            require_plan_approval: true,
            suggest_plans_for_complex: false,
            auto_link_proposals: false,
        };

        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.plan_mode, IdeationPlanMode::Required);
        assert!(updated.require_plan_approval);
        assert!(!updated.suggest_plans_for_complex);
        assert!(!updated.auto_link_proposals);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.plan_mode, IdeationPlanMode::Required);
        assert!(retrieved.require_plan_approval);
        assert!(!retrieved.suggest_plans_for_complex);
        assert!(!retrieved.auto_link_proposals);
    }

    #[tokio::test]
    async fn test_update_settings_all_modes() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteIdeationSettingsRepository::new(conn);

        // Test Required mode
        let required_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Required,
            ..Default::default()
        };
        repo.update_settings(&required_settings).await.unwrap();
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.plan_mode, IdeationPlanMode::Required);

        // Test Optional mode
        let optional_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Optional,
            ..Default::default()
        };
        repo.update_settings(&optional_settings).await.unwrap();
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.plan_mode, IdeationPlanMode::Optional);

        // Test Parallel mode
        let parallel_settings = IdeationSettings {
            plan_mode: IdeationPlanMode::Parallel,
            ..Default::default()
        };
        repo.update_settings(&parallel_settings).await.unwrap();
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.plan_mode, IdeationPlanMode::Parallel);
    }
}
