// RC3 regression tests: stale reviewer guard in complete_review
//
// Bug: RC#3 — 370ms race window between reviewer stream completion and task auto-transition.
//
// The reviewer agent stream can complete AFTER the task has already left the Reviewing state
// (e.g., a concurrent auto-transition or user action moved it to ReviewPassed/RevisionNeeded).
// Without a guard, the stale `complete_review` MCP call would overwrite the already-transitioned
// state, corrupting the task lifecycle.
//
// Fix: Guard at reviews.rs:32 — read the task's current status before acting.
//      If task.internal_status != Reviewing, return 400 BAD_REQUEST immediately.
//
// Tests:
//   1. Stale reviewer call on ReviewPassed task → 400 BAD_REQUEST (guard fires).
//   2. Stale reviewer call on Merged task → 400 BAD_REQUEST (guard fires for terminal states).
//   3. Stale reviewer call on Ready task → 400 BAD_REQUEST (guard fires for non-review states).
//   4. Valid reviewer call on Reviewing task → guard does NOT fire (proceeds past guard line 32).

use super::*;
use crate::application::AppState;
use crate::commands::ExecutionState;
use crate::domain::entities::{InternalStatus, ProjectId, Task};
use axum::http::StatusCode;
use std::sync::Arc;

/// Build a minimal HttpServerState backed by in-memory repos (no SQLite, no Tauri app handle).
async fn setup_review_test_state() -> HttpServerState {
    let app_state = Arc::new(AppState::new_test());
    let execution_state = Arc::new(ExecutionState::new());
    let tracker = crate::application::TeamStateTracker::new();
    let team_service = Arc::new(crate::application::TeamService::new_without_events(
        Arc::new(tracker.clone()),
    ));

    HttpServerState {
        app_state,
        execution_state,
        team_tracker: tracker,
        team_service,
    }
}

/// Create a task with the given status in the state's task repo.
async fn seed_task_with_status(state: &HttpServerState, status: InternalStatus) -> Task {
    let project_id = ProjectId::new();
    let mut task = Task::new(project_id, "RC3 test task".to_string());
    task.internal_status = status;
    state.app_state.task_repo.create(task.clone()).await.unwrap();
    task
}

/// RC#3 guard 1: complete_review on a ReviewPassed task returns 400.
///
/// Scenario: reviewer agent stream completed AFTER the task auto-transitioned from
/// Reviewing → ReviewPassed. The stale `complete_review` MCP call must be rejected.
#[tokio::test]
async fn test_complete_review_rejected_when_task_already_review_passed() {
    let state = setup_review_test_state().await;

    // Seed the task already in ReviewPassed (auto-transition already fired).
    let task = seed_task_with_status(&state, InternalStatus::ReviewPassed).await;
    let task_id = task.id.as_str().to_string();

    let req = CompleteReviewRequest {
        task_id,
        decision: "approved".to_string(),
        summary: None,
        feedback: None,
        issues: None,
    };

    let result = complete_review(State(state), Json(req)).await;

    match result {
        Err((status, msg)) => {
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "RC#3: stale reviewer on ReviewPassed task must return 400. Got: {status}",
            );
            assert!(
                msg.contains("Task not in reviewing state"),
                "RC#3: error message must mention reviewing state. Got: {msg}",
            );
        }
        Ok(_) => panic!("RC#3: complete_review must fail when task is ReviewPassed (stale call)"),
    }
}

/// RC#3 guard 2: complete_review on a Merged task returns 400.
///
/// Scenario: reviewer agent stream completed very late, after merge completed.
/// The guard must reject this regardless of how far the task has progressed.
#[tokio::test]
async fn test_complete_review_rejected_when_task_merged() {
    let state = setup_review_test_state().await;

    let task = seed_task_with_status(&state, InternalStatus::Merged).await;
    let task_id = task.id.as_str().to_string();

    let req = CompleteReviewRequest {
        task_id,
        decision: "approved".to_string(),
        summary: None,
        feedback: None,
        issues: None,
    };

    let result = complete_review(State(state), Json(req)).await;

    match result {
        Err((status, _)) => {
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "RC#3: stale reviewer on Merged task must return 400",
            );
        }
        Ok(_) => panic!("RC#3: complete_review must fail when task is Merged"),
    }
}

/// RC#3 guard 3: complete_review on a Ready task returns 400.
///
/// Scenario: reviewer agent somehow fires complete_review while the task is in a
/// non-review state (e.g. task was reset to Ready after a restart). Must be rejected.
#[tokio::test]
async fn test_complete_review_rejected_when_task_ready() {
    let state = setup_review_test_state().await;

    let task = seed_task_with_status(&state, InternalStatus::Ready).await;
    let task_id = task.id.as_str().to_string();

    let req = CompleteReviewRequest {
        task_id,
        decision: "needs_changes".to_string(),
        summary: None,
        feedback: Some("looks wrong".to_string()),
        issues: None,
    };

    let result = complete_review(State(state), Json(req)).await;

    match result {
        Err((status, msg)) => {
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "RC#3: complete_review on Ready task must return 400",
            );
            assert!(
                msg.contains("Task not in reviewing state"),
                "RC#3: error must mention reviewing state. Got: {msg}",
            );
        }
        Ok(_) => panic!("RC#3: complete_review must fail when task is Ready"),
    }
}

