use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use futures::stream::{self, StreamExt};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

use crate::application::chat_attribution_backfill_transcript::{
    build_claude_transcript_index, parse_claude_session_transcript_from_path,
    HistoricalTranscriptIndex,
};
use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{
    AgentRunAttribution, AttributionBackfillStatus, ChatContextType, ChatMessageAttribution,
    ConversationAttributionBackfillState, ConversationAttributionBackfillSummary, MessageRole,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
};
use crate::error::AppResult;

const CLAUDE_BACKFILL_SOURCE: &str = "claude_project_jsonl";
pub const CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT: &str = "chat:attribution_backfill_progress";
const DEFAULT_BATCH_CONCURRENCY: usize = 1;
const STARTUP_BATCH_SIZE: u32 = 100;
const STARTUP_BATCH_CONCURRENCY: usize = 4;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatAttributionBackfillProgressPayload {
    pub processed_in_batch: u32,
    pub eligible_conversation_count: u64,
    pub pending_count: u64,
    pub running_count: u64,
    pub completed_count: u64,
    pub partial_count: u64,
    pub session_not_found_count: u64,
    pub parse_failed_count: u64,
    pub remaining_count: u64,
    pub terminal_count: u64,
    pub attention_count: u64,
    pub is_idle: bool,
}

impl ChatAttributionBackfillProgressPayload {
    fn from_summary(
        summary: ConversationAttributionBackfillSummary,
        processed_in_batch: u32,
    ) -> Self {
        Self {
            processed_in_batch,
            eligible_conversation_count: summary.eligible_conversation_count,
            pending_count: summary.pending_count,
            running_count: summary.running_count,
            completed_count: summary.completed_count,
            partial_count: summary.partial_count,
            session_not_found_count: summary.session_not_found_count,
            parse_failed_count: summary.parse_failed_count,
            remaining_count: summary.remaining_count(),
            terminal_count: summary.terminal_count(),
            attention_count: summary.attention_count(),
            is_idle: summary.is_idle(),
        }
    }
}

pub struct ChatAttributionBackfillService {
    conversation_repo: Arc<dyn ChatConversationRepository>,
    chat_message_repo: Arc<dyn ChatMessageRepository>,
    agent_run_repo: Arc<dyn AgentRunRepository>,
    transcript_root: PathBuf,
}

impl ChatAttributionBackfillService {
    pub fn new(
        conversation_repo: Arc<dyn ChatConversationRepository>,
        chat_message_repo: Arc<dyn ChatMessageRepository>,
        agent_run_repo: Arc<dyn AgentRunRepository>,
        transcript_root: PathBuf,
    ) -> Self {
        Self {
            conversation_repo,
            chat_message_repo,
            agent_run_repo,
            transcript_root,
        }
    }

