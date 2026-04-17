// Shared event emission helpers for plan verification lifecycle events.
//
// All 5 emission points use `emit_verification_status_changed()` to prevent payload
// drift across reconciliation, recovery, revert-and-skip, artifact reset, and the
// main post_verification_status handler.

use tauri::{AppHandle, Emitter, Runtime};

use crate::domain::entities::{VerificationRunSnapshot, VerificationStatus};
use crate::domain::services::gap_score;

/// Build the canonical native snapshot for a freshly started verification run.
///
/// This keeps verification-start event payloads consistent across all entry points
/// (manual verify, external trigger, auto-verify on plan creation, re-verify).
pub fn build_verification_started_snapshot(
    generation: i32,
    max_rounds: u32,
) -> VerificationRunSnapshot {
    VerificationRunSnapshot {
        generation,
        status: VerificationStatus::Reviewing,
        in_progress: true,
        current_round: 0,
        max_rounds,
        best_round_index: None,
        convergence_reason: None,
        current_gaps: Vec::new(),
        rounds: Vec::new(),
    }
}

/// Emits `plan_verification:status_changed` with the canonical payload shape.
///
/// - `snapshot`: `Some` → includes round/max_rounds/gap_score/current_gaps/rounds.
///   `None` → all those fields are null / empty arrays.
/// - `convergence_reason`: explicit override. When `snapshot` is `Some` and this is
///   `None`, the convergence_reason stored inside the snapshot is used instead.
///
/// All emission points must call this function to maintain a consistent frontend
/// contract and prevent partial payload bugs (B2, B3, B4).
pub fn emit_verification_status_changed<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
    status: VerificationStatus,
    in_progress: bool,
    snapshot: Option<&VerificationRunSnapshot>,
    convergence_reason: Option<&str>,
    generation: Option<i32>,
) {
    // ImportedVerified is a terminal import state — no UI event emitted.
    // The frontend learns this status via polling/initial load, not via real-time events.
    if status == VerificationStatus::ImportedVerified {
        return;
    }
    let payload = build_verification_payload(
        session_id,
        status,
        in_progress,
        snapshot,
        convergence_reason,
        generation,
    );
    let _ = app_handle.emit("plan_verification:status_changed", payload);
}

/// Emit the canonical "verification started" event.
pub fn emit_verification_started<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
    generation: i32,
    max_rounds: u32,
) {
    let snapshot = build_verification_started_snapshot(generation, max_rounds);
    emit_verification_status_changed(
        app_handle,
        session_id,
        VerificationStatus::Reviewing,
        true,
        Some(&snapshot),
        None,
        Some(generation),
    );
}

/// Emits `verification:pending_confirmation` event when a plan needs user confirmation before verification.
///
/// This is emitted after a plan artifact is saved for UI sessions, signaling the frontend
/// to show the VerificationConfirmDialog with the specialist selection checkboxes.
pub fn emit_verification_pending_confirmation<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
    session_title: &str,
    plan_artifact_id: &str,
) {
    let payload = serde_json::json!({
        "session_id": session_id,
        "session_title": session_title,
        "plan_artifact_id": plan_artifact_id,
    });
    let _ = app_handle.emit("verification:pending_confirmation", payload);
}

/// Build the canonical JSON payload. Extracted so it can be tested without a real AppHandle.
pub fn build_verification_payload(
    session_id: &str,
    status: VerificationStatus,
    in_progress: bool,
    snapshot: Option<&VerificationRunSnapshot>,
    convergence_reason: Option<&str>,
    generation: Option<i32>,
) -> serde_json::Value {
    if let Some(snapshot) = snapshot {
        let weighted_gap_score = gap_score(&snapshot.current_gaps);

        let reason = convergence_reason.or(snapshot.convergence_reason.as_deref());

        serde_json::json!({
            "session_id": session_id,
            "status": status.to_string(),
            "in_progress": in_progress,
            "generation": generation,
            "round": if snapshot.current_round > 0 { serde_json::Value::from(snapshot.current_round) } else { serde_json::Value::Null },
            "max_rounds": if snapshot.max_rounds > 0 { serde_json::Value::from(snapshot.max_rounds) } else { serde_json::Value::Null },
            "gap_score": weighted_gap_score,
            "convergence_reason": reason,
            "current_gaps": snapshot.current_gaps,
            "rounds": snapshot.rounds,
        })
    } else {
        serde_json::json!({
            "session_id": session_id,
            "status": status.to_string(),
            "in_progress": in_progress,
            "generation": generation,
            "round": serde_json::Value::Null,
            "max_rounds": serde_json::Value::Null,
            "gap_score": serde_json::Value::Null,
            "convergence_reason": convergence_reason,
            "current_gaps": serde_json::Value::Array(vec![]),
            "rounds": serde_json::Value::Array(vec![]),
        })
    }
}

#[cfg(test)]
#[path = "verification_events_tests.rs"]
mod tests;
