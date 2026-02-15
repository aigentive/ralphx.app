// Integration tests for SpawnOrchestratorJob functionality
//
// Tests:
// - Job creation and persistence
// - Job status lifecycle (pending -> running -> done/failed)
// - Atomic claim_pending operation
// - Retry logic (max attempts)

use ralphx_lib::domain::entities::{
    IdeationSessionId, Project, ProjectId, SpawnOrchestratorJob, SpawnOrchestratorJobId,
    SpawnOrchestratorJobStatus,
};
use ralphx_lib::domain::repositories::{ProjectRepository, SpawnOrchestratorJobRepository};
use ralphx_lib::infrastructure::sqlite::{
    open_connection, run_migrations, SqliteProjectRepository, SqliteSpawnOrchestratorJobRepository,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// Test Setup Helpers
// ============================================================================

struct TestContext {
    job_repo: Arc<dyn SpawnOrchestratorJobRepository>,
    project_repo: Arc<dyn ProjectRepository>,
    project_id: ProjectId,
}

impl TestContext {
    fn new() -> Self {
        let conn = open_connection(&PathBuf::from(":memory:")).expect("Failed to create in-memory DB");
        run_migrations(&conn).expect("Failed to run migrations");
        let shared_conn = Arc::new(Mutex::new(conn));

        let job_repo = Arc::new(SqliteSpawnOrchestratorJobRepository::from_shared(Arc::clone(&shared_conn)));
        let project_repo = Arc::new(SqliteProjectRepository::from_shared(Arc::clone(&shared_conn)));

        Self {
            job_repo,
            project_repo,
            project_id: ProjectId::new(),
        }
    }

    async fn setup(&self) {
        // Create a project to satisfy foreign key constraint
        let project = Project::new("Test Project".to_string(), "/test/path".to_string());
        // Override the ID with our predetermined one
        let project_with_id = Project {
            id: self.project_id.clone(),
            ..project
        };
        self.project_repo.create(project_with_id).await.expect("Failed to create project");
    }

    fn create_job(&self, description: &str) -> SpawnOrchestratorJob {
        SpawnOrchestratorJob::new(
            IdeationSessionId::from_string("test-session-id"),
            self.project_id.clone(),
            description.to_string(),
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[tokio::test]
async fn test_create_job() {
    let ctx = TestContext::new();
    ctx.setup().await;
    let job = ctx.create_job("Test description for orchestrator job");
    let job_id = job.id.clone();

    let created = ctx.job_repo.create(job).await.expect("Failed to create job");

    assert_eq!(created.id, job_id);
    assert_eq!(created.session_id.as_str(), "test-session-id");
    assert_eq!(created.project_id, ctx.project_id);
    assert_eq!(created.description, "Test description for orchestrator job");
    assert_eq!(created.status, SpawnOrchestratorJobStatus::Pending);
    assert!(created.error_message.is_none());
    assert!(created.started_at.is_none());
    assert!(created.completed_at.is_none());
    assert_eq!(created.attempt_count, 0);
}

#[tokio::test]
async fn test_get_by_id() {
    let ctx = TestContext::new();
    ctx.setup().await;
    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();

    ctx.job_repo.create(job).await.expect("Failed to create job");

    let retrieved = ctx.job_repo.get_by_id(&job_id).await.expect("Failed to get job");
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, job_id);

    // Non-existent job returns None
    let non_existent = ctx.job_repo
        .get_by_id(&SpawnOrchestratorJobId::from("non-existent".to_string()))
        .await
        .expect("Failed to query");
    assert!(non_existent.is_none());
}

#[tokio::test]
async fn test_get_pending() {
    let ctx = TestContext::new();
    ctx.setup().await;

    // Create multiple jobs
    let job1 = ctx.create_job("First job");
    let job2 = SpawnOrchestratorJob::new(
        IdeationSessionId::from_string("test-session-2"),
        ctx.project_id.clone(),
        "Second job".to_string(),
    );

    ctx.job_repo.create(job1).await.expect("Failed to create job1");
    ctx.job_repo.create(job2).await.expect("Failed to create job2");

    let pending = ctx.job_repo.get_pending().await.expect("Failed to get pending");
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn test_update_status_to_running() {
    let ctx = TestContext::new();
    ctx.setup().await;
    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();

    ctx.job_repo.create(job).await.expect("Failed to create job");

    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Running, None)
        .await
        .expect("Failed to update status");

    let updated = ctx.job_repo
        .get_by_id(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(updated.status, SpawnOrchestratorJobStatus::Running);
    assert!(updated.started_at.is_some());
    assert!(updated.completed_at.is_none());
}

#[tokio::test]
async fn test_update_status_to_done() {
    let ctx = TestContext::new();
    ctx.setup().await;
    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();

    ctx.job_repo.create(job).await.expect("Failed to create job");

    // First set to running
    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Running, None)
        .await
        .expect("Failed to update to running");

    // Then set to done
    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Done, None)
        .await
        .expect("Failed to update to done");

    let updated = ctx.job_repo
        .get_by_id(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(updated.status, SpawnOrchestratorJobStatus::Done);
    assert!(updated.completed_at.is_some());
}

#[tokio::test]
async fn test_update_status_to_failed() {
    let ctx = TestContext::new();
    ctx.setup().await;
    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();

    ctx.job_repo.create(job).await.expect("Failed to create job");

    // First set to running
    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Running, None)
        .await
        .expect("Failed to update to running");

    // Then set to failed with error message
    ctx.job_repo.update_status(
        &job_id,
        SpawnOrchestratorJobStatus::Failed,
        Some("Test error message".to_string()),
    )
    .await
    .expect("Failed to update to failed");

    let updated = ctx.job_repo
        .get_by_id(&job_id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(updated.status, SpawnOrchestratorJobStatus::Failed);
    assert!(updated.completed_at.is_some());
    assert_eq!(updated.error_message, Some("Test error message".to_string()));
}

#[tokio::test]
async fn test_claim_pending_returns_oldest_first() {
    let ctx = TestContext::new();
    ctx.setup().await;

    // Create two jobs with a small delay to ensure different timestamps
    let job1 = ctx.create_job("First job");
    let job1_id = job1.id.clone();
    ctx.job_repo.create(job1).await.expect("Failed to create job1");

    // Small delay to ensure different created_at
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let job2 = SpawnOrchestratorJob::new(
        IdeationSessionId::from_string("test-session-2"),
        ctx.project_id.clone(),
        "Second job".to_string(),
    );
    let job2_id = job2.id.clone();
    ctx.job_repo.create(job2).await.expect("Failed to create job2");

    // Claim should return job1 (oldest)
    let claimed1 = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim pending");
    assert!(claimed1.is_some());
    let claimed1 = claimed1.unwrap();
    assert_eq!(claimed1.id, job1_id);
    assert_eq!(claimed1.status, SpawnOrchestratorJobStatus::Running);

    // Claim should return job2
    let claimed2 = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim pending");
    assert!(claimed2.is_some());
    let claimed2 = claimed2.unwrap();
    assert_eq!(claimed2.id, job2_id);
}

#[tokio::test]
async fn test_claim_pending_returns_none_when_empty() {
    let ctx = TestContext::new();
    ctx.setup().await;

    let claimed = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim pending");
    assert!(claimed.is_none());
}

#[tokio::test]
async fn test_claim_pending_increments_attempt_count() {
    let ctx = TestContext::new();
    ctx.setup().await;

    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();
    ctx.job_repo.create(job).await.expect("Failed to create job");

    // First claim
    let claimed = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim")
        .expect("Job not found");
    assert_eq!(claimed.attempt_count, 1);

    // Mark as failed
    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Failed, Some("Error".to_string()))
        .await
        .expect("Failed to update");

    // Second claim (retry)
    let claimed = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim")
        .expect("Job not found");
    assert_eq!(claimed.attempt_count, 2);
}

#[tokio::test]
async fn test_get_pending_excludes_non_pending() {
    let ctx = TestContext::new();
    ctx.setup().await;

    // Create and claim job (status = running)
    let job1 = ctx.create_job("First job");
    let job1_id = job1.id.clone();
    ctx.job_repo.create(job1).await.expect("Failed to create job1");
    ctx.job_repo.claim_pending()
        .await
        .expect("Failed to claim")
        .expect("Job not found");

    // Create another job
    let job2 = SpawnOrchestratorJob::new(
        IdeationSessionId::from_string("test-session-2"),
        ctx.project_id.clone(),
        "Second job".to_string(),
    );
    ctx.job_repo.create(job2).await.expect("Failed to create job2");

    // Only job2 should be pending
    let pending = ctx.job_repo.get_pending().await.expect("Failed to get pending");
    assert_eq!(pending.len(), 1);
    assert_ne!(pending[0].id, job1_id);
}

#[tokio::test]
async fn test_job_lifecycle_complete() {
    let ctx = TestContext::new();
    ctx.setup().await;

    // Create job
    let job = ctx.create_job("Test job");
    let job_id = job.id.clone();
    ctx.job_repo.create(job).await.expect("Failed to create job");

    // Verify initial state
    let initial = ctx.job_repo
        .get_by_id(&job_id)
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(initial.status, SpawnOrchestratorJobStatus::Pending);
    assert!(initial.can_claim());

    // Claim job
    let claimed = ctx.job_repo
        .claim_pending()
        .await
        .expect("Failed to claim")
        .expect("Not found");
    assert_eq!(claimed.status, SpawnOrchestratorJobStatus::Running);
    assert!(!claimed.can_claim());
    assert!(claimed.started_at.is_some());

    // Complete job
    ctx.job_repo.update_status(&job_id, SpawnOrchestratorJobStatus::Done, None)
        .await
        .expect("Failed to complete");

    let completed = ctx.job_repo
        .get_by_id(&job_id)
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(completed.status, SpawnOrchestratorJobStatus::Done);
    assert!(!completed.can_claim());
    assert!(completed.completed_at.is_some());
    assert!(completed.error_message.is_none());
}
