// Session recovery logic for stale Claude sessions.
//
// Extracted from chat_service_send_background.rs to reduce file size.
// Handles rebuilding conversation history and spawning fresh sessions.

use std::path::Path;
use std::sync::Arc;

use super::chat_service_context;
use super::chat_service_replay::{
    build_rehydration_prompt, IdeationRecoveryMetadata, ReplayBuilder,
};
use super::chat_service_streaming::process_stream_background;
use super::streaming_state_cache::StreamingStateCache;
use crate::domain::entities::{ChatContextType, ChatConversation, ChatConversationId};
use crate::domain::repositories::{
    ChatAttachmentRepository, ChatConversationRepository, ChatMessageRepository,
    IdeationSessionRepository, TaskProposalRepository,
};
use crate::error::{AppError, AppResult};

/// Attempt to recover from a stale Claude session by rebuilding conversation history
/// and spawning a fresh session.
///
/// # Arguments
/// - `conversation_id`: The conversation ID
/// - `conversation`: The conversation entity with stale session
/// - `context_type`: The chat context type
/// - `context_id`: The context ID
/// - `new_message`: The user message that triggered the recovery
/// - `cli_path`: Path to Claude CLI
/// - `plugin_dir`: Path to plugin directory
/// - `working_directory`: Working directory for spawned commands
/// - `resolved_project_id`: Optional project ID for RALPHX_PROJECT_ID
/// - `chat_message_repo`: Message repository
/// - `conversation_repo`: Conversation repository
/// - `ideation_session_repo`: Optional ideation session repository for Ideation context
/// - `task_proposal_repo`: Optional proposal repository for Ideation context
///
/// # Returns
/// - `Ok(new_session_id)`: Recovery succeeded, new session ID
/// - `Err(AppError)`: Recovery failed
#[allow(clippy::too_many_arguments)]
pub(super) async fn attempt_session_recovery(
    conversation_id: &ChatConversationId,
    conversation: &ChatConversation,
    context_type: ChatContextType,
    context_id: &str,
    new_message: &str,
    cli_path: &Path,
    plugin_dir: &Path,
    working_directory: &Path,
    _resolved_project_id: Option<String>,
    team_mode: bool,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    conversation_repo: Arc<dyn ChatConversationRepository>,
    chat_attachment_repo: Arc<dyn ChatAttachmentRepository>,
    ideation_session_repo: Option<Arc<dyn IdeationSessionRepository>>,
    task_proposal_repo: Option<Arc<dyn TaskProposalRepository>>,
    old_session_id: &str,
) -> AppResult<String> {
    let recovery_start = std::time::Instant::now();

    // Helper closure to log failure with duration
    let log_failure = |error: &AppError| {
        tracing::error!(
            event = "rehydrate_failure",
            conversation_id = conversation_id.as_str(),
            error = %error,
            duration_ms = recovery_start.elapsed().as_millis(),
            "Session recovery failed"
        );
    };

    // 1. Build replay from history
    let replay_builder = ReplayBuilder::new(100_000); // 100K token budget
    let replay = match replay_builder
        .build_replay(&chat_message_repo, conversation_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            log_failure(&e);
            return Err(e);
        }
    };

    tracing::debug!(
        conversation_id = conversation_id.as_str(),
        turns = replay.turns.len(),
        estimated_tokens = replay.total_tokens,
        truncated = replay.is_truncated,
        "Built conversation replay for rehydration"
    );

    // 2. Build ideation recovery metadata if context is Ideation
    let ideation_metadata = if context_type == ChatContextType::Ideation {
        build_ideation_recovery_metadata(
            context_id,
            ideation_session_repo.as_ref(),
            task_proposal_repo.as_ref(),
        )
        .await
    } else {
        None
    };

    // 3. Generate rehydration prompt
    let bootstrap_prompt = build_rehydration_prompt(
        &replay,
        context_type,
        context_id,
        new_message,
        ideation_metadata.as_ref(),
    );

    // 4. Spawn fresh Claude session with history
    let spawnable = match chat_service_context::build_command(
        cli_path,
        plugin_dir,
        conversation,
        &bootstrap_prompt,
        working_directory,
        None, // entity_status
        _resolved_project_id.as_deref(),
        team_mode,
        chat_attachment_repo,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            let err = AppError::Infrastructure(format!("Failed to build recovery command: {}", e));
            log_failure(&err);
            return Err(err);
        }
    };

    let child = match spawnable.spawn().await {
        Ok(c) => c,
        Err(e) => {
            let err = AppError::Infrastructure(format!("Failed to spawn recovery session: {}", e));
            log_failure(&err);
            return Err(err);
        }
    };

    // 5. Process stream to capture new session ID
    let outcome = match process_stream_background::<tauri::Wry>(
        child,
        context_type,
        context_id,
        conversation_id,
        None,                                       // no app_handle, silent recovery
        None,                                       // no activity persistence
        None,                                       // no task repo
        None,                                       // no incremental message update
        None,                                       // no assistant message ID
        None,                                       // no question state
        tokio_util::sync::CancellationToken::new(), // standalone token for recovery
        None,                                       // no team tracker for recovery
        false,                                      // not team mode
        StreamingStateCache::new(),                 // fresh cache for recovery (no UI to hydrate)
        None,                                       // no heartbeat for recovery sessions
        None,                                       // no agent_run_repo for recovery
        None,                                       // no agent_run_id for recovery
        None,                                       // no execution state for recovery
        None,                                       // no conversation_repo for recovery
    )
    .await
    {
        Ok(o) => o,
        Err(e) => {
            let err = AppError::Infrastructure(format!("Recovery stream processing failed: {}", e));
            log_failure(&err);
            return Err(err);
        }
    };

    let new_session_id = match outcome.session_id {
        Some(id) => id,
        None => {
            let err = AppError::Infrastructure("Recovery failed: no session ID captured".into());
            log_failure(&err);
            return Err(err);
        }
    };

    // 6. Update conversation with new session ID
    if let Err(e) = conversation_repo
        .update_claude_session_id(conversation_id, &new_session_id)
        .await
    {
        let err = AppError::Database(format!("Failed to update session ID: {}", e));
        log_failure(&err);
        return Err(err);
    }

    // 7. Log telemetry
    tracing::info!(
        event = "rehydrate_success",
        conversation_id = conversation_id.as_str(),
        old_session_id = old_session_id,
        new_session_id = %new_session_id,
        replay_turns = replay.turns.len(),
        estimated_tokens = replay.total_tokens,
        duration_ms = recovery_start.elapsed().as_millis(),
    );

    Ok(new_session_id)
}

