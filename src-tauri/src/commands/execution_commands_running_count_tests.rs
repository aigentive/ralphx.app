// Regression tests for running count vs process list mismatch bug (a4f4fa6d).
//
// Bug: `get_execution_status` counted registry entries without checking task
// status, while `get_running_processes` filtered via
// `context_matches_running_status_for_gc`. A Failed task with a stale registry
// entry inflated the "Running: 2/10" count while only 1 task was visible.

use super::*;
use crate::application::AppState;
use crate::domain::entities::{InternalStatus, Project, Task, TaskId};
use crate::domain::repositories::{ProjectRepository, TaskRepository};
use crate::domain::services::{RunningAgentKey, RunningAgentRegistry};
use crate::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};
use std::sync::Arc;

/// Shared helper: create an AppState with memory repos and a fresh project+task.
/// Returns (app_state, task) with the task already persisted in the given status.
async fn setup_with_task(status: InternalStatus) -> (AppState, Task) {
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project = Project::new("Test Project".to_string(), "/test/path".to_string());
    let pid = project.id.clone();
    project_repo.create(project).await.unwrap();

    let mut task = Task::new(pid, "Test Task".to_string());
    task.internal_status = status;
    let task = task_repo.create(task).await.unwrap();

    let state = AppState::with_repos(task_repo, project_repo);
    (state, task)
}

/// Register a task_execution entry in the registry for the given task.
async fn register_task_execution(registry: &dyn RunningAgentRegistry, task_id: &TaskId) {
    let key = RunningAgentKey::new("task_execution", task_id.as_str());
    registry
        .register(
            key,
            9999, // fake pid
            "conv-test".to_string(),
            "run-test".to_string(),
            None,
            None,
        )
        .await;
}

/// Replicate the counting logic from `get_execution_status` so we can
/// verify it without needing Tauri `State<'_>` wrappers.
async fn count_running(app_state: &AppState) -> u32 {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut running_count = 0u32;

    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !matches!(
            context_type,
            ChatContextType::TaskExecution | ChatContextType::Review | ChatContextType::Merge
        ) {
            continue;
        }

        let task_id = TaskId::from_string(key.context_id);
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if !context_matches_running_status_for_gc(context_type, task.internal_status) {
            continue;
        }

        running_count += 1;
    }

    running_count
}

/// Same filter used by `get_running_processes` — returns how many entries
/// survive the guard.
async fn count_visible_processes(app_state: &AppState) -> usize {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut count = 0usize;

    for (key, _) in registry_entries {
        if !is_execution_context_type(&key.context_type) {
            continue;
        }

        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let task_id = TaskId::from_string(key.context_id);
        let task = match app_state.task_repo.get_by_id(&task_id).await {
            Ok(Some(task)) => task,
            _ => continue,
        };

        if !context_matches_running_status_for_gc(context_type, task.internal_status) {
            continue;
        }

        count += 1;
    }

    count
}

// =========================================================================
// Regression: running count must exclude Failed tasks with stale registry
// =========================================================================

#[tokio::test]
async fn test_running_count_excludes_failed_task_with_stale_registry() {
    let (state, task) = setup_with_task(InternalStatus::Failed).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;

    // Registry has 1 entry, but the task is Failed → count must be 0.
    let entries = state.running_agent_registry.list_all().await;
    assert_eq!(entries.len(), 1, "registry should have 1 entry");

    let count = count_running(&state).await;
    assert_eq!(count, 0, "Failed task must not be counted as running");
}

// =========================================================================
// Positive: Executing task with registry entry IS counted
// =========================================================================

#[tokio::test]
async fn test_running_count_includes_executing_task() {
    let (state, task) = setup_with_task(InternalStatus::Executing).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;

    let count = count_running(&state).await;
    assert_eq!(count, 1, "Executing task must be counted as running");
}

// =========================================================================
// Negative: terminal/completed statuses must not be counted
// =========================================================================

