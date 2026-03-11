use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tauri::Emitter;
use tracing::error;

use super::*;
use crate::application::chat_service::{ChatService, ClaudeChatService};
use crate::domain::entities::{
    Artifact, ArtifactBucketId, ArtifactContent, ArtifactId, ArtifactMetadata, ArtifactType,
    ChatContextType, IdeationSessionId, VerificationStatus,
};
use crate::domain::services::emit_verification_status_changed;
use crate::error::AppError;
use crate::infrastructure::agents::claude::verification_config;
use crate::infrastructure::sqlite::{
    SqliteArtifactRepository as ArtifactRepo, SqliteIdeationSessionRepository as SessionRepo,
    SqliteTaskProposalRepository as ProposalRepo,
};

/// Map an AppError to an HttpError for handler responses.
fn map_app_err(e: AppError) -> HttpError {
    match e {
        AppError::Validation(msg) => HttpError::validation(msg),
        AppError::NotFound(_) => StatusCode::NOT_FOUND.into(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into(),
    }
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
async fn spawn_auto_verifier(
    state: &HttpServerState,
    session_id: &str,
    generation: i32,
) -> Result<(), String> {
    let cfg = verification_config();
    let max_rounds = cfg.max_rounds;

    let prompt = format!(
        "AUTO-VERIFICATION MODE for session {session_id}. Generation: {generation}. Max rounds: {max_rounds}. NO user is present — run the complete verification loop WITHOUT waiting for user input.\n\
         \n\
         ## MANDATORY FIRST STEP\n\
         \n\
         Call get_session_plan(session_id: \"{session_id}\") to read the current plan.\n\
         Store the artifact_id from the response (you will need it for update_plan_artifact calls).\n\
         If the plan content exceeds 3000 tokens, truncate it and prepend: \"TRUNCATED TO 3000 TOKENS:\"\n\
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
         ### C. LAYER 1 — Completeness Critic (always run)\n\
         Spawn a Task(general-purpose) subagent with the following prompt (insert the plan content where indicated):\n\
         \n\
         ---LAYER1-PROMPT-START---\n\
         You are an adversarial plan critic. Review the following plan for gaps, risks, and missing details.\n\
         \n\
         OUTPUT FORMAT: You MUST respond with ONLY a JSON object in this exact format, no prose before or after:\n\
         {{\n\
           \"gaps\": [\n\
             {{\n\
               \"severity\": \"critical|high|medium|low\",\n\
               \"category\": \"architecture|security|testing|performance|scalability|maintainability|completeness\",\n\
               \"description\": \"Concise description of the gap\",\n\
               \"why_it_matters\": \"Concrete impact if not addressed\"\n\
             }}\n\
           ],\n\
           \"summary\": \"One-sentence synthesis of the plan's main risk\"\n\
         }}\n\
         \n\
         Severity guide:\n\
         - critical: Blocks implementation or causes data loss/security breach\n\
         - high: Significant rework required if not addressed\n\
         - medium: Adds risk but workable with care\n\
         - low: Nice-to-have improvement\n\
         \n\
         PLAN CONTENT:\n\
         [insert plan content here]\n\
         ---LAYER1-PROMPT-END---\n\
         \n\
         ### D. LAYER 2 — Alpha + Beta Critics (run only if plan contains code indicators)\n\
         Check if the plan content matches: /(?:src[-\\/]|\\.rs\\b|\\.tsx?\\b|Affected Files|## Implementation)/\n\
         If it matches, emit BOTH Task calls in ONE response (parallel execution):\n\
         \n\
         Alpha Task prompt (insert plan content where indicated):\n\
         ---ALPHA-PROMPT-START---\n\
         You are reviewing an implementation plan. Argue for the MINIMAL fix. Read the actual code at proposed locations if file paths are given. Find functional gaps — scenarios where the proposed changes would fail, cause regressions, or miss edge cases. Rate each gap CRITICAL/HIGH/MEDIUM/LOW. Focus: Is this change sufficient? What can be safely skipped?\n\
         \n\
         OUTPUT FORMAT: ONLY a JSON object:\n\
         {{\n\
           \"gaps\": [\n\
             {{\n\
               \"severity\": \"critical|high|medium|low\",\n\
               \"category\": \"architecture|security|testing|performance|scalability|maintainability|completeness\",\n\
               \"description\": \"Concise gap with specific scenario (\\\"if X happens, Y breaks because Z does W\\\")\",\n\
               \"why_it_matters\": \"What breaks if not addressed\"\n\
             }}\n\
           ],\n\
           \"summary\": \"One-sentence assessment from the minimal-fix perspective\"\n\
         }}\n\
         \n\
         PLAN: [insert plan content here]\n\
         ---ALPHA-PROMPT-END---\n\
         \n\
         Beta Task prompt (insert plan content where indicated):\n\
         ---BETA-PROMPT-START---\n\
         You are reviewing an implementation plan. Argue for COMPREHENSIVE defense-in-depth. Read the actual code at proposed locations if file paths are given. Find functional gaps the minimal approach would miss — race conditions, uncovered code paths, missing cleanup. Rate each gap CRITICAL/HIGH/MEDIUM/LOW. Focus: What additional protections are needed? What paths are left unguarded?\n\
         \n\
         OUTPUT FORMAT: ONLY a JSON object:\n\
         {{\n\
           \"gaps\": [\n\
             {{\n\
               \"severity\": \"critical|high|medium|low\",\n\
               \"category\": \"architecture|security|testing|performance|scalability|maintainability|completeness\",\n\
               \"description\": \"Concise gap with specific scenario (\\\"if X happens, Y breaks because Z does W\\\")\",\n\
               \"why_it_matters\": \"What breaks if not addressed\"\n\
             }}\n\
           ],\n\
           \"summary\": \"One-sentence assessment from the comprehensive-defense perspective\"\n\
         }}\n\
         \n\
         PLAN: [insert plan content here]\n\
         ---BETA-PROMPT-END---\n\
         \n\
         ### E. Merge and deduplicate gaps\n\
         Collect all gaps from all Task agents that completed. Deduplicate by description similarity (merge gaps where descriptions are >80% similar). Assign the higher severity when merging.\n\
         \n\
         ### F. Report to backend\n\
         Call: update_plan_verification(session_id: \"{session_id}\", status: \"reviewing\", in_progress: true, round: N, gaps: [all_gaps], generation: {generation})\n\
         NOTE: Always send status: \"reviewing\" — the backend auto-corrects to \"needs_revision\" when appropriate. NEVER send \"needs_revision\" directly.\n\
         \n\
         ### G. EMPTY ROUND GUARD\n\
         If all_gaps is empty AND round == 1:\n\
         - Call update_plan_verification with 0 gaps as above\n\
         - CONTINUE to round 2 automatically — do NOT stop. Empty round 1 requires confirmation in round 2.\n\
         \n\
         ### H. Revise plan if gaps found\n\
         Compute gap_score = (critical_count × 10) + (high_count × 3) + (medium_count × 1).\n\
         If gap_score > 0:\n\
         - Call get_session_plan(\"{session_id}\") to get the latest plan content\n\
         - Synthesize fixes for ALL critical and high gaps; address medium gaps if straightforward\n\
         - Write a revised plan — ONLY modify sections related to identified gaps, NEVER remove user content\n\
         - Call update_plan_artifact(artifact_id: <current_artifact_id>, content: <revised_plan_content>)\n\
         - Store the new artifact_id returned from the response\n\
         \n\
         ### I. CONVERGENCE CHECK\n\
         Call get_plan_verification(\"{session_id}\").\n\
         If convergence_reason is not null, proceed to FINAL CLEANUP.\n\
         \n\
         ### J. Max rounds check\n\
         If round >= {max_rounds}: proceed to FINAL CLEANUP with convergence_reason: \"max_rounds\".\n\
         \n\
         ### K. Re-read and loop\n\
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
         - zero_critical: 0 critical + high_count <= prev round + 0 medium from Layer 2\n\
         - jaccard_converged: Jaccard similarity >= 0.8 between consecutive rounds\n\
         - max_rounds: round >= {max_rounds}\n\
         - critic_parse_failure: >= 3 parse failures in 5 rounds\n\
         The backend computes convergence — always check get_plan_verification for convergence_reason after each update_plan_verification call.",
        session_id = session_id,
        generation = generation,
        max_rounds = max_rounds,
    );

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
        .send_message(ChatContextType::Ideation, session_id, &prompt)
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

            // 5. Create NEW artifact with incremented version (version chain, not in-place update)
            let new_artifact = Artifact {
                id: ArtifactId::new(),
                artifact_type: old_artifact.artifact_type.clone(),
                name: old_artifact.name.clone(),
                content: ArtifactContent::Inline { text: content },
                metadata: ArtifactMetadata::new(&old_artifact.metadata.created_by)
                    .with_version(old_artifact.metadata.version + 1),
                derived_from: vec![],
                bucket_id: old_artifact.bucket_id.clone(),
            };
            let created =
                ArtifactRepo::create_with_previous_version_sync(conn, new_artifact, &old_id)?;

            // 6. Batch-update all sessions pointing to old artifact to point to new one
            let session_ids: Vec<String> = owning_sessions
                .iter()
                .map(|s| s.id.as_str().to_string())
                .collect();
            SessionRepo::batch_update_artifact_id_sync(conn, &session_ids, created.id.as_str())?;

            // 7. Fetch proposals linked to old artifact (before batch-updating them)
            let linked_proposals = ProposalRepo::get_by_plan_artifact_id_sync(conn, &old_id)?;
            let linked_proposal_ids: Vec<String> =
                linked_proposals.iter().map(|p| p.id.to_string()).collect();

            // 8. Batch-update all linked proposals to point to the new artifact version.
            //    plan_version_at_creation is intentionally NOT changed (preserves original birth version).
            ProposalRepo::batch_update_artifact_id_sync(conn, &old_id, created.id.as_str())?;

            // 9. Conditionally reset verification — only when verification_in_progress = 0.
            //    Prevents the loop-reset paradox where auto-corrections reset verification mid-loop.
            let reset = if let Some(session) = owning_sessions.first() {
                SessionRepo::reset_verification_sync(conn, session.id.as_str())?
            } else {
                false
            };

            Ok((created, old_id, owning_sessions, linked_proposal_ids, reset))
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

pub async fn get_plan_artifact(
    State(state): State<HttpServerState>,
    Path(artifact_id): Path<String>,
) -> Result<Json<ArtifactResponse>, StatusCode> {
    let artifact_id = ArtifactId::from_string(artifact_id);

    let artifact = state
        .app_state
        .artifact_repo
        .get_by_id(&artifact_id)
        .await
        .map_err(|e| {
            error!("Failed to get artifact {}: {}", artifact_id.as_str(), e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ArtifactResponse::from(artifact)))
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

pub async fn get_plan_artifact_history(
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