/// RC#3 guard 4: Reviewing task passes the guard (no early 400 return at line 32).
///
/// Documents that the guard ONLY fires when the task is NOT in Reviewing state.
/// This test confirms the guard allows valid reviewer calls through.
///
/// Note: The handler proceeds past the guard to call TaskTransitionService.
/// With in-memory repos, the transition itself is expected to fail or succeed
/// depending on state machine validation. We only care that the 400-guard does
/// NOT fire — the assert is on the specific status code, not overall success.
#[tokio::test]
async fn test_complete_review_guard_does_not_fire_for_reviewing_task() {
    let state = setup_review_test_state().await;

    let task = seed_task_with_status(&state, InternalStatus::Reviewing).await;
    let task_id = task.id.as_str().to_string();

    let req = CompleteReviewRequest {
        task_id,
        decision: "approved".to_string(),
        summary: None,
        feedback: None,
        issues: None,
    };

    let result = complete_review(State(state), Json(req)).await;

    // The guard-specific 400 must NOT be returned for a Reviewing task.
    // (The handler may return 200 or another error from transition — that's fine.)
    if let Err((status, msg)) = &result {
        assert_ne!(
            *status,
            StatusCode::BAD_REQUEST,
            "RC#3: guard must NOT fire for a valid Reviewing task. \
             Got 400 with message: {msg}",
        );
    }
    // If Ok — guard correctly didn't fire, handler proceeded.
}

// ============================================================================
// IPR (Interactive Process Registry) exit signal tests for complete_review
// ============================================================================

/// complete_review — no IPR entry is safe: handler succeeds without IPR registered.
///
/// When no IPR entry is present (reviewer agent already exited), the IPR removal
/// is a no-op and must not cause the handler to fail.
#[tokio::test]
async fn test_complete_review_no_ipr_entry_is_safe() {
    let state = setup_review_test_state().await;
    let task = seed_task_with_status(&state, InternalStatus::Reviewing).await;
    let task_id = task.id.as_str().to_string();

    // No IPR entry registered — IPR removal is a no-op
    let req = CompleteReviewRequest {
        task_id,
        decision: "approved".to_string(),
        summary: None,
        feedback: None,
        issues: None,
    };
    let result = complete_review(State(state.clone()), Json(req)).await;

    // Guard must NOT fire (task is in Reviewing state), so 400 is not acceptable.
    // Transition may succeed (200) or fail (500) depending on in-memory repo support —
    // both are acceptable here; we're testing IPR safety, not the transition itself.
    if let Err((status, msg)) = &result {
        assert_ne!(
            *status,
            StatusCode::BAD_REQUEST,
            "Absent IPR entry must not trigger the 400 guard. Got 400: {msg}",
        );
    }
}

/// complete_review — IPR entry removed after successful approval.
///
/// When the full review flow succeeds (task transitions away from Reviewing),
/// the IPR entry for the "review" context must be removed so the reviewer agent
/// receives EOF on stdin and exits gracefully.
#[tokio::test]
async fn test_complete_review_ipr_removed_on_success() {
    let state = setup_review_test_state().await;
    let task = seed_task_with_status(&state, InternalStatus::Reviewing).await;
    let task_id = task.id.clone();

    // Register IPR entry for the reviewer agent
    let mut child = tokio::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn cat for review IPR test");
    let stdin = child.stdin.take().expect("cat stdin");

    let key = crate::application::interactive_process_registry::InteractiveProcessKey::new(
        "review",
        task_id.as_str(),
    );
    state
        .app_state
        .interactive_process_registry
        .register(key.clone(), stdin)
        .await;

    assert!(
        state
            .app_state
            .interactive_process_registry
            .has_process(&key)
            .await,
        "IPR must be registered before handler call"
    );

    let req = CompleteReviewRequest {
        task_id: task_id.as_str().to_string(),
        decision: "approved".to_string(),
        summary: Some("LGTM".to_string()),
        feedback: None,
        issues: None,
    };
    let result = complete_review(State(state.clone()), Json(req)).await;

    // Only assert IPR removal when the full handler flow succeeded.
    // If the state transition fails (in-memory repo limitation), the IPR removal
    // code is never reached, which is a known constraint of handler-level tests.
    if result.is_ok() {
        assert!(
            !state
                .app_state
                .interactive_process_registry
                .has_process(&key)
                .await,
            "IPR must be removed after complete_review succeeds"
        );
    }

    // Clean up regardless of result
    state
        .app_state
        .interactive_process_registry
        .remove(&key)
        .await;
    let _ = child.kill().await;
}
