// Question state for handling inline AskUserQuestion from agents
// Used by the question bridge system to coordinate between MCP tools and frontend
// Mirrors the permission_state.rs pattern exactly

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{watch, Mutex};

/// Answer provided by the user in the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswer {
    pub selected_options: Vec<String>,
    pub text: Option<String>,
}

/// Metadata for a pending question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingQuestionInfo {
    pub request_id: String,
    pub session_id: String,
    pub question: String,
    pub header: Option<String>,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
}

/// A single option in a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// A pending question with its signaling channel
pub struct PendingQuestion {
    pub info: PendingQuestionInfo,
    pub sender: watch::Sender<Option<QuestionAnswer>>,
}

/// Shared state for managing pending questions from agents
///
/// Uses tokio::sync::watch channels to allow long-polling:
/// - MCP server registers a question and waits on a receiver
/// - Frontend resolves the question by sending through the channel
pub struct QuestionState {
    pub pending: Mutex<HashMap<String, PendingQuestion>>,
}

impl QuestionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// Get info about all pending questions
    pub async fn get_pending_info(&self) -> Vec<PendingQuestionInfo> {
        let pending = self.pending.lock().await;
        pending.values().map(|p| p.info.clone()).collect()
    }

    /// Register a new pending question
    pub async fn register(
        &self,
        request_id: String,
        session_id: String,
        question: String,
        header: Option<String>,
        options: Vec<QuestionOption>,
        multi_select: bool,
    ) -> watch::Receiver<Option<QuestionAnswer>> {
        let (tx, rx) = watch::channel(None);
        let request = PendingQuestion {
            info: PendingQuestionInfo {
                request_id: request_id.clone(),
                session_id,
                question,
                header,
                options,
                multi_select,
            },
            sender: tx,
        };
        self.pending.lock().await.insert(request_id, request);
        rx
    }

    /// Resolve a pending question with an answer
    /// Returns true if the question was found and resolved
    pub async fn resolve(&self, request_id: &str, answer: QuestionAnswer) -> bool {
        let pending = self.pending.lock().await;
        if let Some(question) = pending.get(request_id) {
            let _ = question.sender.send(Some(answer));
            true
        } else {
            false
        }
    }

    /// Remove a pending question
    pub async fn remove(&self, request_id: &str) -> bool {
        self.pending.lock().await.remove(request_id).is_some()
    }
}

impl Default for QuestionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_question_state_new() {
        let state = QuestionState::new();
        let pending = state.pending.lock().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_question_state_default() {
        let state = QuestionState::default();
        let pending = state.pending.lock().await;
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_question_answer_clone() {
        let answer = QuestionAnswer {
            selected_options: vec!["opt1".to_string()],
            text: Some("Custom text".to_string()),
        };
        let cloned = answer.clone();
        assert_eq!(cloned.selected_options, vec!["opt1"]);
        assert_eq!(cloned.text, Some("Custom text".to_string()));
    }

    #[tokio::test]
    async fn test_question_answer_serialization() {
        let answer = QuestionAnswer {
            selected_options: vec!["a".to_string(), "b".to_string()],
            text: None,
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert!(json.contains("\"selected_options\""));

        let deserialized: QuestionAnswer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.selected_options.len(), 2);
        assert!(deserialized.text.is_none());
    }

    #[tokio::test]
    async fn test_pending_question_info_serialization() {
        let info = PendingQuestionInfo {
            request_id: "req-123".to_string(),
            session_id: "session-456".to_string(),
            question: "Which approach?".to_string(),
            header: Some("Architecture Decision".to_string()),
            options: vec![QuestionOption {
                value: "a".to_string(),
                label: "Option A".to_string(),
                description: Some("First approach".to_string()),
            }],
            multi_select: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"request_id\":\"req-123\""));
        assert!(json.contains("\"session_id\":\"session-456\""));
        assert!(json.contains("\"question\":\"Which approach?\""));
    }

    #[tokio::test]
    async fn test_register_and_resolve_question() {
        let state = QuestionState::new();

        let request_id = "test-question-123".to_string();
        let rx = state
            .register(
                request_id.clone(),
                "session-1".to_string(),
                "Which framework?".to_string(),
                None,
                vec![
                    QuestionOption {
                        value: "react".to_string(),
                        label: "React".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        value: "vue".to_string(),
                        label: "Vue".to_string(),
                        description: None,
                    },
                ],
                false,
            )
            .await;

        // Verify it's in pending
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
            let question = pending.get(&request_id).unwrap();
            assert_eq!(question.info.question, "Which framework?");
        }

        // Resolve with an answer
        let resolved = state
            .resolve(
                &request_id,
                QuestionAnswer {
                    selected_options: vec!["react".to_string()],
                    text: None,
                },
            )
            .await;
        assert!(resolved);

        // Check the answer was received
        let answer = rx.borrow().clone();
        assert!(answer.is_some());
        let answer = answer.unwrap();
        assert_eq!(answer.selected_options, vec!["react"]);
    }

    #[tokio::test]
    async fn test_get_pending_info() {
        let state = QuestionState::new();

        for i in 0..3 {
            state
                .register(
                    format!("request-{}", i),
                    "session-1".to_string(),
                    format!("Question {}", i),
                    None,
                    vec![],
                    false,
                )
                .await;
        }

        let pending_info = state.get_pending_info().await;
        assert_eq!(pending_info.len(), 3);

        let request_ids: Vec<_> = pending_info.iter().map(|p| p.request_id.as_str()).collect();
        assert!(request_ids.contains(&"request-0"));
        assert!(request_ids.contains(&"request-1"));
        assert!(request_ids.contains(&"request-2"));
    }

    #[tokio::test]
    async fn test_remove_pending_question() {
        let state = QuestionState::new();

        let request_id = "to-remove".to_string();
        state
            .register(
                request_id.clone(),
                "session-1".to_string(),
                "Remove me?".to_string(),
                None,
                vec![],
                false,
            )
            .await;

        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
        }

        let removed = state.remove(&request_id).await;
        assert!(removed);

        {
            let pending = state.pending.lock().await;
            assert!(!pending.contains_key(&request_id));
        }

        let removed_again = state.remove(&request_id).await;
        assert!(!removed_again);
    }

    #[tokio::test]
    async fn test_resolve_nonexistent_question() {
        let state = QuestionState::new();

        let resolved = state
            .resolve(
                "nonexistent",
                QuestionAnswer {
                    selected_options: vec![],
                    text: None,
                },
            )
            .await;
        assert!(!resolved);
    }
}
