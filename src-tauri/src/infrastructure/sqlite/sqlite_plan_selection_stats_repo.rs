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
#[path = "sqlite_plan_selection_stats_repo_tests.rs"]
mod tests;
