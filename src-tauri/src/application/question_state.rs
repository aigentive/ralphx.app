// Question state for handling inline AskUserQuestion from agents
// Used by the question bridge system to coordinate between MCP tools and frontend
// Mirrors the permission_state.rs pattern exactly

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use tracing::{error, info};

use crate::domain::repositories::QuestionRepository;

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
///
/// Optionally backed by a repository for persistence (SQLite).
/// Repo calls are fire-and-forget: errors are logged but never block channel ops.
pub struct QuestionState {
    pub pending: Mutex<HashMap<String, PendingQuestion>>,
    repo: Option<Arc<dyn QuestionRepository>>,
}

impl QuestionState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            repo: None,
        }
    }

    pub fn with_repo(repo: Arc<dyn QuestionRepository>) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            repo: Some(repo),
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
        let info = PendingQuestionInfo {
            request_id: request_id.clone(),
            session_id,
            question,
            header,
            options,
            multi_select,
        };

        // Fire-and-forget persist to repo
        if let Some(repo) = &self.repo {
            if let Err(e) = repo.create_pending(&info).await {
                error!("Failed to persist pending question {}: {}", request_id, e);
            }
        }

        let request = PendingQuestion { info, sender: tx };
        self.pending.lock().await.insert(request_id, request);
        rx
    }

    /// Resolve a pending question with an answer
    /// Returns true if the question was found and resolved
    pub async fn resolve(&self, request_id: &str, answer: QuestionAnswer) -> bool {
        let pending = self.pending.lock().await;
        if let Some(question) = pending.get(request_id) {
            let _ = question.sender.send(Some(answer.clone()));

            // Fire-and-forget persist to repo
            if let Some(repo) = &self.repo {
                if let Err(e) = repo.resolve(request_id, &answer).await {
                    error!("Failed to persist question resolution {}: {}", request_id, e);
                }
            }

            true
        } else {
            false
        }
    }

    /// Remove a pending question
    pub async fn remove(&self, request_id: &str) -> bool {
        let removed = self.pending.lock().await.remove(request_id).is_some();

        // Fire-and-forget persist to repo
        if removed {
            if let Some(repo) = &self.repo {
                if let Err(e) = repo.remove(request_id).await {
                    error!("Failed to persist question removal {}: {}", request_id, e);
                }
            }
        }

        removed
    }

    /// Expire all stale pending questions in the repository on startup.
    /// Call this once after constructing with `with_repo()` to clean up
    /// questions from agents that are no longer running.
    pub async fn expire_stale_on_startup(&self) {
        if let Some(repo) = &self.repo {
            match repo.expire_all_pending().await {
                Ok(count) if count > 0 => {
                    info!("Expired {} stale pending questions on startup", count);
                }
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to expire stale pending questions: {}", e);
                }
            }
        }
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

    // --- Tests with repo persistence ---

    mod with_repo {
        use super::*;
        use crate::domain::repositories::QuestionRepository;
        use crate::infrastructure::memory::MemoryQuestionRepository;
        use std::sync::Arc;

        fn make_state_with_repo() -> (QuestionState, Arc<MemoryQuestionRepository>) {
            let repo = Arc::new(MemoryQuestionRepository::new());
            let state = QuestionState::with_repo(repo.clone());
            (state, repo)
        }

        #[tokio::test]
        async fn test_with_repo_constructor() {
            let (state, _repo) = make_state_with_repo();
            assert!(state.repo.is_some());
            let pending = state.pending.lock().await;
            assert!(pending.is_empty());
        }

        #[tokio::test]
        async fn test_register_persists_to_repo() {
            let (state, repo) = make_state_with_repo();

            state
                .register(
                    "req-1".to_string(),
                    "session-1".to_string(),
                    "Which framework?".to_string(),
                    None,
                    vec![QuestionOption {
                        value: "react".to_string(),
                        label: "React".to_string(),
                        description: None,
                    }],
                    false,
                )
                .await;

            // Verify persisted in repo
            let repo_pending = repo.get_pending().await.unwrap();
            assert_eq!(repo_pending.len(), 1);
            assert_eq!(repo_pending[0].request_id, "req-1");
            assert_eq!(repo_pending[0].question, "Which framework?");
        }

        #[tokio::test]
        async fn test_resolve_persists_to_repo() {
            let (state, repo) = make_state_with_repo();

            state
                .register(
                    "req-1".to_string(),
                    "session-1".to_string(),
                    "Pick one".to_string(),
                    None,
                    vec![],
                    false,
                )
                .await;

            let answer = QuestionAnswer {
                selected_options: vec!["a".to_string()],
                text: None,
            };
            let resolved = state.resolve("req-1", answer).await;
            assert!(resolved);

            // After resolve, repo should have no pending
            let repo_pending = repo.get_pending().await.unwrap();
            assert!(repo_pending.is_empty());

            // But the record still exists
            let found = repo.get_by_request_id("req-1").await.unwrap();
            assert!(found.is_some());
        }

        #[tokio::test]
        async fn test_remove_persists_to_repo() {
            let (state, repo) = make_state_with_repo();

            state
                .register(
                    "req-rm".to_string(),
                    "session-1".to_string(),
                    "Remove me".to_string(),
                    None,
                    vec![],
                    false,
                )
                .await;

            let removed = state.remove("req-rm").await;
            assert!(removed);

            // Repo record should be gone
            let found = repo.get_by_request_id("req-rm").await.unwrap();
            assert!(found.is_none());
        }

        #[tokio::test]
        async fn test_expire_stale_on_startup() {
            let repo = Arc::new(MemoryQuestionRepository::new());

            // Seed repo with pending questions (simulating leftover from previous run)
            for i in 0..3 {
                let info = PendingQuestionInfo {
                    request_id: format!("stale-{}", i),
                    session_id: "old-session".to_string(),
                    question: format!("Stale question {}", i),
                    header: None,
                    options: vec![],
                    multi_select: false,
                };
                repo.create_pending(&info).await.unwrap();
            }

            assert_eq!(repo.get_pending().await.unwrap().len(), 3);

            let state = QuestionState::with_repo(repo.clone());
            state.expire_stale_on_startup().await;

            // All stale questions should be expired
            assert!(repo.get_pending().await.unwrap().is_empty());
        }

        #[tokio::test]
        async fn test_expire_stale_noop_without_repo() {
            let state = QuestionState::new();
            // Should not panic when no repo
            state.expire_stale_on_startup().await;
        }
    }
}
