use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::application::question_state::{PendingQuestionInfo, QuestionAnswer};
use crate::domain::repositories::QuestionRepository;
use crate::error::AppResult;

pub struct MemoryQuestionRepository {
    questions: RwLock<HashMap<String, (PendingQuestionInfo, Option<QuestionAnswer>)>>,
}

impl MemoryQuestionRepository {
    pub fn new() -> Self {
        Self {
            questions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryQuestionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QuestionRepository for MemoryQuestionRepository {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::question_state::QuestionOption;

    fn sample_question(request_id: &str) -> PendingQuestionInfo {
        PendingQuestionInfo {
            request_id: request_id.to_string(),
            session_id: "session-1".to_string(),
            question: "Which approach?".to_string(),
            header: None,
            options: vec![QuestionOption {
                value: "a".to_string(),
                label: "Option A".to_string(),
                description: None,
            }],
            multi_select: false,
        }
    }

    #[tokio::test]
    async fn test_create_and_get_pending() {
        let repo = MemoryQuestionRepository::new();
        repo.create_pending(&sample_question("req-1")).await.unwrap();

        let pending = repo.get_pending().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].request_id, "req-1");
    }

    #[tokio::test]
    async fn test_get_by_request_id() {
        let repo = MemoryQuestionRepository::new();
        repo.create_pending(&sample_question("req-42")).await.unwrap();

        let found = repo.get_by_request_id("req-42").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().question, "Which approach?");

        let not_found = repo.get_by_request_id("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_resolve() {
        let repo = MemoryQuestionRepository::new();
        repo.create_pending(&sample_question("req-1")).await.unwrap();

        let answer = QuestionAnswer {
            selected_options: vec!["a".to_string()],
            text: None,
        };
        assert!(repo.resolve("req-1", &answer).await.unwrap());

        let pending = repo.get_pending().await.unwrap();
        assert!(pending.is_empty());

        // Record still exists
        assert!(repo.get_by_request_id("req-1").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_resolve_nonexistent() {
        let repo = MemoryQuestionRepository::new();
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: None,
        };
        assert!(!repo.resolve("nope", &answer).await.unwrap());
    }

    #[tokio::test]
    async fn test_expire_all_pending() {
        let repo = MemoryQuestionRepository::new();
        for i in 0..3 {
            repo.create_pending(&sample_question(&format!("req-{i}")))
                .await
                .unwrap();
        }

        // Resolve one
        let answer = QuestionAnswer {
            selected_options: vec![],
            text: Some("done".to_string()),
        };
        repo.resolve("req-0", &answer).await.unwrap();

        let expired = repo.expire_all_pending().await.unwrap();
        assert_eq!(expired, 2);
        assert!(repo.get_pending().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_remove() {
        let repo = MemoryQuestionRepository::new();
        repo.create_pending(&sample_question("req-rm")).await.unwrap();

        assert!(repo.remove("req-rm").await.unwrap());
        assert!(repo.get_by_request_id("req-rm").await.unwrap().is_none());
        assert!(!repo.remove("req-rm").await.unwrap());
    }

    #[test]
    fn test_default_impl() {
        let repo = MemoryQuestionRepository::default();
        let questions = repo.questions.read().unwrap();
        assert!(questions.is_empty());
    }
}
