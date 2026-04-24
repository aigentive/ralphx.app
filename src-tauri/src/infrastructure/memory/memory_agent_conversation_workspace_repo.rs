use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::domain::entities::{
    AgentConversationWorkspace, AgentConversationWorkspaceStatus, ChatConversationId,
    IdeationSessionId, PlanBranchId, ProjectId,
};
use crate::domain::repositories::AgentConversationWorkspaceRepository;
use crate::error::AppResult;

pub struct MemoryAgentConversationWorkspaceRepository {
    workspaces: RwLock<HashMap<ChatConversationId, AgentConversationWorkspace>>,
}

impl MemoryAgentConversationWorkspaceRepository {
    pub fn new() -> Self {
        Self {
            workspaces: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryAgentConversationWorkspaceRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentConversationWorkspaceRepository for MemoryAgentConversationWorkspaceRepository {
    async fn create_or_update(
        &self,
        mut workspace: AgentConversationWorkspace,
    ) -> AppResult<AgentConversationWorkspace> {
        let mut workspaces = self.workspaces.write().await;
        if let Some(existing) = workspaces.get(&workspace.conversation_id) {
            workspace.created_at = existing.created_at;
        }
        workspace.updated_at = Utc::now();
        workspaces.insert(workspace.conversation_id, workspace.clone());
        Ok(workspace)
    }

    async fn get_by_conversation_id(
        &self,
        conversation_id: &ChatConversationId,
    ) -> AppResult<Option<AgentConversationWorkspace>> {
        Ok(self.workspaces.read().await.get(conversation_id).cloned())
    }

    async fn get_by_project_id(
        &self,
        project_id: &ProjectId,
    ) -> AppResult<Vec<AgentConversationWorkspace>> {
        Ok(self
            .workspaces
            .read()
            .await
            .values()
            .filter(|workspace| workspace.project_id == *project_id)
            .cloned()
            .collect())
    }

    async fn update_links(
        &self,
        conversation_id: &ChatConversationId,
        ideation_session_id: Option<&IdeationSessionId>,
        plan_branch_id: Option<&PlanBranchId>,
    ) -> AppResult<()> {
        if let Some(workspace) = self.workspaces.write().await.get_mut(conversation_id) {
            workspace.linked_ideation_session_id = ideation_session_id.cloned();
            workspace.linked_plan_branch_id = plan_branch_id.cloned();
            workspace.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_publication(
        &self,
        conversation_id: &ChatConversationId,
        pr_number: Option<i64>,
        pr_url: Option<&str>,
        pr_status: Option<&str>,
        push_status: Option<&str>,
    ) -> AppResult<()> {
        if let Some(workspace) = self.workspaces.write().await.get_mut(conversation_id) {
            workspace.publication_pr_number = pr_number;
            workspace.publication_pr_url = pr_url.map(str::to_string);
            workspace.publication_pr_status = pr_status.map(str::to_string);
            workspace.publication_push_status = push_status.map(str::to_string);
            workspace.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn update_status(
        &self,
        conversation_id: &ChatConversationId,
        status: AgentConversationWorkspaceStatus,
    ) -> AppResult<()> {
        if let Some(workspace) = self.workspaces.write().await.get_mut(conversation_id) {
            workspace.status = status;
            workspace.updated_at = Utc::now();
        }
        Ok(())
    }

    async fn delete(&self, conversation_id: &ChatConversationId) -> AppResult<()> {
        self.workspaces.write().await.remove(conversation_id);
        Ok(())
    }
}
