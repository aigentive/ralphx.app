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
    async fn resolve(&self, request_id: &str, answer: &QuestionAnswer) -> AppResult<bool>;

    /// Get all currently pending questions
    async fn get_pending(&self) -> AppResult<Vec<PendingQuestionInfo>>;

    /// Get a single question by its request_id
    async fn get_by_request_id(&self, request_id: &str) -> AppResult<Option<PendingQuestionInfo>>;

    /// Expire all pending questions (e.g., on startup — agents that asked are gone)
    async fn expire_all_pending(&self) -> AppResult<u64>;

    /// Remove a question record by request_id
    async fn remove(&self, request_id: &str) -> AppResult<bool>;
}

#[cfg(test)]
#[path = "question_repository_tests.rs"]
mod tests;
