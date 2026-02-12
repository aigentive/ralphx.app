// In-memory implementation of MemoryEntryRepository for testing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use async_trait::async_trait;

use crate::domain::entities::{MemoryBucket, MemoryEntry, MemoryEntryId, MemoryStatus};
use crate::domain::entities::types::ProjectId;
use crate::domain::repositories::MemoryEntryRepository;
use crate::error::{AppError, AppResult};

pub struct InMemoryMemoryEntryRepository {
    entries: Arc<RwLock<HashMap<MemoryEntryId, MemoryEntry>>>,
}

impl Default for InMemoryMemoryEntryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMemoryEntryRepository {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MemoryEntryRepository for InMemoryMemoryEntryRepository {
    async fn create(&self, entry: MemoryEntry) -> AppResult<MemoryEntry> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id.clone(), entry.clone());
        Ok(entry)
    }

    async fn get_by_id(&self, id: &MemoryEntryId) -> AppResult<Option<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(id).cloned())
    }

    async fn find_by_content_hash(
        &self,
        project_id: &ProjectId,
        bucket: &MemoryBucket,
        content_hash: &str,
    ) -> AppResult<Option<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .find(|e| {
                e.project_id == *project_id
                    && e.bucket == *bucket
                    && e.content_hash == content_hash
                    && e.status == MemoryStatus::Active
            })
            .cloned())
    }

    async fn get_by_project(&self, project_id: &ProjectId) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.project_id == *project_id && e.status == MemoryStatus::Active)
            .cloned()
            .collect())
    }

    async fn get_by_project_and_status(
        &self,
        project_id: &ProjectId,
        status: MemoryStatus,
    ) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.project_id == *project_id && e.status == status)
            .cloned()
            .collect())
    }

    async fn get_by_project_and_bucket(
        &self,
        project_id: &ProjectId,
        bucket: MemoryBucket,
    ) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| {
                e.project_id == *project_id
                    && e.bucket == bucket
                    && e.status == MemoryStatus::Active
            })
            .cloned()
            .collect())
    }

    async fn get_by_rule_file(
        &self,
        project_id: &ProjectId,
        rule_file: &str,
    ) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| {
                e.project_id == *project_id
                    && e.source_rule_file.as_deref() == Some(rule_file)
            })
            .cloned()
            .collect())
    }

    async fn get_by_content_hash(&self, content_hash: &str) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|e| e.content_hash == content_hash)
            .cloned()
            .collect())
    }

    async fn update_status(&self, id: &MemoryEntryId, status: MemoryStatus) -> AppResult<()> {
        let mut entries = self.entries.write().await;
        match entries.get_mut(id) {
            Some(entry) => {
                entry.status = status;
                entry.updated_at = chrono::Utc::now();
                Ok(())
            }
            None => Err(AppError::NotFound(format!("Memory entry not found: {}", id))),
        }
    }

    async fn update(&self, entry: &MemoryEntry) -> AppResult<()> {
        let mut entries = self.entries.write().await;
        if entries.contains_key(&entry.id) {
            entries.insert(entry.id.clone(), entry.clone());
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "Memory entry not found: {}",
                entry.id
            )))
        }
    }

    async fn delete(&self, id: &MemoryEntryId) -> AppResult<()> {
        let mut entries = self.entries.write().await;
        if entries.remove(id).is_some() {
            Ok(())
        } else {
            Err(AppError::NotFound(format!("Memory entry not found: {}", id)))
        }
    }

    async fn get_by_paths(
        &self,
        project_id: &ProjectId,
        paths: &[String],
    ) -> AppResult<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries
            .values()
            .filter(|entry| {
                entry.project_id == *project_id
                    && entry.status == MemoryStatus::Active
                    && paths.iter().any(|path| {
                        entry.scope_paths.iter().any(|glob| {
                            let glob_prefix = glob.trim_end_matches("**").trim_end_matches('*');
                            path.starts_with(glob_prefix)
                        })
                    })
            })
            .cloned()
            .collect())
    }
}
