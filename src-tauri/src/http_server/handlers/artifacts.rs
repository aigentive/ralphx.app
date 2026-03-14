use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tauri::Emitter;
use tracing::error;

use super::*;
use crate::application::chat_service::{ChatService, ClaudeChatService, SendMessageOptions};
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata, ArtifactType,
    ChatContextType, IdeationSession, IdeationSessionId, VerificationStatus,
};
use rusqlite::Connection;
use crate::domain::services::emit_verification_status_changed;
use crate::error::AppError;
use crate::infrastructure::agents::claude::verification_config;
use crate::infrastructure::sqlite::{
    SqliteArtifactRepository as ArtifactRepo, SqliteIdeationSessionRepository as SessionRepo,
    SqliteTaskProposalRepository as ProposalRepo,
};

/// Metadata key used to mark auto-verification messages.
pub(crate) const AUTO_VERIFICATION_KEY: &str = "auto_verification";

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
        // Find exact match
        let pos = result.find(&edit.old_text).ok_or_else(|| EditError::AnchorNotFound {
            edit_index: i,
            old_text_preview: edit.old_text.chars().take(80).collect(),
        })?;

        // Verify unique match — check for second occurrence AFTER the first match ends.
        // Use pos + old_text.len() to skip the matched text itself.
        if result[pos + edit.old_text.len()..].contains(&edit.old_text) {
            return Err(EditError::AmbiguousAnchor {
                edit_index: i,
                old_text_preview: edit.old_text.chars().take(80).collect(),
            });
        }

        // Apply replacement
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
fn map_app_err(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
}

/// Shared core for both update_plan_artifact and (future) edit_plan_artifact.
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
fn finalize_plan_update(
    conn: &Connection,
    old_artifact: &Artifact,
    new_content: String,
) -> Result<(Artifact, String, Vec<IdeationSession>, Vec<String>, bool), AppError> {
    let old_id = old_artifact.id.as_str().to_string();

    // 1. Create NEW artifact with incremented version (version chain, not in-place update)
    let new_artifact = Artifact {
        id: ArtifactId::new(),
        artifact_type: old_artifact.artifact_type.clone(),
        name: old_artifact.name.clone(),
        content: ArtifactContent::Inline { text: new_content },
        metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
            .with_version(old_artifact.metadata.version + 1),
        derived_from: vec![],
        bucket_id: old_artifact.bucket_id.clone(),
    };
    let created =
        ArtifactRepo::create_with_previous_version_sync(conn, new_artifact, &old_id)?;

    // 2. Batch-update all sessions pointing to old artifact to point to new one
    let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let session_ids: Vec<String> = owning_sessions
        .iter()
        .map(|s| s.id.as_str().to_string())
        .collect();
    SessionRepo::batch_update_artifact_id_sync(conn, &session_ids, created.id.as_str())?;

    // 3. Fetch proposals linked to old artifact (before batch-updating them)
    let linked_proposals = ProposalRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
    let linked_proposal_ids: Vec<String> =
        linked_proposals.iter().map(|p| p.id.to_string()).collect();

    // 4. Batch-update all linked proposals to point to the new artifact version.
    //    plan_version_at_creation is intentionally NOT changed (preserves original birth version).
    ProposalRepo::batch_update_artifact_id_sync(conn, &old_id, created.id.as_str())?;

    // 5. Conditionally reset verification — only when verification_in_progress = 0.
    //    Prevents the loop-reset paradox where auto-corrections reset verification mid-loop.
    let verification_reset = if let Some(session) = owning_sessions.first() {
        SessionRepo::reset_verification_sync(conn, session.id.as_str())?
    } else {
        false
    };

    Ok((created, old_id, owning_sessions, linked_proposal_ids, verification_reset))
}

