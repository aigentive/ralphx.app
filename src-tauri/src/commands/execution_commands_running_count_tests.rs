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

/// Register an ideation entry in the registry with a session ID.
async fn register_ideation(registry: &dyn RunningAgentRegistry, session_id: &str) {
    let key = RunningAgentKey::new("ideation", session_id);
    registry
        .register(
            key,
            9999, // fake pid
            "conv-ideation".to_string(),
            "run-ideation".to_string(),
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

        if !uses_execution_slot(context_type) {
            continue;
        }

        // Ideation uses session IDs (not task IDs) — no task lookup or GC needed.
        if matches!(context_type, ChatContextType::Ideation) {
            running_count += 1;
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
/// survive the guard (task-based contexts only, excludes ideation).
async fn count_visible_processes(app_state: &AppState) -> usize {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut count = 0usize;

    for (key, _) in registry_entries {
        let context_type = match ChatContextType::from_str(&key.context_type) {
            Ok(value) => value,
            Err(_) => continue,
        };

        // Only task-based execution contexts appear in the process list
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

// =========================================================================
// Ideation: counted in running count but excluded from process list
// =========================================================================

#[tokio::test]
async fn test_ideation_entry_counted_in_running_count() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    register_ideation(&*state.running_agent_registry, "session-abc").await;

    let count = count_running(&state).await;
    assert_eq!(count, 1, "Ideation entry must be counted as running");
}

#[tokio::test]
async fn test_ideation_entry_excluded_from_process_list() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    register_ideation(&*state.running_agent_registry, "session-abc").await;

    let process_count = count_visible_processes(&state).await;
    assert_eq!(
        process_count, 0,
        "Ideation entry must NOT appear in process list"
    );
}

#[tokio::test]
async fn test_mixed_executing_task_and_ideation() {
    // 1 executing task + 1 ideation session → count=2, process list=1
    let (state, task) = setup_with_task(InternalStatus::Executing).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;
    register_ideation(&*state.running_agent_registry, "session-xyz").await;

    let entries = state.running_agent_registry.list_all().await;
    assert_eq!(entries.len(), 2, "registry should have 2 entries");

    let running_count = count_running(&state).await;
    let process_count = count_visible_processes(&state).await;

    assert_eq!(
        running_count, 2,
        "Both executing task and ideation must be counted"
    );
    assert_eq!(
        process_count, 1,
        "Only the executing task should appear in process list"
    );
}

#[tokio::test]
async fn test_ideation_not_affected_by_failed_task_regression() {
    // 1 ideation + 1 failed task (stale) → count=1, process list=0
    let (state, task) = setup_with_task(InternalStatus::Failed).await;
    register_task_execution(&*state.running_agent_registry, &task.id).await;
    register_ideation(&*state.running_agent_registry, "session-live").await;

    let running_count = count_running(&state).await;
    let process_count = count_visible_processes(&state).await;

    assert_eq!(
        running_count, 1,
        "Only ideation should be counted (failed task excluded)"
    );
    assert_eq!(
        process_count, 0,
        "Failed task must not appear in process list"
    );
}

// =========================================================================
// uses_execution_slot: ideation IS an execution slot context
// =========================================================================

#[test]
fn test_uses_execution_slot_includes_ideation() {
    assert!(
        uses_execution_slot(ChatContextType::Ideation),
        "Ideation must use an execution slot"
    );
    assert!(uses_execution_slot(ChatContextType::TaskExecution));
    assert!(uses_execution_slot(ChatContextType::Review));
    assert!(uses_execution_slot(ChatContextType::Merge));
    assert!(
        !uses_execution_slot(ChatContextType::Task),
        "Task chat must NOT use an execution slot"
    );
    assert!(
        !uses_execution_slot(ChatContextType::Project),
        "Project chat must NOT use an execution slot"
    );
}
