// Memory-based ActivePlanRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::domain::entities::{IdeationSessionId, ProjectId};
use crate::domain::repositories::ActivePlanRepository;

#[derive(Debug, Clone)]
struct SelectionStats {
    selected_count: u32,
    last_selected_at: String,
    last_selected_source: String,
}

/// In-memory implementation of ActivePlanRepository for testing
pub struct MemoryActivePlanRepository {
    active_plans: Arc<RwLock<HashMap<String, String>>>, // project_id -> session_id
    selection_stats: Arc<RwLock<HashMap<(String, String), SelectionStats>>>, // (project_id, session_id) -> stats
}

impl Default for MemoryActivePlanRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryActivePlanRepository {
    /// Create a new empty in-memory active plan repository
    pub fn new() -> Self {
        Self {
            active_plans: Arc::new(RwLock::new(HashMap::new())),
            selection_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl ActivePlanRepository for MemoryActivePlanRepository {
    async fn get(
        &self,
        project_id: &ProjectId,
    ) -> Result<Option<IdeationSessionId>, Box<dyn std::error::Error>> {
        let plans = self.active_plans.read().await;
        Ok(plans
            .get(project_id.as_str())
            .map(|s| IdeationSessionId::from_string(s.clone())))
    }

    async fn set(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut plans = self.active_plans.write().await;
        plans.insert(
            project_id.as_str().to_string(),
            ideation_session_id.as_str().to_string(),
        );
        Ok(())
    }

    async fn clear(&self, project_id: &ProjectId) -> Result<(), Box<dyn std::error::Error>> {
        let mut plans = self.active_plans.write().await;
        plans.remove(project_id.as_str());
        Ok(())
    }

    async fn exists(&self, project_id: &ProjectId) -> Result<bool, Box<dyn std::error::Error>> {
        let plans = self.active_plans.read().await;
        Ok(plans.contains_key(project_id.as_str()))
    }

    async fn record_selection(
        &self,
        project_id: &ProjectId,
        ideation_session_id: &IdeationSessionId,
        source: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stats = self.selection_stats.write().await;
        let key = (
            project_id.as_str().to_string(),
            ideation_session_id.as_str().to_string(),
        );

        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S+00:00")
            .to_string();

        stats
            .entry(key)
            .and_modify(|s| {
                s.selected_count += 1;
                s.last_selected_at = now.clone();
                s.last_selected_source = source.to_string();
            })
            .or_insert(SelectionStats {
                selected_count: 1,
                last_selected_at: now,
                last_selected_source: source.to_string(),
            });

        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_active_plan_repo_tests.rs"]
mod tests;