pub async fn create_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<CreatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let session_id_str = req.session_id.clone();
    let title = req.title.clone();
    let content = req.content.clone();
    let cfg = verification_config();
    let auto_verify_enabled = cfg.auto_verify;

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    // Returns auto_verify_generation=Some(gen) if auto-verify trigger was atomically applied.
    let (session_id, created, auto_verify_generation) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            let sid = IdeationSessionId::from_string(session_id_str);

            // Get session and check for existing plan
            let session = SessionRepo::get_by_id_sync(conn, sid.as_str())?
                .ok_or_else(|| AppError::NotFound(format!("Session {} not found", sid)))?;

            // Guard: reject mutations on Archived/Accepted sessions
            crate::http_server::helpers::assert_session_mutable(&session)?;

            // Create the specification artifact
            let bucket_id = ArtifactBucketId::from_string("prd-library");
            let artifact = Artifact {
                id: ArtifactId::new(),
                artifact_type: ArtifactType::Specification,
                name: title,
                content: ArtifactContent::inline(&content),
                metadata: ArtifactMetadata::new("orchestrator").with_version(1),
                derived_from: vec![],
                bucket_id: Some(bucket_id),
            };

            // Chain only to the session's OWN plan (plan_artifact_id), never to an inherited one.
            // Child sessions with inherit_context=true have plan_artifact_id=None and
            // inherited_plan_artifact_id=Some(parent_id). The else branch creates a fresh,
            // independent artifact for them — not chained to the parent's plan.
            let created = if let Some(existing_plan_id) = &session.plan_artifact_id {
                let prev_id = existing_plan_id.as_str().to_string();
                ArtifactRepo::create_with_previous_version_sync(conn, artifact, &prev_id)?
            } else {
                ArtifactRepo::create_sync(conn, artifact)?
            };

            // Link artifact to session
            SessionRepo::update_plan_artifact_id_sync(
                conn,
                sid.as_str(),
                Some(created.id.as_str()),
            )?;

            // Atomically trigger auto-verify within the same transaction.
            // Condition: auto_verify enabled AND verification_in_progress == 0.
            // Sets: status=reviewing, in_progress=1, generation++ in a single UPDATE.
            let auto_verify_generation = if auto_verify_enabled {
                SessionRepo::trigger_auto_verify_sync(conn, sid.as_str())?
            } else {
                None
            };

            Ok((sid, created, auto_verify_generation))
        })
        .await
        .map_err(|e| {
            error!("create_plan_artifact transaction failed: {}", e);
            map_app_err(e)
        })?;

    // Emit event for real-time UI update (outside lock — acceptable crash gap)
    if let Some(app_handle) = &state.app_state.app_handle {
        let content_text = match &created.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };
        let _ = app_handle.emit(
            "plan_artifact:created",
            serde_json::json!({
                "sessionId": session_id.as_str(),
                "artifact": {
                    "id": created.id.as_str(),
                    "name": created.name,
                    "content": content_text,
                    "version": created.metadata.version,
                }
            }),
        );
    }

    // Spawn auto-verifier after commit, if trigger was applied.
    // Fire-and-forget: spawn failure resets in_progress so reconciler/user can retry.
    if let Some(generation) = auto_verify_generation {
        if let Err(e) = spawn_auto_verifier(&state, session_id.as_str(), generation).await {
            error!(
                "Auto-verifier spawn failed for session {}: {}",
                session_id.as_str(),
                e
            );
            // Recovery: reset in_progress=0 so the session is not permanently locked.
            let sid_str = session_id.as_str().to_string();
            if let Err(reset_err) = state.app_state.db.run(move |conn| {
                SessionRepo::reset_auto_verify_sync(conn, &sid_str)
            }).await {
                error!(
                    "Failed to reset auto-verify state for session {} after spawn failure: {}",
                    session_id.as_str(),
                    reset_err
                );
            }
        }
    }

    Ok(Json(ArtifactResponse::from(created)))
}

