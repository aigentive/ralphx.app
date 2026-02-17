// Conversation replay builder for session recovery
//
// Rebuilds conversation history from database for rehydrating fresh Claude sessions.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::entities::{ChatContextType, ChatConversationId, MessageRole};
use crate::domain::repositories::ChatMessageRepository;
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationReplay {
    pub turns: Vec<Turn>,
    pub total_tokens: usize,
    pub is_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Vec<serde_json::Value>,
    pub tool_results: Vec<serde_json::Value>,
}

pub struct ReplayBuilder {
    token_budget: usize,
}

impl ReplayBuilder {
    pub fn new(token_budget: usize) -> Self {
        Self { token_budget }
    }

    pub async fn build_replay(
        &self,
        chat_message_repo: &Arc<dyn ChatMessageRepository>,
        conversation_id: &ChatConversationId,
    ) -> AppResult<ConversationReplay> {
        // Load all messages ordered by created_at ASC
        let messages = chat_message_repo
            .get_by_conversation(conversation_id)
            .await
            .map_err(|e| crate::error::AppError::Database(e.to_string()))?;

        let mut turns = Vec::new();
        let mut current_token_count = 0;
        let total_messages = messages.len();

        // Process newest-first for budget, then reverse for chronological order
        for msg in messages.iter().rev() {
            if Self::should_skip_message(&msg.role, &msg.content) {
                continue;
            }

            let turn = Self::message_to_turn(msg)?;
            let estimated_tokens = Self::estimate_tokens(&turn);

            if current_token_count + estimated_tokens > self.token_budget {
                break;
            }

            turns.insert(0, turn); // maintain chronological order
            current_token_count += estimated_tokens;
        }

        let turns_len = turns.len();
        Ok(ConversationReplay {
            turns,
            total_tokens: current_token_count,
            is_truncated: turns_len < total_messages,
        })
    }

    fn should_skip_message(role: &MessageRole, content: &str) -> bool {
        // Skip system messages containing error wrappers
        if *role == MessageRole::System {
            return content.contains(super::AGENT_ERROR_PREFIX);
        }
        false
    }

    fn message_to_turn(msg: &crate::domain::entities::ChatMessage) -> AppResult<Turn> {
        let tool_calls = if let Some(ref tc_json) = msg.tool_calls {
            serde_json::from_str(tc_json).map_err(|e| {
                crate::error::AppError::Infrastructure(format!("Failed to parse tool_calls: {}", e))
            })?
        } else {
            vec![]
        };

        Ok(Turn {
            role: msg.role,
            content: msg.content.clone(),
            tool_calls,
            tool_results: vec![], // extracted from content_blocks if needed
        })
    }

    fn estimate_tokens(turn: &Turn) -> usize {
        // Rough heuristic: 4 chars per token
        (turn.content.len() / 4) + (turn.tool_calls.len() * 200) // average tool call JSON
    }
}

/// Build rehydration prompt with conversation history
pub fn build_rehydration_prompt(
    replay: &ConversationReplay,
    context_type: ChatContextType,
    context_id: &str,
    new_user_message: &str,
) -> String {
    let history_xml = replay
        .turns
        .iter()
        .map(|turn| {
            let tool_info = if !turn.tool_calls.is_empty() {
                format!(" tools=\"{}\"", turn.tool_calls.len())
            } else {
                String::new()
            };
            format!(
                "<turn role=\"{}\"{}>{}</turn>",
                turn.role, tool_info, turn.content
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "<instructions>\n\
         Your previous session expired. Here is the conversation history restored from local storage.\n\
         Continue the conversation naturally from where it left off.\n\
         Context: {} ({})\n\
         </instructions>\n\
         <conversation_history>\n{}\n</conversation_history>\n\
         <current_message>{}</current_message>",
        context_type, context_id, history_xml, new_user_message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_error_messages() {
        assert!(ReplayBuilder::should_skip_message(
            &MessageRole::System,
            &format!("{} timeout]", super::AGENT_ERROR_PREFIX)
        ));
        assert!(!ReplayBuilder::should_skip_message(
            &MessageRole::User,
            "Hello"
        ));
    }

    #[test]
    fn test_estimate_tokens() {
        let turn = Turn {
            role: MessageRole::User,
            content: "Hello world".to_string(), // 11 chars ~ 2-3 tokens
            tool_calls: vec![],
            tool_results: vec![],
        };
        let tokens = ReplayBuilder::estimate_tokens(&turn);
        assert_eq!(tokens, 2); // 11 / 4 = 2
    }

    #[test]
    fn test_estimate_tokens_with_tool_calls() {
        let turn = Turn {
            role: MessageRole::Orchestrator,
            content: "Processing".to_string(),
            tool_calls: vec![serde_json::json!({"name": "test"})],
            tool_results: vec![],
        };
        let tokens = ReplayBuilder::estimate_tokens(&turn);
        assert_eq!(tokens, 202); // 10/4 + 200 = 2 + 200
    }

    #[test]
    fn test_build_rehydration_prompt() {
        let replay = ConversationReplay {
            turns: vec![
                Turn {
                    role: MessageRole::User,
                    content: "Hello".to_string(),
                    tool_calls: vec![],
                    tool_results: vec![],
                },
                Turn {
                    role: MessageRole::Orchestrator,
                    content: "Hi!".to_string(),
                    tool_calls: vec![],
                    tool_results: vec![],
                },
            ],
            total_tokens: 100,
            is_truncated: false,
        };

        let prompt = build_rehydration_prompt(
            &replay,
            ChatContextType::Ideation,
            "session-123",
            "Continue conversation",
        );

        assert!(prompt.contains("Hello"));
        assert!(prompt.contains("Hi!"));
        assert!(prompt.contains("<current_message>Continue conversation</current_message>"));
        assert!(prompt.contains("ideation"));
        assert!(prompt.contains("session-123"));
        assert!(prompt.contains("<turn role"));
        assert!(prompt.contains("</turn>"));
    }
}
