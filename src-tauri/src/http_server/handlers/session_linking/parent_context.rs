use super::*;

pub async fn get_parent_session_context(
    State(state): State<HttpServerState>,
    Path(session_id_str): Path<String>,
) -> Result<Json<ParentContextResponse>, JsonError> {
    let session_id = IdeationSessionId::from_string(session_id_str.clone());

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!("Failed to fetch session {}: {}", session_id.as_str(), e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch session: {}", e),
            )
        })?
        .ok_or_else(|| {
            error!("Session {} not found", session_id.as_str());
            json_error(StatusCode::NOT_FOUND, "Session not found")
        })?;

    let parent_id = session.parent_session_id.ok_or_else(|| {
        tracing::debug!(
            session_id = session_id.as_str(),
            "Session does not have a parent"
        );
        json_error(
            StatusCode::NOT_FOUND,
            "Session does not have a parent session",
        )
    })?;

    let parent = state
        .app_state
        .ideation_session_repo
        .get_by_id(&parent_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to fetch parent session {}: {}",
                parent_id.as_str(),
                e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch parent session: {}", e),
            )
        })?
        .ok_or_else(|| {
            error!("Parent session {} not found", parent_id.as_str());
            json_error(StatusCode::NOT_FOUND, "Parent session not found")
        })?;

    Ok(Json(load_parent_context(&state, &parent).await))
}
