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

    async fn resolve(&self, request_id: &str, answer: &QuestionAnswer) -> AppResult<bool> {
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

    async fn get_by_request_id(&self, request_id: &str) -> AppResult<Option<PendingQuestionInfo>> {
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
#[path = "memory_question_repo_tests.rs"]
mod tests;
