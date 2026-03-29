use super::*;

pub async fn get_review_notes(
    State(state): State<HttpServerState>,
    scope: ProjectScope,
    Path(task_id): Path<String>,
) -> Result<Json<ReviewNotesResponse>, (StatusCode, String)> {
    let task_id = TaskId::from_string(task_id);

    // Load task to enforce project scope before returning review notes
    let task = state
        .app_state
        .task_repo
        .get_by_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Task not found".to_string()))?;
    task.assert_project_scope(&scope)
        .map_err(|e| (e.status, e.message.unwrap_or_default()))?;

    // 1. Fetch all review notes for this task
    let notes = state
        .app_state
        .review_repo
        .get_notes_by_task_id(&task_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. Calculate revision count (count of changes_requested outcomes)
    let revision_count = notes
        .iter()
        .filter(|n| n.outcome == ReviewOutcome::ChangesRequested)
        .count() as u32;

    // 3. Get max_revisions from review settings
    let review_settings = state
        .app_state
        .review_settings_repo
        .get_settings()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let max_revisions = review_settings.max_revision_cycles;

    // 4. Convert notes to response format
    let reviews: Vec<ReviewNoteResponse> = notes
        .into_iter()
        .map(|note| {
            // Convert issues from domain type to HTTP type
            let issues = note.issues.map(|issues| {
                issues
                    .into_iter()
                    .map(|i| super::ReviewIssue {
                        severity: i.severity,
                        file: i.file,
                        line: i.line.map(|l| l as u32),
                        description: i.description,
                    })
                    .collect()
            });

            ReviewNoteResponse {
                id: note.id.as_str().to_string(),
                reviewer: note.reviewer.to_string(),
                outcome: note.outcome.to_string(),
                summary: note.summary,
                notes: note.notes,
                issues,
                followup_session_id: note.followup_session_id,
                created_at: note.created_at.to_rfc3339(),
            }
        })
        .collect();

    // 5. Return response
    Ok(Json(ReviewNotesResponse {
        task_id: task_id.as_str().to_string(),
        revision_count,
        max_revisions,
        reviews,
    }))
}