/// Spawn the auto-verifier agent on the session using the ChatService pipeline.
///
/// Uses the same pattern as `session_linking.rs` (spawn_child_session_agent):
/// constructs a ClaudeChatService, then calls send_message with the verification prompt.
/// The orchestrator-ideation agent (registered for ChatContextType::Ideation) picks up
/// the message and runs the iterative verification loop.
/// Build the inner prompt for the auto-verifier agent.
///
/// Extracted as a pure function to enable unit testing without a full ChatService stack.
/// `prior_gaps` contains gap descriptions from a previous generation's verification run;
/// when non-empty they are injected inside each delimiter block so critics do not
/// re-escalate already-addressed findings.
pub(crate) fn build_auto_verifier_prompt(
    session_id: &str,
    generation: i32,
    max_rounds: u32,
    prior_gaps: &[String],
) -> String {
    // Pre-seeded prior gaps for the initial call (in practice always empty).
    // On subsequent rounds the orchestrator injects prior gaps dynamically (see step C).
    let initial_prior_gaps_block = if prior_gaps.is_empty() {
        String::new()
    } else {
        let gap_lines: Vec<String> = prior_gaps
            .iter()
            .map(|g| {
                format!(
                    "- {g} — ADDRESSED in revision (do not re-flag unless the fix is inadequate)"
                )
            })
            .collect();
        format!(
            "PRIOR ROUND CONTEXT (round N-1 findings that were addressed in the current plan revision):\n\
             {}\n\
             Only re-flag a prior gap if the revision's fix is INSUFFICIENT or INCORRECT. \
             Do not re-flag just because the code has not been written yet.\n\n",
            gap_lines.join("\n")
        )
    };

    format!(
        "AUTO-VERIFICATION MODE for session {session_id}. Generation: {generation}. Max rounds: {max_rounds}. NO user is present — run the complete verification loop WITHOUT waiting for user input.\n\
         \n\
         ## MANDATORY FIRST STEP\n\
         \n\
         Call get_session_plan(session_id: \"{session_id}\") to read the current plan.\n\
         Store the artifact_id from the response (you will need it for update_plan_artifact calls).\n\
         Use the plan content for the Layer 2 guard regex check in Step E ONLY — do NOT embed plan content in the critic prompt.\n\
         \n\
         ## VERIFICATION LOOP\n\
         \n\
         Repeat until convergence OR round >= {max_rounds}:\n\
         \n\
         ### A. ZOMBIE CHECK\n\
         Call get_plan_verification(\"{session_id}\").\n\
         If verification_generation != {generation}, EXIT IMMEDIATELY — a newer verification run has superseded this one.\n\
         \n\
         ### B. Round counter\n\
         Compute round = current_round + 1.\n\
         \n\
         ### C. Build critic prompt\n\
         Compose the critic_prompt string to pass to each Agent:\n\
         - If round == 1 and there are pre-seeded prior gaps, start with:\n\
           {initial_prior_gaps_block}\
           SESSION_ID: {session_id}\n\
         - If round > 1, prepend the following section BEFORE the SESSION_ID line (using gap descriptions from the previous get_plan_verification response):\n\
           ```\n\
           PRIOR ROUND CONTEXT (gaps from round N-1 addressed in the current plan revision):\n\
           - <gap1 description> — ADDRESSED in revision (do not re-flag unless the fix is inadequate)\n\
           - <gap2 description> — ADDRESSED in revision (do not re-flag unless the fix is inadequate)\n\
           ...\n\
           Only re-flag a prior gap if the revision's fix is INSUFFICIENT or INCORRECT. Do not re-flag just because the code has not been written yet.\n\
           \n\
           SESSION_ID: {session_id}\n\
           ```\n\
         - If round == 1 (no pre-seeded gaps), critic_prompt = SESSION_ID: {session_id}\n\
         \n\
         ### D. LAYER 1 — Completeness Critic (always run)\n\
         Spawn: Agent(subagent_type: \"ralphx:plan-critic-layer1\", prompt: critic_prompt)\n\
         Wait for the result.\n\
         \n\
         ### E. LAYER 2 — Dual-lens critic (run only if plan contains code indicators)\n\
         Check if the plan content matches: /(?:src[-\\/]|\\.rs\\b|\\.tsx?\\b|Affected Files|## Implementation)/\n\
         If it matches, spawn the Layer 2 critic (single agent, dual-lens analysis):\n\
         Agent(subagent_type: \"ralphx:plan-critic-layer2\", prompt: critic_prompt)\n\
         Wait for the result.\n\
         \n\
         ### F. Merge and deduplicate gaps\n\
         Collect all gaps from all Agent results that completed. Deduplicate by description similarity (merge gaps where descriptions are >80% similar). Assign the higher severity when merging.\n\
         \n\
         ### G. Report to backend\n\
         Call: update_plan_verification(session_id: \"{session_id}\", status: \"reviewing\", in_progress: true, round: N, gaps: [all_gaps], generation: {generation})\n\
         NOTE: Always send status: \"reviewing\" — the backend auto-corrects to \"needs_revision\" when appropriate. NEVER send \"needs_revision\" directly.\n\
         \n\
         ### H. EMPTY ROUND GUARD\n\
         If all_gaps is empty AND round == 1:\n\
         - Call update_plan_verification with 0 gaps as above\n\
         - CONTINUE to round 2 automatically — do NOT stop. Empty round 1 requires confirmation in round 2.\n\
         \n\
         ### I. Revise plan if gaps found\n\
         Compute gap_score = (critical_count × 10) + (high_count × 3) + (medium_count × 1).\n\
         If gap_score > 0:\n\
         - Call get_session_plan(\"{session_id}\") to get the latest plan content\n\
         - Synthesize fixes for ALL critical and high gaps; address medium gaps if straightforward\n\
         - Write a revised plan — when revising, follow these protection rules:\n\
           * NEVER remove or modify sections that describe proposed additions (new files, new columns, new migrations) unless a critic identified that the addition itself is wrong.\n\
           * Only ADD or CLARIFY content — do not restructure or remove existing plan sections.\n\
           * Preserve ALL user-authored content (architecture decisions, phase descriptions, affected files).\n\
           * If a gap says \"X is missing\", add X to the plan — do not remove other proposed items to make room.\n\
         - ONLY modify sections related to identified gaps, NEVER remove user content\n\
         - Call update_plan_artifact(artifact_id: <current_artifact_id>, content: <revised_plan_content>)\n\
         - Store the new artifact_id returned from the response\n\
         \n\
         ### J. CONVERGENCE CHECK\n\
         Call get_plan_verification(\"{session_id}\").\n\
         If convergence_reason is not null, proceed to FINAL CLEANUP.\n\
         \n\
         ### K. Max rounds check\n\
         If round >= {max_rounds}: proceed to FINAL CLEANUP with convergence_reason: \"max_rounds\".\n\
         \n\
         ### L. Re-read and loop\n\
         Call get_session_plan(\"{session_id}\") to re-read the (possibly updated) plan, then return to step A.\n\
         \n\
         ## FINAL CLEANUP\n\
         \n\
         Call update_plan_verification with:\n\
         - session_id: \"{session_id}\"\n\
         - status: <status from the last get_plan_verification call>\n\
         - in_progress: false\n\
         - convergence_reason: <reason>\n\
         - generation: {generation}\n\
         \n\
         For max_rounds exit: use status: \"verified\", convergence_reason: \"max_rounds\".\n\
         \n\
         EXIT — work is complete.\n\
         \n\
         ## ERROR HANDLING\n\
         \n\
         - Any MCP call failure: wait 2 seconds, retry once.\n\
         - If retry also fails: call update_plan_verification(status: \"needs_revision\", in_progress: false, convergence_reason: \"agent_error\", generation: {generation}), then EXIT.\n\
         - If update_plan_verification returns a \"generation mismatch\" error: EXIT immediately without further calls.\n\
         \n\
         ## CONVERGENCE RULES (backend computes — check get_plan_verification after each update)\n\
         \n\
         - zero_blocking: critical=0 AND high=0 AND medium=0 (round >= 2)\n\
         - jaccard_converged: Jaccard similarity >= 0.8 between consecutive rounds\n\
         - max_rounds: round >= {max_rounds}\n\
         - critic_parse_failure: >= 3 parse failures in 5 rounds\n\
         The backend computes convergence — always check get_plan_verification for convergence_reason after each update_plan_verification call.",
        initial_prior_gaps_block = initial_prior_gaps_block,
    )
}

