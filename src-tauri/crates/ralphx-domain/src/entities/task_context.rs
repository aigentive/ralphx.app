use chrono::{DateTime, Utc};

use super::{ArtifactId, ArtifactType, InternalStatus, Task, TaskId, TaskProposalId, TaskStep};
use serde::{Deserialize, Serialize};

use super::task_step::StepProgressSummary;

/// Backend-computed comparison between planned coarse scope and actual changed files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeDriftStatus {
    /// No reliable planned scope or diff context was available.
    Unbounded,
    /// Actual changed files stayed within the declared coarse scope.
    WithinScope,
    /// Actual changed files expanded beyond the declared coarse scope.
    ScopeExpansion,
}

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

    /// Git branch assigned to this task (if git isolation is active).
    /// Agents MUST work only on this branch — do not checkout other branches.
    pub task_branch: Option<String>,

    /// Worktree path for this task (Worktree git mode only).
    /// When set, agents should work exclusively within this directory.
    pub worktree_path: Option<String>,

    /// Validation cache from last execution (if available).
    /// Backend pre-computes a validation_hint so agents don't compare SHAs themselves.
    pub validation_cache: Option<ValidationCacheData>,

    /// Actual changed files between the task branch/worktree and its review base.
    pub actual_changed_files: Vec<String>,

    /// Backend-computed scope drift status for reviewers/agents.
    pub scope_drift_status: ScopeDriftStatus,

    /// Changed files outside the proposal's declared coarse scope.
    pub out_of_scope_files: Vec<String>,
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
    /// Coarse planned file/path scope hints captured during ideation.
    pub affected_paths: Vec<String>,
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

/// Lightweight validation cache view for TaskContext responses.
/// Subset of ValidationCacheMetadata with a pre-computed hint so agents
/// never need to compare SHAs themselves — they follow the hint only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValidationCacheData {
    /// HEAD commit SHA when cache was captured
    pub commit_sha: String,
    /// Whether any tests were run during execution
    pub tests_ran: bool,
    /// Whether all tests passed (only meaningful when tests_ran=true)
    pub tests_passed: bool,
    /// Brief test result summary (e.g., "6758 passed, 0 failed")
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub test_summary: Option<String>,
    /// When this cache entry was captured
    pub captured_at: DateTime<Utc>,
    /// Pre-computed hint for agents:
    /// "skip_tests"         — tests passed on same SHA, skip test re-run
    /// "skip_test_validation" — no tests existed at execution time
    /// "run_tests"          — SHA mismatch, cache missing, or tests failed
    pub validation_hint: String,
    /// Human-readable explanation of the hint
    pub hint_message: String,
}

#[cfg(test)]
#[path = "task_context_tests.rs"]
mod tests;
