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
    ArtifactSummary, StepProgressSummary, Task, TaskContext, TaskDependencySummary, TaskId,
    TaskProposalSummary,
};
use crate::domain::repositories::{
    ArtifactRepository, TaskDependencyRepository, TaskProposalRepository, TaskRepository,
    TaskStepRepository,
};
use crate::error::{AppError, AppResult};

/// Service for aggregating task context for worker execution
pub struct TaskContextService {
    task_repo: Arc<dyn TaskRepository>,
    task_dependency_repo: Arc<dyn TaskDependencyRepository>,
    proposal_repo: Arc<dyn TaskProposalRepository>,
    artifact_repo: Arc<dyn ArtifactRepository>,
    step_repo: Arc<dyn TaskStepRepository>,
}

impl TaskContextService {
    /// Create a new TaskContextService with the given repositories
    pub fn new(
        task_repo: Arc<dyn TaskRepository>,
        task_dependency_repo: Arc<dyn TaskDependencyRepository>,
        proposal_repo: Arc<dyn TaskProposalRepository>,
        artifact_repo: Arc<dyn ArtifactRepository>,
        step_repo: Arc<dyn TaskStepRepository>,
    ) -> Self {
        Self {
            task_repo,
            task_dependency_repo,
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
                        priority_score: proposal.priority_score,
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

        // 7. Fetch task dependencies (blockers and dependents) via TaskDependencyRepository
        let blocker_ids = self.task_dependency_repo.get_blockers(task_id).await?;
        let mut blocked_by: Vec<TaskDependencySummary> = Vec::new();
        for blocker_id in &blocker_ids {
            if let Some(blocker_task) = self.task_repo.get_by_id(blocker_id).await? {
                blocked_by.push(TaskDependencySummary {
                    id: blocker_task.id.clone(),
                    title: blocker_task.title.clone(),
                    internal_status: blocker_task.internal_status,
                });
            }
        }

        let dependent_ids = self.task_dependency_repo.get_blocked_by(task_id).await?;
        let mut blocks: Vec<TaskDependencySummary> = Vec::new();
        for dep_id in &dependent_ids {
            if let Some(dep_task) = self.task_repo.get_by_id(dep_id).await? {
                blocks.push(TaskDependencySummary {
                    id: dep_task.id.clone(),
                    title: dep_task.title.clone(),
                    internal_status: dep_task.internal_status,
                });
            }
        }

        // 8. Compute tier from dependency depth
        // Tier 1 = no blockers, Tier N = depends on tasks in tier N-1
        // For now, use simple heuristic: tier = number of incomplete blockers + 1
        let tier = if blocked_by.is_empty() {
            Some(1)
        } else {
            // Count how many blockers are not yet completed
            let incomplete_blockers = blocked_by
                .iter()
                .filter(|b| {
                    !matches!(
                        b.internal_status,
                        crate::domain::entities::InternalStatus::Approved
                    )
                })
                .count();
            Some((incomplete_blockers as u32) + 1)
        };

        // 9. Generate context_hints based on what's available
        let context_hints = self.generate_context_hints(
            &task,
            source_proposal.is_some(),
            plan_artifact.is_some(),
            related_artifacts.len(),
            steps.len(),
            &blocked_by,
            &blocks,
        );

        // 10. Return TaskContext
        let task_branch = task.task_branch.clone();
        let worktree_path = task.worktree_path.clone();
        Ok(TaskContext {
            task,
            source_proposal,
            plan_artifact,
            related_artifacts,
            steps,
            step_progress,
            context_hints,
            blocked_by,
            blocks,
            tier,
            task_branch,
            worktree_path,
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

        // Take first 500 chars (char-boundary safe)
        if full_content.chars().count() <= 500 {
            full_content
        } else {
            let truncated: String = full_content.chars().take(500).collect();
            format!("{truncated}...")
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
        blocked_by: &[TaskDependencySummary],
        blocks: &[TaskDependencySummary],
    ) -> Vec<String> {
        let mut hints = Vec::new();

        // CRITICAL: Dependency hints come first - worker must check these before starting
        if !blocked_by.is_empty() {
            let incomplete: Vec<_> = blocked_by
                .iter()
                .filter(|b| {
                    !matches!(
                        b.internal_status,
                        crate::domain::entities::InternalStatus::Approved
                    )
                })
                .collect();
            if !incomplete.is_empty() {
                let names: Vec<_> = incomplete.iter().map(|t| t.title.as_str()).collect();
                hints.push(format!(
                    "BLOCKED: Task cannot proceed - waiting for: {}",
                    names.join(", ")
                ));
            } else {
                hints.push("All blocking tasks completed - ready to execute".to_string());
            }
        }

        if !blocks.is_empty() {
            let names: Vec<_> = blocks.iter().map(|t| t.title.as_str()).collect();
            hints.push(format!(
                "Downstream impact: completing this task unblocks: {}",
                names.join(", ")
            ));
        }

        // CRITICAL: Branch safety hint — agents must stay on their assigned branch
        if let Some(ref branch) = task.task_branch {
            hints.push(format!(
                "GIT BRANCH: You are on branch '{}'. Do NOT checkout other branches (especially main/master). All work must stay on this branch.",
                branch
            ));
        }

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
#[path = "task_context_service_tests.rs"]
mod tests;