    pub fn default_claude_projects_root() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("projects")
    }

    pub async fn get_backfill_summary(&self) -> AppResult<ConversationAttributionBackfillSummary> {
        self.conversation_repo
            .get_attribution_backfill_summary()
            .await
    }

    pub async fn run_pending_batch(&self, limit: u32) -> AppResult<u32> {
        let transcript_index = Arc::new(self.build_transcript_index().await);
        self.run_pending_batch_with_options(
            limit,
            transcript_index,
            None,
            DEFAULT_BATCH_CONCURRENCY,
        )
        .await
    }

    async fn build_transcript_index(&self) -> HistoricalTranscriptIndex {
        let transcript_root = self.transcript_root.clone();
        match tokio::task::spawn_blocking(move || build_claude_transcript_index(&transcript_root))
            .await
        {
            Ok(index) => index,
            Err(error) => {
                warn!(
                    error = %error,
                    "Failed to build Claude transcript index for attribution backfill"
                );
                HistoricalTranscriptIndex::default()
            }
        }
    }

    async fn run_pending_batch_with_options(
        &self,
        limit: u32,
        transcript_index: Arc<HistoricalTranscriptIndex>,
        app_handle: Option<&AppHandle>,
        concurrency: usize,
    ) -> AppResult<u32> {
        let conversations = self
            .conversation_repo
            .list_needing_attribution_backfill(limit)
            .await?;

        let mut in_flight = stream::iter(conversations.into_iter().map(|conversation| {
            let transcript_index = Arc::clone(&transcript_index);
            let app_handle = app_handle.cloned();
            async move {
                self.process_backfill_conversation(conversation, transcript_index, app_handle)
                    .await
            }
        }))
        .buffer_unordered(concurrency.max(1));

        let mut processed = 0u32;
        while let Some(result) = in_flight.next().await {
            processed += result? as u32;
        }

        Ok(processed)
    }

    async fn process_backfill_conversation(
        &self,
        conversation: crate::domain::entities::ChatConversation,
        transcript_index: Arc<HistoricalTranscriptIndex>,
        app_handle: Option<AppHandle>,
    ) -> AppResult<bool> {
        let Some(session_id) = conversation.claude_session_id.clone() else {
            return Ok(false);
        };

        let started_at = Utc::now();
        self.conversation_repo
            .update_attribution_backfill_state(
                &conversation.id,
                ConversationAttributionBackfillState {
                    status: Some(AttributionBackfillStatus::Running),
                    source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                    source_path: conversation.attribution_backfill_source_path.clone(),
                    last_attempted_at: Some(started_at),
                    completed_at: None,
                    error_summary: None,
                },
            )
            .await?;

        let final_state = match self
            .import_session_transcript(&conversation, &session_id, transcript_index.as_ref())
            .await
        {
            Ok(ImportOutcome::NotFound) => ConversationAttributionBackfillState {
                status: Some(AttributionBackfillStatus::SessionNotFound),
                source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                source_path: None,
                last_attempted_at: Some(started_at),
                completed_at: None,
                error_summary: None,
            },
            Ok(ImportOutcome::Completed { path }) => ConversationAttributionBackfillState {
                status: Some(AttributionBackfillStatus::Completed),
                source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                source_path: Some(path.display().to_string()),
                last_attempted_at: Some(started_at),
                completed_at: Some(Utc::now()),
                error_summary: None,
            },
            Ok(ImportOutcome::Partial { path, reason }) => ConversationAttributionBackfillState {
                status: Some(AttributionBackfillStatus::Partial),
                source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                source_path: Some(path.display().to_string()),
                last_attempted_at: Some(started_at),
                completed_at: None,
                error_summary: Some(truncate_error(&reason)),
            },
            Err(error) => ConversationAttributionBackfillState {
                status: Some(AttributionBackfillStatus::ParseFailed),
                source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                source_path: None,
                last_attempted_at: Some(started_at),
                completed_at: None,
                error_summary: Some(truncate_error(&error.to_string())),
            },
        };

        self.conversation_repo
            .update_attribution_backfill_state(&conversation.id, final_state)
            .await?;

        emit_backfill_progress(self, app_handle.as_ref(), 1).await;
        Ok(true)
    }

    async fn import_session_transcript(
        &self,
        conversation: &crate::domain::entities::ChatConversation,
        session_id: &str,
        transcript_index: &HistoricalTranscriptIndex,
    ) -> Result<ImportOutcome, String> {
        let Some(path) = transcript_index.get(session_id).cloned() else {
            return Ok(ImportOutcome::NotFound);
        };

        let summary =
            tokio::task::spawn_blocking(move || parse_claude_session_transcript_from_path(&path))
                .await
                .map_err(|error| format!("transcript parse task failed: {}", error))??;

        if conversation.provider_session_id.is_none() || conversation.provider_harness.is_none() {
            self.conversation_repo
                .update_provider_session_ref(
                    &conversation.id,
                    &ProviderSessionRef {
                        harness: AgentHarnessKind::Claude,
                        provider_session_id: session_id.to_string(),
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
        }

        self.conversation_repo
            .update_provider_origin(
                &conversation.id,
                summary.upstream_provider().as_deref(),
                summary.provider_profile_name().as_deref(),
            )
            .await
            .map_err(|error| error.to_string())?;

        let attribution = ChatMessageAttribution {
            attribution_source: Some(summary.attribution_source()),
            provider_harness: Some(AgentHarnessKind::Claude),
            provider_session_id: Some(session_id.to_string()),
            upstream_provider: summary.upstream_provider(),
            provider_profile: summary.provider_profile_name(),
            logical_model: summary.primary_model.clone(),
            effective_model_id: summary.primary_model.clone(),
            logical_effort: None,
            effective_effort: None,
        };
        let run_attribution = AgentRunAttribution {
            harness: Some(AgentHarnessKind::Claude),
            provider_session_id: Some(session_id.to_string()),
            upstream_provider: summary.upstream_provider(),
            provider_profile: summary.provider_profile_name(),
            logical_model: summary.primary_model.clone(),
            effective_model_id: summary.primary_model.clone(),
            logical_effort: None,
            effective_effort: None,
        };

        let mut issues = Vec::new();
        let runs = self
            .agent_run_repo
            .get_by_conversation(&conversation.id)
            .await
            .map_err(|error| error.to_string())?;
        match runs.as_slice() {
            [run] => {
                self.agent_run_repo
                    .update_attribution(&run.id, &run_attribution)
                    .await
                    .map_err(|error| error.to_string())?;
                self.agent_run_repo
                    .update_usage(&run.id, &summary.total_usage)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            [] => issues.push("no agent runs found for conversation".to_string()),
            many => issues.push(format!(
                "conversation has {} agent runs; run attribution left unchanged",
                many.len()
            )),
        }

        let provider_role = provider_message_role_for_context(conversation.context_type);
        let provider_messages: Vec<_> = self
            .chat_message_repo
            .get_by_conversation(&conversation.id)
            .await
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|message| message.role == provider_role)
            .collect();

        match provider_messages.as_slice() {
            [] => issues.push("no provider messages found for conversation".to_string()),
            [message] => {
                self.chat_message_repo
                    .update_attribution(&message.id, &attribution)
                    .await
                    .map_err(|error| error.to_string())?;
                self.chat_message_repo
                    .update_usage(&message.id, &summary.total_usage)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            many => {
                for message in many {
                    self.chat_message_repo
                        .update_attribution(&message.id, &attribution)
                        .await
                        .map_err(|error| error.to_string())?;
                }
                issues.push(format!(
                    "conversation has {} provider messages across {} transcript turns; only attribution was imported onto messages",
                    many.len(),
                    summary.assistant_turn_count
                ));
            }
        }

        if issues.is_empty() {
            Ok(ImportOutcome::Completed { path: summary.path })
        } else {
            Ok(ImportOutcome::Partial {
                path: summary.path,
                reason: issues.join("; "),
            })
        }
    }
}

fn provider_message_role_for_context(context_type: ChatContextType) -> MessageRole {
    match context_type {
        ChatContextType::Ideation | ChatContextType::Project | ChatContextType::Task => {
            MessageRole::Orchestrator
        }
        ChatContextType::TaskExecution => MessageRole::Worker,
        ChatContextType::Review => MessageRole::Reviewer,
        ChatContextType::Merge => MessageRole::Merger,
    }
}

fn truncate_error(error: &str) -> String {
    const MAX_LEN: usize = 240;
    if error.len() <= MAX_LEN {
        error.to_string()
    } else {
        format!("{}...", &error[..MAX_LEN])
    }
}

enum ImportOutcome {
    Completed { path: PathBuf },
    Partial { path: PathBuf, reason: String },
    NotFound,
}

pub async fn run_startup_chat_attribution_backfill(service: Arc<ChatAttributionBackfillService>) {
    run_startup_chat_attribution_backfill_with_events(service, None).await;
}

pub async fn run_startup_chat_attribution_backfill_with_events(
    service: Arc<ChatAttributionBackfillService>,
    app_handle: Option<AppHandle>,
) {
    match service
        .conversation_repo
        .reset_running_attribution_backfill_to_pending()
        .await
    {
        Ok(reset_count) if reset_count > 0 => {
            info!(
                reset_count,
                "Reset stale running Claude attribution backfill rows to pending"
            );
        }
        Ok(_) => {}
        Err(error) => {
            warn!(
                error = %error,
                "Failed to reset stale running Claude attribution backfill rows before startup pass"
            );
        }
    }

    let transcript_index = Arc::new(service.build_transcript_index().await);

    loop {
        match service
            .run_pending_batch_with_options(
                STARTUP_BATCH_SIZE,
                Arc::clone(&transcript_index),
                app_handle.as_ref(),
                STARTUP_BATCH_CONCURRENCY,
            )
            .await
        {
            Ok(0) => {
                emit_backfill_progress(&service, app_handle.as_ref(), 0).await;
                info!("Claude attribution backfill startup pass complete");
                break;
            }
            Ok(processed) => {
                emit_backfill_progress(&service, app_handle.as_ref(), processed).await;
                info!(processed, "Processed Claude attribution backfill batch");
            }
            Err(error) => {
                emit_backfill_progress(&service, app_handle.as_ref(), 0).await;
                warn!(error = %error, "Claude attribution backfill startup pass failed");
                break;
            }
        }
    }
}

async fn emit_backfill_progress(
    service: &ChatAttributionBackfillService,
    app_handle: Option<&AppHandle>,
    processed_in_batch: u32,
) {
    let Some(handle) = app_handle else {
        return;
    };

    match service.get_backfill_summary().await {
        Ok(summary) => {
            let _ = handle.emit(
                CHAT_ATTRIBUTION_BACKFILL_PROGRESS_EVENT,
                ChatAttributionBackfillProgressPayload::from_summary(summary, processed_in_batch),
            );
        }
        Err(error) => {
            warn!(
                error = %error,
                "Failed to emit chat attribution backfill progress event"
            );
        }
    }
}
