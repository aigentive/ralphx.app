// Shared event emission helpers for plan verification lifecycle events.
//
// All 5 emission points use `emit_verification_status_changed()` to prevent payload
// drift across reconciliation, recovery, revert-and-skip, artifact reset, and the
// main update_plan_verification handler.

use tauri::{AppHandle, Emitter, Runtime};

use crate::domain::entities::{VerificationMetadata, VerificationStatus};

/// Build the canonical metadata blob for a freshly started verification run.
///
/// This keeps verification-start event payloads consistent across all entry points
/// (manual verify, external trigger, auto-verify on plan creation, re-verify).
pub fn build_verification_started_metadata(max_rounds: u32) -> VerificationMetadata {
    VerificationMetadata {
        max_rounds,
        ..Default::default()
    }
}

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
        metadata,
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
    let metadata = build_verification_started_metadata(max_rounds);
    emit_verification_status_changed(
        app_handle,
        session_id,
        VerificationStatus::Reviewing,
        true,
        Some(&metadata),
        None,
        Some(generation),
    );
}

/// Build the canonical JSON payload. Extracted so it can be tested without a real AppHandle.
pub fn build_verification_payload(
    session_id: &str,
    status: VerificationStatus,
    in_progress: bool,
    metadata: Option<&VerificationMetadata>,
    convergence_reason: Option<&str>,
    generation: Option<i32>,
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
            "generation": generation,
            "round": if m.current_round > 0 { serde_json::Value::from(m.current_round) } else { serde_json::Value::Null },
            "max_rounds": if m.max_rounds > 0 { serde_json::Value::from(m.max_rounds) } else { serde_json::Value::Null },
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
