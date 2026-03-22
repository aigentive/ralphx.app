use super::*;

pub(super) fn emit_plan_update_events(
    app_handle: &tauri::AppHandle,
    created: &Artifact,
    old_artifact_id_str: &str,
    sessions: &[IdeationSession],
    linked_proposal_ids: Vec<String>,
    verification_reset: bool,
) {
    let content_text = match &created.content {
        ArtifactContent::Inline { text } => text.clone(),
        ArtifactContent::File { path } => format!("[File: {}]", path),
    };

    if verification_reset {
        if let Some(session) = sessions.first() {
            emit_verification_status_changed(
                app_handle,
                session.id.as_str(),
                VerificationStatus::Unverified,
                false,
                None,
                None,
                Some(session.verification_generation),
            );
        }
    }

    let _ = app_handle.emit(
        "plan_artifact:updated",
        serde_json::json!({
            "artifactId": created.id.as_str(),
            "previousArtifactId": old_artifact_id_str,
            "sessionId": sessions.first().map(|s| s.id.as_str()),
            "artifact": {
                "id": created.id.as_str(),
                "name": created.name,
                "content": content_text,
                "version": created.metadata.version,
            }
        }),
    );

    if !linked_proposal_ids.is_empty() {
        let payload = PlanProposalsSyncPayload {
            artifact_id: created.id.to_string(),
            previous_artifact_id: old_artifact_id_str.to_string(),
            proposal_ids: linked_proposal_ids,
            new_version: created.metadata.version,
            session_id: sessions.first().map(|s| s.id.to_string()),
            proposals_relinked: true,
        };
        let _ = app_handle.emit("plan:proposals_may_need_update", payload);
    }
}
