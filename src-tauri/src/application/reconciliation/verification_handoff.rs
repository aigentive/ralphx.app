//! Verification result handoff — synthesizes and injects `<verification-result>` XML
//! into the parent ideation session's conversation when reconcile returns `NeedsRevision`.
//!
//! The message is stored directly via repos (fire-and-forget) rather than going through
//! the full `ClaudeChatService` stack, because the handler call site only has access to
//! individual repos (not `artifact_repo` / `project_repo` needed for ClaudeChatService).

use std::sync::Arc;

use tracing::warn;

use crate::domain::entities::{
    ChatContextType, ChatConversation, ChatMessage, IdeationSessionId, VerificationGap,
    VerificationMetadata, VerificationStatus,
};
use crate::domain::repositories::{ChatConversationRepository, ChatMessageRepository};
use crate::domain::services::MessageQueue;

/// Dedup guard: skip synthesis when plan-verifier already delivered a structured
/// `<escalation type="verification">` message via the same parent session.
pub(crate) const ESCALATED_TO_PARENT: &str = "escalated_to_parent";

/// Result returned by `reconcile_verification_on_child_complete`.
///
/// `None` is returned for early exits (parent not found, already resolved, etc.).
/// `Some` is returned when reconciliation ran and determined a terminal status.
pub struct ReconcileChildCompleteResult {
    pub terminal_status: VerificationStatus,
    pub parsed_meta: Option<VerificationMetadata>,
}

/// Inject a `<verification-result>` XML message into the parent ideation session when:
/// - `result.terminal_status == NeedsRevision`, AND
/// - `convergence_reason != "escalated_to_parent"` (dedup guard)
///
/// Fire-and-forget: logs errors but never blocks the caller.
pub async fn maybe_inject_verification_result_message(
    parent_id: &IdeationSessionId,
    result: &ReconcileChildCompleteResult,
    conversation_repo: &Arc<dyn ChatConversationRepository>,
    chat_message_repo: &Arc<dyn ChatMessageRepository>,
    message_queue: &Arc<MessageQueue>,
) {
    // Only synthesize for NeedsRevision terminal status
    if result.terminal_status != VerificationStatus::NeedsRevision {
        return;
    }

    // Dedup guard: skip when escalation was already delivered to the parent
    let convergence_reason = result
        .parsed_meta
        .as_ref()
        .and_then(|m| m.convergence_reason.as_deref());
    if convergence_reason == Some(ESCALATED_TO_PARENT) {
        return;
    }

    // Build XML payload
    let payload = format_verification_result_xml(
        parent_id.as_str(),
        convergence_reason,
        result
            .parsed_meta
            .as_ref()
            .map(|m| m.current_round)
            .unwrap_or(0),
        result
            .parsed_meta
            .as_ref()
            .map(|m| m.max_rounds)
            .unwrap_or(0),
        result
            .parsed_meta
            .as_ref()
            .map(|m| m.current_gaps.as_slice())
            .unwrap_or(&[]),
    );

    // Find or create the active conversation for the parent session
    let conversation_id = match conversation_repo
        .get_active_for_context(ChatContextType::Ideation, parent_id.as_str())
        .await
    {
        Ok(Some(conv)) => Some(conv.id),
        Ok(None) => {
            let new_conv = ChatConversation::new_ideation(parent_id.clone());
            match conversation_repo.create(new_conv).await {
                Ok(created) => Some(created.id),
                Err(e) => {
                    warn!(
                        parent_id = %parent_id.as_str(),
                        error = %e,
                        "Failed to create conversation for verification-result injection"
                    );
                    None
                }
            }
        }
        Err(e) => {
            warn!(
                parent_id = %parent_id.as_str(),
                error = %e,
                "Failed to look up conversation for verification-result injection"
            );
            None
        }
    };

    // Build and persist the system message
    let mut message = ChatMessage::system_in_session(parent_id.clone(), payload.clone());
    message.conversation_id = conversation_id;

    if let Err(e) = chat_message_repo.create(message).await {
        warn!(
            parent_id = %parent_id.as_str(),
            error = %e,
            "Failed to store verification-result message — continuing"
        );
    }

    // Best-effort: forward to any running agent on the parent session so it sees
    // the message immediately without waiting for the next spawn cycle.
    message_queue.queue(ChatContextType::Ideation, parent_id.as_str(), payload);
}

