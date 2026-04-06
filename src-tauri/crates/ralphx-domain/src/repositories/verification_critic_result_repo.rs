use async_trait::async_trait;

use crate::entities::verification_critic_result::{CriticKind, VerificationCriticResult};
use crate::error::AppResult;

/// Input for submitting a critic result. The `verification_generation` is resolved
/// server-side from the `IdeationSession` and passed in by the HTTP handler.
pub struct SubmitCriticResultInput {
    pub parent_session_id: String,
    pub verification_session_id: String,
    pub verification_generation: i32,
    pub round: i32,
    pub critic_kind: CriticKind,
    pub title: String,
    pub content: String,
    /// Artifact type string (e.g. "TeamResearch"). Defaults to "TeamResearch" if None.
    pub artifact_type: Option<String>,
}

/// Stable identities returned after a successful submit.
#[derive(Debug)]
pub struct SubmitCriticResultOutput {
    pub artifact_id: String,
    pub result_id: String,
}

/// Repository for verification critic results.
///
/// # Errors
///
/// `submit` returns `AppError::Conflict` when a duplicate (parent_session_id,
/// verification_generation, round, critic_kind) is detected.
#[async_trait]
pub trait VerificationCriticResultRepo: Send + Sync {
    /// Atomically create an artifact row and a verification_critic_results row.
    async fn submit(
        &self,
        input: SubmitCriticResultInput,
    ) -> AppResult<SubmitCriticResultOutput>;

    /// Return all results for a (parent_session_id, generation, round) triple.
    async fn get_round_results(
        &self,
        parent_session_id: &str,
        generation: i32,
        round: i32,
    ) -> AppResult<Vec<VerificationCriticResult>>;
}
