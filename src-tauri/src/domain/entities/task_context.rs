use super::{ArtifactId, ArtifactType, InternalStatus, Task, TaskId, TaskProposalId, TaskStep};
use serde::{Deserialize, Serialize};

use super::task_step::StepProgressSummary;

/// Rich context returned by get_task_context MCP tool
/// Contains the task being executed along with linked artifacts and proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// The task being executed
    pub task: Task,

    /// Source proposal if task was created from ideation
    pub source_proposal: Option<TaskProposalSummary>,

    /// Implementation plan artifact (summary, not full content)
    pub plan_artifact: Option<ArtifactSummary>,

    /// Other artifacts related to the plan
    pub related_artifacts: Vec<ArtifactSummary>,

    /// Steps defined for this task (for progress tracking)
    pub steps: Vec<TaskStep>,

    /// Progress summary for the task's steps
    pub step_progress: Option<StepProgressSummary>,

    /// Hints for worker about what context might be useful
    pub context_hints: Vec<String>,

    /// Tasks that must complete before this task can start (blockers)
    /// If not empty, the worker should NOT proceed with execution
    pub blocked_by: Vec<TaskDependencySummary>,

    /// Tasks that are waiting for this task to complete (dependents)
    /// For context: completing this task may unblock these downstream tasks
    pub blocks: Vec<TaskDependencySummary>,

    /// Execution tier from dependency graph (lower = earlier in chain)
    /// Tier 1 tasks have no blockers, higher tiers depend on lower tiers
    pub tier: Option<u32>,
}

/// Summary of a task proposal for context purposes
/// Excludes fields not relevant for worker context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(test, derive(Default))]
pub struct TaskProposalSummary {
    pub id: TaskProposalId,
    pub title: String,
    pub description: String,
    pub acceptance_criteria: Vec<String>,
    pub implementation_notes: Option<String>,
    /// Version of plan when proposal was created
    pub plan_version_at_creation: Option<u32>,
    /// Numeric priority score (0-100, higher = more important)
    pub priority_score: i32,
}

/// Summary of a task for dependency context (blocker or dependent)
/// Contains minimal info needed to understand the dependency relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDependencySummary {
    pub id: TaskId,
    pub title: String,
    pub internal_status: InternalStatus,
}