#[tokio::test]
async fn test_running_count_excludes_completed_task() {
    // Merged is a terminal "completed" status
    let (state, task) = setup_with_task(InternalStatus::Merged).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;

    let count = count_running(&state).await;
    assert_eq!(count, 0, "Merged task must not be counted as running");
}

#[tokio::test]
async fn test_running_count_excludes_approved_task() {
    let (state, task) = setup_with_task(InternalStatus::Approved).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;

    let count = count_running(&state).await;
    assert_eq!(count, 0, "Approved task must not be counted as running");
}

// =========================================================================
// Core regression: running count MUST equal process list count
// =========================================================================

#[tokio::test]
async fn test_running_count_matches_process_list_count() {
    // Mixed scenario: 1 Executing + 1 Failed (stale registry entry)
    let task_repo = Arc::new(MemoryTaskRepository::new());
    let project_repo = Arc::new(MemoryProjectRepository::new());

    let project = Project::new("Mixed Project".to_string(), "/test/mixed".to_string());
    let pid = project.id.clone();
    project_repo.create(project).await.unwrap();

    // Task A: genuinely executing
    let mut task_a = Task::new(pid.clone(), "Executing Task".to_string());
    task_a.internal_status = InternalStatus::Executing;
    let task_a = task_repo.create(task_a).await.unwrap();

    // Task B: failed but has stale registry entry (the bug scenario)
    let mut task_b = Task::new(pid, "Failed Task".to_string());
    task_b.internal_status = InternalStatus::Failed;
    let task_b = task_repo.create(task_b).await.unwrap();

    let state = AppState::with_repos(task_repo, project_repo);

    register_task_execution(&*state.running_agent_registry, &task_a.id).await;
    register_task_execution(&*state.running_agent_registry, &task_b.id).await;

    // Registry has 2 entries
    let entries = state.running_agent_registry.list_all().await;
    assert_eq!(entries.len(), 2, "registry should have 2 entries");

    let running_count = count_running(&state).await;
    let process_count = count_visible_processes(&state).await;

    assert_eq!(
        running_count, 1,
        "only the Executing task should be counted"
    );
    assert_eq!(
        running_count as usize, process_count,
        "running count must match visible process count"
    );
}

// =========================================================================
// Guard function unit tests: context_matches_running_status_for_gc
// =========================================================================

#[test]
fn test_guard_task_execution_only_matches_executing_or_reexecuting() {
    let ctx = ChatContextType::TaskExecution;
    assert!(context_matches_running_status_for_gc(ctx, InternalStatus::Executing));
    assert!(context_matches_running_status_for_gc(ctx, InternalStatus::ReExecuting));

    // Every other status must NOT match
    for status in [
        InternalStatus::Backlog,
        InternalStatus::Ready,
        InternalStatus::Failed,
        InternalStatus::Cancelled,
        InternalStatus::Merged,
        InternalStatus::Approved,
        InternalStatus::PendingReview,
        InternalStatus::Reviewing,
        InternalStatus::Merging,
        InternalStatus::Stopped,
        InternalStatus::Paused,
    ] {
        assert!(
            !context_matches_running_status_for_gc(ctx, status),
            "TaskExecution context should NOT match {status:?}"
        );
    }
}

#[test]
fn test_guard_review_only_matches_reviewing() {
    let ctx = ChatContextType::Review;
    assert!(context_matches_running_status_for_gc(ctx, InternalStatus::Reviewing));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::PendingReview));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::Executing));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::Failed));
}

#[test]
fn test_guard_merge_only_matches_merging() {
    let ctx = ChatContextType::Merge;
    assert!(context_matches_running_status_for_gc(ctx, InternalStatus::Merging));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::PendingMerge));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::Merged));
    assert!(!context_matches_running_status_for_gc(ctx, InternalStatus::Executing));
}

#[test]
fn test_guard_non_execution_contexts_always_false() {
    for ctx in [
        ChatContextType::Task,
        ChatContextType::Ideation,
        ChatContextType::Project,
    ] {
        assert!(
            !context_matches_running_status_for_gc(ctx, InternalStatus::Executing),
            "{ctx:?} should never match any status"
        );
    }
}
