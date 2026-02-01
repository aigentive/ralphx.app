// Helper functions for task_commands module

use tauri::{Emitter, State};
use crate::application::AppState;
use crate::domain::entities::{InternalStatus, ProjectId};

/// Default target for inject_task command
pub fn default_target() -> String {
    "backlog".to_string()
}

/// Emit execution:queue_changed event with current Ready task count.
///
/// This is called when a task moves to or from Ready status, providing real-time
/// queue count updates to the frontend's ExecutionControlBar.
///
/// This function is public so it can be reused by other command modules that
/// transition tasks to/from Ready status (e.g., review_commands::approve_fix_task).
pub async fn emit_queue_changed(
    state: &State<'_, AppState>,
    project_id: &ProjectId,
    app: &tauri::AppHandle,
) {
    // Count tasks currently in Ready status
    let queued_count = match state.task_repo.get_by_status(project_id, InternalStatus::Ready).await
    {
        Ok(tasks) => tasks.len(),
        Err(e) => {
            tracing::warn!("Failed to count Ready tasks for queue_changed event: {}", e);
            return;
        }
    };

    let _ = app.emit(
        "execution:queue_changed",
        serde_json::json!({
            "queuedCount": queued_count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    tracing::debug!(queued_count, "Emitted execution:queue_changed event");
}

/// Emit a task lifecycle event (archived, restored, deleted).
///
/// These events share a common payload structure with task and project IDs.
pub fn emit_task_lifecycle_event(app: &tauri::AppHandle, event_name: &str, task_id: &str, project_id: &str) {
    let _ = app.emit(
        event_name,
        serde_json::json!({
            "taskId": task_id,
            "projectId": project_id,
        }),
    );
}

/// Maps an InternalStatus to a user-friendly label for the status dropdown
pub fn status_to_label(status: InternalStatus) -> String {
    match status {
        InternalStatus::Backlog => "Move to Backlog".to_string(),
        InternalStatus::Ready => "Ready for Work".to_string(),
        InternalStatus::Blocked => "Mark as Blocked".to_string(),
        InternalStatus::Executing => "Start Execution".to_string(),
        InternalStatus::QaRefining => "QA Refining".to_string(),
        InternalStatus::QaTesting => "QA Testing".to_string(),
        InternalStatus::QaPassed => "QA Passed".to_string(),
        InternalStatus::QaFailed => "QA Failed".to_string(),
        InternalStatus::PendingReview => "Send to Review".to_string(),
        InternalStatus::Reviewing => "AI Reviewing".to_string(),
        InternalStatus::ReviewPassed => "Review Passed".to_string(),
        InternalStatus::Escalated => "Escalated".to_string(),
        InternalStatus::RevisionNeeded => "Needs Revision".to_string(),
        InternalStatus::ReExecuting => "Re-executing".to_string(),
        InternalStatus::Approved => "Approve".to_string(),
        InternalStatus::PendingMerge => "Merging...".to_string(),
        InternalStatus::Merging => "Resolving Conflicts".to_string(),
        InternalStatus::MergeConflict => "Merge Conflict".to_string(),
        InternalStatus::Merged => "Merged".to_string(),
        InternalStatus::Failed => "Mark as Failed".to_string(),
        InternalStatus::Cancelled => "Cancel".to_string(),
    }
}
