use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use crate::application::chat_attribution_backfill_transcript::parse_claude_session_transcript;
use crate::domain::agents::{AgentHarnessKind, ProviderSessionRef};
use crate::domain::entities::{
    AgentRunAttribution, AttributionBackfillStatus, ChatContextType, ChatMessageAttribution,
    ConversationAttributionBackfillState, MessageRole,
};
use crate::domain::repositories::{
    AgentRunRepository, ChatConversationRepository, ChatMessageRepository,
};
use crate::error::AppResult;

const CLAUDE_BACKFILL_SOURCE: &str = "claude_project_jsonl";

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

    pub async fn run_pending_batch(&self, limit: u32) -> AppResult<u32> {
        let conversations = self
            .conversation_repo
            .list_needing_attribution_backfill(limit)
            .await?;

        let mut processed = 0u32;
        for conversation in conversations {
            let Some(session_id) = conversation.claude_session_id.clone() else {
                continue;
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
                .import_session_transcript(&conversation, &session_id)
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
                Ok(ImportOutcome::Partial { path, reason }) => {
                    ConversationAttributionBackfillState {
                        status: Some(AttributionBackfillStatus::Partial),
                        source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                        source_path: Some(path.display().to_string()),
                        last_attempted_at: Some(started_at),
                        completed_at: None,
                        error_summary: Some(truncate_error(&reason)),
                    }
                }
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
            processed += 1;
        }

        Ok(processed)
    }

    async fn import_session_transcript(
        &self,
        conversation: &crate::domain::entities::ChatConversation,
        session_id: &str,
    ) -> Result<ImportOutcome, String> {
        let Some(summary) = parse_claude_session_transcript(&self.transcript_root, session_id)?
        else {
            return Ok(ImportOutcome::NotFound);
        };

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

        let attribution = ChatMessageAttribution {
            attribution_source: Some(summary.attribution_source()),
            provider_harness: Some(AgentHarnessKind::Claude),
            provider_session_id: Some(session_id.to_string()),
            logical_model: summary.primary_model.clone(),
            effective_model_id: summary.primary_model.clone(),
            logical_effort: None,
            effective_effort: None,
        };
        let run_attribution = AgentRunAttribution {
            harness: Some(AgentHarnessKind::Claude),
            provider_session_id: Some(session_id.to_string()),
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
    const BATCH_SIZE: u32 = 50;

    loop {
        match service.run_pending_batch(BATCH_SIZE).await {
            Ok(0) => {
                info!("Claude attribution backfill startup pass complete");
                break;
            }
            Ok(processed) => {
                info!(processed, "Processed Claude attribution backfill batch");
            }
            Err(error) => {
                warn!(error = %error, "Claude attribution backfill startup pass failed");
                break;
            }
        }
    }
}