/// Summary of an artifact for context purposes
/// Includes preview but not full content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArtifactSummary {
    pub id: ArtifactId,
    pub title: String,
    pub artifact_type: ArtifactType,
    pub current_version: u32,
    /// First ~500 chars of content as preview
    pub content_preview: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ArtifactId, InternalStatus, ProjectId, Task, TaskId, TaskProposalId};

    #[test]
    fn test_task_context_creation() {
        let task = Task::new(ProjectId::new(), "Test Task".to_string());

        let context = TaskContext {
            task: task.clone(),
            source_proposal: None,
            plan_artifact: None,
            related_artifacts: vec![],
            steps: vec![],
            step_progress: None,
            context_hints: vec!["No additional context available".to_string()],
            blocked_by: vec![],
            blocks: vec![],
            tier: None,
        };

        assert_eq!(context.task.id, task.id);
        assert!(context.source_proposal.is_none());
        assert!(context.plan_artifact.is_none());
        assert_eq!(context.related_artifacts.len(), 0);
        assert_eq!(context.steps.len(), 0);
        assert!(context.step_progress.is_none());
        assert_eq!(context.context_hints.len(), 1);
        assert!(context.blocked_by.is_empty());
        assert!(context.blocks.is_empty());
        assert!(context.tier.is_none());
    }

    #[test]
    fn test_task_proposal_summary_creation() {
        let summary = TaskProposalSummary {
            id: TaskProposalId::new(),
            title: "Test Proposal".to_string(),
            description: "Proposal description".to_string(),
            acceptance_criteria: vec!["AC1".to_string(), "AC2".to_string()],
            implementation_notes: Some("Notes here".to_string()),
            plan_version_at_creation: Some(1),
            priority_score: 75,
        };

        assert_eq!(summary.title, "Test Proposal");
        assert_eq!(summary.acceptance_criteria.len(), 2);
        assert!(summary.implementation_notes.is_some());
        assert_eq!(summary.plan_version_at_creation, Some(1));
        assert_eq!(summary.priority_score, 75);
    }

    #[test]
    fn test_task_dependency_summary_creation() {
        let summary = TaskDependencySummary {
            id: TaskId::new(),
            title: "Blocker Task".to_string(),
            internal_status: InternalStatus::Executing,
        };

        assert_eq!(summary.title, "Blocker Task");
        assert_eq!(summary.internal_status, InternalStatus::Executing);
    }

    #[test]
    fn test_artifact_summary_creation() {
        let summary = ArtifactSummary {
            id: ArtifactId::new(),
            title: "Implementation Plan".to_string(),
            artifact_type: ArtifactType::Specification,
            current_version: 2,
            content_preview: "This is a preview of the artifact content...".to_string(),
        };

        assert_eq!(summary.title, "Implementation Plan");
        assert_eq!(summary.artifact_type, ArtifactType::Specification);
        assert_eq!(summary.current_version, 2);
        assert!(!summary.content_preview.is_empty());
    }

    #[test]
    fn test_task_context_with_full_context() {
        let mut task = Task::new(ProjectId::new(), "Complex Task".to_string());
        task.set_description(Some("Task with full context".to_string()));
        task.set_priority(10);
        task.internal_status = InternalStatus::Executing;
        task.source_proposal_id = Some(TaskProposalId::new());
        task.plan_artifact_id = Some(ArtifactId::new());

        let proposal_summary = TaskProposalSummary {
            id: task.source_proposal_id.clone().unwrap(),
            title: "Original Proposal".to_string(),
            description: "Proposal description".to_string(),
            acceptance_criteria: vec!["AC1".to_string()],
            implementation_notes: Some("Follow pattern X".to_string()),
            plan_version_at_creation: Some(1),
            priority_score: 80,
        };

        let plan_summary = ArtifactSummary {
            id: task.plan_artifact_id.clone().unwrap(),
            title: "Implementation Plan".to_string(),
            artifact_type: ArtifactType::Specification,
            current_version: 1,
            content_preview: "# Implementation Plan\n\nThis plan describes...".to_string(),
        };

        let related_artifact = ArtifactSummary {
            id: ArtifactId::new(),
            title: "Research Document".to_string(),
            artifact_type: ArtifactType::ResearchDocument,
            current_version: 1,
            content_preview: "Research findings...".to_string(),
        };

        // Create blocker and dependent tasks for testing dependency context
        let blocker_task = TaskDependencySummary {
            id: TaskId::new(),
            title: "Setup Database".to_string(),
            internal_status: InternalStatus::Approved,
        };

        let dependent_task = TaskDependencySummary {
            id: TaskId::new(),
            title: "Add UI Components".to_string(),
            internal_status: InternalStatus::Blocked,
        };

        let context = TaskContext {
            task: task.clone(),
            source_proposal: Some(proposal_summary.clone()),
            plan_artifact: Some(plan_summary.clone()),
            related_artifacts: vec![related_artifact],
            steps: vec![],
            step_progress: None,
            context_hints: vec![
                "Implementation plan available".to_string(),
                "Related research document found".to_string(),
            ],
            blocked_by: vec![blocker_task],
            blocks: vec![dependent_task],
            tier: Some(2),
        };

        assert_eq!(context.task.id, task.id);
        assert!(context.source_proposal.is_some());
        assert_eq!(
            context.source_proposal.unwrap().title,
            "Original Proposal"
        );
        assert!(context.plan_artifact.is_some());
        assert_eq!(
            context.plan_artifact.unwrap().title,
            "Implementation Plan"
        );
        assert_eq!(context.related_artifacts.len(), 1);
        assert_eq!(context.steps.len(), 0);
        assert!(context.step_progress.is_none());
        assert_eq!(context.context_hints.len(), 2);
        assert_eq!(context.blocked_by.len(), 1);
        assert_eq!(context.blocked_by[0].title, "Setup Database");
        assert_eq!(context.blocks.len(), 1);
        assert_eq!(context.blocks[0].title, "Add UI Components");
        assert_eq!(context.tier, Some(2));
    }

    #[test]
    fn test_serialization() {
        let summary = ArtifactSummary {
            id: ArtifactId::new(),
            title: "Test".to_string(),
            artifact_type: ArtifactType::Specification,
            current_version: 1,
            content_preview: "Preview".to_string(),
        };

        // Test that serialization works
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("Test"));
        assert!(json.contains("Preview"));

        // Test that deserialization works
        let deserialized: ArtifactSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, summary.title);
        assert_eq!(deserialized.artifact_type, summary.artifact_type);
    }
}
