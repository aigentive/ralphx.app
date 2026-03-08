// Shared event emission helper for plan verification status changes.
//
// All 5 emission points use `emit_verification_status_changed()` to prevent payload
// drift across reconciliation, recovery, revert-and-skip, artifact reset, and the
// main update_plan_verification handler.

use tauri::{AppHandle, Emitter, Runtime};

use crate::domain::entities::{VerificationMetadata, VerificationStatus};

/// Emits `plan_verification:status_changed` with the canonical payload shape.
///
/// - `metadata`: `Some` → includes round/max_rounds/gap_score/current_gaps/rounds.
///   `None` → all those fields are null / empty arrays.
/// - `convergence_reason`: explicit override. When `metadata` is `Some` and this is
///   `None`, the convergence_reason stored inside the metadata struct is used instead.
///
/// All emission points must call this function to maintain a consistent frontend
/// contract and prevent partial payload bugs (B2, B3, B4).
pub fn emit_verification_status_changed<R: Runtime>(
    app_handle: &AppHandle<R>,
    session_id: &str,
    status: VerificationStatus,
    in_progress: bool,
    metadata: Option<&VerificationMetadata>,
    convergence_reason: Option<&str>,
) {
    let payload = build_verification_payload(session_id, status, in_progress, metadata, convergence_reason);
    let _ = app_handle.emit("plan_verification:status_changed", payload);
}

/// Build the canonical JSON payload. Extracted so it can be tested without a real AppHandle.
pub fn build_verification_payload(
    session_id: &str,
    status: VerificationStatus,
    in_progress: bool,
    metadata: Option<&VerificationMetadata>,
    convergence_reason: Option<&str>,
) -> serde_json::Value {
    if let Some(m) = metadata {
        let gap_score: u32 = m.current_gaps.iter().map(|g| match g.severity.as_str() {
            "critical" => 10u32,
            "high" => 3,
            "medium" => 1,
            _ => 0,
        }).sum();

        // Prefer explicit override; fall back to metadata's convergence_reason
        let reason = convergence_reason.or(m.convergence_reason.as_deref());

        serde_json::json!({
            "session_id": session_id,
            "status": status.to_string(),
            "in_progress": in_progress,
            "round": m.current_round,
            "max_rounds": m.max_rounds,
            "gap_score": gap_score,
            "convergence_reason": reason,
            "current_gaps": m.current_gaps,
            "rounds": m.rounds,
        })
    } else {
        serde_json::json!({
            "session_id": session_id,
            "status": status.to_string(),
            "in_progress": in_progress,
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
