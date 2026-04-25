use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::domain::entities::{
    DesignApprovalStatus, DesignFeedbackStatus, DesignRun, DesignRunId, DesignSchemaVersion,
    DesignSchemaVersionId, DesignStyleguideFeedback, DesignStyleguideFeedbackId,
    DesignStyleguideItem, DesignStyleguideItemId, DesignSystem, DesignSystemId, DesignSystemSource,
    ProjectId,
};
use crate::domain::repositories::{
    DesignRunRepository, DesignSchemaRepository, DesignStyleguideFeedbackRepository,
    DesignStyleguideRepository, DesignSystemRepository, DesignSystemSourceRepository,
};
use crate::error::AppResult;

pub struct MemoryDesignSystemRepository {
    systems: Arc<RwLock<HashMap<DesignSystemId, DesignSystem>>>,
}

impl MemoryDesignSystemRepository {
    pub fn new() -> Self {
        Self {
            systems: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignSystemRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignSystemRepository for MemoryDesignSystemRepository {
    async fn create(&self, system: DesignSystem) -> AppResult<DesignSystem> {
        self.systems
            .write()
            .await
            .insert(system.id.clone(), system.clone());
        Ok(system)
    }

    async fn get_by_id(&self, id: &DesignSystemId) -> AppResult<Option<DesignSystem>> {
        Ok(self.systems.read().await.get(id).cloned())
    }

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
        include_archived: bool,
    ) -> AppResult<Vec<DesignSystem>> {
        let mut systems: Vec<DesignSystem> = self
            .systems
            .read()
            .await
            .values()
            .filter(|system| {
                &system.primary_project_id == project_id
                    && (include_archived || system.archived_at.is_none())
            })
            .cloned()
            .collect();
        systems.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(systems)
    }

    async fn update(&self, system: &DesignSystem) -> AppResult<()> {
        self.systems
            .write()
            .await
            .insert(system.id.clone(), system.clone());
        Ok(())
    }

    async fn archive(&self, id: &DesignSystemId) -> AppResult<()> {
        if let Some(system) = self.systems.write().await.get_mut(id) {
            let now = Utc::now();
            system.archived_at = Some(now);
            system.updated_at = now;
        }
        Ok(())
    }
}

pub struct MemoryDesignSystemSourceRepository {
    sources_by_system: Arc<RwLock<HashMap<DesignSystemId, Vec<DesignSystemSource>>>>,
}

impl MemoryDesignSystemSourceRepository {
    pub fn new() -> Self {
        Self {
            sources_by_system: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignSystemSourceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignSystemSourceRepository for MemoryDesignSystemSourceRepository {
    async fn replace_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
        sources: Vec<DesignSystemSource>,
    ) -> AppResult<()> {
        self.sources_by_system
            .write()
            .await
            .insert(design_system_id.clone(), sources);
        Ok(())
    }

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSystemSource>> {
        let mut sources = self
            .sources_by_system
            .read()
            .await
            .get(design_system_id)
            .cloned()
            .unwrap_or_default();
        sources.sort_by(|left, right| {
            format!("{:?}", left.role)
                .cmp(&format!("{:?}", right.role))
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        Ok(sources)
    }
}

pub struct MemoryDesignSchemaRepository {
    versions: Arc<RwLock<HashMap<DesignSchemaVersionId, DesignSchemaVersion>>>,
}

impl MemoryDesignSchemaRepository {
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignSchemaRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignSchemaRepository for MemoryDesignSchemaRepository {
    async fn create_version(&self, version: DesignSchemaVersion) -> AppResult<DesignSchemaVersion> {
        self.versions
            .write()
            .await
            .insert(version.id.clone(), version.clone());
        Ok(version)
    }

    async fn get_version(
        &self,
        id: &DesignSchemaVersionId,
    ) -> AppResult<Option<DesignSchemaVersion>> {
        Ok(self.versions.read().await.get(id).cloned())
    }

    async fn get_current_for_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Option<DesignSchemaVersion>> {
        Ok(self
            .versions
            .read()
            .await
            .values()
            .filter(|version| &version.design_system_id == design_system_id)
            .max_by_key(|version| version.created_at)
            .cloned())
    }

    async fn list_versions(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignSchemaVersion>> {
        let mut versions: Vec<DesignSchemaVersion> = self
            .versions
            .read()
            .await
            .values()
            .filter(|version| &version.design_system_id == design_system_id)
            .cloned()
            .collect();
        versions.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(versions)
    }
}

pub struct MemoryDesignStyleguideRepository {
    items: Arc<RwLock<HashMap<DesignStyleguideItemId, DesignStyleguideItem>>>,
}

impl MemoryDesignStyleguideRepository {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignStyleguideRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignStyleguideRepository for MemoryDesignStyleguideRepository {
    async fn replace_items_for_schema_version(
        &self,
        schema_version_id: &DesignSchemaVersionId,
        items: Vec<DesignStyleguideItem>,
    ) -> AppResult<()> {
        let mut stored_items = self.items.write().await;
        stored_items.retain(|_, item| &item.schema_version_id != schema_version_id);
        for item in items {
            stored_items.insert(item.id.clone(), item);
        }
        Ok(())
    }

    async fn list_items(
        &self,
        design_system_id: &DesignSystemId,
        schema_version_id: Option<&DesignSchemaVersionId>,
    ) -> AppResult<Vec<DesignStyleguideItem>> {
        let mut items: Vec<DesignStyleguideItem> = self
            .items
            .read()
            .await
            .values()
            .filter(|item| {
                &item.design_system_id == design_system_id
                    && schema_version_id.map_or(true, |id| &item.schema_version_id == id)
            })
            .cloned()
            .collect();
        items.sort_by(|left, right| {
            format!("{:?}", left.group)
                .cmp(&format!("{:?}", right.group))
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        Ok(items)
    }

    async fn get_item(
        &self,
        design_system_id: &DesignSystemId,
        item_id: &str,
    ) -> AppResult<Option<DesignStyleguideItem>> {
        Ok(self
            .items
            .read()
            .await
            .values()
            .filter(|item| &item.design_system_id == design_system_id && item.item_id == item_id)
            .max_by_key(|item| item.updated_at)
            .cloned())
    }

    async fn update_item(&self, item: &DesignStyleguideItem) -> AppResult<()> {
        self.items
            .write()
            .await
            .insert(item.id.clone(), item.clone());
        Ok(())
    }

    async fn approve_item(&self, id: &DesignStyleguideItemId) -> AppResult<()> {
        if let Some(item) = self.items.write().await.get_mut(id) {
            item.approval_status = DesignApprovalStatus::Approved;
            item.feedback_status = DesignFeedbackStatus::Resolved;
            item.updated_at = Utc::now();
        }
        Ok(())
    }
}

pub struct MemoryDesignStyleguideFeedbackRepository {
    feedback: Arc<RwLock<HashMap<DesignStyleguideFeedbackId, DesignStyleguideFeedback>>>,
}

impl MemoryDesignStyleguideFeedbackRepository {
    pub fn new() -> Self {
        Self {
            feedback: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignStyleguideFeedbackRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignStyleguideFeedbackRepository for MemoryDesignStyleguideFeedbackRepository {
    async fn create(
        &self,
        feedback: DesignStyleguideFeedback,
    ) -> AppResult<DesignStyleguideFeedback> {
        self.feedback
            .write()
            .await
            .insert(feedback.id.clone(), feedback.clone());
        Ok(feedback)
    }

    async fn get_by_id(
        &self,
        id: &DesignStyleguideFeedbackId,
    ) -> AppResult<Option<DesignStyleguideFeedback>> {
        Ok(self.feedback.read().await.get(id).cloned())
    }

    async fn list_open_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignStyleguideFeedback>> {
        let mut feedback: Vec<DesignStyleguideFeedback> = self
            .feedback
            .read()
            .await
            .values()
            .filter(|feedback| {
                &feedback.design_system_id == design_system_id
                    && !matches!(
                        feedback.status,
                        DesignFeedbackStatus::Resolved | DesignFeedbackStatus::Dismissed
                    )
            })
            .cloned()
            .collect();
        feedback.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(feedback)
    }

    async fn update(&self, feedback: &DesignStyleguideFeedback) -> AppResult<()> {
        self.feedback
            .write()
            .await
            .insert(feedback.id.clone(), feedback.clone());
        Ok(())
    }
}

pub struct MemoryDesignRunRepository {
    runs: Arc<RwLock<HashMap<DesignRunId, DesignRun>>>,
}

impl MemoryDesignRunRepository {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDesignRunRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DesignRunRepository for MemoryDesignRunRepository {
    async fn create(&self, run: DesignRun) -> AppResult<DesignRun> {
        self.runs.write().await.insert(run.id.clone(), run.clone());
        Ok(run)
    }

    async fn get_by_id(&self, id: &DesignRunId) -> AppResult<Option<DesignRun>> {
        Ok(self.runs.read().await.get(id).cloned())
    }

    async fn list_by_design_system(
        &self,
        design_system_id: &DesignSystemId,
    ) -> AppResult<Vec<DesignRun>> {
        let mut runs: Vec<DesignRun> = self
            .runs
            .read()
            .await
            .values()
            .filter(|run| &run.design_system_id == design_system_id)
            .cloned()
            .collect();
        runs.sort_by(|left, right| {
            let left_at = left.started_at.or(left.completed_at);
            let right_at = right.started_at.or(right.completed_at);
            right_at
                .cmp(&left_at)
                .then_with(|| left.id.0.cmp(&right.id.0))
        });
        Ok(runs)
    }

    async fn update(&self, run: &DesignRun) -> AppResult<()> {
        self.runs.write().await.insert(run.id.clone(), run.clone());
        Ok(())
    }
}

#[cfg(test)]
#[path = "memory_design_repository_tests.rs"]
mod tests;