/// Build the `<verification-result>` XML payload.
pub fn format_verification_result_xml(
    child_session_id: &str,
    convergence_reason: Option<&str>,
    current_round: u32,
    max_rounds: u32,
    gaps: &[VerificationGap],
) -> String {
    let reason = convergence_reason.unwrap_or("unknown");
    let summary = summarize_gaps(gaps);
    let blockers = top_3_blockers(gaps);
    let action = derive_recommended_action(convergence_reason);

    let blockers_section = if blockers.is_empty() {
        String::new()
    } else {
        let blocker_lines: String = blockers
            .iter()
            .map(|(severity, desc)| {
                format!("    <blocker severity=\"{severity}\">{desc}</blocker>")
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("  <top_blockers>\n{blocker_lines}\n  </top_blockers>\n")
    };

    format!(
        "<verification-result>\n\
         <child_session_id>{child_session_id}</child_session_id>\n\
         <status>needs_revision</status>\n\
         <convergence_reason>{reason}</convergence_reason>\n\
         <round>{current_round}</round>\n\
         <max_rounds>{max_rounds}</max_rounds>\n\
         <summary>{summary}</summary>\n\
         {blockers_section}\
         <recommended_next_action>{action}</recommended_next_action>\n\
         </verification-result>"
    )
}

/// Summarize gap severity distribution as a human-readable sentence.
pub(crate) fn summarize_gaps(gaps: &[VerificationGap]) -> String {
    if gaps.is_empty() {
        return "Agent completed without producing gap analysis (possible mid-round crash)."
            .to_string();
    }

    let mut critical = 0u32;
    let mut high = 0u32;
    let mut medium = 0u32;
    let mut low = 0u32;

    for gap in gaps {
        match gap.severity.as_str() {
            "critical" => critical += 1,
            "high" => high += 1,
            "medium" => medium += 1,
            "low" => low += 1,
            _ => {}
        }
    }

    let total = gaps.len();
    let mut parts = Vec::new();
    if critical > 0 {
        parts.push(format!("{critical} critical"));
    }
    if high > 0 {
        parts.push(format!("{high} high"));
    }
    if medium > 0 {
        parts.push(format!("{medium} medium"));
    }
    if low > 0 {
        parts.push(format!("{low} low"));
    }

    if parts.is_empty() {
        format!("{total} unclassified gap(s) remain unresolved.")
    } else {
        format!("{total} gap(s) remain: {}.", parts.join(", "))
    }
}

/// Return up to 3 top blockers (highest severity first), descriptions capped at 200 chars.
pub(crate) fn top_3_blockers(gaps: &[VerificationGap]) -> Vec<(String, String)> {
    fn severity_rank(s: &str) -> u8 {
        match s {
            "critical" => 0,
            "high" => 1,
            "medium" => 2,
            "low" => 3,
            _ => 4,
        }
    }

    let mut sorted: Vec<&VerificationGap> = gaps.iter().collect();
    sorted.sort_by_key(|g| severity_rank(g.severity.as_str()));

    sorted
        .iter()
        .take(3)
        .map(|g| {
            let desc = if g.description.len() > 200 {
                // Truncate to 199 chars + ellipsis (byte-safe via char boundary)
                let truncated = g
                    .description
                    .char_indices()
                    .take_while(|(i, _)| *i < 199)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .map(|end| &g.description[..end])
                    .unwrap_or("");
                format!("{truncated}…")
            } else {
                g.description.clone()
            };
            (g.severity.clone(), desc)
        })
        .collect()
}

/// Deterministically map convergence_reason to a recommended_next_action enum value.
///
/// | Reason | Action |
/// |--------|--------|
/// | `max_rounds` | `revise_plan` — rounds exhausted; plan needs a revision pass |
/// | `jaccard_converged` | `revise_plan` — gaps stable; plan needs revision |
/// | `gap_score_plateau` | `explore_code_paths` — plateau suggests missing context |
/// | `agent_crashed_mid_round` | `rerun_verification` — transient failure; retry first |
/// | `agent_completed_without_update` | `rerun_verification` — no output; retry first |
/// | anything else / unknown | `rerun_verification` — safe default |
pub(crate) fn derive_recommended_action(convergence_reason: Option<&str>) -> &'static str {
    match convergence_reason {
        Some("max_rounds") => "revise_plan",
        Some("jaccard_converged") => "revise_plan",
        Some("gap_score_plateau") => "explore_code_paths",
        Some("agent_crashed_mid_round") | Some("agent_completed_without_update") => {
            "rerun_verification"
        }
        _ => "rerun_verification",
    }
}

#[cfg(test)]
#[path = "verification_handoff_tests.rs"]
mod tests;
