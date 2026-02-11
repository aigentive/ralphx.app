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
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_selection_creates_new_entry() {
        let repo = MemoryPlanSelectionStatsRepository::new();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
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
        let repo = MemoryPlanSelectionStatsRepository::new();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();
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
        let repo = MemoryPlanSelectionStatsRepository::new();
        let project_id = ProjectId::new();
        let session1 = IdeationSessionId::new();
        let session2 = IdeationSessionId::new();
        let session3 = IdeationSessionId::new(); // Not recorded
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
        assert!(results[2].is_none()); // session3 not recorded
    }

    #[tokio::test]
    async fn test_get_stats_nonexistent() {
        let repo = MemoryPlanSelectionStatsRepository::new();
        let project_id = ProjectId::new();
        let session_id = IdeationSessionId::new();

        let stats = repo.get_stats(&project_id, &session_id).await.unwrap();
        assert!(stats.is_none());
    }
}
