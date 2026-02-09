// Question repository trait - domain layer abstraction for question persistence
//
// This trait defines the contract for persisting pending questions (from AskUserQuestion).
// SQLite stores data for restart resilience + audit trail; in-memory channels remain for signaling.
// Types imported from crate::application::question_state.

use async_trait::async_trait;

use crate::application::question_state::{PendingQuestionInfo, QuestionAnswer};
use crate::error::AppResult;

/// Repository trait for pending question persistence.
/// Implementations can use SQLite, in-memory, etc.
#[async_trait]
pub trait QuestionRepository: Send + Sync {
    /// Persist a new pending question
    async fn create_pending(&self, info: &PendingQuestionInfo) -> AppResult<()>;

    /// Mark a question as resolved with the given answer
    async fn resolve(
        &self,
        request_id: &str,
        answer: &QuestionAnswer,
    ) -> AppResult<bool>;

    /// Get all currently pending questions
    async fn get_pending(&self) -> AppResult<Vec<PendingQuestionInfo>>;

    /// Get a single question by its request_id
    async fn get_by_request_id(
        &self,
        request_id: &str,
    ) -> AppResult<Option<PendingQuestionInfo>>;

    /// Expire all pending questions (e.g., on startup — agents that asked are gone)
    async fn expire_all_pending(&self) -> AppResult<u64>;

    /// Remove a question record by request_id
    async fn remove(&self, request_id: &str) -> AppResult<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::question_state::QuestionOption;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockQuestionRepository {
        questions: RwLock<HashMap<String, (PendingQuestionInfo, Option<QuestionAnswer>)>>,
    }

    impl MockQuestionRepository {
        fn new() -> Self {
            Self {
                questions: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl QuestionRepository for MockQuestionRepository {
        async fn create_pending(&self, info: &PendingQuestionInfo) -> AppResult<()> {
            let mut questions = self.questions.write().unwrap();
            questions.insert(info.request_id.clone(), (info.clone(), None));
            Ok(())
        }

        async fn resolve(
            &self,
            request_id: &str,
            answer: &QuestionAnswer,
        ) -> AppResult<bool> {
            let mut questions = self.questions.write().unwrap();
            if let Some(entry) = questions.get_mut(request_id) {
                entry.1 = Some(answer.clone());
                Ok(true)
            } else {
                Ok(false)
            }
        }

        async fn get_pending(&self) -> AppResult<Vec<PendingQuestionInfo>> {
            let questions = self.questions.read().unwrap();
            Ok(questions
                .values()
                .filter(|(_, answer)| answer.is_none())
                .map(|(info, _)| info.clone())
                .collect())
        }

        async fn get_by_request_id(
            &self,
            request_id: &str,
        ) -> AppResult<Option<PendingQuestionInfo>> {
            let questions = self.questions.read().unwrap();
            Ok(questions.get(request_id).map(|(info, _)| info.clone()))
        }

        async fn expire_all_pending(&self) -> AppResult<u64> {
            let mut questions = self.questions.write().unwrap();
            let pending_ids: Vec<String> = questions
                .iter()
                .filter(|(_, (_, answer))| answer.is_none())
                .map(|(id, _)| id.clone())
                .collect();
            let count = pending_ids.len() as u64;
            for id in pending_ids {
                questions.remove(&id);
            }
            Ok(count)
        }

        async fn remove(&self, request_id: &str) -> AppResult<bool> {
            let mut questions = self.questions.write().unwrap();
            Ok(questions.remove(request_id).is_some())
        }
    }

    #[test]
    fn test_question_repository_trait_is_object_safe() {
        let repo: std::sync::Arc<dyn QuestionRepository> =
            std::sync::Arc::new(MockQuestionRepository::new());
        assert_eq!(std::sync::Arc::strong_count(&repo), 1);
    }

    #[tokio::test]
    async fn test_create_and_get_pending() {
        let repo = MockQuestionRepository::new();
        let info = PendingQuestionInfo {
            request_id: "req-1".to_string(),
            session_id: "session-1".to_string(),
            question: "Which approach?".to_string(),
            header: None,
            options: vec![QuestionOption {
                value: "a".to_string(),
                label: "Option A".to_string(),
                description: None,
            }],
            multi_select: false,
        };

        repo.create_pending(&info).await.unwrap();

        let pending = repo.get_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, "req-1");
    }

    #[tokio::test]
    async fn test_get_by_request_id() {
        let repo = MockQuestionRepository::new();
        let info = PendingQuestionInfo {
            request_id: "req-42".to_string(),
            session_id: "session-1".to_string(),
            question: "Pick one".to_string(),
            header: Some("Header".to_string()),
            options: vec![],
            multi_select: false,
        };

        repo.create_pending(&info).await.unwrap();

        let found = repo.get_by_request_id("req-42").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().question, "Pick one");

        let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resolve() {
        let repo = MockQuestionRepository::new();
        let info = PendingQuestionInfo {
            request_id: "req-1".to_string(),
            session_id: "session-1".to_string(),
            question: "Which?".to_string(),
            header: None,
            options: vec![],
            multi_select: false,
        };

        repo.create_pending(&info).await.unwrap();

        let answer = QuestionAnswer {
            selected_options: vec!["a".to_string()],
            text: None,
        };
        let resolved = repo.resolve("req-1", &answer).await.unwrap();
        assert!(resolved);

        // After resolving, it should no longer appear in get_pending
        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());

        // But get_by_request_id still returns it (record exists)
        let found = repo.get_by_request_id("req-1").await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let repo = MockQuestionRepository::new();
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: None,
        };
        let resolved = repo.resolve("nope", &answer).await.unwrap();
        assert!(!resolved);
    }

    #[tokio::test]
    async fn test_expire_all_pending() {
        let repo = MockQuestionRepository::new();

        for i in 0..3 {
            let info = PendingQuestionInfo {
                request_id: format!("req-{}", i),
                session_id: "session-1".to_string(),
                question: format!("Q{}", i),
                header: None,
                options: vec![],
                multi_select: false,
            };
            repo.create_pending(&info).await.unwrap();
        }

        // Resolve one so it's not pending
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: Some("done".to_string()),
        };
        repo.resolve("req-0", &answer).await.unwrap();

        // Expire remaining pending
        let expired = repo.expire_all_pending().await.unwrap();
        assert_eq!(expired, 2);

        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn test_remove() {
        let repo = MockQuestionRepository::new();
        let info = PendingQuestionInfo {
            request_id: "req-rm".to_string(),
            session_id: "session-1".to_string(),
            question: "Remove me".to_string(),
            header: None,
            options: vec![],
            multi_select: false,
        };

        repo.create_pending(&info).await.unwrap();
        let removed = repo.remove("req-rm").await.unwrap();
        assert!(removed);

        let found = repo.get_by_request_id("req-rm").await.unwrap();
        assert!(found.is_none());

        let removed_again = repo.remove("req-rm").await.unwrap();
        assert!(!removed_again);
    }
}
