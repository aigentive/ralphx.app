use crate::entities::{
    ActivityEvent, ActivityEventRole, ActivityEventType, IdeationSession, InternalStatus,
    ReviewScopeMetadata, Task, TaskContext, TaskId,
};

use super::{
    build_unrelated_drift_followup_prompt, compute_out_of_scope_blocker_fingerprint,
    ReviewSettings, ReviewToolOutcome, ScopeDriftClassification,
};

const FOLLOWUP_ACTIVITY_CONTENT: &str =
    "Linked follow-up ideation session to handle unresolved unrelated scope drift separately.";
const OUT_OF_SCOPE_FOLLOWUP_DESCRIPTION: &str =
    "Separate follow-up spawned automatically because repeated revise cycles could not resolve unrelated scope drift in the original task.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrelatedDriftFollowupDraft {
    pub title: String,
    pub description: String,
    pub prompt: String,
    pub blocker_fingerprint: Option<String>,
}

pub fn update_review_scope_metadata(
    existing_metadata: Option<&str>,
    task_context: &TaskContext,
    scope_drift_classification: Option<ScopeDriftClassification>,
    scope_drift_notes: Option<String>,
) -> Result<Option<String>, serde_json::Error> {
    let planned_paths = task_context
        .source_proposal
        .as_ref()
        .map(|proposal| proposal.affected_paths.clone())
        .unwrap_or_default();

    if planned_paths.is_empty() {
        return ReviewScopeMetadata::clear_from_task_metadata(existing_metadata);
    }

    let review_scope = ReviewScopeMetadata::new(
        planned_paths,
        task_context.out_of_scope_files.clone(),
        scope_drift_classification.map(|classification| classification.to_string()),
        scope_drift_notes,
    );
    review_scope
        .update_task_metadata(existing_metadata)
        .map(Some)
}

pub fn should_spawn_unrelated_drift_followup(
    outcome: ReviewToolOutcome,
    scope_drift_classification: Option<ScopeDriftClassification>,
    revision_count: u32,
    review_settings: &ReviewSettings,
) -> bool {
    matches!(outcome, ReviewToolOutcome::Escalate)
        && matches!(
            scope_drift_classification,
            Some(ScopeDriftClassification::UnrelatedDrift)
        )
        && review_settings.exceeded_max_revisions(revision_count)
}

pub fn build_unrelated_drift_followup_draft(
    task: &Task,
    task_context: &TaskContext,
    summary: Option<&str>,
    feedback: Option<&str>,
    escalation_reason: Option<&str>,
    revision_count: u32,
    review_settings: &ReviewSettings,
) -> UnrelatedDriftFollowupDraft {
    UnrelatedDriftFollowupDraft {
        title: format!("Follow-up: {}", task.title),
        description: OUT_OF_SCOPE_FOLLOWUP_DESCRIPTION.to_string(),
        prompt: build_unrelated_drift_followup_prompt(
            task,
            task_context,
            summary,
            feedback,
            escalation_reason,
            revision_count,
            review_settings.max_revision_cycles,
        ),
        blocker_fingerprint: compute_out_of_scope_blocker_fingerprint(
            &task.id,
            &task_context.out_of_scope_files,
        ),
    }
}

pub fn matching_unrelated_drift_followup_session_id(
    children: &[IdeationSession],
    task_id: &TaskId,
    blocker_fingerprint: Option<&str>,
) -> Option<String> {
    children
        .iter()
        .find(|session| {
            session.archived_at.is_none()
                && session.source_task_id.as_ref() == Some(task_id)
                && match blocker_fingerprint {
                    Some(fingerprint) => {
                        session.blocker_fingerprint.as_deref() == Some(fingerprint)
                    }
                    None => {
                        session.source_context_type.as_deref() == Some("review")
                            && session.spawn_reason.as_deref() == Some("out_of_scope_failure")
                    }
                }
        })
        .map(|session| session.id.as_str().to_string())
}

pub fn build_followup_activity_event(
    task_id: TaskId,
    new_status: InternalStatus,
    followup_session_id: Option<&str>,
    review_note_id: &str,
) -> Option<ActivityEvent> {
    let followup_session_id = followup_session_id?;
    let metadata = serde_json::json!({
        "followupSessionId": followup_session_id,
        "reviewNoteId": review_note_id,
        "spawnReason": "out_of_scope_failure",
        "sourceContextType": "review",
    });

    Some(
        ActivityEvent::new_task_event(
            task_id,
            ActivityEventType::System,
            FOLLOWUP_ACTIVITY_CONTENT,
        )
        .with_role(ActivityEventRole::System)
        .with_status(new_status)
        .with_metadata(metadata.to_string()),
    )
}

#[cfg(test)]
#[path = "complete_followup_tests.rs"]
mod tests;