pub(crate) async fn spawn_auto_verifier(
    state: &HttpServerState,
    session_id: &str,
    generation: i32,
) -> Result<(), String> {
    let cfg = verification_config();
    let max_rounds = cfg.max_rounds;

    let trigger_time = chrono::Utc::now();
    let inner_prompt = build_auto_verifier_prompt(session_id, generation, max_rounds, &[]);
    let prompt = format!("<auto-verification>\n{}\n</auto-verification>", inner_prompt);

    let app = &state.app_state;
    let mut chat_service = ClaudeChatService::new(
        Arc::clone(&app.chat_message_repo),
        Arc::clone(&app.chat_attachment_repo),
        Arc::clone(&app.chat_conversation_repo),
        Arc::clone(&app.agent_run_repo),
        Arc::clone(&app.project_repo),
        Arc::clone(&app.task_repo),
        Arc::clone(&app.task_dependency_repo),
        Arc::clone(&app.ideation_session_repo),
        Arc::clone(&app.activity_event_repo),
        Arc::clone(&app.message_queue),
        Arc::clone(&app.running_agent_registry),
        Arc::clone(&app.memory_event_repo),
    )
    .with_execution_state(Arc::clone(&state.execution_state))
    .with_plan_branch_repo(Arc::clone(&app.plan_branch_repo))
    .with_task_proposal_repo(Arc::clone(&app.task_proposal_repo))
    .with_interactive_process_registry(Arc::clone(&app.interactive_process_registry));

    if let Some(ref handle) = app.app_handle {
        chat_service = chat_service.with_app_handle(handle.clone());
    }

    chat_service
        .send_message(ChatContextType::Ideation, session_id, &prompt, SendMessageOptions {
            metadata: Some(serde_json::json!({AUTO_VERIFICATION_KEY: true}).to_string()),
            created_at: Some(trigger_time),
        })
        .await
        .map(|_| ())
        .map_err(|e| format!("send_message failed: {}", e))
}

