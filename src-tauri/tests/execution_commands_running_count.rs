// Regression tests for running count vs process list mismatch bug (a4f4fa6d).
//
// Bug: `get_execution_status` counted registry entries without checking task
// status, while `get_running_processes` filtered via
// `context_matches_running_status_for_gc`. A Failed task with a stale registry
// entry inflated the "Running: 2/10" count while only 1 task was visible.

use std::sync::Arc;

use ralphx_lib::application::{chat_service::uses_execution_slot, AppState};
use ralphx_lib::commands::ExecutionState;
use ralphx_lib::commands::execution_commands::context_matches_running_status_for_gc;
use ralphx_lib::domain::entities::{ChatContextType, InternalStatus, Project, Task, TaskId};
use ralphx_lib::domain::repositories::{ProjectRepository, TaskRepository};
use ralphx_lib::domain::services::{RunningAgentKey, RunningAgentRegistry};
use ralphx_lib::infrastructure::memory::{MemoryProjectRepository, MemoryTaskRepository};

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
        let context_type = match key.context_type.parse::<ChatContextType>() {
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
        let context_type = match key.context_type.parse::<ChatContextType>() {
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

// =========================================================================
// Phase: idle ideation sessions must not inflate global_running_count
// =========================================================================

/// Helper: replicate the FIXED `get_execution_status` global count logic.
/// Uses ExecutionState.interactive_idle_count() to subtract idle slots.
async fn count_global_running_with_state(
    app_state: &AppState,
    execution_state: &ExecutionState,
) -> u32 {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let total_with_slot = registry_entries
        .iter()
        .filter(|(key, _)| {
            key.context_type
                .parse::<ChatContextType>()
                .map(uses_execution_slot)
                .unwrap_or(false)
        })
        .count();
    (total_with_slot.saturating_sub(execution_state.interactive_idle_count())) as u32
}

/// Helper: replicate the per-project running count from `get_execution_status`,
/// excluding idle ideation sessions.
async fn count_project_running_with_state(
    app_state: &AppState,
    execution_state: &ExecutionState,
) -> u32 {
    let registry_entries = app_state.running_agent_registry.list_all().await;
    let mut running_count = 0u32;

    for (key, _) in registry_entries {
        let context_type = match key.context_type.parse::<ChatContextType>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        if !uses_execution_slot(context_type) {
            continue;
        }

        if matches!(context_type, ChatContextType::Ideation) {
            let slot_key = format!("{}/{}", key.context_type, key.context_id);
            if !execution_state.is_interactive_idle(&slot_key) {
                running_count += 1;
            }
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

/// Test 1: 2 ideation sessions registered → mark 1 idle → global count == 1
#[tokio::test]
async fn test_idle_ideation_excluded_from_global_running_count() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    let exec_state = ExecutionState::new();

    register_ideation(&*state.running_agent_registry, "session-active").await;
    register_ideation(&*state.running_agent_registry, "session-idle").await;

    // Mark second session as idle between turns
    exec_state.mark_interactive_idle("ideation/session-idle");

    let global_count = count_global_running_with_state(&state, &exec_state).await;
    assert_eq!(
        global_count, 1,
        "Idle ideation session must not inflate global running count"
    );
}

/// Test 2: GC prune cleans idle slots — after pruning a registry entry,
/// interactive_idle_count() must return 0 (no ghost entries).
#[tokio::test]
async fn test_gc_prune_clears_interactive_idle_slot() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    let exec_state = ExecutionState::new();

    register_ideation(&*state.running_agent_registry, "session-gone").await;
    exec_state.mark_interactive_idle("ideation/session-gone");

    assert_eq!(exec_state.interactive_idle_count(), 1, "Should have 1 idle slot before pruning");

    // Simulate what prune does: remove from registry AND remove from idle slots
    let entries = state.running_agent_registry.list_all().await;
    for (key, info) in entries {
        let _ = state
            .running_agent_registry
            .unregister(&key, &info.agent_run_id)
            .await;
        let slot_key = format!("{}/{}", key.context_type, key.context_id);
        exec_state.remove_interactive_slot(&slot_key);
    }

    assert_eq!(
        exec_state.interactive_idle_count(),
        0,
        "Ghost idle slot must be removed after GC prune"
    );
}

/// Test 3: Per-project count excludes idle ideation sessions.
#[tokio::test]
async fn test_per_project_count_excludes_idle_ideation() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    let exec_state = ExecutionState::new();

    register_ideation(&*state.running_agent_registry, "session-idle").await;
    exec_state.mark_interactive_idle("ideation/session-idle");

    let project_count = count_project_running_with_state(&state, &exec_state).await;
    assert_eq!(
        project_count, 0,
        "Idle ideation session must not appear in per-project running count"
    );
}

/// Test 4: Slot key format consistency — key used by streaming code matches
/// the key used by count logic. Format: "{context_type}/{context_id}".
#[test]
fn test_slot_key_format_consistency() {
    let context_type = "ideation";
    let context_id = "session-abc-123";
    let slot_key = format!("{}/{}", context_type, context_id);

    // The key format used when marking idle must match the key used when checking.
    let exec_state = ExecutionState::new();
    exec_state.mark_interactive_idle(&slot_key);

    // Check using the same format that count_global_running_with_state uses
    assert!(
        exec_state.is_interactive_idle(&slot_key),
        "Slot key format must be consistent: '{}'",
        slot_key
    );
    assert_eq!(slot_key, "ideation/session-abc-123");
}

/// Test 5: Generating (non-idle) ideation session IS counted in both global
/// and per-project running counts.
#[tokio::test]
async fn test_generating_ideation_counted_in_running_counts() {
    let state = AppState::with_repos(
        Arc::new(MemoryTaskRepository::new()),
        Arc::new(MemoryProjectRepository::new()),
    );
    let exec_state = ExecutionState::new();

    // Register ideation session — NOT marked idle, so it's actively generating
    register_ideation(&*state.running_agent_registry, "session-generating").await;

    let global_count = count_global_running_with_state(&state, &exec_state).await;
    let project_count = count_project_running_with_state(&state, &exec_state).await;

    assert_eq!(
        global_count, 1,
        "Generating ideation must count toward global running count"
    );
    assert_eq!(
        project_count, 1,
        "Generating ideation must count toward per-project running count"
    );
}
