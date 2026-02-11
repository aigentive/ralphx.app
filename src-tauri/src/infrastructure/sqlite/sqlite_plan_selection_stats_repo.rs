// SQLite-based PlanSelectionStatsRepository implementation for production use
// Uses rusqlite with connection pooling for thread-safe access

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::domain::entities::{IdeationSessionId, PlanSelectionStats, ProjectId, SelectionSource};
use crate::domain::repositories::PlanSelectionStatsRepository;
use crate::error::{AppError, AppResult};

/// SQLite implementation of PlanSelectionStatsRepository for production use
/// Uses a mutex-protected connection for thread-safe access
pub struct SqlitePlanSelectionStatsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl SqlitePlanSelectionStatsRepository {
    /// Create a new SQLite plan selection stats repository with the given connection
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Create from an Arc-wrapped mutex connection (for sharing)
    pub fn from_shared(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl PlanSelectionStatsRepository for SqlitePlanSelectionStatsRepository {
    async fn record_selection(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
        source: SelectionSource,
        timestamp: DateTime<Utc>,
    ) -> AppResult<()> {
        let conn = self.conn.lock().await;

        // UPSERT: increment count if exists, create new entry if not
        conn.execute(
            "INSERT INTO plan_selection_stats (project_id, ideation_session_id, selected_count, last_selected_at, last_selected_source)
             VALUES (?1, ?2, 1, ?3, ?4)
             ON CONFLICT(project_id, ideation_session_id) DO UPDATE SET
                 selected_count = selected_count + 1,
                 last_selected_at = excluded.last_selected_at,
                 last_selected_source = excluded.last_selected_source",
            rusqlite::params![
                project_id.as_str(),
                session_id.as_str(),
                timestamp.to_rfc3339(),
                source.to_db_string(),
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_stats(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
    ) -> AppResult<Option<PlanSelectionStats>> {
        let conn = self.conn.lock().await;

        let result = conn.query_row(
            "SELECT project_id, ideation_session_id, selected_count, last_selected_at, last_selected_source
             FROM plan_selection_stats
             WHERE project_id = ?1 AND ideation_session_id = ?2",
            rusqlite::params![project_id.as_str(), session_id.as_str()],
            |row| PlanSelectionStats::from_row(row),
        );

        match result {
            Ok(stats) => Ok(Some(stats)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    async fn get_stats_batch(
        &self,
        project_id: &ProjectId,
        session_ids: &[IdeationSessionId],
    ) -> AppResult<Vec<Option<PlanSelectionStats>>> {
        if session_ids.is_empty() {
            return Ok(vec![]);
        }

        let conn = self.conn.lock().await;

        // Build query with IN clause
        let placeholders = session_ids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query = format!(
            "SELECT project_id, ideation_session_id, selected_count, last_selected_at, last_selected_source
             FROM plan_selection_stats
             WHERE project_id = ? AND ideation_session_id IN ({})",
            placeholders
        );

        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Build params: first is project_id, rest are session_ids
        let project_id_str = project_id.as_str().to_string();
        let session_id_strs: Vec<String> = session_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();

        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&project_id_str as &dyn rusqlite::ToSql];
        for id_str in &session_id_strs {
            params.push(id_str as &dyn rusqlite::ToSql);
        }

        let rows = stmt
            .query_map(params.as_slice(), PlanSelectionStats::from_row)
            .map_err(|e| AppError::Database(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))?;

        // Build a map for fast lookup
        let mut stats_map = std::collections::HashMap::new();
        for stat in rows {
            stats_map.insert(stat.ideation_session_id.clone(), stat);
        }

        // Return in same order as input, with None for missing entries
        let result = session_ids
            .iter()
            .map(|id| stats_map.get(id).cloned())
            .collect();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::open_memory_connection;
    use crate::infrastructure::sqlite::run_migrations;

    fn setup_test_db() -> (
        SqlitePlanSelectionStatsRepository,
        ProjectId,
        IdeationSessionId,
    ) {
        let conn = open_memory_connection().unwrap();
        run_migrations(&conn).unwrap();

        // Create project
        let project_id = ProjectId::new();
        conn.execute(
            "INSERT INTO projects (id, name, working_directory, git_mode, merge_validation_mode, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                project_id.as_str(),
                "Test Project",
                "/test/path",
                "local",
                "block",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        // Create ideation session
        let session_id = IdeationSessionId::new();
        conn.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session_id.as_str(),
                project_id.as_str(),
                "Test Session",
                "accepted",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )
        .unwrap();

        (
            SqlitePlanSelectionStatsRepository::new(conn),
            project_id,
            session_id,
        )
    }

    #[tokio::test]
    async fn test_record_selection_creates_new_entry() {
        let (repo, project_id, session_id) = setup_test_db();
        let timestamp = Utc::now();

        repo.record_selection(
            &project_id,
            &session_id,
            SelectionSource::KanbanInline,
            timestamp,
        )
        .await
        .unwrap();

        let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.selected_count, 1);
        assert_eq!(
            stats.last_selected_source,
            Some("kanban_inline".to_string())
        );
    }

    #[tokio::test]
    async fn test_record_selection_increments_count() {
        let (repo, project_id, session_id) = setup_test_db();
        let timestamp1 = Utc::now();

        // First selection
        repo.record_selection(
            &project_id,
            &session_id,
            SelectionSource::KanbanInline,
            timestamp1,
        )
        .await
        .unwrap();

        // Second selection
        let timestamp2 = Utc::now();
        repo.record_selection(
            &project_id,
            &session_id,
            SelectionSource::QuickSwitcher,
            timestamp2,
        )
        .await
        .unwrap();

        let stats = repo
            .get_stats(&project_id, &session_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stats.selected_count, 2);
        assert_eq!(
            stats.last_selected_source,
            Some("quick_switcher".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_stats_batch() {
        let (repo, project_id, session1) = setup_test_db();

        // Create second session
        let session2 = IdeationSessionId::new();
        repo.conn.lock().await.execute(
            "INSERT INTO ideation_sessions (id, project_id, title, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session2.as_str(),
                project_id.as_str(),
                "Test Session 2",
                "accepted",
                Utc::now().to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        ).unwrap();

        let session3 = IdeationSessionId::new(); // Not in DB
        let timestamp = Utc::now();

        // Record stats for session1 and session2
        repo.record_selection(
            &project_id,
            &session1,
            SelectionSource::KanbanInline,
            timestamp,
        )
        .await
        .unwrap();
        repo.record_selection(
            &project_id,
            &session2,
            SelectionSource::GraphInline,
            timestamp,
        )
        .await
        .unwrap();

        // Query batch
        let results = repo
            .get_stats_batch(
                &project_id,
                &[session1.clone(), session2.clone(), session3.clone()],
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert_eq!(results[0].as_ref().unwrap().ideation_session_id, session1);
        assert!(results[1].is_some());
        assert_eq!(results[1].as_ref().unwrap().ideation_session_id, session2);
        assert!(results[2].is_none()); // session3 not in DB
    }

    #[tokio::test]
    async fn test_get_stats_nonexistent() {
        let (repo, project_id, session_id) = setup_test_db();

        let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
        assert!(stats.is_none());
    }
}