pub async fn update_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<UpdatePlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let content = req.content;

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let (created, old_artifact_id_str, sessions, linked_proposal_ids, verification_reset) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            // 1. Resolve stale IDs: walk the version chain forward to find the latest version.
            //    Makes the endpoint idempotent — agents can pass any version ID and it works.
            let old_id = ArtifactRepo::resolve_latest_sync(conn, &input_artifact_id)?;

            // 2. Get existing artifact (using resolved ID)
            let old_artifact = ArtifactRepo::get_by_id_sync(conn, &old_id)?
                .ok_or_else(|| AppError::NotFound(format!("Artifact {} not found", old_id)))?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Guard: reject update if this artifact is only referenced as an inherited plan
            if owning_sessions.is_empty() {
                let inherited =
                    SessionRepo::get_by_inherited_plan_artifact_id_sync(conn, &old_id)?;
                if !inherited.is_empty() {
                    return Err(AppError::Validation(
                        "Cannot update inherited plan. Use create_plan_artifact to create a session-specific plan.".to_string(),
                    ));
                }
            }

            // 5. Shared finalization: create version, batch-update, verification reset
            //    Does NOT trigger auto-verify — that's only in create_plan_artifact
            finalize_plan_update(conn, &old_artifact, content)
        })
        .await
        .map_err(|e| {
            error!("update_plan_artifact transaction failed: {}", e);
            map_app_err(e)
        })?;

    // Emit events outside the lock (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_state.app_handle {
        let content_text = match &created.content {
            ArtifactContent::Inline { text } => text.clone(),
            ArtifactContent::File { path } => format!("[File: {}]", path),
        };

        if verification_reset {
            if let Some(session) = sessions.first() {
                // B4: use shared helper for canonical payload (was missing round/gaps/rounds fields)
                emit_verification_status_changed(
                    app_handle,
                    session.id.as_str(),
                    VerificationStatus::Unverified,
                    false,
                    None,
                    None,
                );
            }
        }

        // Emit plan_artifact:updated event with the NEW artifact info
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

        // If there are linked proposals, emit sync notification
        if !linked_proposal_ids.is_empty() {
            let payload = PlanProposalsSyncPayload {
                artifact_id: created.id.to_string(),
                previous_artifact_id: old_artifact_id_str.clone(),
                proposal_ids: linked_proposal_ids,
                new_version: created.metadata.version,
                session_id: sessions.first().map(|s| s.id.to_string()),
                proposals_relinked: true,
            };
            let _ = app_handle.emit("plan:proposals_may_need_update", payload);
        }
    }

    let mut response = ArtifactResponse::from(created);
    response.previous_artifact_id = Some(old_artifact_id_str);
    response.session_id = sessions.first().map(|s| s.id.to_string());

    Ok(Json(response))
}

