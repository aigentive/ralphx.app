// Tests for merge_helpers: task_targets_branch + is_task_in_merge_workflow
//
// Extracted from side_effects.rs (lines 6713–6809).

use super::helpers::*;
use super::super::merge_helpers::{is_task_in_merge_workflow, task_targets_branch};
use crate::domain::entities::{InternalStatus, PlanBranchStatus, TaskId};
use crate::domain::repositories::{PlanBranchRepository, TaskRepository};
use crate::infrastructure::memory::{MemoryPlanBranchRepository, MemoryTaskRepository};
use std::sync::Arc;

// ==================
// task_targets_branch tests
// ==================

#[tokio::test]
async fn task_targets_branch_returns_true_for_matching_target() {
    let project = make_project(Some("main"));
    let mut task = make_task(None, Some("ralphx/test/task-123"));
    task.id = TaskId::from_string("task-123".to_string());

    let repo: Option<Arc<dyn PlanBranchRepository>> = None;
    // A standalone task merges into project base branch (main)
    assert!(task_targets_branch(&task, &project, &repo, "main").await);
}

#[tokio::test]
async fn task_targets_branch_returns_false_for_non_matching_target() {
    let project = make_project(Some("main"));
    let mut task = make_task(None, Some("ralphx/test/task-123"));
    task.id = TaskId::from_string("task-123".to_string());

    let repo: Option<Arc<dyn PlanBranchRepository>> = None;
    assert!(!task_targets_branch(&task, &project, &repo, "develop").await);
}

#[tokio::test]
async fn task_targets_branch_plan_task_targets_feature_branch() {
    let project = make_project(Some("main"));
    let mut task =
        make_task_with_session(Some("art-1"), Some("ralphx/test/task-456"), Some("sess-1"));
    task.id = TaskId::from_string("task-456".to_string());

    let mem_repo = Arc::new(MemoryPlanBranchRepository::new());
    let pb = make_plan_branch(
        "art-1",
        "ralphx/test/plan-abc123",
        PlanBranchStatus::Active,
        None,
    );
    mem_repo.create(pb).await.unwrap();

    let repo: Option<Arc<dyn PlanBranchRepository>> = Some(mem_repo);
    // Plan task merges into feature branch, not main
    assert!(task_targets_branch(&task, &project, &repo, "ralphx/test/plan-abc123").await);
    assert!(!task_targets_branch(&task, &project, &repo, "main").await);
}

// ==================
// is_task_in_merge_workflow tests
// ==================

#[tokio::test]
async fn test_is_task_in_merge_workflow_pending_merge() {
    let task = make_task_with_status("task-1", InternalStatus::PendingMerge);
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
    assert!(is_task_in_merge_workflow(&repo, "task-1").await);
}

#[tokio::test]
async fn test_is_task_in_merge_workflow_merging() {
    let task = make_task_with_status("task-1", InternalStatus::Merging);
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
    assert!(is_task_in_merge_workflow(&repo, "task-1").await);
}

#[tokio::test]
async fn test_is_task_in_merge_workflow_merge_incomplete() {
    let task = make_task_with_status("task-1", InternalStatus::MergeIncomplete);
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
    assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
}

#[tokio::test]
async fn test_is_task_in_merge_workflow_merge_conflict() {
    let task = make_task_with_status("task-1", InternalStatus::MergeConflict);
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
    assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
}

#[tokio::test]
async fn test_is_task_in_merge_workflow_executing_returns_false() {
    let task = make_task_with_status("task-1", InternalStatus::Executing);
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::with_tasks(vec![task]));
    assert!(!is_task_in_merge_workflow(&repo, "task-1").await);
}

#[tokio::test]
async fn test_is_task_in_merge_workflow_nonexistent_task() {
    let repo: Arc<dyn TaskRepository> = Arc::new(MemoryTaskRepository::new());
    assert!(!is_task_in_merge_workflow(&repo, "nonexistent-id").await);
}
