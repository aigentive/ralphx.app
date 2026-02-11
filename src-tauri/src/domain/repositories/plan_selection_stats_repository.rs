// PlanSelectionStatsRepository trait - domain layer abstraction for plan selection tracking
//
// This trait defines the contract for plan selection stats persistence.
// Implementations can use SQLite, in-memory storage, etc.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::entities::{IdeationSessionId, PlanSelectionStats, ProjectId, SelectionSource};
use crate::error::AppResult;

/// Repository trait for PlanSelectionStats persistence.
/// Provides operations for tracking plan selection interactions.
#[async_trait]
pub trait PlanSelectionStatsRepository: Send + Sync {
    // ═══════════════════════════════════════════════════════════════════════
    // Core Operations
    // ═══════════════════════════════════════════════════════════════════════

    /// Record a plan selection event.
    /// Uses UPSERT semantics: increments count if exists, creates new entry if not.
    async fn record_selection(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
        source: SelectionSource,
        timestamp: DateTime<Utc>,
    ) -> AppResult<()>;

    /// Get stats for a single session
    async fn get_stats(
        &self,
        project_id: &ProjectId,
        session_id: &IdeationSessionId,
    ) -> AppResult<Option<PlanSelectionStats>>;

    /// Get stats for multiple sessions in a single query (for ranking)
    /// Returns a Vec of stats in the same order as session_ids (None for missing entries)
    async fn get_stats_batch(
        &self,
        project_id: &ProjectId,
        session_ids: &[IdeationSessionId],
    ) -> AppResult<Vec<Option<PlanSelectionStats>>>;
}