pub async fn edit_plan_artifact(
    State(state): State<HttpServerState>,
    Json(req): Json<EditPlanArtifactRequest>,
) -> Result<Json<ArtifactResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let edits = req.edits;

    // Pre-transaction input validation (defense-in-depth — MCP schema validates too)
    if edits.is_empty() {
        return Err(HttpError::validation("edits array must not be empty".to_string()));
    }
    for (i, edit) in edits.iter().enumerate() {
        if edit.old_text.is_empty() {
            return Err(HttpError::validation(format!(
                "Edit #{i}: old_text must not be empty"
            )));
        }
        if edit.old_text.len() > 100_000 || edit.new_text.len() > 100_000 {
            return Err(HttpError::validation(format!(
                "Edit #{i}: old_text/new_text exceeds 100KB limit"
            )));
        }
    }

    // Single lock acquisition: all DB work in one transaction.
    // Events emitted after db.run_transaction() returns (acceptable crash-consistency gap).
    let (created, old_artifact_id_str, sessions, linked_proposal_ids, verification_reset) = state
        .app_state
        .db
        .run_transaction(move |conn| {
            // 1. Resolve stale IDs: walk the version chain forward to find the latest version.
            //    Makes the endpoint idempotent — agents can pass any version ID and it works.
            let old_id = ArtifactRepo::resolve_latest_sync(conn, &input_artifact_id)?;

            // 2. Get existing artifact (using resolved ID)
            let old_artifact = ArtifactRepo::get_by_id_sync(conn, &old_id)?
                .ok_or_else(|| AppError::NotFound(format!("Artifact {old_id} not found")))?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions = SessionRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Guard: reject edit if this artifact is only referenced as an inherited plan
            if owning_sessions.is_empty() {
                let inherited =
                    SessionRepo::get_by_inherited_plan_artifact_id_sync(conn, &old_id)?;
                if !inherited.is_empty() {
                    return Err(AppError::Validation(
                        "Cannot edit inherited plan. Use create_plan_artifact to create a session-specific plan.".to_string(),
                    ));
                }
            }

            // 5. Guard: only inline content is supported (file-backed artifacts cannot be edited)
            let current_content = match &old_artifact.content {
                ArtifactContent::Inline { text } => text.clone(),
                ArtifactContent::File { .. } => {
                    return Err(AppError::Validation(
                        "Cannot edit file-backed artifacts. Use update_plan_artifact with full content.".to_string(),
                    ));
                }
            };

            // 6. Apply edits (pure function — returns error if any anchor not found/ambiguous)
            let new_content = apply_edits(&current_content, &edits).map_err(|e| {
                let http_err: HttpError = e.into();
                AppError::Validation(
                    http_err
                        .message
                        .unwrap_or_else(|| "Edit failed".to_string()),
                )
            })?;

            // 7. Guard: post-apply content size (prevent unbounded growth)
            if new_content.len() > 500_000 {
                return Err(AppError::Validation(format!(
                    "Resulting plan content exceeds 500KB limit ({} bytes). Use fewer/smaller edits.",
                    new_content.len()
                )));
            }

            // 8. Shared finalization: create version, batch-update, verification reset
            //    Does NOT trigger auto-verify — that's only in create_plan_artifact
            finalize_plan_update(conn, &old_artifact, new_content)
        })
        .await
        .map_err(|e| {
            error!("edit_plan_artifact transaction failed: {}", e);
            map_app_err(e)
        })?;

    // Emit events outside the lock (acceptable crash-consistency gap)
    if let Some(app_handle) = &state.app_state.app_handle {
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
                );
            }
        }

        // Emit plan_artifact:updated event with the NEW artifact info
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

        // If there are linked proposals, emit sync notification
        if !linked_proposal_ids.is_empty() {
            let payload = PlanProposalsSyncPayload {
                artifact_id: created.id.to_string(),
                previous_artifact_id: old_artifact_id_str.clone(),
                proposal_ids: linked_proposal_ids,
                new_version: created.metadata.version,
                session_id: sessions.first().map(|s| s.id.to_string()),
                proposals_relinked: true,
            };
            let _ = app_handle.emit("plan:proposals_may_need_update", payload);
        }
    }

    let mut response = ArtifactResponse::from(created);
    response.previous_artifact_id = Some(old_artifact_id_str);
    response.session_id = sessions.first().map(|s| s.id.to_string());

    Ok(Json(response))
}

