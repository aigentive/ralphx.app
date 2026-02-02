use crate::domain::execution::ExecutionSettings;
use crate::domain::repositories::ExecutionSettingsRepository;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    async fn get_settings(&self) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        let mut stmt = conn.prepare(
            "SELECT max_concurrent_tasks, auto_commit, pause_on_failure
             FROM execution_settings WHERE id = 1",
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

    async fn update_settings(
        &self,
        settings: &ExecutionSettings,
    ) -> Result<ExecutionSettings, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().await;

        conn.execute(
            "UPDATE execution_settings
             SET max_concurrent_tasks = ?1,
                 auto_commit = ?2,
                 pause_on_failure = ?3,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
             WHERE id = 1",
            rusqlite::params![
                settings.max_concurrent_tasks as i64,
                settings.auto_commit as i64,
                settings.pause_on_failure as i64,
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
        let repo = SqliteExecutionSettingsRepository::new(conn);

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
        assert!(settings.auto_commit);
        assert!(settings.pause_on_failure);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteExecutionSettingsRepository::new(conn);

        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 4,
            auto_commit: false,
            pause_on_failure: false,
        };

        let updated = repo.update_settings(&new_settings).await.unwrap();
        assert_eq!(updated.max_concurrent_tasks, 4);
        assert!(!updated.auto_commit);
        assert!(!updated.pause_on_failure);

        // Verify persistence
        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 4);
        assert!(!retrieved.auto_commit);
        assert!(!retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_update_max_concurrent_only() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let repo = SqliteExecutionSettingsRepository::new(conn);

        // Update only max_concurrent_tasks
        let new_settings = ExecutionSettings {
            max_concurrent_tasks: 8,
            auto_commit: true,
            pause_on_failure: true,
        };

        repo.update_settings(&new_settings).await.unwrap();

        let retrieved = repo.get_settings().await.unwrap();
        assert_eq!(retrieved.max_concurrent_tasks, 8);
        assert!(retrieved.auto_commit);
        assert!(retrieved.pause_on_failure);
    }

    #[tokio::test]
    async fn test_shared_connection() {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();
        let shared_conn = Arc::new(Mutex::new(conn));

        let repo = SqliteExecutionSettingsRepository::from_shared(Arc::clone(&shared_conn));

        let settings = repo.get_settings().await.unwrap();
        assert_eq!(settings.max_concurrent_tasks, 2);
    }
}
