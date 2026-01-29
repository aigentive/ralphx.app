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
    ArtifactSummary, Task, TaskContext, TaskId, TaskProposalSummary, StepProgressSummary,
};
use crate::domain::repositories::{ArtifactRepository, TaskProposalRepository, TaskRepository, TaskStepRepository};
use crate::error::{AppError, AppResult};

/// Service for aggregating task context for worker execution
pub struct TaskContextService {
    task_repo: Arc<dyn TaskRepository>,
    proposal_repo: Arc<dyn TaskProposalRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
    step_repo: Arc<dyn TaskStepRepository>,
}

impl TaskContextService
{
    /// Create a new TaskContextService with the given repositories
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        proposal_repo: Arc<dyn TaskProposalRepository>,
        artifact_repo: Arc<dyn ArtifactRepository>,
        step_repo: Arc<dyn TaskStepRepository>,
    ) -> Self {
        Self {
            task_repo,
            proposal_repo,
            artifact_repo,
            step_repo,
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

        // 5. Fetch steps for the task
        let steps = self.step_repo.get_by_task(task_id).await?;

        // 6. Calculate step progress summary if steps exist
        let step_progress = if !steps.is_empty() {
            Some(StepProgressSummary::from_steps(task_id, &steps))
        } else {
            None
        };

        // 7. Generate context_hints based on what's available
        let context_hints = self.generate_context_hints(
            &task,
            source_proposal.is_some(),
            plan_artifact.is_some(),
            related_artifacts.len(),
            steps.len(),
        );

        // 8. Return TaskContext
        Ok(TaskContext {
            task,
            source_proposal,
            plan_artifact,
            related_artifacts,
            steps,
            step_progress,
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
        step_count: usize,
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

        if step_count > 0 {
            hints.push(format!(
                "Task has {} step{} defined - use get_task_steps to see them",
                step_count,
                if step_count == 1 { "" } else { "s" }
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
        TaskStep, TaskStepId,
    };
    use crate::domain::repositories::{ArtifactRepository, TaskProposalRepository, TaskRepository, TaskStepRepository};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

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
            _statuses: Option<Vec<InternalStatus>>,
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

        async fn search(
            &self,
            _project_id: &ProjectId,
            _query: &str,
            _include_archived: bool,
        ) -> AppResult<Vec<Task>> {
            Ok(vec![])
        }

        async fn get_oldest_ready_task(&self) -> AppResult<Option<Task>> {
            Ok(None)
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

    struct MockTaskStepRepository {
        steps: Mutex<HashMap<String, TaskStep>>,
    }

    impl MockTaskStepRepository {
        fn empty() -> Self {
            Self {
                steps: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl TaskStepRepository for MockTaskStepRepository {
        async fn create(&self, step: TaskStep) -> AppResult<TaskStep> {
            self.steps
                .lock()
                .unwrap()
                .insert(step.id.to_string(), step.clone());
            Ok(step)
        }

        async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>> {
            Ok(self.steps.lock().unwrap().get(&id.to_string()).cloned())
        }

        async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>> {
            let mut steps: Vec<_> = self
                .steps
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.task_id == task_id)
                .cloned()
                .collect();
            steps.sort_by_key(|s| s.sort_order);
            Ok(steps)
        }

        async fn get_by_task_and_status(
            &self,
            task_id: &TaskId,
            status: crate::domain::entities::TaskStepStatus,
        ) -> AppResult<Vec<TaskStep>> {
            let mut steps: Vec<_> = self
                .steps
                .lock()
                .unwrap()
                .values()
                .filter(|s| &s.task_id == task_id && s.status == status)
                .cloned()
                .collect();
            steps.sort_by_key(|s| s.sort_order);
            Ok(steps)
        }

        async fn update(&self, step: &TaskStep) -> AppResult<()> {
            self.steps
                .lock()
                .unwrap()
                .insert(step.id.to_string(), step.clone());
            Ok(())
        }

        async fn delete(&self, id: &TaskStepId) -> AppResult<()> {
            self.steps.lock().unwrap().remove(&id.to_string());
            Ok(())
        }

        async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()> {
            self.steps
                .lock()
                .unwrap()
                .retain(|_, s| &s.task_id != task_id);
            Ok(())
        }

        async fn count_by_status(
            &self,
            task_id: &TaskId,
        ) -> AppResult<HashMap<crate::domain::entities::TaskStepStatus, u32>> {
            let steps = self.get_by_task(task_id).await?;
            let mut counts = HashMap::new();
            for step in steps {
                *counts.entry(step.status).or_insert(0) += 1;
            }
            Ok(counts)
        }

        async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>> {
            for step in &steps {
                self.steps
                    .lock()
                    .unwrap()
                    .insert(step.id.to_string(), step.clone());
            }
            Ok(steps)
        }

        async fn reorder(&self, _task_id: &TaskId, _step_ids: Vec<TaskStepId>) -> AppResult<()> {
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
            Arc::new(MockTaskStepRepository::empty()),
        );

        let context = service.get_task_context(&task_id).await.unwrap();

        assert_eq!(context.task.id, task_id);
        assert!(context.source_proposal.is_none());
        assert!(context.plan_artifact.is_none());
        assert_eq!(context.related_artifacts.len(), 0);
        assert_eq!(context.steps.len(), 0);
        assert!(context.step_progress.is_none());
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
            Arc::new(MockTaskStepRepository::empty()),
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
            Arc::new(MockTaskStepRepository::empty()),
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
            Arc::new(MockTaskStepRepository::empty()),
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
            Arc::new(MockTaskStepRepository::empty()),
        );

        let result = service.get_task_context(&TaskId::new()).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }
}
