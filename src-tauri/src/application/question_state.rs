// Question state for handling inline ask_user_question from agents
// Clones the permission_request bridge pattern: MCP tool → backend → Tauri event → frontend → resolve

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{watch, Mutex};

/// Answer provided by the user in the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionAnswer {
    /// Selected option(s) or free-text answer
    pub answer: Vec<String>,
    /// Whether the user dismissed/skipped the question
    pub dismissed: bool,
}

/// Metadata for a pending question (sent to frontend for rendering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionInfo {
    pub request_id: String,
    pub session_id: String,
    pub question: String,
    pub header: Option<String>,
    pub options: Option<Vec<String>>,
    pub multi_select: bool,
}

/// A pending question with its signaling channel
pub struct PendingQuestion {
    pub info: QuestionInfo,
    pub sender: watch::Sender<Option<QuestionAnswer>>,
}

/// Shared state for managing pending ask_user_question requests
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
    pub async fn get_pending_info(&self) -> Vec<QuestionInfo> {
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
        options: Option<Vec<String>>,
        multi_select: bool,
    ) -> watch::Receiver<Option<QuestionAnswer>> {
        let (tx, rx) = watch::channel(None);
        let pending_question = PendingQuestion {
            info: QuestionInfo {
                request_id: request_id.clone(),
                session_id,
                question,
                header,
                options,
                multi_select,
            },
            sender: tx,
        };
        self.pending.lock().await.insert(request_id, pending_question);
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
            answer: vec!["Option A".to_string()],
            dismissed: false,
        };
        let cloned = answer.clone();
        assert_eq!(cloned.answer, vec!["Option A"]);
        assert!(!cloned.dismissed);
    }

    #[tokio::test]
    async fn test_question_answer_serialization() {
        let answer = QuestionAnswer {
            answer: vec!["yes".to_string()],
            dismissed: false,
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert!(json.contains("\"answer\":[\"yes\"]"));
        assert!(json.contains("\"dismissed\":false"));

        let deserialized: QuestionAnswer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.answer, vec!["yes"]);
        assert!(!deserialized.dismissed);
    }

    #[tokio::test]
    async fn test_question_answer_dismissed() {
        let answer = QuestionAnswer {
            answer: vec![],
            dismissed: true,
        };
        let json = serde_json::to_string(&answer).unwrap();
        let deserialized: QuestionAnswer = serde_json::from_str(&json).unwrap();
        assert!(deserialized.answer.is_empty());
        assert!(deserialized.dismissed);
    }

    #[tokio::test]
    async fn test_question_info_serialization() {
        let info = QuestionInfo {
            request_id: "q-123".to_string(),
            session_id: "session-abc".to_string(),
            question: "What framework?".to_string(),
            header: Some("Tech Choice".to_string()),
            options: Some(vec!["React".to_string(), "Vue".to_string()]),
            multi_select: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"request_id\":\"q-123\""));
        assert!(json.contains("\"session_id\":\"session-abc\""));
        assert!(json.contains("\"question\":\"What framework?\""));
        assert!(json.contains("\"header\":\"Tech Choice\""));
        assert!(json.contains("\"multi_select\":false"));
    }

    #[tokio::test]
    async fn test_question_info_optional_fields() {
        let info = QuestionInfo {
            request_id: "q-456".to_string(),
            session_id: "session-def".to_string(),
            question: "Free text question".to_string(),
            header: None,
            options: None,
            multi_select: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"header\":null"));
        assert!(json.contains("\"options\":null"));
    }

    #[tokio::test]
    async fn test_register_and_resolve_question() {
        let state = QuestionState::new();

        let request_id = "test-q-123".to_string();
        let rx = state
            .register(
                request_id.clone(),
                "session-1".to_string(),
                "Which database?".to_string(),
                Some("Architecture".to_string()),
                Some(vec!["PostgreSQL".to_string(), "SQLite".to_string()]),
                false,
            )
            .await;

        // Verify it's in pending
        {
            let pending = state.pending.lock().await;
            assert!(pending.contains_key(&request_id));
            let question = pending.get(&request_id).unwrap();
            assert_eq!(question.info.question, "Which database?");
            assert_eq!(question.info.session_id, "session-1");
        }

        // Resolve with an answer
        let resolved = state
            .resolve(
                &request_id,
                QuestionAnswer {
                    answer: vec!["SQLite".to_string()],
                    dismissed: false,
                },
            )
            .await;
        assert!(resolved);

        // Check the answer was received
        let answer = rx.borrow().clone();
        assert!(answer.is_some());
        let answer = answer.unwrap();
        assert_eq!(answer.answer, vec!["SQLite"]);
        assert!(!answer.dismissed);
    }

    #[tokio::test]
    async fn test_register_multi_select() {
        let state = QuestionState::new();

        let rx = state
            .register(
                "ms-q-1".to_string(),
                "session-2".to_string(),
                "Select features".to_string(),
                None,
                Some(vec!["Auth".to_string(), "DB".to_string(), "API".to_string()]),
                true,
            )
            .await;

        let resolved = state
            .resolve(
                "ms-q-1",
                QuestionAnswer {
                    answer: vec!["Auth".to_string(), "API".to_string()],
                    dismissed: false,
                },
            )
            .await;
        assert!(resolved);

        let answer = rx.borrow().clone().unwrap();
        assert_eq!(answer.answer.len(), 2);
        assert!(answer.answer.contains(&"Auth".to_string()));
        assert!(answer.answer.contains(&"API".to_string()));
    }

    #[tokio::test]
    async fn test_get_pending_info() {
        let state = QuestionState::new();

        for i in 0..3 {
            state
                .register(
                    format!("q-{}", i),
                    format!("session-{}", i),
                    format!("Question {}", i),
                    None,
                    None,
                    false,
                )
                .await;
        }

        let pending_info = state.get_pending_info().await;
        assert_eq!(pending_info.len(), 3);

        let request_ids: Vec<_> = pending_info.iter().map(|p| p.request_id.as_str()).collect();
        assert!(request_ids.contains(&"q-0"));
        assert!(request_ids.contains(&"q-1"));
        assert!(request_ids.contains(&"q-2"));
    }

    #[tokio::test]
    async fn test_multiple_pending_questions() {
        let state = QuestionState::new();

        for i in 0..5 {
            state
                .register(
                    format!("q-{}", i),
                    "session-1".to_string(),
                    format!("Question {}", i),
                    None,
                    None,
                    false,
                )
                .await;
        }

        let pending = state.pending.lock().await;
        assert_eq!(pending.len(), 5);
        for i in 0..5 {
            assert!(pending.contains_key(&format!("q-{}", i)));
        }
    }

    #[tokio::test]
    async fn test_remove_pending_question() {
        let state = QuestionState::new();

        let request_id = "to-remove".to_string();
        state
            .register(
                request_id.clone(),
                "session-1".to_string(),
                "Remove me".to_string(),
                None,
                None,
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

        // Try to remove again - should return false
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
                    answer: vec![],
                    dismissed: true,
                },
            )
            .await;
        assert!(!resolved);
    }
}
