use super::*;

pub(crate) async fn create_verification_child_session(
    state: &HttpServerState,
    parent_session_id: &str,
    description: &str,
    title: &str,
) -> Result<bool, String> {
    let parent_id = IdeationSessionId::from_string(parent_session_id.to_string());

    let parent = state
        .app_state
        .ideation_session_repo
        .get_by_id(&parent_id)
        .await
        .map_err(|e| format!("Failed to fetch parent session: {}", e))?
        .ok_or_else(|| format!("Parent session {} not found", parent_session_id))?;

    let child_session = IdeationSession {
        id: IdeationSessionId::new(),
        project_id: parent.project_id.clone(),
        title: Some(title.to_string()),
        status: IdeationSessionStatus::Active,
        plan_artifact_id: None,
        inherited_plan_artifact_id: parent.plan_artifact_id.clone(),
        seed_task_id: None,
        parent_session_id: Some(parent_id.clone()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        archived_at: None,
        converted_at: None,
        team_mode: None,
        team_config_json: None,
        title_source: None,
        verification_status: VerificationStatus::default(),
        verification_in_progress: false,
        verification_metadata: None,
        verification_generation: 0,
        source_project_id: parent.source_project_id.clone(),
        source_session_id: parent.source_session_id.clone(),
        source_task_id: None,
        source_context_type: None,
        source_context_id: None,
        spawn_reason: None,
        session_purpose: SessionPurpose::Verification,
        cross_project_checked: true,
        plan_version_last_read: None,
        origin: parent.origin,
        expected_proposal_count: None,
        auto_accept_status: None,
        auto_accept_started_at: None,
        api_key_id: None,
        idempotency_key: None,
        external_activity_phase: None,
        external_last_read_message_id: None,
        dependencies_acknowledged: false,
        pending_initial_prompt: None,
    };

    let child_id = child_session.id.clone();
    let child_session_str = child_id.as_str().to_string();
    let parent_session_str = parent_session_id.to_string();

    let created_session = state
        .app_state
        .ideation_session_repo
        .create(child_session)
        .await
        .map_err(|e| format!("Failed to create verification child session: {}", e))?;

    let link = SessionLink::new(
        parent_id.clone(),
        child_id.clone(),
        SessionRelationship::FollowOn,
    );
    state
        .app_state
        .session_link_repo
        .create(link)
        .await
        .map_err(|e| format!("Failed to create session link: {}", e))?;

    let chat_service = build_ideation_chat_service(state, &created_session);
    let orchestration_triggered = match chat_service
        .send_message(
            ChatContextType::Ideation,
            &child_session_str,
            description,
            Default::default(),
        )
        .await
    {
        Ok(_) => true,
        Err(e) => {
            error!(
                "Failed to spawn plan-verifier on verification child session {}: {}",
                child_session_str, e
            );
            // Archive the child row so it does not linger as an orphan
            if let Err(archive_err) = state
                .app_state
                .ideation_session_repo
                .update_status(&child_id, IdeationSessionStatus::Archived)
                .await
            {
                error!(
                    "Failed to archive verification child session {} after spawn failure: {}",
                    child_session_str, archive_err
                );
            }
            false
        }
    };

    if let Some(app_handle) = &state.app_state.app_handle {
        let session_title = created_session
            .title
            .clone()
            .unwrap_or_else(|| title.to_string());
        let _ = app_handle.emit(
            "ideation:child_session_created",
            serde_json::json!({
                "sessionId": child_session_str,
                "parentSessionId": parent_session_str,
                "title": session_title,
                "purpose": "verification"
            }),
        );
    }

    Ok(orchestration_triggered)
}
