use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use tracing::{info, warn};

use crate::domain::entities::{AttributionBackfillStatus, ConversationAttributionBackfillState};
use crate::domain::repositories::ChatConversationRepository;
use crate::error::AppResult;

const CLAUDE_BACKFILL_SOURCE: &str = "claude_project_jsonl";

pub struct ChatAttributionBackfillService {
    conversation_repo: Arc<dyn ChatConversationRepository>,
    transcript_root: PathBuf,
}

impl ChatAttributionBackfillService {
    pub fn new(
        conversation_repo: Arc<dyn ChatConversationRepository>,
        transcript_root: PathBuf,
    ) -> Self {
        Self {
            conversation_repo,
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

            let final_state = match self.inspect_session_transcript(&session_id) {
                Ok(TranscriptInspection::Found { path }) => ConversationAttributionBackfillState {
                    status: Some(AttributionBackfillStatus::Completed),
                    source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                    source_path: Some(path.display().to_string()),
                    last_attempted_at: Some(started_at),
                    completed_at: Some(Utc::now()),
                    error_summary: None,
                },
                Ok(TranscriptInspection::Partial { path, reason }) => {
                    ConversationAttributionBackfillState {
                        status: Some(AttributionBackfillStatus::Partial),
                        source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                        source_path: Some(path.display().to_string()),
                        last_attempted_at: Some(started_at),
                        completed_at: None,
                        error_summary: Some(reason),
                    }
                }
                Ok(TranscriptInspection::NotFound) => ConversationAttributionBackfillState {
                    status: Some(AttributionBackfillStatus::SessionNotFound),
                    source: Some(CLAUDE_BACKFILL_SOURCE.to_string()),
                    source_path: None,
                    last_attempted_at: Some(started_at),
                    completed_at: None,
                    error_summary: None,
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
            processed += 1;
        }

        Ok(processed)
    }

    fn inspect_session_transcript(&self, session_id: &str) -> Result<TranscriptInspection, String> {
        let Some(path) = find_session_transcript_path(&self.transcript_root, session_id) else {
            return Ok(TranscriptInspection::NotFound);
        };
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read transcript {}: {}", path.display(), error))?;

        let mut saw_assistant_message = false;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(line)
                .map_err(|error| format!("failed to parse transcript {}: {}", path.display(), error))?;

            let message = value.get("message").and_then(|raw| raw.as_object());
            let model = message
                .and_then(|message| message.get("model"))
                .and_then(|raw| raw.as_str());
            let entry_type = value.get("type").and_then(|raw| raw.as_str());

            if entry_type == Some("assistant") && model.is_some() {
                saw_assistant_message = true;
                break;
            }
        }

        if saw_assistant_message {
            Ok(TranscriptInspection::Found { path })
        } else {
            Ok(TranscriptInspection::Partial {
                path,
                reason: "transcript found but no assistant model-bearing events were present"
                    .to_string(),
            })
        }
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

fn find_session_transcript_path(root: &Path, session_id: &str) -> Option<PathBuf> {
    if !root.exists() {
        return None;
    }

    let mut stack = vec![root.to_path_buf()];
    let target = format!("{}.jsonl", session_id);

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path
                .file_name()
                .and_then(|raw| raw.to_str())
                .is_some_and(|name| name == target)
            {
                return Some(path);
            }
        }
    }

    None
}

enum TranscriptInspection {
    Found { path: PathBuf },
    Partial { path: PathBuf, reason: String },
    NotFound,
}

pub async fn run_startup_chat_attribution_backfill(
    service: Arc<ChatAttributionBackfillService>,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{ChatConversation, IdeationSessionId};
    use crate::infrastructure::memory::MemoryChatConversationRepository;
    use std::io::Write;

    #[tokio::test]
    async fn test_run_pending_batch_marks_found_transcript_completed() {
        let repo = Arc::new(MemoryChatConversationRepository::new());
        let temp = tempfile::tempdir().unwrap();
        let transcript_path = temp.path().join("session-1.jsonl");
        let mut file = std::fs::File::create(&transcript_path).unwrap();
        writeln!(
            file,
            "{{\"type\":\"assistant\",\"message\":{{\"model\":\"claude-sonnet-4-6\",\"content\":[],\"usage\":{{\"input_tokens\":1,\"output_tokens\":2}}}}}}"
        )
        .unwrap();

        let mut conversation = ChatConversation::new_ideation(IdeationSessionId::new());
        conversation.claude_session_id = Some("session-1".to_string());
        repo.create(conversation.clone()).await.unwrap();

        let service = ChatAttributionBackfillService::new(repo.clone(), temp.path().to_path_buf());
        assert_eq!(service.run_pending_batch(20).await.unwrap(), 1);

        let updated = repo.get_by_id(&conversation.id).await.unwrap().unwrap();
        assert_eq!(
            updated.attribution_backfill_status,
            Some(AttributionBackfillStatus::Completed)
        );
        assert_eq!(
            updated.attribution_backfill_source.as_deref(),
            Some(CLAUDE_BACKFILL_SOURCE)
        );
        assert!(updated.attribution_backfill_source_path.is_some());
    }
}