/// Build ideation recovery metadata from repositories.
///
/// Fetches the ideation session and counts proposals to populate metadata
/// for enriching the recovery prompt with ideation-specific context.
async fn build_ideation_recovery_metadata(
    context_id: &str,
    ideation_session_repo: Option<&Arc<dyn IdeationSessionRepository>>,
    task_proposal_repo: Option<&Arc<dyn TaskProposalRepository>>,
) -> Option<IdeationRecoveryMetadata> {
    // Both repositories are required for ideation metadata
    let (session_repo, proposal_repo) = (ideation_session_repo?, task_proposal_repo?);

    // Parse context_id as IdeationSessionId
    let session_id =
        crate::domain::entities::IdeationSessionId::from_string(context_id.to_string());

    // Fetch the session
    let session = match session_repo.get_by_id(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::warn!(
                session_id = session_id.as_str(),
                "Ideation session not found for recovery metadata"
            );
            return None;
        }
        Err(e) => {
            tracing::error!(
                session_id = session_id.as_str(),
                error = %e,
                "Failed to fetch ideation session for recovery metadata"
            );
            return None;
        }
    };

    // Count proposals for this session
    let proposal_count = match proposal_repo.count_by_session(&session_id).await {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!(
                session_id = session_id.as_str(),
                error = %e,
                "Failed to count proposals for recovery metadata, using 0"
            );
            0
        }
    };

    // Extract verification state before (potentially) resetting it
    let verification_was_in_progress = session.verification_in_progress;
    let verification_status_str = session.verification_status.to_string();
    let current_round = session
        .verification_metadata
        .as_deref()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|v| v.get("current_round").and_then(|r| r.as_u64()))
        .map(|r| r as u32)
        .unwrap_or(0);

    // If verification was in-progress when the session crashed, force-reset it.
    // A stuck `verification_in_progress=1` would block reconciliation and confuse the recovered agent.
    // Use update_verification_state (unconditional) because reset_verification() guards on
    // in_progress=false (it is only for conditional resets on plan artifact updates).
    if verification_was_in_progress {
        if let Err(e) = session_repo
            .update_verification_state(
                &session_id,
                crate::domain::entities::VerificationStatus::Unverified,
                false,
                None,
            )
            .await
        {
            tracing::warn!(
                session_id = session_id.as_str(),
                error = %e,
                "Failed to reset verification state during session recovery"
            );
        } else {
            tracing::info!(
                session_id = session_id.as_str(),
                round = current_round,
                "Verification in-progress reset during session recovery"
            );
        }
    }

    Some(IdeationRecoveryMetadata {
        session_status: session.status.to_string(),
        plan_artifact_id: session.plan_artifact_id.map(|id| id.to_string()),
        proposal_count,
        parent_session_id: session.parent_session_id.map(|id| id.to_string()),
        team_mode: session.team_mode,
        session_title: session.title,
        verification_status: verification_status_str,
        verification_in_progress: verification_was_in_progress,
        current_round,
    })
}

#[cfg(test)]
#[path = "chat_service_recovery_tests.rs"]
mod tests;
