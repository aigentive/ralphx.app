use super::*;

/// POST /api/ideation/sessions/:id/verification
///
/// Update verification state for a session's plan (from MCP orchestrator).
/// Validates the state machine transition and persists gap metadata.
pub async fn update_plan_verification(
    State(state): State<HttpServerState>,
    Path(session_id): Path<String>,
    Json(req): Json<UpdateVerificationRequest>,
) -> Result<Json<VerificationResponse>, JsonError> {
    use std::collections::HashSet;
    use crate::domain::entities::ideation::{
        VerificationGap, VerificationMetadata, VerificationRound, VerificationStatus,
    };
    use crate::domain::services::{gap_fingerprint, gap_score, jaccard_similarity};

    let session_id_obj = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());

    // Fetch session
    let session = state
        .app_state
        .ideation_session_repo
        .get_by_id(&session_id_obj)
        .await
        .map_err(|e| {
            error!("Failed to get session {}: {}", session_id, e);
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get session: {}", e),
            )
        })?
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "Session not found"))?;

    // Guard: reject calls targeting verification child sessions — plan-verifier must use parent session_id
    if session.session_purpose == crate::domain::entities::SessionPurpose::Verification {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Cannot update verification state on a verification child session. Use the parent session_id.",
        ));
    }

    // Server-side generation guard: when generation is provided, verify it matches.
    // Applies to ALL calls (including terminal in_progress=false) to prevent zombie agents
    // from writing stale terminal status (e.g., verified/needs_revision after a reset).
    if let Some(req_gen) = req.generation {
        if req_gen != session.verification_generation {
            return Err(json_error(
                StatusCode::CONFLICT,
                format!(
                    "Generation mismatch: request generation {} != current generation {}. \
                     Verification was reset — zombie agent detected.",
                    req_gen, session.verification_generation
                ),
            ));
        }
    }

    // Parse new status (mut — server-side convergence conditions may override)
    let mut new_status: VerificationStatus = req.status.parse().map_err(|_| {
        json_error(
            StatusCode::BAD_REQUEST,
            format!("Invalid status: {}", req.status),
        )
    })?;
    // in_progress may be overridden by condition 6 (reviewing+gaps → needs_revision)
    let mut effective_in_progress = req.in_progress;

    // Guard: external sessions cannot skip plan verification
    if new_status == VerificationStatus::Skipped
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
    {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "External sessions cannot skip plan verification. Run verification to completion (update_plan_verification with status 'reviewing').",
        ));
    }

    // Transition validation matrix
    let current = session.verification_status;
    let has_convergence_reason = req.convergence_reason.is_some();
    let is_valid = match (current, new_status) {
        (_, VerificationStatus::Skipped) => true,
        // Skipped can transition to Reviewing to allow users to verify after skipping
        (VerificationStatus::Skipped, VerificationStatus::Reviewing) => true,
        (VerificationStatus::Skipped, _) => false,
        (VerificationStatus::Unverified, VerificationStatus::Reviewing) => true,
        (VerificationStatus::Reviewing, VerificationStatus::NeedsRevision) => true,
        (VerificationStatus::Reviewing, VerificationStatus::Verified) => true,
        (VerificationStatus::NeedsRevision, VerificationStatus::Reviewing) => true,
        // Allow needs_revision → verified ONLY when convergence_reason is provided
        (VerificationStatus::NeedsRevision, VerificationStatus::Verified) => has_convergence_reason,
        // ImportedVerified can transition to Reviewing to re-run verification if desired
        (VerificationStatus::ImportedVerified, VerificationStatus::Reviewing) => true,
        // Verified can transition to Reviewing to re-run verification
        (VerificationStatus::Verified, VerificationStatus::Reviewing) => true,
        _ => false,
    };

    if !is_valid {
        if matches!(current, VerificationStatus::Skipped) {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Verification was skipped — cannot update from critic",
            ));
        }
        if matches!(
            (current, new_status),
            (VerificationStatus::NeedsRevision, VerificationStatus::Verified)
        ) {
            return Err(json_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Cannot transition needs_revision → verified without convergence_reason",
            ));
        }
        return Err(json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "Invalid verification transition: {} → {}",
                current, new_status
            ),
        ));
    }

    // Re-verify fast path: terminal → Reviewing (Verified, Skipped, ImportedVerified)
    // Atomically clears stale metadata + increments generation + sets Reviewing+in_progress.
    // Skips update_verification_state entirely — reset_and_begin_reverify does everything.
    let is_reverify = matches!(
        current,
        VerificationStatus::Verified
            | VerificationStatus::Skipped
            | VerificationStatus::ImportedVerified
    ) && new_status == VerificationStatus::Reviewing;

    if is_reverify {
        let (new_gen, cleared_metadata) = state
            .app_state
            .ideation_session_repo
            .reset_and_begin_reverify(&session_id)
            .await
            .map_err(|e| {
                error!("Failed to reset verification for {}: {}", session_id, e);
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to reset verification state",
                )
            })?;

        tracing::info!(
            session_id = %session_id,
            from_status = %current,
            new_gen = new_gen,
            "Re-verify: stale metadata cleared, generation incremented"
        );

        if let Some(app_handle) = &state.app_state.app_handle {
            emit_verification_status_changed(
                app_handle,
                &session_id,
                VerificationStatus::Reviewing,
                true,
                Some(&cleared_metadata),
                None,
                Some(new_gen),
            );
        }

        return Ok(Json(VerificationResponse {
            session_id,
            status: VerificationStatus::Reviewing.to_string(),
            in_progress: true,
            current_round: None,
            max_rounds: None,
            gap_score: Some(0),
            convergence_reason: None,
            best_round_index: None,
            current_gaps: vec![],
            rounds: vec![],
            plan_version: None,
            verification_generation: new_gen,
            verification_child: None,
        }));
    }

    // Build/update metadata
    let mut metadata: VerificationMetadata = session
        .verification_metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    if let Some(max_r) = req.max_rounds {
        metadata.max_rounds = max_r;
    }

    // Process gaps if provided
    if let Some(ref gaps_req) = req.gaps {
        let gaps: Vec<VerificationGap> = gaps_req
            .iter()
            .map(|g| VerificationGap {
                severity: g.severity.clone(),
                category: g.category.clone(),
                description: g.description.clone(),
                why_it_matters: g.why_it_matters.clone(),
                source: g.source.clone(),
            })
            .collect();

        let fingerprints: Vec<String> = gaps
            .iter()
            .map(|g| gap_fingerprint(&g.description))
            .collect();
        let score = gap_score(&gaps);

        if let Some(round) = req.round {
            metadata.current_round = round;
        }

        // ── Server-side convergence evaluation (D3) ──
        // Evaluate before pushing new round — metadata.current_gaps = previous round's gaps.

        // Condition 1: 0 critical AND 0 high AND 0 medium (zero_blocking, AD3)
        let critical_count = gaps_req.iter().filter(|g| g.severity == "critical").count() as u32;
        let high_count = gaps_req.iter().filter(|g| g.severity == "high").count() as u32;
        let medium_count = gaps_req.iter().filter(|g| g.severity == "medium").count() as u32;
        let zero_blocking_converged = critical_count == 0 && high_count == 0 && medium_count == 0;

        // Condition 2: Jaccard ≥ 0.8 for 2 consecutive rounds (R4-C2)
        let jaccard_converged = if metadata.rounds.len() >= 2 {
            let prev_round = metadata.rounds.last().unwrap();
            let prev_prev_round = &metadata.rounds[metadata.rounds.len() - 2];
            let new_fp_set: HashSet<String> = fingerprints.iter().cloned().collect();
            let prev_fp_set: HashSet<String> = prev_round.fingerprints.iter().cloned().collect();
            let prev_prev_fp_set: HashSet<String> =
                prev_prev_round.fingerprints.iter().cloned().collect();
            let jaccard_curr = jaccard_similarity(&new_fp_set, &prev_fp_set);
            let jaccard_prev = jaccard_similarity(&prev_fp_set, &prev_prev_fp_set);
            tracing::info!(
                session_id = %session_id,
                round = metadata.current_round,
                jaccard_curr = jaccard_curr,
                jaccard_prev = jaccard_prev,
                "Verification Jaccard similarity (2-round check)"
            );
            jaccard_curr >= 0.8 && jaccard_prev >= 0.8
        } else if metadata.rounds.len() == 1 {
            let prev_round = metadata.rounds.last().unwrap();
            let new_fp_set: HashSet<String> = fingerprints.iter().cloned().collect();
            let prev_fp_set: HashSet<String> = prev_round.fingerprints.iter().cloned().collect();
            let jaccard = jaccard_similarity(&new_fp_set, &prev_fp_set);
            tracing::info!(
                session_id = %session_id,
                round = metadata.current_round,
                jaccard = jaccard,
                "Verification Jaccard similarity (need 2 consecutive rounds for convergence)"
            );
            false // need at least 2 consecutive rounds
        } else {
            false
        };

        // Track best version (lowest gap_score)
        let round_idx = metadata.rounds.len() as u32;
        let is_better = metadata.best_round_index.is_none() || {
            let best_idx = metadata.best_round_index.unwrap() as usize;
            metadata
                .rounds
                .get(best_idx)
                .map(|r| r.gap_score)
                .unwrap_or(u32::MAX)
                > score
        };
        if is_better {
            metadata.best_round_index = Some(round_idx);
        }

        metadata
            .rounds
            .push(VerificationRound { fingerprints, gap_score: score });
        metadata.current_gaps = gaps;

        // Auto-converge: override NeedsRevision → Verified when conditions are met
        if new_status == VerificationStatus::NeedsRevision {
            // R1 empty round guard: require at least round 2 before zero_blocking convergence.
            // Round 1 with 0 gaps may be a broken critic — need round 2 to confirm.
            let current_round_for_convergence = req.round.unwrap_or(metadata.current_round);
            if zero_blocking_converged && current_round_for_convergence >= 2 {
                new_status = VerificationStatus::Verified;
                if metadata.convergence_reason.is_none() {
                    metadata.convergence_reason = Some("zero_blocking".to_string());
                }
                tracing::info!(
                    session_id = %session_id,
                    round = current_round_for_convergence,
                    "Server-side convergence: 0 critical + 0 high + 0 medium → Verified"
                );
            } else if jaccard_converged {
                new_status = VerificationStatus::Verified;
                if metadata.convergence_reason.is_none() {
                    metadata.convergence_reason = Some("jaccard_converged".to_string());
                }
                tracing::info!(
                    session_id = %session_id,
                    "Server-side convergence: Jaccard ≥ 0.8 × 2 rounds → Verified"
                );
            }
        }
    }

    // Condition 3: max_rounds hard cap (R4-H3)
    if !matches!(new_status, VerificationStatus::Verified | VerificationStatus::Skipped) {
        let current_round = req.round.unwrap_or(metadata.current_round);
        if metadata.max_rounds > 0 && current_round >= metadata.max_rounds {
            new_status = VerificationStatus::Verified;
            if metadata.convergence_reason.is_none() {
                metadata.convergence_reason = Some("max_rounds".to_string());
            }
            tracing::info!(
                session_id = %session_id,
                round = current_round,
                max_rounds = metadata.max_rounds,
                "Server-side convergence: max_rounds reached → Verified"
            );
        }
    }

    // Condition 4: parse failure tracking — sliding window ≥ 3 of last 5 rounds (R4-M3)
    if req.parse_failed == Some(true) {
        if let Some(round) = req.round {
            metadata.parse_failures.push(round);
        }
        let last_5_failures = metadata.parse_failures.iter().rev().take(5).count();
        if last_5_failures >= 3
            && !matches!(new_status, VerificationStatus::Verified | VerificationStatus::Skipped)
        {
            new_status = VerificationStatus::Verified;
            if metadata.convergence_reason.is_none() {
                metadata.convergence_reason = Some("critic_parse_failure".to_string());
            }
            tracing::warn!(
                session_id = %session_id,
                failures = last_5_failures,
                "Server-side convergence: critic parse failures ≥ 3/5 → Verified"
            );
        }
    }

    if let Some(ref reason) = req.convergence_reason {
        // Orchestrator-provided reason takes precedence only if not already set server-side
        if metadata.convergence_reason.is_none() {
            metadata.convergence_reason = Some(reason.clone());
        }
    }

    // Condition 6: reviewing with gaps → needs_revision (auto-override, placed after convergence
    // checks so convergence always takes priority). Triggers on ANY gap severity.
    // Rule A: do NOT force effective_in_progress = false here — the verification loop is still
    // active when there is no convergence_reason. Preserve the caller's in_progress value.
    // The terminal convergence guard below handles the in_progress reset for terminal states.
    // TODO: Extract auto-transition logic to domain service state machine
    if new_status == VerificationStatus::Reviewing && !metadata.current_gaps.is_empty() {
        new_status = VerificationStatus::NeedsRevision;
        tracing::info!(
            session_id = %session_id,
            gap_count = metadata.current_gaps.len(),
            "Server-side auto-transition: reviewing with gaps → NeedsRevision"
        );
    }

    // Terminal convergence guard (Rule B): after all convergence evaluation and auto-transition
    // logic completes, force effective_in_progress = false whenever the session has reached a
    // terminal state. This catches auto-convergence paths (conditions 1–4, ~lines 287/297/313/335)
    // that override status to Verified without explicitly resetting in_progress, as well as any
    // path where the orchestrator provides a convergence_reason.
    if metadata.convergence_reason.is_some()
        || matches!(
            new_status,
            VerificationStatus::Verified | VerificationStatus::Skipped
        )
    {
        effective_in_progress = false;
    }

    let current_gap_score = gap_score(&metadata.current_gaps);
    let metadata_json = serde_json::to_string(&metadata).ok();

    // Persist state
    state
        .app_state
        .ideation_session_repo
        .update_verification_state(&session_id_obj, new_status, effective_in_progress, metadata_json)
        .await
        .map_err(|e| {
            error!(
                "Failed to update verification state for {}: {}",
                session_id, e
            );
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update verification state",
            )
        })?;

    tracing::info!(
        session_id = %session_id,
        status = %new_status,
        round = ?req.round,
        "Verification state updated"
    );

    // For skipped sessions, stop the verification child immediately.
    // Verified sessions defer child shutdown until after verified-side effects complete so
    // external follow-on work (event emission, auto-propose) is not cut off by the child's
    // own termination.
    if matches!(new_status, VerificationStatus::Skipped) {
        stop_verification_children(&session_id, &state.app_state).await.ok();
    }

    // Defense-in-depth: increment generation on skip so any in-flight zombie agent
    // calls get rejected with 409 Conflict.
    if matches!(new_status, VerificationStatus::Skipped) {
        state
            .app_state
            .ideation_session_repo
            .increment_verification_generation(&session_id_obj)
            .await
            .ok();
    }

    // Emit plan_verification:status_changed event (B1: includes current_gaps + last 5 rounds)
    if let Some(app_handle) = &state.app_state.app_handle {
        emit_verification_status_changed(
            app_handle,
            &session_id,
            new_status,
            effective_in_progress,
            Some(&metadata),
            None,
            Some(session.verification_generation),
        );
    }

    // Layer 2+3 for IdeationVerified — only when new_status == Verified (non-fatal)
    if new_status == VerificationStatus::Verified {
        tracing::info!(
            session_id = %session_id,
            convergence_reason = ?metadata.convergence_reason,
            origin = %session.origin,
            "Verification reached terminal verified state — running verified side effects"
        );

        // Project lookup for webhook enrichment (non-fatal if not found)
        let project_name = state
            .app_state
            .project_repo
            .get_by_id(&session.project_id)
            .await
            .ok()
            .flatten()
            .map(|p| p.name);

        let presentation_ctx = crate::domain::services::WebhookPresentationContext {
            project_name,
            session_title: session.title.clone(),
            presentation_kind: Some(crate::domain::services::PresentationKind::Verified),
            task_title: None,
        };

        let mut verified_payload = serde_json::json!({
            "session_id": session_id,
            "project_id": session.project_id.as_str(),
            "convergence_reason": metadata.convergence_reason,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Enrich payload for external channel
        presentation_ctx.inject_into(&mut verified_payload);

        // External emit via mandatory helper
        if let Some(ref publisher) = state.app_state.webhook_publisher {
            if let Err(msg) = crate::domain::services::emit_external_webhook_event(
                "ideation:verified",
                session.project_id.as_str(),
                verified_payload,
                &state.app_state.external_events_repo,
                publisher,
            )
            .await
            {
                tracing::warn!(error = %msg, session_id = %session_id, "Failed to emit ideation:verified external event (non-fatal)");
            }
        } else if let Err(e) = state
            .app_state
            .external_events_repo
            .insert_event(
                "ideation:verified",
                session.project_id.as_str(),
                &verified_payload.to_string(),
            )
            .await
        {
            tracing::warn!(error = %e, session_id = %session_id, "Failed to persist ideation:verified event");
        }
    }

    // Auto-propose for external sessions that converged via zero_blocking.
    // Run this detached from the verifier's request lifecycle so the child session can finish
    // cleanly without cancelling the follow-on orchestrator spawn mid-flight.
    if new_status == VerificationStatus::Verified
        && metadata.convergence_reason.as_deref() == Some("zero_blocking")
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
    {
        tracing::info!(
            session_id = %session_id,
            "Scheduling external auto-propose after zero_blocking convergence"
        );
        let state_for_auto_propose = state.clone();
        let session_for_auto_propose = session.clone();
        let session_id_for_auto_propose = session_id.clone();
        tauri::async_runtime::spawn(async move {
            auto_propose_for_external(
                &session_id_for_auto_propose,
                &session_for_auto_propose,
                &state_for_auto_propose,
            )
            .await;
        });
    }

    // For external sessions that reach Verified WITHOUT auto-propose (non-zero_blocking):
    // transition to "ready" immediately since there's no proposing phase
    if new_status == VerificationStatus::Verified
        && session.origin == crate::domain::entities::ideation::SessionOrigin::External
        && metadata.convergence_reason.as_deref() != Some("zero_blocking")
    {
        let sid = crate::domain::entities::IdeationSessionId::from_string(session_id.clone());
        if let Err(e) = state
            .app_state
            .ideation_session_repo
            .update_external_activity_phase(&sid, Some("ready"))
            .await
        {
            error!(
                "Failed to set activity phase 'ready' for session {}: {}",
                sid.as_str(),
                e
            );
        }
    }

    // Verified sessions stop verification children only after their follow-on side effects
    // have been scheduled/emitted. This avoids cutting off the external auto-propose path
    // when the verifier child is itself the caller that reported Verified.
    if matches!(new_status, VerificationStatus::Verified) {
        stop_verification_children(&session_id, &state.app_state).await.ok();
    }

    use crate::http_server::types::{VerificationGapResponse, VerificationRoundSummary};

    let post_current_gaps = metadata
        .current_gaps
        .iter()
        .map(|g| VerificationGapResponse {
            severity: g.severity.clone(),
            category: g.category.clone(),
            description: g.description.clone(),
            why_it_matters: g.why_it_matters.clone(),
            source: g.source.clone(),
        })
        .collect::<Vec<_>>();

    let post_rounds = metadata
        .rounds
        .iter()
        .enumerate()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|(i, r)| VerificationRoundSummary {
            round: (i + 1) as u32,
            gap_score: r.gap_score,
            gap_count: r.fingerprints.len() as u32,
        })
        .collect::<Vec<_>>();

    Ok(Json(VerificationResponse {
        session_id,
        status: new_status.to_string(),
        in_progress: effective_in_progress,
        current_round: if metadata.current_round > 0 {
            Some(metadata.current_round)
        } else {
            None
        },
        max_rounds: if metadata.max_rounds > 0 {
            Some(metadata.max_rounds)
        } else {
            None
        },
        gap_score: Some(current_gap_score),
        convergence_reason: metadata.convergence_reason,
        best_round_index: metadata.best_round_index,
        current_gaps: post_current_gaps,
        rounds: post_rounds,
        plan_version: None,
        verification_generation: session.verification_generation,
        verification_child: None,
    }))
}
