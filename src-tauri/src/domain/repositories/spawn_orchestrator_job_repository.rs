// Spawn orchestrator job repository trait
//
// Repository trait for SpawnOrchestratorJob persistence.
// Implementations live in the infrastructure layer (sqlite/memeory).

use async_trait::async_trait;

use crate::domain::entities::{
    SpawnOrchestratorJob, SpawnOrchestratorJobId, SpawnOrchestratorJobStatus,
};
use crate::error::AppResult;

/// Repository trait for SpawnOrchestratorJob persistence
#[async_trait]
pub trait SpawnOrchestratorJobRepository: Send + Sync {
    /// Create a new spawn orchestrator job
    async fn create(&self, job: SpawnOrchestratorJob) -> AppResult<SpawnOrchestratorJob>;

    /// Get job by ID
    async fn get_by_id(
        &self,
        id: &SpawnOrchestratorJobId,
    ) -> AppResult<Option<SpawnOrchestratorJob>>;

    /// Get all pending jobs
    async fn get_pending(&self) -> AppResult<Vec<SpawnOrchestratorJob>>;

    /// Update job status with optional error message
    async fn update_status(
        &self,
        id: &SpawnOrchestratorJobId,
        status: SpawnOrchestratorJobStatus,
        error_message: Option<String>,
    ) -> AppResult<()>;

    /// Atomically claim a pending job for processing
    ///
    /// This method finds the oldest pending job and marks it as running
    /// in a single atomic operation, preventing race conditions when
    /// multiple workers try to claim jobs simultaneously.
    async fn claim_pending(&self) -> AppResult<Option<SpawnOrchestratorJob>>;
}