pub async fn link_proposals_to_plan(
    State(state): State<HttpServerState>,
    Json(req): Json<LinkProposalsToPlanRequest>,
) -> Result<Json<SuccessResponse>, HttpError> {
    let input_artifact_id = req.artifact_id.clone();
    let proposal_id_strs = req.proposal_ids;

    // Single lock acquisition: all DB work in one transaction.
    state
        .app_state
        .db
        .run_transaction(move |conn| {
            // 1. Resolve stale artifact ID to latest version in the chain
            let artifact_id_str = ArtifactRepo::resolve_latest_sync(conn, &input_artifact_id)?;

            // 2. Verify resolved artifact exists (and get version for plan_version_at_creation)
            let artifact = ArtifactRepo::get_by_id_sync(conn, &artifact_id_str)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("Artifact {} not found", artifact_id_str))
                })?;

            // 3. Guard: reject mutations on Archived/Accepted sessions
            let owning_sessions =
                SessionRepo::get_by_plan_artifact_id_sync(conn, &artifact_id_str)?;
            if let Some(session) = owning_sessions.first() {
                crate::http_server::helpers::assert_session_mutable(session)?;
            }

            // 4. Batch-link all proposals: set plan_artifact_id + plan_version_at_creation
            ProposalRepo::batch_link_proposals_sync(
                conn,
                &proposal_id_strs,
                &artifact_id_str,
                artifact.metadata.version,
            )?;

            Ok(())
        })
        .await
        .map_err(|e| {
            error!("link_proposals_to_plan transaction failed: {}", e);
            map_app_err(e)
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        message: "Proposals linked to plan successfully".to_string(),
    }))
}

pub async fn get_session_plan(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
) -> Result<Json<Option<ArtifactResponse>>, StatusCode> {
    let session_id = IdeationSessionId::from_string(session_id);

    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get session {} for plan retrieval: {}",
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Prefer the session's own plan; fall back to the inherited plan (read-only)
    let (artifact_id, is_inherited) = if let Some(own_plan_id) = session.plan_artifact_id {
        (own_plan_id, false)
    } else if let Some(inherited_id) = session.inherited_plan_artifact_id {
        (inherited_id, true)
    } else {
        return Ok(Json(None));
    };

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get plan artifact {} for session {}: {}",
                artifact_id.as_str(),
                session_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut response = ArtifactResponse::from(artifact);
    response.is_inherited = Some(is_inherited);
    Ok(Json(Some(response)))
}

/// Get version history for a plan artifact
/// Returns list of version summaries from newest to oldest
#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod tests;

pub async fn get_artifact_history(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<Vec<ArtifactVersionSummaryResponse>>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    // Verify artifact exists
    state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get artifact {} for history: {}",
                artifact_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get the version history
    let history = state
        .app_state
        .artifact_repo
        .get_version_history(&artifact_id)
        .await
        .map_err(|e| {
            error!(
                "Failed to get history for artifact {}: {}",
                artifact_id.as_str(),
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(
        history
            .into_iter()
            .map(ArtifactVersionSummaryResponse::from)
            .collect(),
    ))
}
