// Memory-based IdeationEffortSettingsRepository implementation for testing
// Uses RwLock for thread-safe storage without a real database

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::domain::entities::ProjectId;
use crate::domain::ideation::{EffortLevel, IdeationEffortSettings};
use crate::domain::repositories::IdeationEffortSettingsRepository;

/// In-memory implementation of IdeationEffortSettingsRepository for testing.
/// Two-field storage mirrors the DB semantics clearly:
/// - `global_row`: the NULL project_id row
/// - `project_rows`: per-project override rows keyed by project_id string
pub struct MemoryIdeationEffortSettingsRepository {
    global_row: Arc<RwLock<Option<IdeationEffortSettings>>>,
    project_rows: Arc<RwLock<HashMap<String, IdeationEffortSettings>>>,
}

impl Default for MemoryIdeationEffortSettingsRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryIdeationEffortSettingsRepository {
    /// Create a new empty in-memory ideation effort settings repository
    pub fn new() -> Self {
        Self {
            global_row: Arc::new(RwLock::new(None)),
            project_rows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl IdeationEffortSettingsRepository for MemoryIdeationEffortSettingsRepository {
    async fn get_by_project_id(
        &self,
        project_id: Option<&str>,
    ) -> Result<Option<IdeationEffortSettings>, Box<dyn std::error::Error>> {
        match project_id {
            None => {
                let row = self.global_row.read().await;
                Ok(row.clone())
            }
            Some(pid) => {
                let rows = self.project_rows.read().await;
                Ok(rows.get(pid).cloned())
            }
        }
    }

    async fn upsert(
        &self,
        project_id: Option<&str>,
        primary_effort: &str,
        verifier_effort: &str,
    ) -> Result<IdeationEffortSettings, Box<dyn std::error::Error>> {
        let primary = EffortLevel::from_str(primary_effort)
            .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
        let verifier = EffortLevel::from_str(verifier_effort)
            .map_err(|e| Box::<dyn std::error::Error>::from(e))?;

        match project_id {
            None => {
                let mut row = self.global_row.write().await;
                let id = row.as_ref().map(|r| r.id).unwrap_or(1);
                let settings = IdeationEffortSettings {
                    id,
                    project_id: None,
                    primary_effort: primary,
                    verifier_effort: verifier,
                    updated_at: Utc::now(),
                };
                *row = Some(settings.clone());
                Ok(settings)
            }
            Some(pid) => {
                let mut rows = self.project_rows.write().await;
                let id = rows
                    .get(pid)
                    .map(|r| r.id)
                    .unwrap_or_else(|| rows.len() as i64 + 2);
                let settings = IdeationEffortSettings {
                    id,
                    project_id: Some(ProjectId(pid.to_string())),
                    primary_effort: primary,
                    verifier_effort: verifier,
                    updated_at: Utc::now(),
                };
                rows.insert(pid.to_string(), settings.clone());
                Ok(settings)
            }
        }
    }
}

#[cfg(test)]
#[path = "memory_ideation_effort_settings_repo_tests.rs"]
mod tests;
