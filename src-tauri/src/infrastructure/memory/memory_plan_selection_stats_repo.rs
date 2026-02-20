// Memory-based PlanSelectionStatsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{IdeationSessionId, PlanSelectionStats, ProjectId, SelectionSource};
use crate::domain::repositories::PlanSelectionStatsRepository;
use crate::error::AppResult;

type StatsKey = (String, String); // (project_id, session_id)

/// In-memory implementation of PlanSelectionStatsRepository for testing
pub struct MemoryPlanSelectionStatsRepository {
    stats: Arc<RwLock<HashMap<StatsKey, PlanSelectionStats>>>,
}

impl Default for MemoryPlanSelectionStatsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryPlanSelectionStatsRepository {
    /// Create a new empty in-memory plan selection stats repository
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl PlanSelectionStatsRepository for MemoryPlanSelectionStatsRepository {
    async fn record_selection(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
        source: SelectionSource,
        timestamp: DateTime<Utc>,
    ) -> AppResult<()> {
        let mut stats_map = self.stats.write().await;
        let key = (
            project_id.as_str().to_string(),
            session_id.as_str().to_string(),
        );

        stats_map
            .entry(key)
            .and_modify(|stats| {
                stats.selected_count += 1;
                stats.last_selected_at = Some(timestamp);
                stats.last_selected_source = Some(source.to_db_string().to_string());
            })
            .or_insert_with(|| {
                let mut stats = PlanSelectionStats::new(project_id.clone(), session_id.clone());
                stats.selected_count = 1;
                stats.last_selected_at = Some(timestamp);
                stats.last_selected_source = Some(source.to_db_string().to_string());
                stats
            });

        Ok(())
    }

    async fn get_stats(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
    ) -> AppResult<Option<PlanSelectionStats>> {
        let stats_map = self.stats.read().await;
        let key = (
            project_id.as_str().to_string(),
            session_id.as_str().to_string(),
        );
        Ok(stats_map.get(&key).cloned())
    }

    async fn get_stats_batch(
        &self,
        project_id: &ProjectId,
        session_ids: &[IdeationSessionId],
    ) -> AppResult<Vec<Option<PlanSelectionStats>>> {
        let stats_map = self.stats.read().await;
        let result = session_ids
            .iter()
            .map(|session_id| {
                let key = (
                    project_id.as_str().to_string(),
                    session_id.as_str().to_string(),
                );
                stats_map.get(&key).cloned()
            })
            .collect();
        Ok(result)
    }
}

#[cfg(test)]
#[path = "memory_plan_selection_stats_repo_tests.rs"]
mod tests;
