// TaskContextService - aggregates task context with related artifacts and proposals
//
// Provides rich context for workers executing tasks by fetching:
// - Task details
// - Source proposal (if task was created from ideation)
// - Implementation plan artifact summary
// - Related artifacts
// - Context hints for workers

use std::sync::Arc;

use crate::domain::entities::{
    ArtifactSummary, Task, TaskContext, TaskId, TaskProposalSummary,
};
use crate::domain::repositories::{ArtifactRepository, TaskProposalRepository, TaskRepository};
use crate::error::{AppError, AppResult};

/// Service for aggregating task context for worker execution
pub struct TaskContextService {
    task_repo: Arc<dyn TaskRepository>,
    proposal_repo: Arc<dyn TaskProposalRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
}

impl TaskContextService
{
    /// Create a new TaskContextService with the given repositories
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        proposal_repo: Arc<dyn TaskProposalRepository>,
        artifact_repo: Arc<dyn ArtifactRepository>,
    ) -> Self {
        Self {
            task_repo,
            proposal_repo,
            artifact_repo,
        }
    }

    /// Get rich context for a task including linked artifacts and proposals
    ///
    /// Returns TaskContext with:
    /// - The task being executed
    /// - Source proposal summary (if exists)
    /// - Plan artifact summary with 500-char preview (if exists)
    /// - Related artifacts
    /// - Context hints for worker
    pub async fn get_task_context(&self, task_id: &TaskId) -> AppResult<TaskContext> {
        // 1. Fetch task by ID
        let task = self
            .task_repo
            .get_by_id(task_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Task not found: {}", task_id)))?;

        // 2. If source_proposal_id present, fetch proposal and create TaskProposalSummary
        let source_proposal = if let Some(proposal_id) = &task.source_proposal_id {
            match self.proposal_repo.get_by_id(proposal_id).await? {
                Some(proposal) => {
                    // Parse acceptance_criteria from JSON string to Vec<String>
                    let acceptance_criteria: Vec<String> = proposal
                        .acceptance_criteria
                        .as_ref()
                        .and_then(|json_str| serde_json::from_str(json_str).ok())
                        .unwrap_or_default();

                    Some(TaskProposalSummary {
                        id: proposal.id.clone(),
                        title: proposal.title.clone(),
                        description: proposal.description.clone().unwrap_or_default(),
                        acceptance_criteria,
                        implementation_notes: None, // TaskProposal doesn't have implementation_notes field
                        plan_version_at_creation: proposal.plan_version_at_creation,
                    })
                }
                None => None,
            }
        } else {
            None
        };

        // 3. If plan_artifact_id present, fetch artifact and create ArtifactSummary (500-char preview)
        let plan_artifact = if let Some(artifact_id) = &task.plan_artifact_id {
            match self.artifact_repo.get_by_id(artifact_id).await? {
                Some(artifact) => {
                    let content_preview = Self::create_content_preview(&artifact);
                    Some(ArtifactSummary {
                        id: artifact.id.clone(),
                        title: artifact.name.clone(),
                        artifact_type: artifact.artifact_type,
                        current_version: artifact.metadata.version,
                        content_preview,
                    })
                }
                None => None,
            }
        } else {
            None
        };

        // 4. Fetch related artifacts via ArtifactRelation
        let related_artifacts = if let Some(artifact_id) = &task.plan_artifact_id {
            let related = self.artifact_repo.get_related(artifact_id).await?;
            related
                .into_iter()
                .map(|artifact| {
                    let content_preview = Self::create_content_preview(&artifact);
                    ArtifactSummary {
                        id: artifact.id.clone(),
                        title: artifact.name.clone(),
                        artifact_type: artifact.artifact_type,
                        current_version: artifact.metadata.version,
                        content_preview,
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // 5. Generate context_hints based on what's available
        let context_hints = self.generate_context_hints(
            &task,
            source_proposal.is_some(),
            plan_artifact.is_some(),
            related_artifacts.len(),
        );

        // 6. Return TaskContext
        Ok(TaskContext {
            task,
            source_proposal,
            plan_artifact,
            related_artifacts,
            context_hints,
        })
    }

    /// Create a 500-character preview of artifact content
    fn create_content_preview(artifact: &crate::domain::entities::Artifact) -> String {
        use crate::domain::entities::ArtifactContent;

        let full_content = match &artifact.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => {
                // For file-based artifacts, we can't read the file here
                // Return a message indicating it's a file
                format!("[File artifact at: {}]", path)
            }
        };

        // Take first 500 chars
        if full_content.len() <= 500 {
            full_content
        } else {
            format!("{}...", &full_content[..500])
        }
    }

    /// Generate context hints for the worker based on available context
    fn generate_context_hints(
        &self,
        task: &Task,
        has_proposal: bool,
        has_plan: bool,
        related_count: usize,
    ) -> Vec<String> {
        let mut hints = Vec::new();

        if has_proposal {
            hints.push(
                "Task was created from ideation proposal - check acceptance criteria".to_string(),
            );
        }

        if has_plan {
            hints.push("Implementation plan available - use get_artifact to read full plan before starting".to_string());
        }

        if related_count > 0 {
            hints.push(format!(
                "{} related artifact{} found - may contain useful context",
                related_count,
                if related_count == 1 { "" } else { "s" }
            ));
        }

        if task.description.is_some() {
            hints.push("Task has description with additional details".to_string());
        }

        if hints.is_empty() {
            hints.push("No additional context artifacts found - proceed with task description and acceptance criteria".to_string());
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        Artifact, ArtifactId, ArtifactRelation, ArtifactRelationType, ArtifactType,
        InternalStatus, Priority, ProjectId, TaskCategory, TaskProposal, TaskProposalId,
    };
    use crate::domain::repositories::{ArtifactRepository, TaskProposalRepository, TaskRepository};
    use async_trait::async_trait;
    use std::sync::Arc;

    // Mock repositories for testing
    struct MockTaskRepository {
        task: Option<Task>,
    }

    impl MockTaskRepository {
        fn with_task(task: Task) -> Self {
            Self { task: Some(task) }
        }
    }

    #[async_trait]
    impl TaskRepository for MockTaskRepository {
        async fn create(&self, task: Task) -> AppResult<Task> {
            Ok(task)
        }

        async fn get_by_id(&self, _id: &TaskId) -> AppResult<Option<Task>> {
            Ok(self.task.clone())
        }

        async fn get_by_project(&self, _project_id: &ProjectId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn update(&self, _task: &Task) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn get_by_status(
            &self,
            _project_id: &ProjectId,
            _status: InternalStatus,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn persist_status_change(
            &self,
            _id: &TaskId,
            _from: InternalStatus,
            _to: InternalStatus,
            _trigger: &str,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_status_history(
            &self,
            _id: &TaskId,
        ) -> AppResult<Vec<crate::domain::repositories::StatusTransition>> {
            Ok(vec![])
        }

        async fn get_next_executable(&self, _project_id: &ProjectId) -> AppResult<Option<Task>> {
            Ok(None)
        }

        async fn get_blockers(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_dependents(&self, _id: &TaskId) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn add_blocker(&self, _task_id: &TaskId, _blocker_id: &TaskId) -> AppResult<()> {
            Ok(())
        }

        async fn resolve_blocker(
            &self,
            _task_id: &TaskId,
            _blocker_id: &TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_by_project_filtered(
            &self,
            _project_id: &ProjectId,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn archive(&self, task_id: &TaskId) -> AppResult<Task> {
            if let Some(task) = &self.task {
                let mut archived = task.clone();
                archived.archived_at = Some(chrono::Utc::now());
                Ok(archived)
            } else {
                Err(crate::error::AppError::NotFound(format!(
                    "Task {} not found",
                    task_id.as_str()
                )))
            }
        }

        async fn restore(&self, task_id: &TaskId) -> AppResult<Task> {
            if let Some(task) = &self.task {
                let mut restored = task.clone();
                restored.archived_at = None;
                Ok(restored)
            } else {
                Err(crate::error::AppError::NotFound(format!(
                    "Task {} not found",
                    task_id.as_str()
                )))
            }
        }

        async fn get_archived_count(&self, _project_id: &ProjectId) -> AppResult<u32> {
            Ok(0)
        }

        async fn list_paginated(
            &self,
            _project_id: &ProjectId,
            _status: Option<InternalStatus>,
            _offset: u32,
            _limit: u32,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            if let Some(task) = &self.task {
                Ok(vec![task.clone()])
            } else {
                Ok(vec![])
            }
        }

        async fn count_tasks(&self, _project_id: &ProjectId, _include_archived: bool) -> AppResult<u32> {
            Ok(if self.task.is_some() { 1 } else { 0 })
        }
    }

    struct MockTaskProposalRepository {
        proposal: Option<TaskProposal>,
    }

    impl MockTaskProposalRepository {
        fn empty() -> Self {
            Self { proposal: None }
        }

        fn with_proposal(proposal: TaskProposal) -> Self {
            Self {
                proposal: Some(proposal),
            }
        }
    }

    #[async_trait]
    impl TaskProposalRepository for MockTaskProposalRepository {
        async fn create(&self, proposal: TaskProposal) -> AppResult<TaskProposal> {
            Ok(proposal)
        }

        async fn get_by_id(&self, _id: &TaskProposalId) -> AppResult<Option<TaskProposal>> {
            Ok(self.proposal.clone())
        }

        async fn get_by_session(
            &self,
            _session_id: &crate::domain::entities::IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            Ok(vec![])
        }

        async fn update(&self, _proposal: &TaskProposal) -> AppResult<()> {
            Ok(())
        }

        async fn update_priority(
            &self,
            _id: &TaskProposalId,
            _assessment: &crate::domain::entities::PriorityAssessment,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn update_selection(&self, _id: &TaskProposalId, _selected: bool) -> AppResult<()> {
            Ok(())
        }

        async fn set_created_task_id(
            &self,
            _id: &TaskProposalId,
            _task_id: &TaskId,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &TaskProposalId) -> AppResult<()> {
            Ok(())
        }

        async fn reorder(
            &self,
            _session_id: &crate::domain::entities::IdeationSessionId,
            _proposal_ids: Vec<TaskProposalId>,
        ) -> AppResult<()> {
            Ok(())
        }

        async fn get_selected_by_session(
            &self,
            _session_id: &crate::domain::entities::IdeationSessionId,
        ) -> AppResult<Vec<TaskProposal>> {
            Ok(vec![])
        }

        async fn count_by_session(
            &self,
            _session_id: &crate::domain::entities::IdeationSessionId,
        ) -> AppResult<u32> {
            Ok(0)
        }

        async fn count_selected_by_session(
            &self,
            _session_id: &crate::domain::entities::IdeationSessionId,
        ) -> AppResult<u32> {
            Ok(0)
        }

        async fn get_by_plan_artifact_id(
            &self,
            _artifact_id: &ArtifactId,
        ) -> AppResult<Vec<TaskProposal>> {
            Ok(vec![])
        }
    }

    struct MockArtifactRepository {
        artifact: Option<Artifact>,
        related: Vec<Artifact>,
    }

    impl MockArtifactRepository {
        fn empty() -> Self {
            Self {
                artifact: None,
                related: vec![],
            }
        }

        fn with_artifact(artifact: Artifact) -> Self {
            Self {
                artifact: Some(artifact),
                related: vec![],
            }
        }

        fn with_related(artifact: Artifact, related: Vec<Artifact>) -> Self {
            Self {
                artifact: Some(artifact),
                related,
            }
        }
    }

    #[async_trait]
    impl ArtifactRepository for MockArtifactRepository {
        async fn create(&self, artifact: Artifact) -> AppResult<Artifact> {
            Ok(artifact)
        }

        async fn get_by_id(&self, _id: &ArtifactId) -> AppResult<Option<Artifact>> {
            Ok(self.artifact.clone())
        }

        async fn get_by_id_at_version(
            &self,
            _id: &ArtifactId,
            _version: u32,
        ) -> AppResult<Option<Artifact>> {
            Ok(self.artifact.clone())
        }

        async fn get_by_bucket(
            &self,
            _bucket_id: &crate::domain::entities::ArtifactBucketId,
        ) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn get_by_type(&self, _artifact_type: ArtifactType) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn get_by_task(&self, _task_id: &TaskId) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn get_by_process(
            &self,
            _process_id: &crate::domain::entities::ProcessId,
        ) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn update(&self, _artifact: &Artifact) -> AppResult<()> {
            Ok(())
        }

        async fn delete(&self, _id: &ArtifactId) -> AppResult<()> {
            Ok(())
        }

        async fn get_derived_from(&self, _artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
            Ok(vec![])
        }

        async fn get_related(&self, _artifact_id: &ArtifactId) -> AppResult<Vec<Artifact>> {
            Ok(self.related.clone())
        }

        async fn add_relation(&self, relation: ArtifactRelation) -> AppResult<ArtifactRelation> {
            Ok(relation)
        }

        async fn get_relations(
            &self,
            _artifact_id: &ArtifactId,
        ) -> AppResult<Vec<ArtifactRelation>> {
            Ok(vec![])
        }

        async fn get_relations_by_type(
            &self,
            _artifact_id: &ArtifactId,
            _relation_type: ArtifactRelationType,
        ) -> AppResult<Vec<ArtifactRelation>> {
            Ok(vec![])
        }

        async fn delete_relation(
            &self,
            _from_id: &ArtifactId,
            _to_id: &ArtifactId,
        ) -> AppResult<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_get_task_context_basic() {
        let task = Task::new(ProjectId::new(), "Test Task".to_string());
        let task_id = task.id.clone();

        let service = TaskContextService::new(
            Arc::new(MockTaskRepository::with_task(task.clone())),
            Arc::new(MockTaskProposalRepository::empty()),
            Arc::new(MockArtifactRepository::empty()),
        );

        let context = service.get_task_context(&task_id).await.unwrap();

        assert_eq!(context.task.id, task_id);
        assert!(context.source_proposal.is_none());
        assert!(context.plan_artifact.is_none());
        assert_eq!(context.related_artifacts.len(), 0);
        assert!(!context.context_hints.is_empty());
    }

    #[tokio::test]
    async fn test_get_task_context_with_proposal() {
        let mut task = Task::new(ProjectId::new(), "Test Task".to_string());
        let proposal_id = TaskProposalId::new();
        task.source_proposal_id = Some(proposal_id.clone());
        let task_id = task.id.clone();

        let mut proposal = TaskProposal::new(
            crate::domain::entities::IdeationSessionId::new(),
            "Test Proposal",
            TaskCategory::Feature,
            Priority::Medium,
        );
        proposal.id = proposal_id;
        proposal.description = Some("Proposal description".to_string());
        // acceptance_criteria is stored as JSON string
        proposal.acceptance_criteria = Some(serde_json::to_string(&vec!["AC1"]).unwrap());

        let service = TaskContextService::new(
            Arc::new(MockTaskRepository::with_task(task.clone())),
            Arc::new(MockTaskProposalRepository::with_proposal(proposal)),
            Arc::new(MockArtifactRepository::empty()),
        );

        let context = service.get_task_context(&task_id).await.unwrap();

        assert_eq!(context.task.id, task_id);
        assert!(context.source_proposal.is_some());
        let proposal_summary = context.source_proposal.unwrap();
        assert_eq!(proposal_summary.title, "Test Proposal");
        assert_eq!(proposal_summary.acceptance_criteria.len(), 1);
        assert!(context.context_hints.iter().any(|h| h.contains("ideation")));
    }

    #[tokio::test]
    async fn test_get_task_context_with_plan_artifact() {
        let mut task = Task::new(ProjectId::new(), "Test Task".to_string());
        let artifact_id = ArtifactId::new();
        task.plan_artifact_id = Some(artifact_id.clone());
        let task_id = task.id.clone();

        let artifact = Artifact::new_inline(
            "Implementation Plan",
            ArtifactType::Specification,
            "# Plan\n\nDetailed implementation instructions...".repeat(20),
            "user",
        );

        let service = TaskContextService::new(
            Arc::new(MockTaskRepository::with_task(task.clone())),
            Arc::new(MockTaskProposalRepository::empty()),
            Arc::new(MockArtifactRepository::with_artifact(artifact)),
        );

        let context = service.get_task_context(&task_id).await.unwrap();

        assert_eq!(context.task.id, task_id);
        assert!(context.plan_artifact.is_some());
        let plan = context.plan_artifact.unwrap();
        assert_eq!(plan.title, "Implementation Plan");
        assert!(plan.content_preview.len() <= 503); // 500 + "..."
        assert!(context.context_hints.iter().any(|h| h.contains("plan")));
    }

    #[tokio::test]
    async fn test_get_task_context_with_related_artifacts() {
        let mut task = Task::new(ProjectId::new(), "Test Task".to_string());
        let artifact_id = ArtifactId::new();
        task.plan_artifact_id = Some(artifact_id.clone());
        let task_id = task.id.clone();

        let artifact = Artifact::new_inline(
            "Implementation Plan",
            ArtifactType::Specification,
            "Plan content",
            "user",
        );

        let related1 =
            Artifact::new_inline("Research Doc", ArtifactType::ResearchDocument, "Research", "user");
        let related2 =
            Artifact::new_inline("Design Doc", ArtifactType::DesignDoc, "Design", "user");

        let service = TaskContextService::new(
            Arc::new(MockTaskRepository::with_task(task.clone())),
            Arc::new(MockTaskProposalRepository::empty()),
            Arc::new(MockArtifactRepository::with_related(
                artifact,
                vec![related1, related2],
            )),
        );

        let context = service.get_task_context(&task_id).await.unwrap();

        assert_eq!(context.related_artifacts.len(), 2);
        assert!(context
            .context_hints
            .iter()
            .any(|h| h.contains("2 related artifacts")));
    }

    #[tokio::test]
    async fn test_content_preview_truncation() {
        let short_content = "Short content";
        let artifact = Artifact::new_inline(
            "Test",
            ArtifactType::Specification,
            short_content,
            "user",
        );
        let preview = TaskContextService::create_content_preview(&artifact);
        assert_eq!(preview, short_content);

        let long_content = "x".repeat(600);
        let artifact = Artifact::new_inline("Test", ArtifactType::Specification, long_content, "user");
        let preview = TaskContextService::create_content_preview(&artifact);
        assert_eq!(preview.len(), 503); // 500 + "..."
        assert!(preview.ends_with("..."));
    }

    #[tokio::test]
    async fn test_task_not_found() {
        let service = TaskContextService::new(
            Arc::new(MockTaskRepository { task: None }),
            Arc::new(MockTaskProposalRepository::empty()),
            Arc::new(MockArtifactRepository::empty()),
        );

        let result = service.get_task_context(&TaskId::new()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }
}
