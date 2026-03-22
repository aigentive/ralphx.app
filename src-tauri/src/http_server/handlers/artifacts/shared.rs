use super::*;

// ============================================================================
// EditError Types
// ============================================================================

/// Error type for apply_edits pure function.
#[derive(Debug)]
pub enum EditError {
    /// The old_text anchor was not found in the content.
    AnchorNotFound {
        edit_index: usize,
        old_text_preview: String,
    },
    /// The old_text anchor matches multiple locations (ambiguous).
    AmbiguousAnchor {
        edit_index: usize,
        old_text_preview: String,
    },
}

/// Apply sequential old_text→new_text edits to content.
///
/// Edits are applied SEQUENTIALLY — each edit sees the result of all previous edits,
/// not the original content. If any edit fails (anchor not found or ambiguous),
/// the entire operation returns an error and no changes are applied.
///
/// **Ambiguity check**: Verifies that each old_text appears exactly once in the
/// CURRENT content (after prior edits). The check starts searching AFTER the first
/// match ends (`pos + old_text.len()`) to avoid false positives from the match itself.
///
/// **Phantom match note**: If edit N's `new_text` introduces text matching edit N+1's
/// `old_text`, edit N+1 will operate on the introduced text (by design). Agents should
/// use unique 20+ char anchors to avoid ambiguity from sequential interactions.
#[allow(dead_code)]
pub fn apply_edits(content: &str, edits: &[PlanEdit]) -> Result<String, EditError> {
    let mut result = content.to_string();
    for (i, edit) in edits.iter().enumerate() {
        let pos = result.find(&edit.old_text).ok_or_else(|| EditError::AnchorNotFound {
            edit_index: i,
            old_text_preview: edit.old_text.chars().take(80).collect(),
        })?;

        if result[pos + edit.old_text.len()..].contains(&edit.old_text) {
            return Err(EditError::AmbiguousAnchor {
                edit_index: i,
                old_text_preview: edit.old_text.chars().take(80).collect(),
            });
        }

        result = format!(
            "{}{}{}",
            &result[..pos],
            &edit.new_text,
            &result[pos + edit.old_text.len()..],
        );
    }
    Ok(result)
}

/// Map an AppError to an HttpError for handler responses.
pub(super) fn map_app_err(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        AppError::Conflict(msg) => HttpError {
            status: StatusCode::CONFLICT,
            message: Some(msg),
        },
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

/// Async pre-transaction freeze check. Returns Err(AppError::Conflict) if a verification
/// agent is actively running on a child session of any owning session, UNLESS the caller
/// IS that verification child.
///
/// Runs BEFORE db.run_transaction() — registry methods are async and cannot be called
/// inside the synchronous spawn_blocking closure of db.run().
/// Accepts TOCTOU trade-off (single-user context, self-healing on process exit).
///
/// SIMPLIFICATION: plan-verifier agents are autonomous (no stdin pipes) and do NOT
/// register in InteractiveProcessRegistry. Therefore is_generating = is_running.
/// This was verified during implementation: plan-verifier agents spawn via
/// ChatService::send_message() which registers only in RunningAgentRegistry.
///
/// TRUST MODEL: caller_session_id is cooperative/protocol-based only. :3847 is
/// localhost-only (single-user desktop) — prevents accidental concurrent writes, not adversarial.
#[doc(hidden)]
pub async fn check_verification_freeze(
    owning_sessions: &[IdeationSession],
    caller_session_id: Option<&str>,
    running_registry: &dyn RunningAgentRegistry,
    session_repo: &dyn IdeationSessionRepository,
) -> Result<(), AppError> {
    for session in owning_sessions {
        if !session.verification_in_progress {
            continue;
        }

        let children = session_repo.get_verification_children(&session.id).await?;
        for child in &children {
            if Some(child.id.as_str()) == caller_session_id {
                continue;
            }

            let running_key = RunningAgentKey::new("ideation", child.id.as_str());
            if running_registry.is_running(&running_key).await {
                return Err(AppError::Conflict(format!(
                    "Plan is frozen — verification agent is actively working \
                     (child session: {}). Wait for the verification round to \
                     complete before editing.",
                    child.id.as_str()
                )));
            }
        }
    }
    Ok(())
}

/// Shared core for both update_plan_artifact and edit_plan_artifact.
///
/// Takes the resolved artifact + new content, creates a new version,
/// batch-updates sessions/proposals, resets verification, and returns
/// data needed for event emission.
///
/// IMPORTANT: This helper does NOT trigger auto-verification.
/// Auto-verify is triggered ONLY by create_plan_artifact (which calls
/// trigger_auto_verify_sync separately). Both update and edit handlers
/// use finalize_plan_update, which handles:
///   - Create new version (version + 1, previous_version_id = old.id)
///   - Batch-update sessions pointing to old → new
///   - Batch-update proposals (preserve plan_version_at_creation)
///   - Conditional verification reset (CAS: only if in_progress=0)
///
/// The caller is responsible for emitting events:
///   - plan_artifact:updated { previous_artifact_id: old.id, new_artifact_id: new.id, session_id }
///   - plan:proposals_may_need_update (only if linked proposals exist)
///
/// Returns a tuple containing:
///   - (created_artifact, old_artifact_id, owning_sessions, linked_proposal_ids, verification_reset)
pub(super) fn finalize_plan_update(
    conn: &Connection,
    old_artifact: &Artifact,
    new_content: String,
) -> Result<(Artifact, String, Vec<IdeationSession>, Vec<String>, bool), AppError> {
    let old_id = old_artifact.id.as_str().to_string();

    let new_artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: old_artifact.artifact_type.clone(),
        name: old_artifact.name.clone(),
        content: ArtifactContent::Inline { text: new_content },
        metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
            .with_version(old_artifact.metadata.version + 1),
        derived_from: vec![],
        bucket_id: old_artifact.bucket_id.clone(),
        archived_at: None,
    };
    let created = ArtifactRepo::create_with_previous_version_sync(conn, new_artifact, &old_id)?;

    let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let session_ids: Vec<String> = owning_sessions
        .iter()
        .map(|s| s.id.as_str().to_string())
        .collect();
    SessionRepo::batch_update_artifact_id_sync(conn, &session_ids, created.id.as_str())?;

    let linked_proposals = ProposalRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let linked_proposal_ids: Vec<String> =
        linked_proposals.iter().map(|p| p.id.to_string()).collect();

    ProposalRepo::batch_update_artifact_id_sync(conn, &old_id, created.id.as_str())?;

    let verification_reset = if let Some(session) = owning_sessions.first() {
        SessionRepo::reset_verification_sync(conn, session.id.as_str())?
    } else {
        false
    };

    Ok((created, old_id, owning_sessions, linked_proposal_ids, verification_reset))
}
