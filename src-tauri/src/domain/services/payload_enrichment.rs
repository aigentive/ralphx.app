//! Payload enrichment types and helpers for external webhook events.
//!
//! Provides [`WebhookPresentationContext`] and [`PresentationKind`] for enriching
//! outbound webhook payloads with human-readable project/session/task metadata.
//!
//! Also provides mandatory emission helpers [`emit_external_webhook_event`] and
//! [`log_non_fatal_error`] that all feature enrichment call sites must use.

use std::str::FromStr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::domain::repositories::external_events_repository::ExternalEventsRepository;
use crate::domain::state_machine::services::WebhookPublisher;

/// Identifies the lifecycle event kind for downstream presentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationKind {
    PlanCreated,
    ProposalsReady,
    SessionAccepted,
    Verified,
    MergeReady,
    MergeCompleted,
    PlanDelivered,
    TaskStatusChanged,
}

/// Context injected into external webhook payloads for downstream presentation.
///
/// All fields are optional; [`inject_into`](WebhookPresentationContext::inject_into)
/// silently skips `None` fields and never overwrites keys that already exist in
/// the payload.
#[derive(Debug, Clone, Default)]
pub struct WebhookPresentationContext {
    pub project_name: Option<String>,
    pub session_title: Option<String>,
    pub task_title: Option<String>,
    pub presentation_kind: Option<PresentationKind>,
}

impl WebhookPresentationContext {
    /// Returns a human-readable summary string covering all 8 field-presence
    /// combinations of `project_name`, `session_title`, and `task_title`.
    ///
    /// Format: `[project_name] session_title → task_title`
    /// with graceful degradation when fields are absent.
    pub fn human_context(&self) -> String {
        let mut prefix_parts: Vec<String> = Vec::new();

        if let Some(ref p) = self.project_name {
            prefix_parts.push(format!("[{}]", p));
        }
        if let Some(ref s) = self.session_title {
            prefix_parts.push(s.clone());
        }

        let prefix = prefix_parts.join(" ");

        match (prefix.is_empty(), &self.task_title) {
            (false, Some(t)) => format!("{} \u{2192} {}", prefix, t),
            (false, None) => prefix,
            (true, Some(t)) => t.clone(),
            (true, None) => String::new(),
        }
    }

    /// Injects presentation fields into a JSON payload object.
    ///
    /// Rules:
    /// - Only `Some(...)` fields are written.
    /// - Existing keys are never overwritten.
    /// - A non-empty `human_context` string is also injected under the key
    ///   `"human_context"`.
    /// - If `payload` is not a JSON object, this method is a no-op.
    pub fn inject_into(&self, payload: &mut serde_json::Value) {
        let Some(obj) = payload.as_object_mut() else {
            return;
        };

        if let Some(ref name) = self.project_name {
            obj.entry("project_name")
                .or_insert_with(|| serde_json::Value::String(name.clone()));
        }
        if let Some(ref title) = self.session_title {
            obj.entry("session_title")
                .or_insert_with(|| serde_json::Value::String(title.clone()));
        }
        if let Some(ref title) = self.task_title {
            obj.entry("task_title")
                .or_insert_with(|| serde_json::Value::String(title.clone()));
        }
        if let Some(ref kind) = self.presentation_kind {
            obj.entry("presentation_kind")
                .or_insert_with(|| serde_json::json!(kind));
        }

        let hc = self.human_context();
        if !hc.is_empty() {
            obj.entry("human_context")
                .or_insert_with(|| serde_json::Value::String(hc));
        }
    }
}

/// Persists an external event and delivers it via webhook in a single call.
///
/// Replaces the `insert_event()` + `publisher.publish()` dual-call pattern that
/// appears at every event emit site. All enrichment call sites MUST use this helper.
///
/// # Errors
///
/// Returns `Err(String)` with context if `insert_event` fails. Webhook publish
/// failures are fire-and-forget (the trait returns `()`). If `event_type` is not a
/// recognised [`ralphx_domain::entities::EventType`] variant, a warning is logged
/// and the publish step is skipped (the event is still persisted).
pub async fn emit_external_webhook_event(
    event_type: &str,
    project_id: &str,
    payload: serde_json::Value,
    external_events_repo: &Arc<dyn ExternalEventsRepository>,
    webhook_publisher: &Arc<dyn WebhookPublisher>,
) -> Result<(), String> {
    external_events_repo
        .insert_event(event_type, project_id, &payload.to_string())
        .await
        .map_err(|e| {
            format!("emit_external_webhook_event: insert_event failed for {event_type}: {e}")
        })?;

    match ralphx_domain::entities::EventType::from_str(event_type) {
        Ok(et) => {
            webhook_publisher.publish(et, project_id, payload).await;
        }
        Err(_) => {
            tracing::warn!(
                event_type = event_type,
                "emit_external_webhook_event: unknown event_type, skipping webhook publish"
            );
        }
    }

    Ok(())
}

/// Logs a non-fatal error at the specified tracing level.
///
/// Standardised logging helper for enrichment paths. All enrichment call sites that
/// handle non-fatal errors MUST use this helper instead of inline `tracing::warn!` /
/// `tracing::error!` calls.
pub fn log_non_fatal_error(context: &str, error: &dyn std::error::Error, level: tracing::Level) {
    match level {
        tracing::Level::ERROR => {
            tracing::error!(context = context, error = %error, "Non-fatal error in enrichment path");
        }
        tracing::Level::WARN => {
            tracing::warn!(context = context, error = %error, "Non-fatal error in enrichment path");
        }
        tracing::Level::INFO => {
            tracing::info!(context = context, error = %error, "Non-fatal error in enrichment path");
        }
        tracing::Level::DEBUG => {
            tracing::debug!(context = context, error = %error, "Non-fatal error in enrichment path");
        }
        tracing::Level::TRACE => {
            tracing::trace!(context = context, error = %error, "Non-fatal error in enrichment path");
        }
    }
}

#[cfg(test)]
#[path = "payload_enrichment_tests.rs"]
mod tests;
